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
    // SystemJS caches failures as well as successes. A URL that was absent
    // from the last committed manifest gets a fresh private execution identity
    // on retry. Unaffected URLs keep their original namespace and live exports.
    const executions = new Map(), logicalUrls = new Map(), removed = new Set();
    let attempt = 0;
    const originalContext = loader.createContext.bind(loader);
    loader.createContext = parent => {
        const context = originalContext(parent);
        const origin = catalog.get(logicalUrls.get(parent) || parent)?.origin;
        if (origin) context.url = origin;
        return context;
    };
    loader.prepareImport = () => undefined;
    const moduleRoot = "https://qml.invalid/modules/project/";
    const classicRoot = "https://qml.invalid/modules/classic/project/";
    const componentRoot = "https://qml.invalid/components/project/";
    // Only the embedded catalog can satisfy an import. Directory relationships
    // are virtual and independent of the working directory and operating system.
    function relativeModule(specifier, parent) {
        const root = parent?.startsWith(moduleRoot) ? moduleRoot
            : parent?.startsWith(componentRoot) ? componentRoot
            : parent?.startsWith(classicRoot) ? classicRoot : null;
        if (!root || !specifier || specifier.startsWith("/") ||
            /[:?#\\\x00-\x1f\x7f]/.test(specifier)) return null;
        const parts = parent.slice(root.length).split("/");
        parts.pop();
        for (const raw of specifier.split("/")) {
            let segment;
            try { segment=decodeURIComponent(raw); } catch { return null; }
            if (!segment || segment === ".") continue;
            if (segment === "..") {
                if (!parts.length) return null;
                parts.pop();
            } else {
                if (/[/\\\x00-\x1f\x7f]/.test(segment)) return null;
                parts.push(encodeURIComponent(segment).replace(/[!'()*]/g,
                    c => "%" + c.charCodeAt(0).toString(16).toUpperCase()));
            }
        }
        return moduleRoot + parts.join("/");
    }
    const remoteModules = new Map();
    const originOf = url => catalog.get(url)?.origin || url || "";
    function site(url) { return /^https?:\/\/[^/]+/i.exec(url)?.[0]; }
    function checked(url, parent) {
        const source = originOf(parent);
        if (site(source) && !source.startsWith("https://qml.invalid/") &&
            site(source) !== site(originOf(url))) throw new TypeError("Cross-origin module import");
        return executions.get(url) || url;
    }
    function remoteTarget(specifier, parent) {
        if (!site(parent) || /[\\#\x00-\x1f]/.test(specifier)) return null;
        if (/^https?:\/\//i.test(specifier)) return remoteModules.get(specifier);
        if (!specifier || specifier.startsWith("//") || /^[A-Za-z][A-Za-z0-9+.-]*:/.test(specifier)) return null;
        const origin = site(parent);
        const base = parent.slice(origin.length).split("?")[0];
        const q = specifier.indexOf("?");
        const path = q < 0 ? specifier : specifier.slice(0, q);
        const query = q < 0 ? "" : specifier.slice(q);
        const pieces = path.startsWith("/") ? [] : base.split("/").slice(1,-1);
        for (const part of path.split("/")) {
            if (!part || part === ".") continue;
            if (part === "..") pieces.pop(); else pieces.push(part);
        }
        return remoteModules.get(origin + "/" + pieces.join("/") + query);
    }
    loader.resolve = (specifier, parent) => {
        parent = logicalUrls.get(parent) || parent;
        if (catalog.has(specifier)) {
            if (parent && catalog.get(specifier).kind !== "module")
                throw new TypeError("A classic QML script is not an ES module");
            return checked(specifier,parent);
        }
        const edges = catalog.get(parent)?.dependencies;
        if (edges && owns(edges, specifier)) {
            const url = edges[specifier];
            if (catalog.get(url)?.kind === "module") return checked(url,parent);
            throw new TypeError("A classic QML script is not an ES module");
        }
        const remote = remoteTarget(specifier, originOf(parent));
        if (remote) return checked(remote,parent);
        const url = relativeModule(specifier, parent);
        if (url && catalog.has(url)) return checked(url,parent);
        throw new TypeError("Unknown embedded module " + specifier);
    };
    loader.instantiate = execution => {
        const url = logicalUrls.get(execution) || execution;
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
            readonly.set(facade, facade);
        }
        return facade;
    }
    const systemImport = loader.import.bind(loader);
    loader.import = (specifier, parent, options) => {
        // Coerce at the call site (inside the current QML invocation) but
        // report coercion and resolution failures through the returned Promise.
        try { return systemImport(`${specifier}`, parent, options).then(publicNamespace); }
        catch (error) { return Promise.reject(error); }
    };
    function context(_graph, origin) {
        return Object.freeze({
            import: (specifier, options) => loader.import(specifier, origin, options),
            meta: loader.createContext(origin),
        });
    }

    async function prepare(graph) {
        for (const url of catalog.keys()) {
            if (owns(graph.resources, url)) continue;
            removed.add(url);
            logicalUrls.delete(executions.get(url));
            executions.delete(url);
            modules.delete(url);
            libraries.delete(url);
        }
        catalog.clear();
        remoteModules.clear();
        for (const [url, definition] of Object.entries(graph.resources)) {
            catalog.set(url,definition);
            if (removed.delete(url)) {
                const execution = url + "?qml-attempt=" + (++attempt);
                executions.set(url,execution);
                logicalUrls.set(execution,url);
            }
            if (definition.kind === "module" && definition.origin) remoteModules.set(definition.origin,url);
        }
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

    function namespace(graph, url, scope, qtAt) {
        const definition = graph.resources[url];
        const module = modules.get(url);
        if (!module || !definition) throw new ReferenceError("Unprepared QML script " + url);
        if (definition.kind === "module") return publicNamespace(module);

        let instances = libraries;
        if (definition.kind === "script") {
            const [origin, id] = graph.context;
            if (!components.has(id)) components.set(id, new Map());
            const origins = components.get(id);
            if (!origins.has(origin)) origins.set(origin, new Map());
            instances = origins.get(origin);
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
                    Object.defineProperty(outer, name, {get: () => namespace(graph, imports[name], null, qtAt)});
                }
            } else {
                // .pragma library and scripts with explicit imports have their own scope.
                outer = definition.kind === "script" && scope ? scope : Object.create(null);
            }
            {
                const contextScope = Object.create(outer);
                Object.defineProperty(contextScope,"Qt",{value:qtAt(definition.origin || url,definition.kind === "library")});
                if (definition.hook)
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
        forgetHost(id) { components.delete(String(id)); },
        has: (graph, name) => owns(graph.names, name),
        get(graph, name, scope, qtAt) {
            if (!owns(graph.names, name)) throw new ReferenceError("Unknown QML script import " + name);
            return namespace(graph, graph.names[name], scope, qtAt);
        },
    };
})
