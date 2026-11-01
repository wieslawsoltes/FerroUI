// Bootstrap one private worker global. Only copied data crosses the resource
// callback. Acknowledgements bound output even while a long-running job emits.
(function (modules, capability) {
    "use strict";
    const read = globalThis.__webSceneFetchText;
    const hidden = new Set(["window", "self", "document", "navigator", "location", "fetch",
        "XMLHttpRequest", "WebSocket", "Worker", "SharedWorker", "localStorage", "sessionStorage"]);
    for (const name of Object.getOwnPropertyNames(globalThis)) {
        if (name.startsWith("__webScene") || hidden.has(name)) Reflect.deleteProperty(globalThis, name);
    }
    let sequence = 0;
    let state = "done";

    function post(event) {
        const response = JSON.parse(read("https://qml.invalid/rpc/" + capability + "/worker/" +
            (++sequence) + "/" + encodeURIComponent(JSON.stringify(event))));
        if ("error" in response) throw new Error(response.error);
        return response.ok;
    }
    function encode(value, ancestors = new Set()) {
        if (value === undefined) return {t: "Undefined"};
        if (value === null) return {t: "Null"};
        switch (typeof value) {
            case "string": return {t: "String", v: value};
            case "boolean": return {t: "Bool", v: value};
            case "number": return {t: "Number", v: Object.is(value, -0) ? "-0" : String(value)};
        }
        if (typeof value !== "object" || (!Array.isArray(value) &&
            Object.getPrototypeOf(value) !== Object.prototype && Object.getPrototypeOf(value) !== null)) {
            throw new TypeError("a WorkerScript message must contain plain transferable values");
        }
        if (ancestors.has(value) || ancestors.size >= 64) throw new TypeError("cyclic or deeply nested worker message");
        ancestors.add(value);
        try {
            if (Array.isArray(value)) return {t: "Array", v: Array.from(value, v => encode(v, ancestors))};
            return {t: "Object", v: Object.fromEntries(Object.entries(value).map(([k,v]) => [k, encode(v, ancestors)]))};
        } finally {
            ancestors.delete(value);
        }
    }
    function decode(value) {
        switch (value.t) {
            case "Null": return null;
            case "Undefined": return undefined;
            case "Bool": case "String": return value.v;
            case "Number": return Number(value.v);
            case "Array": return value.v.map(decode);
            case "Object": return Object.fromEntries(Object.entries(value.v).map(([k,v]) => [k, decode(v)]));
            default: throw new TypeError("invalid WorkerScript value");
        }
    }

    const api = Object.seal({onMessage: null, sendMessage(value) { post({t: "Message", v: encode(value)}); }});
    Object.defineProperty(globalThis, "WorkerScript", {value: api});
    async function invoke(action, loading = false) {
        state = "pending";
        try {
            await action();
        } catch (e) {
            const reason = String(e?.stack || e);
            if (loading) { state = reason; return; }
            post({t: "Error", v: reason});
        }
        state = "done";
    }
    return Object.freeze({
        load(init) {
            invoke(async () => {
                await modules.prepare({resources: init.resources, names: {}});
                if (typeof init.classic === "string") (0,eval)(init.classic + "\n//# sourceURL=" + init.url);
                else await modules.context(null, init.url).import(init.url);
                post({t: "Ready"});
            }, true);
        },
        receive(message) { invoke(() => typeof api.onMessage === "function" ? api.onMessage(decode(message)) : undefined); },
        result() { return state; }
    });
})
