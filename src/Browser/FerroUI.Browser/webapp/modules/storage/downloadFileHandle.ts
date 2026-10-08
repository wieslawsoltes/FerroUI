// The file handle behind the save picker of the polyfill: a file that can only be written, and whose
// content leaves the page as a download. A TypeScript form of `src/adapters/downloader.js` of
// native-file-system-adapter (MIT, Copyright (c) 2019 Jimmy Wärting), which the storage provider uses
// in place of the original for one difference, in `createWritable`: the frame that starts the
// download is navigated when the service worker has asked for the first chunk, not right after the
// stream was posted to it.
//
// The original posts the stream to the worker and navigates the frame in the same turn. The message
// and the navigation reach the worker by different routes, and nothing orders them: when the request
// of the frame arrives first, the worker does not know the address yet, the request goes to the
// server, which has no such file, and the download never starts; the stream is registered a moment
// later and nobody asks for it. The worker asks for the first chunk as soon as it has made the stream
// (`pull` of ferroui-sw.ts), so that request is the proof that it knows the address.

const WRITE = 0;
const PULL = 0;
const ERROR = 1;
const ABORT = 1;
const CLOSE = 2;

const GONE: [string, string] = ["A requested file or directory could not be found at the time an operation was processed.", "NotFoundError"];

function isSafari (): boolean {
    const scope = globalThis as any;
    return /constructor/i.test(scope.HTMLElement) || !!scope.safari || !!scope.WebKitPoint;
}

class MessagePortSink implements UnderlyingSink<Uint8Array> {
    private controller?: WritableStreamDefaultController;
    private readyPromise!: Promise<void>;
    private readyResolve!: () => void;
    private readyReject!: (reason: any) => void;
    private readyPending = false;
    private requestedResolve!: () => void;

    // Resolved by the first request of the other side for a chunk; never when the stream fails before.
    public readonly requested = new Promise<void>(resolve => { this.requestedResolve = resolve; });

    constructor (private readonly port: MessagePort) {
        port.onmessage = event => this.onMessage(event.data);
        this.resetReady();
    }

    async start (controller: WritableStreamDefaultController) {
        this.controller = controller;
        // Apply initial backpressure
        return await this.readyPromise;
    }

    async write (chunk: Uint8Array) {
        const message = { type: WRITE, chunk };

        // Send chunk
        this.port.postMessage(message, [chunk.buffer]);

        // Assume backpressure after every write, until sender pulls
        this.resetReady();

        // Apply backpressure
        return await this.readyPromise;
    }

    close () {
        this.port.postMessage({ type: CLOSE });
        this.port.close();
    }

    abort (reason: any) {
        this.port.postMessage({ type: ABORT, reason });
        this.port.close();
    }

    private onMessage (message: { type: number; reason?: any }) {
        if (message.type === PULL) {
            this.readyResolve();
            this.readyPending = false;
            this.requestedResolve();
        }
        if (message.type === ERROR) this.onError(message.reason);
    }

    private onError (reason: any) {
        this.controller?.error(reason);
        if (!this.readyPending) this.resetReady();
        this.readyPromise.catch(() => {});
        this.readyReject(reason);
        this.readyPending = false;
        this.port.close();
    }

    private resetReady () {
        this.readyPromise = new Promise((resolve, reject) => {
            this.readyResolve = resolve;
            this.readyReject = reject;
        });
        this.readyPending = true;
    }
}

export class DownloadFileHandle {
    public readonly kind = "file";

    constructor (public readonly name: string = "unkown") { }

    async getFile (): Promise<File> {
        throw new DOMException(...GONE);
    }

    async isSameEntry (other: any) {
        return this === other;
    }

    async createWritable (options: { size?: number } = {}): Promise<WritableStreamDefaultWriter<any>> {
        const registration = await navigator.serviceWorker?.getRegistration();
        const ts = new TransformStream();
        const sink = ts.writable;

        if (isSafari() || !registration) {
            const link = document.createElement("a");
            link.download = this.name;
            let chunks: Blob[] = [];
            void ts.readable.pipeTo(new WritableStream({
                write (chunk) {
                    chunks.push(new Blob([chunk]));
                },
                close () {
                    const blob = new Blob(chunks, { type: "application/octet-stream; charset=utf-8" });
                    chunks = [];
                    link.href = URL.createObjectURL(blob);
                    link.click();
                    setTimeout(() => URL.revokeObjectURL(link.href), 10000);
                }
            }));
        } else {
            const worker = registration.active as ServiceWorker;
            const channel = new MessageChannel();
            const readablePort = channel.port1;
            const portSink = new MessagePortSink(channel.port2);
            const writable = new WritableStream(portSink);
            // Make filename RFC5987 compatible
            const fileName = encodeURIComponent(this.name).replace(/['()]/g, escape).replace(/\*/g, "%2A");
            const url = registration.scope + fileName;
            const headers = {
                "content-disposition": "attachment; filename*=UTF-8''" + fileName,
                "content-type": "application/octet-stream; charset=utf-8",
                ...(options.size ? { "content-length": options.size } : {})
            };

            const keepAlive = setTimeout(() => worker.postMessage(0), 10000);

            void ts.readable.pipeThrough(new TransformStream({
                async transform (chunk, ctrl) {
                    if (chunk instanceof Uint8Array) {
                        ctrl.enqueue(chunk);
                        return;
                    }
                    const reader = (new Response(chunk).body as ReadableStream<Uint8Array>).getReader();
                    for (let part = await reader.read(); !part.done; part = await reader.read()) {
                        ctrl.enqueue(part.value);
                    }
                }
            })).pipeTo(writable).finally(() => {
                clearTimeout(keepAlive);
            });

            // Transfer the stream to service worker
            worker.postMessage({ url, headers, readablePort }, [readablePort]);

            // Trigger the download with a hidden iframe, once the worker answers for the address.
            void portSink.requested.then(() => {
                const iframe = document.createElement("iframe");
                iframe.hidden = true;
                iframe.src = url;
                document.body.appendChild(iframe);
            });
        }

        return sink.getWriter();
    }
}
