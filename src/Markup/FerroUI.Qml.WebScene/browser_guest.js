// This bootstrap executes only inside the dedicated script worker. The Wasm
// owner's memory and the page's globals never enter the guest realm.
((channel, bootstrap) => {
    const modules = new Map(), now = performance.now.bind(performance);
    const timer = setTimeout.bind(globalThis);
    const handshake = postMessage.bind(globalThis);
    const key = crypto.randomUUID();
    let input, output, runtime, activeTurn = null, started = false;
    // Yield one task for a full microtask checkpoint without applying the
    // nested timer clamp to every ordinary QML property binding.
    const ticks = new MessageChannel(), queued = [];
    ticks.port1.onmessage = () => queued.shift()?.();
    const nextTask = () => new Promise(resolve => { queued.push(resolve); ticks.port2.postMessage(null); });
    const poll = () => new Promise(resolve => timer(resolve, 1));
    function readText(url) {
        if (modules.has(url)) return modules.get(url);
        const prefix = "https://qml.invalid/rpc/" + key + "/";
        if (!url.startsWith(prefix)) throw new Error("Unknown QML resource");
        const pieces = url.slice(prefix.length).split("/");
        if (activeTurn === null || pieces.length !== 3 || pieces[0] !== activeTurn)
            throw new Error("QML host access outside an active transaction");
        const request = JSON.parse(decodeURIComponent(pieces[2]));
        output.write(JSON.stringify({host:request}), Infinity);
        return input.read(Infinity);
    }
    async function run() {
        const cmd = JSON.parse(input.read(Infinity));
        if (typeof cmd.turn !== "string") throw new Error("Missing QML owner turn");
        if (cmd.files) {
            for (const url of modules.keys()) if (!Object.prototype.hasOwnProperty.call(cmd.files,url)) modules.delete(url);
        }
        for (const [url, text] of Object.entries(cmd.files || {})) {
            if (!url.startsWith("https://qml.invalid/modules/") || typeof text !== "string" ||
                (modules.has(url) && modules.get(url) !== text))
                throw new Error("Invalid embedded QML module");
            modules.set(url, text);
        }
        if (cmd.prepare) {
            runtime.prepare(key, cmd.prepare);
            let state;
            while ((state = runtime.preparation(key)) === "pending") await nextTask();
            if (state !== "ready") throw new Error(state);
        }
        activeTurn = cmd.turn;
        try {
            let result;
            if (cmd.op === "await") {
                const end = now() + cmd.waitMs;
                do {
                    result = JSON.parse(runtime.promise(key, cmd));
                    if (result.ok?.[1]?.[0]?.[1] !== "pending" || now() >= end) break;
                    await poll();
                } while (true);
            } else {
                result = JSON.parse(runtime.run(key, cmd));
            }
            // Drain Promise reactions from this operation before releasing the
            // Rust transaction and its host leases, as in the native backend.
            await nextTask();
            const leases = JSON.parse(runtime.collectLeases(key));
            const released = JSON.parse(runtime.sweepHosts(key));
            result.hostLeases = [...(result.hostLeases || []), ...(leases.hostLeases || []), ...released];
            return result;
        } finally {
            activeTurn = null;
        }
    }
    addEventListener("message", async event => {
        if (!started) {
            started = true;
            const [incoming, outgoing] = event.data;
            input = channel(incoming); output = channel(outgoing);
            try {
                Object.defineProperty(globalThis, "__webSceneFetchText", {value:readText, configurable:true});
                runtime = bootstrap(key);
                output.write('{"ready":true}', Infinity);
                handshake({ready:true});
            } catch (error) {
                handshake({failure:String(error?.stack || error)});
            }
        } else {
            let frame;
            try { frame = {result:await run()}; }
            catch (error) { frame = {failure:String(error?.stack || error)}; }
            output.write(JSON.stringify(frame), Infinity);
        }
    });
})
