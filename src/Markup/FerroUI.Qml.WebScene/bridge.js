// Private per-worker V8 state. Host capabilities cross only the component transaction.
(function (modules,observePromise,controlKey) {
    "use strict";
    let activeCalls=0;
    function authorize(key){if(key!==controlKey)throw new Error("Private WebScene QML entry point");}
    const fetchText = globalThis.__webSceneFetchText;
    if (typeof fetchText !== "function") throw new Error("Missing WebScene synchronous text loader");
    const functions = new Map(), objects = new Map(), reverse = new WeakMap(), atoms = new Map();
    const proxies = new Map(), proxyIds = new WeakMap(), scopes = new Map();
    const boundMethods = new Map(), callbackAdapters = new WeakMap();
    const denied = new Set(["window","document","self","navigator","location","fetch","XMLHttpRequest",
        "WebSocket","EventSource","Worker","SharedWorker","indexedDB","localStorage","sessionStorage"]);
    for (const key of Object.getOwnPropertyNames(globalThis)) {
        if (denied.has(key) || key.startsWith("__webScene")) Reflect.deleteProperty(globalThis,key);
    }
    let serial = 0, next = 0, preparing = "ready";
    const intrinsicPromise=Promise, intrinsicEval=eval;
    function atom(value) {
        // Install a rejection handler before WebScene drains its microtask queue.
        // Otherwise a rejected return value is reported as an unhandled engine
        // exception before Rust has the chance to observe the Promise handle.
        if(value instanceof intrinsicPromise) observePromise(value);
        const table = (typeof value === "object" || typeof value === "function") ? reverse : atoms;
        let id = table.get(value);
        if (!id) { id = ++next; table.set(value,id); }
        objects.set(id,value);
        return ["js",id];
    }
    function encode(value, mode, seen = new Set()) {
        if (value === undefined) return ["u"];
        if (value === null) return ["n"];
        switch (typeof value) {
            case "number": return ["d", Object.is(value,-0) ? "-0" : String(value)];
            case "boolean": return ["b",value];
            case "string": return ["s",value];
        }
        if (proxyIds.has(value)) return ["h",proxyIds.get(value)];
        if (mode && typeof value === "object") {
            if (seen.has(value)) throw new TypeError("A cyclic object cannot initialize a typed QML collection");
            seen.add(value);
            let result;
            if (Array.isArray(value)) result = ["a",Array.from(value, v => encode(v,mode,seen))];
            else if (mode === "data" && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null))
                result = ["o", Object.entries(value).map(([k,v])=>[k,encode(v,mode,seen)])];
            seen.delete(value);
            if (result) return result;
        }
        return atom(value);
    }

    function transport(request) {
        return JSON.parse(fetchText("https://qml.invalid/rpc/"+controlKey+"/"+(++serial)+"/"+encodeURIComponent(JSON.stringify(request))));
    }
    function rpc(request) {
        if (!activeCalls) throw new Error("QML host access outside a synchronous JavaScript invocation");
        let msg = transport(request);
        // During a host call a dependent property or QML method can itself run
        // JavaScript. Enter it on this stack in the same V8 realm, then resume.
        while (msg.nested) msg = transport(["returned",execute(msg.nested)]);
        if ("error" in msg) throw new Error(msg.error);
        return msg.ok;
    }
    // Method projections are stable across qualified/unqualified reads. A
    // method used as a slot ignores surplus signal arguments, while ordinary
    // JS functions receive every argument the signal declared.
    function method(value) {
        const [_,id,key,signature,isSignal]=value;
        let slots=boundMethods.get(id);
        if(!slots){slots=new Map();boundMethods.set(id,slots);}
        if(slots.has(key)) return slots.get(key);
        const bound=(...args)=>decode(rpc(["call",id,key,args.map((v,i)=>encode(v,signature[i]))]));
        callbackAdapters.set(bound,(...args)=>bound(...args.slice(0,signature.length)));
        if(isSignal){
            function change(action,args){
                if(!args.length) throw new Error(action+": no arguments given");
                const target=args.length===1?globalThis:args[0];
                const fn=args.length===1?args[0]:args[1];
                if(target!=null && typeof target!=="object" && typeof target!=="function") throw new Error(action+": target this is not an object");
                if(typeof fn!=="function") throw new Error(action+": target is not a function");
                rpc([action,id,key,encode(target==null?globalThis:target),atom(callbackAdapters.get(fn)||fn)]);
            }
            Object.defineProperties(bound,{
                connect:{value:(...args)=>change("connect",args)},
                disconnect:{value:(...args)=>change("disconnect",args)}
            });
        }
        slots.set(key,bound);
        return bound;
    }
    function decode(value) {
        switch(value[0]) {
            case "u": return undefined;
            case "n": return null;
            case "b": case "s": return value[1];
            case "d": return Number(value[1]);
            case "a": return value[1].map(decode);
            case "js":
                if (!objects.has(value[1])) throw new ReferenceError("Released JavaScript reference");
                return objects.get(value[1]);
            case "h": return proxy(value[1],value[2]);
            case "fn": return method(value);
            default: throw new Error("Unknown QML value tag");
        }
    }

    function proxy(id,kind) {
        if(proxies.has(id)) return proxies.get(id);
        const array = kind === "array";
        const target = array ? [] : Object.create(null);
        const result=new Proxy(target,{
            get(t,key,receiver) {
                if(typeof key !== "string") return Reflect.get(t,key,receiver);
                if(array && key in Array.prototype && key !== "length") return Reflect.get(t,key,receiver);
                return decode(rpc(["get",id,key]));
            },
            set(t,key,v) {
                if(typeof key !== "string") return false;
                const mode=rpc(["mode",id,key]);
                rpc(["set",id,key,encode(v,mode)]);
                return true;
            },
            has(t,key){return typeof key === "string" && (Reflect.has(t,key)||rpc(["keys",id]).includes(key));},
            ownKeys(){return [...new Set([...Reflect.ownKeys(target), ...rpc(["keys",id])])];},
            getOwnPropertyDescriptor(t,key){
                const desc=Reflect.getOwnPropertyDescriptor(t,key);
                if(desc) return desc;
                return typeof key === "string" && rpc(["keys",id]).includes(key)
                    ? {configurable:true,enumerable:true} : undefined;
            }
        });
        proxies.set(id,result); proxyIds.set(result,id);
        return result;
    }

    function environment(c) {
        if(scopes.has(c.scope)) return scopes.get(c.scope);
        const names=new Set(c.names), graph=c.graph;
        const groups=new Map();
        for (const name of names) if(name.includes(".")) {
            const [prefix,leaf]=name.split(".");
            if(!groups.has(prefix)) groups.set(prefix,Object.create(null));
            Object.defineProperty(groups.get(prefix),leaf,{enumerable:true,get:()=>decode(rpc(["resolve",c.scope,name]))});
        }
        const target=Object.create(null);
        const scope=new Proxy(target,{
            has(_,k){return typeof k==="string"&&(names.has(k)||groups.has(k)||modules.has(graph,k)||denied.has(k)||k.startsWith("__webScene"));},
            get(_,k){
                if(typeof k!=="string") return undefined;
                if(modules.has(graph,k)) {
                    const root=c.componentScope===c.scope ? scope : environment({
                        scope:c.componentScope,componentScope:c.componentScope,names:c.componentNames,graph
                    });
                    return modules.get(graph,k,root);
                }
                if(groups.has(k)&&!names.has(k)) return groups.get(k);
                if(!names.has(k)) return undefined;
                return decode(rpc(["resolve",c.scope,k]));
            },
            set(_,k,v){
                if(typeof k!=="string"||!names.has(k)) throw new ReferenceError("Unknown QML property: "+String(k));
                rpc(["assign",c.scope,k,encode(v,rpc(["scopeMode",c.scope,k]))]);
                return true;
            }
        });
        scopes.set(c.scope,scope); return scope;
    }
    function release(command) {
        for (const id of command.dropScopes || []) scopes.delete(id);
        for (const id of command.dropHosts || []) proxies.delete(id);
        for (const id of command.release || []) {
            const value = objects.get(id);
            if (typeof value === "bigint" || typeof value === "symbol") atoms.delete(value);
            objects.delete(id);
        }
    }

    function functionFactory(script, parameters) {
        const key = JSON.stringify([script.source, script.form, script.origin, parameters, script.hook]);
        if (functions.has(key)) return functions.get(key);

        // Validate names even for IR assembled directly by the application.
        const identifier = /^[$_\p{ID_Start}][$\u200c\u200d_\p{ID_Continue}]*$/u;
        const helpers = script.hook ? [script.hook] : [];
        if (script.form === "block") helpers.push("eval");
        if (![...parameters, ...helpers].every(name => identifier.test(name))) {
            throw new SyntaxError("Invalid QML parameter");
        }

        let body = script.source;
        if (script.form === "expr") {
            body = "return (\n" + body + "\n);";
        } else if (script.form === "block") {
            // Direct eval preserves the completion value of branches, loops and finally.
            body = "return eval(" + JSON.stringify(body) + ");";
        }
        let factory = "function(" + parameters.join(",") + "){\n" + body + "\n}";
        if (helpers.length) {
            // Closures can capture helpers without exposing them in arguments.
            factory = "function(" + helpers.join(",") + "){return " + factory + "}";
        }
        const origin = script.origin.replace(/[\n\r\u2028\u2029]/g, "");
        const compiled = Function("with(arguments[0]){return " + factory + "}\n//# sourceURL=" + origin);
        if (functions.size >= 4096) functions.delete(functions.keys().next().value);
        functions.set(key, compiled);
        return compiled;
    }

    function evaluate(script) {
        const parameters = script.parameters || Object.keys(script.args);
        const factory = functionFactory(script, parameters);
        let invoke = factory(environment(script));
        const helpers = [];
        if (script.hook) helpers.push(modules.context(script.graph, script.origin));
        if (script.form === "block") helpers.push(intrinsicEval);
        if (helpers.length) invoke = invoke(...helpers);
        return Reflect.apply(invoke, decode(script.owner), parameters.map(name => decode(script.args[name])));
    }

    function perform(command) {
        if (command.op === "eval") return evaluate(command);
        const target = decode(command.target);
        switch (command.op) {
            case "get": return target[command.name];
            case "set":
                target[command.name] = decode(command.value);
                return undefined;
            case "call": return Reflect.apply(target, decode(command.receiver), command.args.map(decode));
            default: throw new Error("Invalid WebScene operation");
        }
    }

    function execute(command) {
        activeCalls++;
        try {
            release(command);
            return {ok: encode(perform(command), command.mode)};
        } catch (error) {
            return {error: String(error?.stack || error)};
        } finally {
            activeCalls--;
        }
    }
    return Object.freeze({
        prepare(key,graph){authorize(key);
            preparing="pending";
            modules.prepare(graph).then(()=>{preparing="ready"},e=>{preparing=String(e?.stack||e)});
        },
        preparation(key){authorize(key);return preparing},
        run(key,c){authorize(key);return JSON.stringify(execute(c))},
        promise(key,c){
            authorize(key); release(c);
            try {
                const {state,value}=observePromise(decode(c.target));
                return JSON.stringify({ok:["a",[["s",state],encode(value)]]});
            } catch(e) {return JSON.stringify({error:String(e?.stack||e)});}
        }
    });
})
