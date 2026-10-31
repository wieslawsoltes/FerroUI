// Each instance of this closure belongs to one WebScene V8 realm.
(() => {
    const modules = new Map();
    const libraries = new Map();
    const components = new Map();
    const loadingClassic = Symbol("initializing classic QML script");
    const owns = (object, key) => Object.prototype.hasOwnProperty.call(object, key);

    async function prepare(graph) {
        // V8 resolves and instantiates ESM dependencies, including cycles. A
        // classic resource imports as a factory with no top-level QML effects.
        for (const url of Object.keys(graph.resources)) {
            if (!modules.has(url)) modules.set(url, await import(url));
        }
    }

    function namespace(graph, url, scope) {
        const definition = graph.resources[url];
        const module = modules.get(url);
        if (!module || !definition) throw new ReferenceError("Unprepared QML script " + url);
        if (definition.kind === "module") return module;

        let instances = libraries;
        if (definition.kind === "script") {
            const key = JSON.stringify(graph.context);
            if (!components.has(key)) components.set(key, new Map());
            instances = components.get(key);
        }
        if (instances.has(url)) {
            const result = instances.get(url);
            if (result === loadingClassic) throw new ReferenceError("Circular classic script initialization: " + url);
            return result;
        }
        instances.set(url, loadingClassic);
        try {
            let outer;
            const imports = definition.imports;
            if (Object.keys(imports).length) {
                outer = Object.create(null);
                for (const name of Object.keys(imports)) {
                    Object.defineProperty(outer, name, {get: () => namespace(graph, imports[name], null)});
                }
            } else {
                // .pragma library and scripts with explicit imports have their own scope.
                outer = definition.kind === "script" && scope ? scope : Object.create(null);
            }
            const result = module.default(outer);
            instances.set(url, result);
            return result;
        } catch (error) {
            instances.delete(url);
            throw error;
        }
    }

    return {
        prepare,
        has: (graph, name) => owns(graph.names, name),
        get(graph, name, scope) {
            if (!owns(graph.names, name)) throw new ReferenceError("Unknown QML script import " + name);
            return namespace(graph, graph.names[name], scope);
        },
    };
})()
