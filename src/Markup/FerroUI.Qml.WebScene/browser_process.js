// The blocking reader lives in the application worker. The browser UI thread
// never waits on Atomics. The guest's event loop receives one wake per top-level
// command. Nested commands and host replies stay within its current call stack.
export function requireBrowserWorker() {
    if (typeof DedicatedWorkerGlobalScope === "undefined" ||
        !(globalThis instanceof DedicatedWorkerGlobalScope) || !globalThis.crossOriginIsolated ||
        typeof SharedArrayBuffer !== "function" || typeof Worker !== "function")
        throw new Error("QML browser scripting requires a cross-origin-isolated dedicated application worker");
}

export async function spawnBrowserWorker(program, channelSource, timeout) {
    requireBrowserWorker();
    const makeChannel = Function("return (" + channelSource + ")")();
    const input = new SharedArrayBuffer(8 + 64 * 1024);
    const output = new SharedArrayBuffer(8 + 64 * 1024);
    const url = URL.createObjectURL(new Blob([program], {type:"text/javascript"}));
    let guest;
    try {
        guest = new Worker(url, {name:"FerroUI QML script"});
        await new Promise((resolve, reject) => {
            const timer = setTimeout(() => reject(new Error("QML browser worker startup deadline exceeded")), timeout);
            guest.onmessage = ({data}) => {
                clearTimeout(timer);
                if (data.ready) resolve(); else reject(new Error(data.failure || "Invalid QML worker handshake"));
            };
            guest.onerror = event => {
                clearTimeout(timer);
                reject(new Error(event.message || "QML browser worker startup failed"));
            };
            guest.postMessage([input, output]);
        });
        return {guest, url, input:makeChannel(input), output:makeChannel(output)};
    } catch (error) {
        guest?.terminate();
        URL.revokeObjectURL(url);
        throw error;
    }
}
export function sendFrame(state, text, begin, milliseconds) {
    if (begin) state.guest.postMessage(null);
    state.input.write(text, performance.now() + milliseconds);
}
export function readFrame(state, milliseconds) {
    return state.output.read(performance.now() + milliseconds);
}
export function disposeBrowserWorker(state) {
    state.guest.terminate();
    URL.revokeObjectURL(state.url);
}
