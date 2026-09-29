const needsReact = /* @__PURE__ */ Symbol("list needs React");
const plans = /* @__PURE__ */ new WeakMap();
const keys = /* @__PURE__ */ new WeakMap();
const plainRecords = /* @__PURE__ */ new WeakSet();
const isProxy = typeof __inkIsProxy === "function" ? __inkIsProxy : undefined;
function bareFunctionPrototype(value, owner) {
    if (value === null || typeof value !== "object" || isProxy?.(value))
        return false;
    if (Object.getPrototypeOf(value) !== Object.prototype || Reflect.ownKeys(value).length !== 1)
        return false;
    const constructor = Object.getOwnPropertyDescriptor(value, "constructor");
    return !!constructor && "value" in constructor && constructor.value === owner;
}
function plainData(value, cache = plainRecords, visiting = new Set()) {
    if (value === null || typeof value !== "object" && typeof value !== "function")
        return true;
    if (cache.has(value))
        return true;
    if (isProxy?.(value))
        return false;
    if (visiting.has(value) || visiting.size >= 64)
        return false;
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== null && prototype !== Object.prototype && prototype !== Array.prototype
        && !(typeof value === "function" && prototype === Function.prototype))
        return false;
    visiting.add(value);
    const properties = Object.getOwnPropertyDescriptors(value);
    const plain = Reflect.ownKeys(properties).every(key => {
        const property = properties[key];
        if (!("value" in property))
            return false;
        if (["toString", "valueOf", Symbol.toPrimitive].includes(key) && typeof property.value === "function")
            return false;
        if (key === "prototype" && typeof value === "function" && bareFunctionPrototype(property.value, value))
            return true;
        return plainData(property.value, cache, visiting);
    });
    visiting.delete(value);
    if (plain && isProxy)
        cache.add(value);
    return plain;
}
/** @internal A compiler-proven field extractor has no captured state. */
export function nativeListKey(extract, identity) {
    keys.set(extract, identity);
    return extract;
}
/** @internal Created by the Ink compiler; application List props stay unchanged. */
export function nativeListRow(render, project, template, scope) {
    plans.set(render, { project: project, template, scope });
    return render;
}
/** @internal Match React's primitive text children without invoking components. */
export function listText(parts) {
    return parts.map(value => {
        if (value == null || typeof value === "boolean")
            return "";
        if (Array.isArray(value))
            return listText(value);
        if (!["string", "number", "bigint"].includes(typeof value))
            throw needsReact;
        return String(value);
    }).join("");
}
/** @internal The same presentation fields as Ink's stateless Row component. */
export function listRowFields(props) {
    const { href } = props;
    if (href !== undefined && props.onPress)
        throw new Error("Row accepts either href or onPress");
    return { image: props.image, title: props.title, titleMaxLines: props.titleMaxLines,
        titleIcon: props.titleIcon, subtitle: props.subtitle, subtitleIcon: props.subtitleIcon,
        onLongPress: props.onLongPress,
        onPress: href === undefined ? props.onPress ?? (props.onLongPress ? () => { } : undefined) : () => navigate(href) };
}
function nativeValue(value, previous) {
    if (typeof value === "function")
        return true;
    if (value == null)
        return null;
    if (Array.isArray(value)) {
        const before = Array.isArray(previous) ? previous : [];
        const next = value.map((item, index) => nativeValue(item, before[index]));
        return before.length === next.length && next.every((item, index) => Object.is(item, before[index])) ? before : next;
    }
    if (typeof value === "object") {
        const before = previous !== null && typeof previous === "object" && !Array.isArray(previous)
            ? previous : {};
        const next = {};
        let unchanged = true;
        for (const key of Object.keys(value)) {
            const item = nativeValue(Reflect.get(value, key), before[key]);
            next[key] = item;
            if (!Object.hasOwn(before, key) || !Object.is(item, before[key]))
                unchanged = false;
        }
        return unchanged && Object.keys(next).length === Object.keys(before).length ? before : next;
    }
    return value;
}
function captureValue(value, cache) {
    if (value === null || typeof value !== "object" && typeof value !== "function")
        return { value };
    const previous = cache.get(value);
    if (previous)
        return previous;
    const properties = Object.getOwnPropertyDescriptors(value);
    const captured = { value, prototype: Object.getPrototypeOf(value), fields: Reflect.ownKeys(properties).map(key => {
            const property = properties[key];
            const field = key === "prototype" && typeof value === "function" && bareFunctionPrototype(property.value, value)
                ? { value: property.value } : captureValue(property.value, cache);
            return [key, !!property.enumerable, field];
        }) };
    cache.set(value, captured);
    return captured;
}
function sameCapture(before, after, seen) {
    // Other JS hosts cannot establish that an object has no Proxy traps.
    if (before.value !== null && (typeof before.value === "object" || typeof before.value === "function") && !isProxy)
        return false;
    if (!Object.is(before.value, after.value) || before.prototype !== after.prototype)
        return false;
    const previous = before.fields, current = after.fields;
    if (!previous || !current)
        return previous === current;
    if (seen.get(before) === after)
        return true;
    seen.set(before, after);
    return previous.length === current.length && current.every(([key, enumerable, value], index) => key === previous[index][0] && enumerable === previous[index][1] && sameCapture(previous[index][2], value, seen));
}
function sameScopeValues(before, after) {
    const seen = new WeakMap();
    return before.length === after.length && after.every((value, index) => sameCapture(before[index], value, seen));
}
export const observation = { plainData, captureValue, sameScopeValues };
