const WRITE = 0;
const PULL = 0;
const ERROR = 1;
const ABORT = 1;
const CLOSE = 2;

class MessagePortSource implements UnderlyingSource {
    private controller?: ReadableStreamController<any>;

    constructor (private readonly port: MessagePort) {
        this.port.onmessage = evt => this.onMessage(evt.data);
    }

    start (controller: ReadableStreamController<any>) {
        this.controller = controller;
    }

    // Differs from the original, which asks for the next chunk only after it received one: the
    // writer of the page waits for the first request before it writes, so nothing was ever sent.
    // The stream asks whenever it wants data, which includes the first chunk.
    pull () {
        this.port.postMessage({ type: PULL });
    }

    cancel (reason: Error) {
    // Firefox can notify a cancel event, chrome can't
    // https://bugs.chromium.org/p/chromium/issues/detail?id=638494
        this.port.postMessage({ type: ERROR, reason: reason.message });
        this.port.close();
    }

    onMessage (message: { type: number; chunk: Uint8Array; reason: any }) {
    // enqueue() will call pull() if needed when there's no backpressure
        if (!this.controller) {
            return;
        }
        if (message.type === WRITE) {
            this.controller.enqueue(message.chunk);
        }
        if (message.type === ABORT) {
            this.controller.error(message.reason);
            this.port.close();
        }
        if (message.type === CLOSE) {
            this.controller.close();
            this.port.close();
        }
    }
}

self.addEventListener("install", () => {
    (self as any).skipWaiting();
});

self.addEventListener("activate", event /* ExtendableEvent */ => {
    (event as any).waitUntil((self as any).clients.claim());
});

(self as any).map = new Map();

// This should be called once per download
// Each event has a dataChannel that the data will be piped through
globalThis.addEventListener("message", evt => {
    const data = evt.data;
    if (data.url && data.readablePort) {
        data.rs = new ReadableStream(
            new MessagePortSource(evt.data.readablePort),
            new CountQueuingStrategy({ highWaterMark: 4 })
        );
        const map = (self as any).map;
        map.set(data.url, data);
    }
});

// The second task of the worker, which is not in the original: it makes the site cross-origin
// isolated on a host that cannot set response headers (GitHub Pages). It is switched on by the
// address the worker is registered with, `ferroui-sw.js?coi=1`; without the parameter the worker
// answers the downloads above and nothing else.
//
// A module built with threads needs `SharedArrayBuffer`, which a browser only gives to a page whose
// response carries `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy:
// require-corp`. A service worker answers the requests of the pages in its scope, the request for
// the page itself included, so it can fetch each response and hand it on with the headers added.
// The page that registers the worker was loaded without it and has to be loaded once more
// (`ensureCrossOriginIsolated` of ferroui-threads.js does both). A scope has one service worker,
// which is why this is not a worker of its own: the page of a threaded site and the platform
// (`register_ferro_service_worker`) register the same address.
//
// Limits: a resource of another origin that is fetched without CORS comes back opaque, its headers
// cannot be changed, and the page then refuses it unless its server sends
// `Cross-Origin-Resource-Policy: cross-origin`. A reload that bypasses the service worker (a forced
// reload) gives a page that is not isolated.
const isolates = new URL(self.location.href).searchParams.get("coi") === "1";

function isolate (source?: HeadersInit): Headers {
    const headers = new Headers(source);
    headers.set("Cross-Origin-Opener-Policy", "same-origin");
    headers.set("Cross-Origin-Embedder-Policy", "require-corp");
    // The resources of the site itself may be embedded by the isolated page.
    headers.set("Cross-Origin-Resource-Policy", "cross-origin");
    return headers;
}

globalThis.addEventListener("fetch", evt => {
    const request: Request = (evt as any).request;
    const url = request.url;
    const map = (self as any).map;
    const data = map.get(url);
    if (data) {
        map.delete(url);
        // The download is loaded into a frame of the page: in an isolated page the response of a
        // frame needs the headers like any other.
        (evt as any).respondWith(new Response(data.rs, {
            headers: isolates ? isolate(data.headers) : data.headers
        }));
        return null;
    }
    if (!isolates) return null;
    // A request the browser makes for its own tools; fetching it from a worker is an error.
    if (request.cache === "only-if-cached" && request.mode !== "same-origin") return null;
    (evt as any).respondWith(fetch(request).then(response => {
        // An opaque response (status 0) can be passed on but not read or rebuilt.
        if (response.status === 0) return response;
        // A response of these statuses has no body, and building one with a body is an error.
        const body = [101, 204, 205, 304].includes(response.status) ? null : response.body;
        return new Response(body, {
            status: response.status,
            statusText: response.statusText,
            headers: isolate(response.headers)
        });
    }));
    return null;
});

export {};
