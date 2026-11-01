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
    // A live proxy preserves its model identity. Pending edits keep the
    // snapshot alive until synchronization, while clean snapshots can collect.
    const models = new Map(), dirty = new Map(), modelIds = new WeakMap();
    const collected = new FinalizationRegistry(({id, ref}) => {
        if (models.get(id) === ref) models.delete(id);
    });
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
        const model = modelIds.get(value);
        if (model) return {t: "Model", v: {id: model.id, revision: model.revision, reference: true, rows: []}};
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
            case "Model": return loadModel(value.v);
            case "Array": return value.v.map(decode);
            case "Object": return Object.fromEntries(Object.entries(value.v).map(([k,v]) => [k, decode(v)]));
            default: throw new TypeError("invalid WorkerScript value");
        }
    }

    function loadModel(snapshot) {
        let state = models.get(snapshot.id)?.deref();
        if (snapshot.reference && state) return state.proxy;
        if (!state) {
            state = {id: snapshot.id, revision: snapshot.revision, rows: [], edits: [], conflict: false};
            state.proxy = modelApi(state);
            modelIds.set(state.proxy, state);
            const ref = new WeakRef(state);
            models.set(state.id, ref);
            collected.register(state, {id: state.id, ref});
        }
        if ((state.conflict || !state.edits.length) && snapshot.revision >= state.revision) {
            state.revision = snapshot.revision;
            state.mayHaveNested = snapshot.rows.some(row => Object.values(row).some(v => v.t === "Model"));
            state.rows = snapshot.rows.map(row => decode({t:"Object",v:row}));
            state.edits = [];
            state.conflict = false;
            dirty.delete(state.id);
        }
        return state.proxy;
    }

    function modelApi(state) {
        function check(i, end = false) {
            if (!Number.isInteger(i) || i < 0 || i >= state.rows.length + (end ? 1 : 0))
                throw new RangeError("ListModel index out of range");
            return i;
        }
        function checkRange(i, n) {
            check(i);
            if (!Number.isInteger(n) || n < 1 || i + n > state.rows.length)
                throw new RangeError("invalid ListModel range");
        }
        function row(value) {
            const encoded = encode(value);
            if (encoded.t !== "Object") throw new TypeError("ListModel expects a role object");
            if (!state.mayHaveNested) state.mayHaveNested = Object.values(encoded.v).some(v => v.t === "Model");
            return decode(encoded);
        }
        function edit(method, ...args) {
            state.edits.push({method, args: args.map(arg => encode(arg))});
            dirty.set(state.id, state);
        }
        return Object.freeze({
            get count() { return state.rows.length; },
            get(i) {
                const item = state.rows[check(i)];
                return new Proxy(item, {
                    set(_, key, value) {
                        const index = state.rows.indexOf(item);
                        if (index < 0) throw new RangeError("removed ListModel row");
                        state.proxy.setProperty(index, key, value);
                        return true;
                    }
                });
            },
            append(value) { const item = row(value); state.rows.push(item); edit("append", item); },
            insert(i,value) { const item = row(value); state.rows.splice(check(i,true),0,item); edit("insert",i,item); },
            clear() { state.rows = []; edit("clear"); },
            remove(i,n=1) { checkRange(i,n); state.rows.splice(i,n); edit("remove",i,n); },
            move(from,to,n) {
                checkRange(from,n); checkRange(to,n);
                const moved = state.rows.splice(from,n);
                state.rows.splice(to,0,...moved);
                edit("move",from,to,n);
            },
            set(i,value) {
                check(i,true);
                const item = row(value);
                if (i === state.rows.length) state.rows.push(item);
                else Object.assign(state.rows[i],item);
                edit("set",i,item);
            },
            setProperty(i, name, value) {
                check(i);
                if (typeof name !== "string") throw new TypeError("invalid ListModel role");
                if (modelIds.has(value)) state.mayHaveNested = true;
                state.rows[i][name] = decode(encode(value));
                edit("setProperty",i,name,value);
            },
            sync() {
                // Root sync commits all reachable dirty model snapshots together.
                // Keep the root revision in the batch even when only children changed.
                if (!dirty.size) return;
                const pending = [state], visited = new Set(), changed = [];
                while (pending.length) {
                    const next = pending.pop();
                    if (visited.has(next)) continue;
                    visited.add(next);
                    if (next === state || next.edits.length) changed.push(next);
                    if (!next.mayHaveNested) continue;
                    for (const row of next.rows) for (const value of Object.values(row)) {
                        const child = modelIds.get(value);
                        if (child) pending.push(child);
                    }
                }
                if (!changed.some(m => m.edits.length)) return;
                try {
                    const revisions = post({t:"SyncTree",v:changed.map(m=>({id:m.id,revision:m.revision,edits:m.edits}))});
                    changed.forEach((m,i) => {
                        m.revision = revisions[i]; m.edits = []; dirty.delete(m.id);
                    });
                } catch (error) {
                    changed.forEach(m => { m.conflict = true; });
                    throw error;
                }
            }
        });
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
