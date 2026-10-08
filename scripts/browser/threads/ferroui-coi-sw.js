// The service worker of the threaded mode: it makes a site cross-origin isolated on a host that
// cannot set response headers (GitHub Pages).
//
// Threads in a page need `SharedArrayBuffer`, which a browser only gives to a page whose response
// carries `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`.
// A service worker answers the requests of the pages in its scope, the request for the page itself
// included, so it can fetch each response and hand it on with the two headers added. The page that
// registers the worker was loaded without it and has to be loaded once more
// (`ensureCrossOriginIsolated` of ferroui-threads.js does both).
//
// `scripts/build-browser.sh --threads` copies this file to the root of the site: the scope of a
// service worker is the directory of its script. A host that sends the headers itself does not need
// the worker, and ferroui-threads.js does not register it there.
//
// Limits: a resource of another origin that is fetched without CORS comes back opaque, its headers
// cannot be changed, and the page then refuses it unless its server sends
// `Cross-Origin-Resource-Policy: cross-origin`. A reload that bypasses the service worker (a forced
// reload) gives a page that is not isolated.

// Take over at once: without this a new version of the worker waits until every page of the old
// one is closed.
self.addEventListener("install", () => { self.skipWaiting(); });
self.addEventListener("activate", (event) => { event.waitUntil(self.clients.claim()); });

self.addEventListener("fetch", (event) => {
    const request = event.request;
    // A request the browser makes for its own tools; fetching it from a worker is an error.
    if (request.cache === "only-if-cached" && request.mode !== "same-origin") { return; }
    event.respondWith(fetch(request).then((response) => {
        // An opaque response (status 0) can be passed on but not read or rebuilt.
        if (response.status === 0) { return response; }
        const headers = new Headers(response.headers);
        headers.set("Cross-Origin-Opener-Policy", "same-origin");
        headers.set("Cross-Origin-Embedder-Policy", "require-corp");
        // The resources of the site itself may be embedded by the isolated page.
        headers.set("Cross-Origin-Resource-Policy", "cross-origin");
        return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
    }));
});
