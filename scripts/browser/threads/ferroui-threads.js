// The check a host page of a threaded build makes before it creates the module.
//
// A module built with `scripts/build-browser.sh --threads` has a shared memory, which exists only in
// a cross-origin isolated page (`self.crossOriginIsolated`). Without the check the module fails
// while it is created, with an error about `SharedArrayBuffer` that says nothing of the cause.
//
//   import { ensureCrossOriginIsolated } from "./ferroui-threads.js";
//   if (await ensureCrossOriginIsolated()) { const runtime = await createRuntime(); ... }
//
// The build copies this file next to the host page. The service worker is the one of the platform,
// ferroui-sw.js (webapp/modules/ferroui-sw.ts), which every site has: registered with `?coi=1` it
// also adds the headers that isolate the page. An application that sets
// `register_ferro_service_worker` registers the same address in a threaded module, so that the two
// registrations are one worker.

// Set for the one reload that follows the registration of the service worker, so that a page that
// is still not isolated after it shows the message instead of reloading for ever.
const RELOADED = "ferroui-threads-reloaded";

// The lock a document holds from the moment it asks for the reload: see `reload`.
const RELOAD_LOCK = "ferroui-threads-reload";

function flag(action) {
    // Session storage is not available in every mode of every browser.
    try { return action(globalThis.sessionStorage); } catch { return null; }
}

// Shows why the page cannot run, in place of the content of `element`.
function showMessage(element, reason) {
    const text = "This page needs threads, and threads need a cross-origin isolated page. " + reason
        + " Serve the site with the headers \"Cross-Origin-Opener-Policy: same-origin\" and"
        + " \"Cross-Origin-Embedder-Policy: require-corp\", or open it over HTTPS or from localhost in a"
        + " window that allows service workers.";
    const box = document.createElement("div");
    box.id = "ferroui-threads-message";
    box.setAttribute("role", "alert");
    box.style.cssText = "font-family: sans-serif; margin: 2rem; max-width: 40rem; line-height: 1.4";
    box.textContent = text;
    (element ?? document.body).replaceChildren(box);
    console.warn(text);
}

// Resolves to true when the page is cross-origin isolated. When it is not, registers the service
// worker that adds the headers and reloads the page once (the promise then never resolves: the page
// is going away); when that is not possible or did not help, shows a message in `element` (default:
// the body) and resolves to false.
//
// `serviceWorker` is the address of the worker script, relative to the page; null or `register:
// false` leaves the worker out, for a site whose host sends the headers.
export async function ensureCrossOriginIsolated({ serviceWorker = "./ferroui-sw.js?coi=1", register = true, element } = {}) {
    if (globalThis.crossOriginIsolated) {
        flag((storage) => storage.removeItem(RELOADED));
        return true;
    }
    if (!register || !serviceWorker) {
        showMessage(element, "The host does not send the headers that isolate it, and the service worker that would add them is turned off.");
        return false;
    }
    if (!globalThis.isSecureContext) {
        showMessage(element, "The page was not loaded over HTTPS or from localhost, so it cannot register the service worker that would add the headers.");
        return false;
    }
    if (!("serviceWorker" in navigator)) {
        showMessage(element, "This window has no service workers (a private window of some browsers, or a setting), so the headers cannot be added in the page.");
        return false;
    }
    if (flag((storage) => storage.getItem(RELOADED)) !== null) {
        flag((storage) => storage.removeItem(RELOADED));
        showMessage(element, "The service worker that adds the headers is registered, but the page is still not isolated after a reload (a forced reload bypasses the worker: reload normally).");
        return false;
    }
    try {
        await navigator.serviceWorker.register(serviceWorker);
        // Resolves once a worker of the scope is active; the next load of the page goes through it.
        await navigator.serviceWorker.ready;
    } catch (error) {
        showMessage(element, `The service worker that would add the headers could not be registered (${error?.message ?? error}).`);
        return false;
    }
    // Without session storage the reload cannot be counted and could repeat for ever, so the
    // user is asked to reload instead.
    if (flag((storage) => { storage.setItem(RELOADED, "1"); return true; }) !== true) {
        showMessage(element, "The service worker that adds the headers is now registered: reload the page.");
        return false;
    }
    return reload();
}

// Reloads the page for the service worker and never resolves. The document that asks is finished:
// it never creates the module. The page that replaces it is isolated and this one is not, so the
// browser puts the two in different groups of contexts, and that makes this document one the
// back/forward cache may keep; Chrome has been seen to bring it back, as it was left, as the answer
// to a later reload of the page. Two things against that: a lock that is never released, which
// keeps a document out of that cache (shared, so that two pages of the site do not wait for each
// other); and, where a browser keeps it all the same, a document that is shown again from the cache
// reloads, this time through the service worker.
function reload() {
    try { navigator.locks?.request(RELOAD_LOCK, { mode: "shared" }, () => new Promise(() => { }))?.catch(() => { }); } catch { }
    globalThis.addEventListener("pageshow", (event) => { if (event.persisted) { globalThis.location.reload(); } });
    globalThis.location.reload();
    return new Promise(() => { });
}
