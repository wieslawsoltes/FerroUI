// Each instance of this closure belongs to one WebScene V8 realm.
(loader => {
    const modules = new Map();
    const libraries = new Map();
    const components = new Map();
    const loadingClassic = Symbol("initializing classic QML script");
    const owns = (object, key) => Object.prototype.hasOwnProperty.call(object, key);

    // Capture the worker's in-memory resource reader before its bootstrap removes
    // the global hook. The catalog is the complete set of resolvable module URLs.
    const readText = globalThis.__webSceneFetchText;
    const catalog = new Map();
    loader.prepareImport = () => undefined;
    loader.resolve = (specifier, parent) => {
        if (catalog.has(specifier)) return specifier;
        const edges = catalog.get(parent)?.dependencies;
        if (edges && owns(edges, specifier)) return edges[specifier];
        throw new TypeError("Unknown embedded module " + specifier);
    };
    loader.instantiate = url => {
        if (!catalog.has(url)) throw new TypeError("Unregistered module " + url);
        const code = readText(url);
        if (typeof code !== "string") throw new TypeError("Missing module " + url);
        Function("System", code + "\n//# sourceURL=" + url)(loader);
        return loader.getRegister();
    };
    const readonly = new WeakMap();
    function publicNamespace(ns) {
        let facade = readonly.get(ns);
        if (!facade) {
            facade = Object.create(null);
            for (const key of Object.keys(ns))
                Object.defineProperty(facade, key, {enumerable: true, get: () => ns[key]});
            Object.defineProperty(facade, Symbol.toStringTag, {value: "Module"});
            Object.freeze(facade);
            readonly.set(ns, facade);
        }
        return facade;
    }
    function context(_graph, origin) {
        return Object.freeze({
            import: specifier => loader.import(specifier, origin).then(publicNamespace),
            meta: loader.createContext(origin),
        });
    }

    async function prepare(graph) {
        for (const [url, definition] of Object.entries(graph.resources)) catalog.set(url, definition);
        // Static dependencies are instantiated by SystemJS in this V8 realm. Classic
        // imports need their factories prepared before synchronous QML calls.
        // A module reached exclusively via import() must keep its lazy effects.
        const visited = new Set();
        async function load(url) {
            if (visited.has(url)) return;
            visited.add(url);
            if (!modules.has(url)) modules.set(url, await loader.import(url));
            for (const child of Object.values(graph.resources[url].imports)) await load(child);
        }
        for (const [url, definition] of Object.entries(graph.resources)) {
            if (definition.preload) await load(url);
        }
        for (const root of Object.values(graph.names)) await load(root);
    }

    function namespace(graph, url, scope) {
        const definition = graph.resources[url];
        const module = modules.get(url);
        if (!module || !definition) throw new ReferenceError("Unprepared QML script " + url);
        if (definition.kind === "module") return publicNamespace(module);

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
            if (definition.hook) {
                const contextScope = Object.create(outer);
                Object.defineProperty(contextScope, definition.hook, {value: context(graph, url)});
                outer = contextScope;
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
        prepare, context,
        has: (graph, name) => owns(graph.names, name),
        get(graph, name, scope) {
            if (!owns(graph.names, name)) throw new ReferenceError("Unknown QML script import " + name);
            return namespace(graph, graph.names[name], scope);
        },
    };
})
