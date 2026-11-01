// Standalone ES module. The page hosts promises and callbacks. Each application's
// dedicated worker owns the Wasm instance and drives its QML event loop.
const protocol = "ferroui-qml-application/1";
const limit = 32 * 1024 * 1024;
const wakeEvent = "ferroui-qml-application-wake";

export const applicationModule = () => import.meta.url;
export const wakeApplication = () => globalThis.dispatchEvent(new Event(wakeEvent));

function encode(value, ancestors = new Set()) {
    if (value === null) return {t:"Null"};
    switch (typeof value) {
        case "undefined": return {t:"Undefined"};
        case "boolean": return {t:"Bool", v:value};
        case "number": return {t:"Number", v:Object.is(value,-0) ? "-0" : String(value)};
        case "string": return {t:"String", v:value};
        case "object": break;
        default: throw new TypeError("QML data must contain plain values");
    }
    if (ancestors.size >= 64 || ancestors.has(value)) throw new TypeError("Cyclic or deeply nested QML data");
    ancestors.add(value);
    try {
        if (Array.isArray(value)) return {t:"Array", v:Array.from(value, x=>encode(x,ancestors))};
        const proto = Object.getPrototypeOf(value);
        if (proto !== null && proto !== Object.prototype) throw new TypeError("Expected a plain QML data object");
        return {t:"Object",v:Object.fromEntries(Object.entries(value).map(([k,x])=>[k,encode(x,ancestors)]))};
    } finally { ancestors.delete(value); }
}

function decode(data) {
    switch (data.t) {
        case "Null": return null;
        case "Undefined": return undefined;
        case "Bool": case "String": return data.v;
        case "Number": return Number(data.v);
        case "Array": return data.v.map(decode);
        case "Object": return Object.fromEntries(Object.entries(data.v).map(([k,v]) => [k,decode(v)]));
        default: throw new Error("Unknown QML response value");
    }
}

function bounded(text) {
    if (text.length > limit || new TextEncoder().encode(text).length > limit) throw new RangeError("QML application frame exceeds 32 MiB");
    return text;
}

export async function openQmlApplication(url, {timeout = 15000, signal, onError = e=>globalThis.reportError(e)} = {}) {
    if (!Number.isFinite(timeout) || timeout <= 0 || timeout > 2147483647) throw new RangeError("Invalid QML application timeout");
    signal?.throwIfAborted();
    const worker = new Worker(url, {type:"module", name:"FerroUI QML application"});
    const {port1:port, port2} = new MessageChannel();
    const pending = new Map(), watches = new Map();
    let closed, next = 0, bytes = 0;
    function close(reason) {
        if (closed) return;
        closed = reason;
        signal?.removeEventListener("abort", abort);
        port.close(); worker.terminate();
        for (const item of pending.values()) { clearTimeout(item.timer); item.reject(reason); }
        pending.clear(); watches.clear();
    }
    const abort = () => close(signal.reason || new Error("QML application aborted"));
    signal?.addEventListener("abort", abort, {once:true});
    function transportFailure(error) {
        if (closed) return;
        const started = !pending.has(0);
        close(error);
        if (started) onError(error);
    }
    worker.onerror = e => { e.preventDefault(); transportFailure(new Error(e.message || "QML application worker failed")); };
    port.onmessageerror = () => transportFailure(new Error("Invalid QML application message"));
    function wait(id, size = 0) {
        return new Promise((resolve,reject) => {
            const timer = setTimeout(()=>close(new Error("QML application request deadline exceeded")),timeout);
            bytes += size; pending.set(id,{resolve,reject,timer,size});
        });
    }
    port.onmessage = ({data}) => {
        if (closed) return;
        try {
            if (data.fatal) { transportFailure(new Error(data.fatal)); return; }
            if (data.issue !== undefined) { onError(new Error(data.issue)); return; }
            if (data.changes) {
                for (const token of data.tokens) {
                    const callback = watches.get(token), update = data.changes;
                    if (callback) {
                        if (update.error) onError(new Error(update.error));
                        else { try { callback(decode(update.value)); } catch(e) { onError(e); } }
                    }
                }
                return;
            }
            const item = pending.get(data.id);
            if (!item) throw new Error("Unexpected QML application reply");
            clearTimeout(item.timer); pending.delete(data.id); bytes -= item.size;
            if (data.error !== undefined) item.reject(new Error(data.error));
            else { try { item.resolve(data.value ? decode(JSON.parse(data.value)) : undefined); } catch(e) { item.reject(e); } }
        } catch(e) { transportFailure(e); }
    };
    const ready = wait(0);
    worker.onmessage = ({data}) => {
        if (data?.protocol !== protocol || !data.ready) return;
        worker.onmessage = null;
        try { worker.postMessage({protocol,port:port2},[port2]); } catch(e) { close(e); }
    };
    try { await ready; } catch(e) { close(e); throw e; }

    function request(op, object, member, data, token) {
        if (closed) return Promise.reject(closed);
        if (pending.size >= 256 || next >= 0xffffffff) return Promise.reject(new RangeError("QML application request limit exceeded"));
        const size = new TextEncoder().encode(data || "").length;
        if (size > limit - bytes) return Promise.reject(new RangeError("QML application queue exceeds 32 MiB"));
        const id = ++next, result = wait(id,size);
        try { port.postMessage({id,op,object,member,data,token}); } catch(e) { close(e); }
        return result;
    }
    return Object.freeze({
        get: (property, object="") => request("get",object,property),
        async set(property,value,object="") { await request("set",object,property,JSON.stringify(encode(value))); },
        async call(method,args=[],object="") {
            if (!Array.isArray(args)) throw new TypeError("QML call arguments must be an array");
            return request("call",object,method,JSON.stringify(args.map(x=>encode(x))));
        },
        async watch(property,callback,object="") {
            if (typeof callback !== "function") throw new TypeError("Expected a QML change callback");
            const token = next + 1;
            await request("watch",object,property,undefined,token);
            watches.set(token,callback);
            return async () => { if (watches.delete(token)) await request("unwatch","","",undefined,token); };
        },
        dispose: () => close(new Error("QML application disposed")),
    });
}

export function serveQmlApplication(create) {
    if (typeof DedicatedWorkerGlobalScope === "undefined" || !(globalThis instanceof DedicatedWorkerGlobalScope))
        throw new Error("A QML application must be served from a dedicated worker");
    async function connect({data}) {
        if (data?.protocol !== protocol || !(data.port instanceof MessagePort)) return;
        removeEventListener("message",connect);
        const port = data.port;
        let model, timer, notified = false;
        function schedule(delay) {
            clearTimeout(timer);
            if (delay >= 0) timer = setTimeout(pump, Math.min(2147483647, delay));
        }
        const wake = () => { notified = true; schedule(0); };
        function changes() {
            for (const {tokens,update} of JSON.parse(model.changes())) {
                try { bounded(JSON.stringify(update)); port.postMessage({tokens,changes:update}); }
                catch(e) { port.postMessage({tokens, changes:{error:String(e)}}); }
            }
        }
        function fail(e) {
            clearTimeout(timer);
            removeEventListener(wakeEvent,wake);
            model?.free(); model = null;
            port.postMessage({fatal:String(e?.message || e)}); port.close();
        }
        function advance() {
            let retry = false;
            try {
                model.pump();
            } catch (e) {
                // BrowserApplication returns string-valued QML errors after
                // rollback. Wasm traps and broken host objects are terminal.
                if (typeof e !== "string") throw e;
                port.postMessage({issue:e});
                retry = true;
            }
            changes();
            return retry;
        }
        function pump() {
            try {
                notified = false;
                const retry = advance();
                schedule(retry ? 16 : notified ? 0 : model.nextWake());
            } catch(e) { fail(e); }
        }
        try {
            model = await create();
            addEventListener(wakeEvent,wake);
            port.onmessage = ({data:r}) => {
                let response;
                try {
                    advance();
                    if (r.op === "watch") model.watch(r.token,r.object,r.member);
                    else if (r.op === "unwatch") model.unwatch(r.token);
                    else response = bounded(model.dispatch(r.op,r.object,r.member,r.data || ""));
                    changes();
                    port.postMessage({id:r.id,value:response});
                } catch(e) {
                    if (e instanceof WebAssembly.RuntimeError) { fail(e); return; }
                    port.postMessage({id:r.id,error:String(e?.message || e)});
                }
                // Earlier asynchronous deliveries may already be waiting.
                wake();
            };
            port.postMessage({id:0}); wake();
        } catch(e) { port.postMessage({id:0,error:String(e?.message || e)}); port.close(); }
    }
    addEventListener("message",connect);
    postMessage({protocol,ready:true});
}
