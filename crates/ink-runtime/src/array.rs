use rquickjs::{Ctx, Error, Function, Result, Value, atom::PredefinedAtom, qjs};

pub(crate) fn install(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals().set(
        "__inkIsDenseArray",
        Function::new(ctx.clone(), is_dense_array)?,
    )
}

fn is_dense_array(value: Value<'_>) -> Result<bool> {
    let ctx = value.ctx().as_raw().as_ptr();
    let array = value.as_raw();
    unsafe {
        // Reject proxies before inspecting properties.
        if qjs::JS_IsProxy(array) || !qjs::JS_IsArray(array) {
            return Ok(false);
        }

        // A real array's own length is always numeric data. Read its full u32
        // range without Array::len's signed-integer assumption.
        let length = qjs::JS_GetProperty(ctx, array, PredefinedAtom::Length as _);
        if qjs::JS_IsException(length) {
            return Err(Error::Exception);
        }
        let mut count = 0;
        let converted = qjs::JS_ToIndex(ctx, &mut count, length);
        qjs::JS_FreeValue(ctx, length);
        if converted < 0 {
            return Err(Error::Exception);
        }

        for index in 0..count as u32 {
            let atom = qjs::JS_NewAtomUInt32(ctx, index);
            if atom == qjs::JS_ATOM_NULL {
                return Err(Error::Exception);
            }
            let mut descriptor = qjs::JSPropertyDescriptor {
                flags: 0,
                value: qjs::JS_UNDEFINED,
                getter: qjs::JS_UNDEFINED,
                setter: qjs::JS_UNDEFINED,
            };
            let found = qjs::JS_GetOwnProperty(ctx, &mut descriptor, array, atom);
            qjs::JS_FreeAtom(ctx, atom);
            let is_data = descriptor.flags & qjs::JS_PROP_GETSET as i32 == 0;
            // The descriptor owns all three returned values. Undefined initial
            // values also make cleanup valid when a property is absent.
            qjs::JS_FreeValue(ctx, descriptor.value);
            qjs::JS_FreeValue(ctx, descriptor.getter);
            qjs::JS_FreeValue(ctx, descriptor.setter);
            if found < 0 {
                return Err(Error::Exception);
            }
            if found == 0 || !is_data {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
