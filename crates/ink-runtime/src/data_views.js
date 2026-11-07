(() => {
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
})()
