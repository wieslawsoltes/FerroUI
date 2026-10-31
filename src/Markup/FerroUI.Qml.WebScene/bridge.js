// Private per-worker V8 state. Host capabilities cross only the component transaction.
(function (modules) {
    const fetchText = globalThis.__webSceneFetchText;
    if (typeof fetchText !== "function") throw new Error("Missing WebScene synchronous text loader");
    const functions = new Map(), objects = new Map(), reverse = new WeakMap(), atoms = new Map();
    const proxies = new Map(), proxyIds = new WeakMap(), scopes = new Map();
    const denied = new Set(["window","document","self","navigator","location","fetch","XMLHttpRequest",
        "WebSocket","EventSource","Worker","SharedWorker","indexedDB","localStorage","sessionStorage"]);
    for (const key of Object.getOwnPropertyNames(globalThis)) {
        if (denied.has(key) || key.startsWith("__webScene")) Reflect.deleteProperty(globalThis,key);
    }
    let serial = 0, next = 0, preparing = "ready";
    function atom(value) {
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
        return JSON.parse(fetchText("https://qml.invalid/rpc/"+(++serial)+"/"+encodeURIComponent(JSON.stringify(request))));
    }
    function rpc(request) {
        let msg = transport(request);
        // During a host call a dependent property or QML method can itself run
        // JavaScript. Enter it on this stack in the same V8 realm, then resume.
        while (msg.nested) msg = transport(["returned",execute(msg.nested)]);
        if ("error" in msg) throw new Error(msg.error);
        return msg.ok;
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
            case "fn": {
                const [_,id,key,signature]=value;
                return (...args)=>decode(rpc(["call",id,key,args.map((v,i)=>encode(v,signature[i]))]));
            }
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
                if(modules.has(graph,k)) return modules.get(graph,k,scope);
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
    function execute(c) {
        try {
            for(const id of c.release || []) {
                const value=objects.get(id);
                if(typeof value === "bigint"||typeof value === "symbol") atoms.delete(value);
                objects.delete(id);
            }
            let result;
            if(c.op==="eval"){
                const args=Object.keys(c.args);
                // Names originate in QML method/signal declarations, but user-
                // assembled IR must not turn an argument name into executable text.
                if(args.some(k=>!/^[$_\p{ID_Start}][$\u200c\u200d_\p{ID_Continue}]*$/u.test(k)))
                    throw new SyntaxError("Invalid QML parameter");
                const key=JSON.stringify([c.source,c.form,c.origin,args]);
                let factory=functions.get(key);
                if(!factory){
                    const origin=c.origin.replace(/[\n\r\u2028\u2029]/g,"");
                    const body=c.form==="expr" ? "return (\n"+c.source+"\n);" : c.source;
                    factory=Function("with(arguments[0]){return function("+args.join(",")+"){\n"+body+"\n}}\n//# sourceURL="+origin);
                    if(functions.size>=4096)functions.delete(functions.keys().next().value);
                    functions.set(key,factory);
                }
                result=factory(environment(c)).apply(decode(c.owner),args.map(k=>decode(c.args[k])));
            }else{
                const value=decode(c.target);
                switch(c.op){
                    case "get": result=value[c.name];break;
                    case "set": value[c.name]=decode(c.value);break;
                    case "call": result=Reflect.apply(value,decode(c.receiver),c.args.map(decode));break;
                    default:throw new Error("Invalid WebScene operation");
                }
            }
            return {ok:encode(result,c.mode)};
        }catch(e){return {error:String(e?.stack||e)};}
    }
    return Object.freeze({
        prepare(graph){
            preparing="pending";
            modules.prepare(graph).then(()=>{preparing="ready"},e=>{preparing=String(e?.stack||e)});
        },
        preparation(){return preparing},
        run(c){return JSON.stringify(execute(c))}
    });
})
