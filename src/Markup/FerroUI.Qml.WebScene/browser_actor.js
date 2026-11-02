// The application worker retains the watchdog and FIFO. A private port receives
// events; only the guest waits for acknowledgements, including ListModel.sync.
import {requireBrowserWorker} from "./browser_process.js";

const limit = 32 * 1024 * 1024;

export function spawnActor(program, channel, init, timeout, publish, stopped) {
    requireBrowserWorker();
    if (new TextEncoder().encode(init).length > limit)
        throw new RangeError("WorkerScript initialization exceeds 32 MiB");
    const url = URL.createObjectURL(new Blob([program], {type:"text/javascript"}));
    const port = new MessageChannel();
    const state = { url, port:port.port1, guest:null, closed:false, queue:[], bytes:0, busy:true, timer:0 };
    try {
        const buffer = new SharedArrayBuffer(8 + 64 * 1024);
        const acknowledgements = Function("return (" + channel + ")")()(buffer);
        const guest = state.guest = new Worker(url, {name:"FerroUI WorkerScript"});
        const fail = reason => {
            if (state.closed) return;
            disposeActor(state);
            stopped(String(reason));
        };
        const arm = () => {
            state.timer = setTimeout(() => fail("WorkerScript deadline exceeded"), timeout);
        };
        state.dispatch = () => {
            if (state.busy || !state.queue.length || state.closed) return;
            state.busy = true;
            arm();
            state.port.postMessage(state.queue[0].message);
        };
        let loading = true;
        state.port.onmessage = async ({data}) => {
            if (state.closed) return;
            try {
                if (typeof data.event === "string") {
                    const reply = await publish(data.event);
                    if (state.closed) return;
                    const status = JSON.parse(reply);
                    if (status.stopped) { fail(status.stopped); return; }
                    acknowledgements.write(reply, performance.now() + timeout);
                } else if (data.done === true) {
                    clearTimeout(state.timer);
                    if (loading) loading = false; else state.bytes -= state.queue.shift().size;
                    state.busy = false;
                    state.dispatch();
                } else { fail(data.failure || "Invalid WorkerScript response"); }
            } catch (e) { fail(e?.stack || e); }
        };
        guest.onerror = e => { e.preventDefault(); fail(e.message || "WorkerScript failed"); };
        state.port.onmessageerror = () => fail("Cannot decode WorkerScript event");
        arm();
        guest.postMessage({port:port.port2, buffer, init}, [port.port2]);
        return state;
    } catch (e) {
        disposeActor(state);
        throw e;
    }
}

export function sendActor(state, message, size) {
    if (state.closed) throw new Error("WorkerScript is no longer running");
    if (state.queue.length >= 256 || size > limit - state.bytes)
        throw new RangeError("WorkerScript input queue limit exceeded");
    state.queue.push({message, size});
    state.bytes += size;
    state.dispatch();
}

export function disposeActor(state) {
    if (state.closed) return;
    state.closed = true;
    clearTimeout(state.timer);
    state.port.onmessage = state.port.onmessageerror = null;
    state.port.close();
    state.guest?.terminate();
    state.queue.length = 0;
    state.bytes = 0;
    URL.revokeObjectURL(state.url);
}
