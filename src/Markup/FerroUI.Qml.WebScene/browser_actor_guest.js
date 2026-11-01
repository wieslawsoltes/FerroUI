// The only synchronous operation is the data-only callback from the worker
// script to the QML owner. Module reads use the immutable embedded catalog.
((channel, bootstrap) => {
    addEventListener("message", async ({data:{port, buffer, init}}) => {
        const ack = channel(buffer);
        const key = crypto.randomUUID();
        const prefix = "https://qml.invalid/rpc/" + key + "/worker/";
        const entry = JSON.parse(init);
        function read(url) {
            if (Object.hasOwn(entry.files, url)) return entry.files[url];
            if (!url.startsWith(prefix)) throw new Error("Unknown WorkerScript resource");
            const body = decodeURIComponent(url.slice(url.indexOf("/",prefix.length) + 1));
            if (body.length > 32 * 1024 * 1024 || new TextEncoder().encode(body).length > 32 * 1024 * 1024)
                throw new RangeError("WorkerScript output exceeds 32 MiB");
            port.postMessage({event:body});
            return ack.read(Infinity);
        }
        try {
            Object.defineProperty(globalThis, "__webSceneFetchText", {value:read, configurable:true});
            const vm = bootstrap(key);
            await vm.load(entry);
            if (vm.result() !== "done") throw new Error(vm.result());
            port.onmessage = async ({data}) => {
                try {
                    await vm.receive(JSON.parse(data));
                    port.postMessage({done:true});
                } catch (e) { port.postMessage({failure:String(e?.stack || e)}); }
            };
            port.postMessage({done:true});
        } catch (e) { port.postMessage({failure:String(e?.stack || e)}); }
    }, {once:true});
})
