use rquickjs::{Ctx, Error, Function, Object, Result, Value, atom::PredefinedAtom, qjs};

const MAX_FIELDS: u32 = 32;
const MAX_CELLS: u32 = 1_048_576;
const WARM_ROWS: u32 = 128;

pub(crate) fn install(ctx: &Ctx<'_>) -> Result<()> {
    // Keep the roots in the JS heap: a Rust closure capturing them would also
    // retain its context, outside QuickJS's cycle collector.
    let intrinsics: Object = ctx.eval(
        r#"(() => {
        const a = Array.prototype, s = String.prototype;
        const uncurry = fn => Function.prototype.call.bind(fn);
        return Object.freeze({
            arrayPrototype: a, objectPrototype: Object.prototype,
            arrayConstructor: Array, stringPrototype: s, dateConstructor: Date,
            filter: a.filter, sort: a.sort, map: a.map, join: a.join,
            lower: s.toLowerCase, includes: s.includes, localeCompare: s.localeCompare,
            dateParse: Date.parse,
            speciesGet: Object.getOwnPropertyDescriptor(Array, Symbol.species).get,
            apply: Reflect.apply, jsonParse: JSON.parse,
            WeakMap, weakMapGet: WeakMap.prototype.get, weakMapSet: WeakMap.prototype.set,
            Map, mapGet: Map.prototype.get, mapSet: Map.prototype.set,
            setTimeout: typeof setTimeout === "function" ? setTimeout : undefined,
            clearTimeout: typeof clearTimeout === "function" ? clearTimeout : undefined,
            calls: Object.freeze({
                filter: uncurry(a.filter), sort: uncurry(a.sort), map: uncurry(a.map),
                join: uncurry(a.join), lower: uncurry(s.toLowerCase),
                includes: uncurry(s.includes), localeCompare: uncurry(s.localeCompare),
                dateParse: Date.parse.bind(Date), jsonParse: JSON.parse.bind(undefined),
                weakMapGet: uncurry(WeakMap.prototype.get), weakMapSet: uncurry(WeakMap.prototype.set),
                mapGet: uncurry(Map.prototype.get), mapSet: uncurry(Map.prototype.set)
            })
        });
    })()"#,
    )?;
    ctx.globals().prop("__inkDataIntrinsics", intrinsics)?;
    ctx.globals()
        .prop("__inkDataFields", Function::new(ctx.clone(), data_fields)?)?;
    ctx.globals()
        .prop("__inkDataSearch", Function::new(ctx.clone(), data_search)?)?;
    ctx.globals().prop(
        "__inkPrepareDataSearch",
        Function::new(ctx.clone(), prepare_data_search)?,
    )
}

fn data_fields<'js>(
    ctx: Ctx<'js>,
    items: Value<'js>,
    fields: Value<'js>,
    previous: Value<'js>,
    mode: Value<'js>,
) -> Result<Value<'js>> {
    let Some(mode) = mode.as_string() else {
        return Ok(Value::new_undefined(ctx));
    };
    let sort = match mode.to_string()?.as_str() {
        "search" => false,
        "sort" => true,
        _ => return Ok(Value::new_undefined(ctx)),
    };
    let guard = Guard::new(ctx.clone())?;
    Ok(guard
        .snapshot(items.as_raw(), fields.as_raw(), &previous, sort, None, None)?
        .map(|snapshot| snapshot.fields)
        .unwrap_or_else(|| Value::new_undefined(ctx)))
}

fn data_search<'js>(
    ctx: Ctx<'js>,
    items: Value<'js>,
    fields: Value<'js>,
    previous: Value<'js>,
    previous_text: Value<'js>,
    separator: Value<'js>,
) -> Result<Value<'js>> {
    let guard = Guard::new(ctx.clone())?;
    let Some(search) = SearchPreparation::new(&guard, previous_text, separator)? else {
        return Ok(Value::new_undefined(ctx));
    };
    let Some(snapshot) = guard.snapshot(
        items.as_raw(),
        fields.as_raw(),
        &previous,
        false,
        Some(search),
        None,
    )?
    else {
        return Ok(Value::new_undefined(ctx));
    };
    let result = Object::new(ctx)?;
    // Define own properties, even when Object.prototype contains setters.
    result.prop("snapshot", snapshot.fields)?;
    result.prop("text", snapshot.text.unwrap())?;
    Ok(result.into_value())
}

fn prepare_data_search<'js>(
    ctx: Ctx<'js>,
    items: Value<'js>,
    fields: Value<'js>,
    separator: Value<'js>,
    cursor: rquickjs::function::Opt<Value<'js>>,
    limit: rquickjs::function::Opt<Value<'js>>,
) -> Result<Value<'js>> {
    let guard = Guard::new(ctx.clone())?;
    let absent = Value::new_undefined(ctx.clone());
    let Some(search) = SearchPreparation::new(&guard, absent.clone(), separator)? else {
        return Ok(absent);
    };
    let limit = match limit.0 {
        Some(value) if !value.is_undefined() => match guard.counter(value.as_raw()) {
            Some(value) if value > 0 => value.min(WARM_ROWS),
            _ => return Ok(absent),
        },
        _ => WARM_ROWS,
    };
    let range = match cursor.0 {
        Some(cursor) if !cursor.is_undefined() => {
            let raw = cursor.as_raw();
            if unsafe {
                qjs::JS_IsProxy(raw)
                    || qjs::JS_GetClassID(raw) != qjs::JS_GetClassID(guard.object.as_raw())
            } {
                return Ok(absent);
            }
            if !guard.same(guard.prototype(raw)?.as_raw(), guard.object.as_raw()) {
                return Ok(absent);
            }
            let Some(offset) = guard.named(raw, "offset")? else {
                return Ok(absent);
            };
            let Some(offset) = offset
                .is_data()
                .then(|| guard.counter(offset.raw.value))
                .flatten()
            else {
                return Ok(absent);
            };
            let Some(snapshot) = guard.named(raw, "snapshot")? else {
                return Ok(absent);
            };
            let Some(text) = guard.named(raw, "text")? else {
                return Ok(absent);
            };
            if !snapshot.is_data() || !text.is_data() {
                return Ok(absent);
            }
            WarmRange {
                offset,
                limit,
                fields: unsafe {
                    Value::from_raw(
                        ctx.clone(),
                        qjs::JS_DupValue(guard.raw_ctx(), snapshot.raw.value),
                    )
                },
                text: unsafe {
                    Value::from_raw(
                        ctx.clone(),
                        qjs::JS_DupValue(guard.raw_ctx(), text.raw.value),
                    )
                },
            }
        }
        _ => WarmRange {
            offset: 0,
            limit,
            fields: guard.new_snapshot()?,
            text: guard.new_snapshot()?,
        },
    };
    if guard
        .array_length(items.as_raw())?
        .is_some_and(|count| count < range.offset)
    {
        // A source which shrank during warming may be retried on a later search.
        return Ok(Value::new_null(ctx));
    }
    let Some(snapshot) = guard.snapshot(
        items.as_raw(),
        fields.as_raw(),
        &absent,
        false,
        Some(search),
        Some(range),
    )?
    else {
        return Ok(absent);
    };
    let result = Object::new(ctx)?;
    result.prop("snapshot", snapshot.fields)?;
    result.prop("text", snapshot.text.unwrap())?;
    result.prop("offset", snapshot.offset)?;
    result.prop("complete", snapshot.complete)?;
    Ok(result.into_value())
}

struct Snapshot<'js> {
    fields: Value<'js>,
    text: Option<Value<'js>>,
    offset: u32,
    complete: bool,
}

struct WarmRange<'js> {
    offset: u32,
    limit: u32,
    fields: Value<'js>,
    text: Value<'js>,
}

struct SearchPreparation<'js> {
    previous: Value<'js>,
    previous_length: Option<u32>,
    separator: Value<'js>,
    join: Value<'js>,
    lower: Value<'js>,
    scratch: Option<Value<'js>>,
    output: Option<Value<'js>>,
}

impl<'js> SearchPreparation<'js> {
    fn new(
        guard: &Guard<'js>,
        previous: Value<'js>,
        separator: Value<'js>,
    ) -> Result<Option<Self>> {
        if !separator.is_null() && !separator.is_string() {
            return Ok(None);
        }
        let previous_length = if previous.is_undefined() {
            None
        } else {
            let Some(length) = guard.array_length(previous.as_raw())? else {
                return Ok(None);
            };
            if length > MAX_CELLS {
                return Ok(None);
            }
            Some(length)
        };
        Ok(Some(Self {
            previous,
            previous_length,
            separator,
            join: guard.intrinsics.get("join")?,
            lower: guard.intrinsics.get("lower")?,
            scratch: None,
            output: None,
        }))
    }
}

struct Guard<'js> {
    ctx: Ctx<'js>,
    intrinsics: Object<'js>,
    array: Value<'js>,
    object: Value<'js>,
    string: Value<'js>,
    constructor: Value<'js>,
}

impl<'js> Guard<'js> {
    fn new(ctx: Ctx<'js>) -> Result<Self> {
        let intrinsics: Object = ctx.globals().get("__inkDataIntrinsics")?;
        Ok(Self {
            array: intrinsics.get("arrayPrototype")?,
            object: intrinsics.get("objectPrototype")?,
            string: intrinsics.get("stringPrototype")?,
            constructor: intrinsics.get("arrayConstructor")?,
            intrinsics,
            ctx,
        })
    }

    fn raw_ctx(&self) -> *mut qjs::JSContext {
        self.ctx.as_raw().as_ptr()
    }

    fn same(&self, a: qjs::JSValue, b: qjs::JSValue) -> bool {
        unsafe { qjs::JS_IsSameValue(self.raw_ctx(), a, b) }
    }

    fn counter(&self, value: qjs::JSValue) -> Option<u32> {
        if !unsafe { qjs::JS_IsNumber(value) } {
            return None;
        }
        let mut number = 0.0;
        if unsafe { qjs::JS_ToFloat64(self.raw_ctx(), &mut number, value) } < 0 {
            return None;
        }
        (number >= 0.0 && number <= MAX_CELLS as f64 && number.fract() == 0.0)
            .then_some(number as u32)
    }

    fn prototype(&self, object: qjs::JSValue) -> Result<Value<'js>> {
        let value = unsafe { qjs::JS_GetPrototype(self.raw_ctx(), object) };
        if unsafe { qjs::JS_IsException(value) } {
            return Err(Error::Exception);
        }
        Ok(unsafe { Value::from_raw(self.ctx.clone(), value) })
    }

    fn own(&self, object: qjs::JSValue, atom: qjs::JSAtom) -> Result<Option<Descriptor<'_, 'js>>> {
        let mut descriptor = Descriptor {
            ctx: &self.ctx,
            raw: qjs::JSPropertyDescriptor {
                flags: 0,
                value: qjs::JS_UNDEFINED,
                getter: qjs::JS_UNDEFINED,
                setter: qjs::JS_UNDEFINED,
            },
        };
        let found =
            unsafe { qjs::JS_GetOwnProperty(self.raw_ctx(), &mut descriptor.raw, object, atom) };
        match found {
            -1 => Err(Error::Exception),
            0 => Ok(None),
            _ => Ok(Some(descriptor)),
        }
    }

    fn named(&self, object: qjs::JSValue, name: &str) -> Result<Option<Descriptor<'_, 'js>>> {
        let atom =
            unsafe { qjs::JS_NewAtomLen(self.raw_ctx(), name.as_ptr().cast(), name.len() as _) };
        let atom = Atom::new(&self.ctx, atom)?;
        self.own(object, atom.raw)
    }

    fn index(&self, object: qjs::JSValue, index: u32) -> Result<Option<Descriptor<'_, 'js>>> {
        let atom = Atom::new(&self.ctx, unsafe {
            qjs::JS_NewAtomUInt32(self.raw_ctx(), index)
        })?;
        self.own(object, atom.raw)
    }

    fn original(&self, object: qjs::JSValue, name: &str, intrinsic: &str) -> Result<bool> {
        let expected: Value = self.intrinsics.get(intrinsic)?;
        Ok(self
            .named(object, name)?
            .is_some_and(|d| d.is_data() && self.same(d.raw.value, expected.as_raw())))
    }

    fn inherited_original(&self, items: qjs::JSValue, name: &str, intrinsic: &str) -> Result<bool> {
        if let Some(d) = self.named(items, name)? {
            let expected: Value = self.intrinsics.get(intrinsic)?;
            return Ok(d.is_data() && self.same(d.raw.value, expected.as_raw()));
        }
        self.original(self.array.as_raw(), name, intrinsic)
    }

    fn array_length(&self, value: qjs::JSValue) -> Result<Option<u32>> {
        if unsafe { qjs::JS_IsProxy(value) || !qjs::JS_IsArray(value) } {
            return Ok(None);
        }
        if !self.same(self.prototype(value)?.as_raw(), self.array.as_raw()) {
            return Ok(None);
        }
        let Some(length) = self.own(value, PredefinedAtom::Length as _)? else {
            return Ok(None);
        };
        if !length.is_data() || !unsafe { qjs::JS_IsNumber(length.raw.value) } {
            return Ok(None);
        }
        let mut count = 0;
        if unsafe { qjs::JS_ToIndex(self.raw_ctx(), &mut count, length.raw.value) } < 0 {
            return Err(Error::Exception);
        }
        Ok(u32::try_from(count).ok())
    }

    fn methods(&self, items: qjs::JSValue, sort: bool) -> Result<bool> {
        if !self.same(
            self.prototype(self.array.as_raw())?.as_raw(),
            self.object.as_raw(),
        ) || !self.prototype(self.object.as_raw())?.is_null()
            || !self.same(
                self.prototype(self.string.as_raw())?.as_raw(),
                self.object.as_raw(),
            )
            || !self.inherited_original(
                items,
                if sort { "sort" } else { "filter" },
                if sort { "sort" } else { "filter" },
            )?
            || !self.inherited_original(items, "constructor", "arrayConstructor")?
            || !self.original(self.array.as_raw(), "constructor", "arrayConstructor")?
        {
            return Ok(false);
        }
        let species = self.own(
            self.constructor.as_raw(),
            PredefinedAtom::SymbolSpecies as _,
        )?;
        let expected: Value = self.intrinsics.get("speciesGet")?;
        if !species.is_some_and(|d| {
            !d.is_data()
                && self.same(d.raw.getter, expected.as_raw())
                && unsafe { qjs::JS_IsUndefined(d.raw.setter) }
        }) {
            return Ok(false);
        }

        if sort {
            let date: Value = self.intrinsics.get("dateConstructor")?;
            Ok(
                self.original(self.ctx.globals().as_raw(), "Date", "dateConstructor")?
                    && self.original(date.as_raw(), "parse", "dateParse")?
                    && self.original(self.string.as_raw(), "localeCompare", "localeCompare")?,
            )
        } else {
            Ok(self.original(self.array.as_raw(), "join", "join")?
                && self.original(self.string.as_raw(), "toLowerCase", "lower")?
                && self.original(self.string.as_raw(), "includes", "includes")?)
        }
    }

    fn new_snapshot(&self) -> Result<Value<'js>> {
        let value = unsafe { qjs::JS_NewArray(self.raw_ctx()) };
        if unsafe { qjs::JS_IsException(value) } {
            return Err(Error::Exception);
        }
        Ok(unsafe { Value::from_raw(self.ctx.clone(), value) })
    }

    fn append(&self, output: &Value<'js>, index: u32, value: qjs::JSValue) -> Result<()> {
        // Define an own property; Set would invoke poisoned numeric prototypes.
        let result = unsafe {
            qjs::JS_DefinePropertyValueUint32(
                self.raw_ctx(),
                output.as_raw(),
                index,
                qjs::JS_DupValue(self.raw_ctx(), value),
                (qjs::JS_PROP_C_W_E | qjs::JS_PROP_THROW) as _,
            )
        };
        if result < 0 {
            return Err(Error::Exception);
        }
        Ok(())
    }

    fn freeze(&self, value: &Value<'js>) -> Result<()> {
        if unsafe { qjs::JS_FreezeObject(self.raw_ctx(), value.as_raw()) } < 0 {
            return Err(Error::Exception);
        }
        Ok(())
    }

    fn call(
        &self,
        function: &Value<'js>,
        receiver: qjs::JSValue,
        arguments: &mut [qjs::JSValue],
    ) -> Result<Value<'js>> {
        let value = unsafe {
            qjs::JS_Call(
                self.raw_ctx(),
                function.as_raw(),
                receiver,
                arguments.len() as _,
                arguments.as_mut_ptr(),
            )
        };
        if unsafe { qjs::JS_IsException(value) } {
            return Err(Error::Exception);
        }
        Ok(unsafe { Value::from_raw(self.ctx.clone(), value) })
    }

    fn snapshot(
        &self,
        items: qjs::JSValue,
        fields: qjs::JSValue,
        previous: &Value<'js>,
        sort: bool,
        mut search: Option<SearchPreparation<'js>>,
        range: Option<WarmRange<'js>>,
    ) -> Result<Option<Snapshot<'js>>> {
        let Some(count) = self.array_length(items)? else {
            return Ok(None);
        };
        let Some(width) = self.array_length(fields)? else {
            return Ok(None);
        };
        if width == 0
            || width > MAX_FIELDS
            || count > MAX_CELLS / width
            || !self.methods(items, sort)?
        {
            return Ok(None);
        }
        let mut atoms = Vec::with_capacity(width as usize);
        for index in 0..width {
            let Some(field) = self.index(fields, index)? else {
                return Ok(None);
            };
            if !field.is_data() || !unsafe { qjs::JS_IsString(field.raw.value) } {
                return Ok(None);
            }
            atoms.push(Atom::new(&self.ctx, unsafe {
                qjs::JS_ValueToAtom(self.raw_ctx(), field.raw.value)
            })?);
        }

        let cells = count * width;
        let previous_length = if previous.is_undefined() {
            None
        } else {
            let Some(length) = self.array_length(previous.as_raw())? else {
                return Ok(None);
            };
            if length > MAX_CELLS {
                return Ok(None);
            }
            Some(length)
        };
        let warming = range.is_some();
        if let Some(search) = &mut search {
            if search.separator.is_null() && width != 1
                || previous_length.map(u64::from)
                    != search
                        .previous_length
                        .map(|length| u64::from(length) * u64::from(width))
            {
                return Ok(None);
            }
            if !warming && search.previous_length != Some(count) {
                search.output = Some(self.new_snapshot()?);
            }
        }
        let mut output = if warming || previous_length == Some(cells) {
            None
        } else {
            Some(self.new_snapshot()?)
        };
        let (start, end) = if let Some(range) = range {
            // Only inspect array headers here. Prefix cells are revalidated by
            // the complete search guard before any cached text is used.
            if range.offset > count
                || self.array_length(range.fields.as_raw())? != Some(range.offset * width)
                || self.array_length(range.text.as_raw())? != Some(range.offset)
            {
                return Ok(None);
            }
            output = Some(range.fields);
            search.as_mut().unwrap().output = Some(range.text);
            (range.offset, count.min(range.offset + range.limit))
        } else {
            (0, count)
        };
        let mut cell = start * width;
        for index in start..end {
            let old_text = if let Some(search) = &search {
                if index < search.previous_length.unwrap_or(0) {
                    let Some(text) = self.index(search.previous.as_raw(), index)? else {
                        return Ok(None);
                    };
                    if !text.is_data() || !unsafe { qjs::JS_IsString(text.raw.value) } {
                        return Ok(None);
                    }
                    Some(text)
                } else {
                    None
                }
            } else {
                None
            };
            let mut row_changed = old_text.is_none();
            let Some(row) = self.index(items, index)? else {
                return Ok(None);
            };
            if !row.is_data() || (sort && row.raw.flags & qjs::JS_PROP_WRITABLE as i32 == 0) {
                return Ok(None);
            }
            let value = row.raw.value;
            // Derive the ordinary-object class ID through the public API.
            if unsafe {
                qjs::JS_IsProxy(value)
                    || qjs::JS_GetClassID(value) != qjs::JS_GetClassID(self.object.as_raw())
            } {
                return Ok(None);
            }
            let prototype = self.prototype(value)?;
            if !prototype.is_null() && !self.same(prototype.as_raw(), self.object.as_raw()) {
                return Ok(None);
            }
            for (column, atom) in atoms.iter().enumerate() {
                let field = self.own(value, atom.raw)?;
                let value = match &field {
                    Some(d) if d.is_data() && is_primitive(d.raw.value) => d.raw.value,
                    Some(_) => return Ok(None),
                    None => {
                        if !prototype.is_null()
                            && self.own(self.object.as_raw(), atom.raw)?.is_some()
                        {
                            return Ok(None);
                        }
                        qjs::JS_UNDEFINED
                    }
                };
                let same = if cell < previous_length.unwrap_or(0) {
                    let Some(old) = self.index(previous.as_raw(), cell)? else {
                        return Ok(None);
                    };
                    if !old.is_data() || !is_primitive(old.raw.value) {
                        return Ok(None);
                    }
                    self.same(value, old.raw.value)
                } else {
                    false
                };
                if output.is_none() && !same {
                    let changed = self.new_snapshot()?;
                    // The prefix has been validated; only pure builtin calls
                    // on primitive scratch values can run during this pass.
                    for prefix in 0..cell {
                        let old = self.index(previous.as_raw(), prefix)?.unwrap();
                        self.append(&changed, prefix, old.raw.value)?;
                    }
                    output = Some(changed);
                }
                if let Some(output) = &output {
                    self.append(output, cell, value)?;
                }
                if let Some(search) = &mut search {
                    if search.separator.is_null() && !unsafe { qjs::JS_IsString(value) } {
                        return Ok(None);
                    }
                    if row_changed || !same {
                        let scratch = match &search.scratch {
                            Some(scratch) => scratch,
                            None => search.scratch.insert(self.new_snapshot()?),
                        };
                        if !row_changed {
                            for prefix in 0..column as u32 {
                                let old = self
                                    .index(previous.as_raw(), index * width + prefix)?
                                    .unwrap();
                                self.append(scratch, prefix, old.raw.value)?;
                            }
                            row_changed = true;
                        }
                        self.append(scratch, column as u32, value)?;
                    }
                }
                cell += 1;
            }
            if let Some(search) = &mut search {
                if row_changed {
                    let scratch = search.scratch.as_ref().unwrap();
                    let text = if search.separator.is_null() {
                        let field = self.index(scratch.as_raw(), 0)?.unwrap();
                        self.call(&search.lower, field.raw.value, &mut [])?
                    } else {
                        let joined = self.call(
                            &search.join,
                            scratch.as_raw(),
                            &mut [search.separator.as_raw()],
                        )?;
                        self.call(&search.lower, joined.as_raw(), &mut [])?
                    };
                    if search.output.is_none() {
                        let changed = self.new_snapshot()?;
                        for prefix in 0..index {
                            let old = self.index(search.previous.as_raw(), prefix)?.unwrap();
                            self.append(&changed, prefix, old.raw.value)?;
                        }
                        search.output = Some(changed);
                    }
                    self.append(search.output.as_ref().unwrap(), index, text.as_raw())?;
                } else if let Some(output) = &search.output {
                    self.append(output, index, old_text.as_ref().unwrap().raw.value)?;
                }
            }
        }
        // A shorter current source still validates every prior descriptor.
        for index in cells..previous_length.unwrap_or(0) {
            let Some(old) = self.index(previous.as_raw(), index)? else {
                return Ok(None);
            };
            if !old.is_data() || !is_primitive(old.raw.value) {
                return Ok(None);
            }
        }
        let complete = end == count;
        let fields = if let Some(output) = output {
            if !warming || complete {
                self.freeze(&output)?;
            }
            output
        } else {
            previous.clone()
        };
        let text = if let Some(search) = search {
            for index in count..search.previous_length.unwrap_or(0) {
                let Some(old) = self.index(search.previous.as_raw(), index)? else {
                    return Ok(None);
                };
                if !old.is_data() || !unsafe { qjs::JS_IsString(old.raw.value) } {
                    return Ok(None);
                }
            }
            Some(if let Some(output) = search.output {
                if !warming || complete {
                    self.freeze(&output)?;
                }
                output
            } else {
                search.previous
            })
        } else {
            None
        };
        Ok(Some(Snapshot {
            fields,
            text,
            offset: end,
            complete,
        }))
    }
}

fn is_primitive(value: qjs::JSValue) -> bool {
    unsafe {
        qjs::JS_IsUndefined(value)
            || qjs::JS_IsNull(value)
            || qjs::JS_IsBool(value)
            || qjs::JS_IsNumber(value)
            || qjs::JS_IsString(value)
            || qjs::JS_IsBigInt(value)
    }
}

struct Descriptor<'a, 'js> {
    ctx: &'a Ctx<'js>,
    raw: qjs::JSPropertyDescriptor,
}

impl Descriptor<'_, '_> {
    fn is_data(&self) -> bool {
        self.raw.flags & qjs::JS_PROP_GETSET as i32 == 0
    }
}

impl Drop for Descriptor<'_, '_> {
    fn drop(&mut self) {
        // All three descriptor values are owned, including on rejection paths.
        unsafe {
            let ctx = self.ctx.as_raw().as_ptr();
            qjs::JS_FreeValue(ctx, self.raw.value);
            qjs::JS_FreeValue(ctx, self.raw.getter);
            qjs::JS_FreeValue(ctx, self.raw.setter);
        }
    }
}

struct Atom<'a, 'js> {
    ctx: &'a Ctx<'js>,
    raw: qjs::JSAtom,
}

impl<'a, 'js> Atom<'a, 'js> {
    fn new(ctx: &'a Ctx<'js>, raw: qjs::JSAtom) -> Result<Self> {
        if raw == qjs::JS_ATOM_NULL {
            return Err(Error::Exception);
        }
        Ok(Self { ctx, raw })
    }
}

impl Drop for Atom<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            qjs::JS_FreeAtom(self.ctx.as_raw().as_ptr(), self.raw);
        }
    }
}
