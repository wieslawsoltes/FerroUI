// Behaviour tests of the threaded build of the browser target on the thread_spawn example, in
// headless Chrome: a thread started with std::thread::spawn sends a value back to the page, and the
// page is cross-origin isolated either by the headers of the server or by the service worker of the
// threaded mode.
//
//   scripts/browser/setup.sh --threads && source .tools/env.sh
//   scripts/build-browser.sh thread_spawn --threads
//   node scripts/browser/tests/thread_spawn.test.mjs [<site directory>]     default: target/browser-threads/thread_spawn
//
// The page writes the state of the example (`threadSpawnState`, a line of `name=value` pairs) into
// the element `result`. See docs/porting/browser-platform.md, "Threads (opt-in)".
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const site = process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser-threads", "thread_spawn");

const RESULT = "document.getElementById('result')?.textContent ?? ''";
const WORKERS = "navigator.serviceWorker.getRegistrations().then((registrations) => registrations.length)";

// Waits for the thread to answer and checks the answer.
async function expectThreadResult(page) {
    await page.waitFor(`/state=(done|failed)/.test(${RESULT})`, 60000);
    const result = await page.evaluate(RESULT);
    assert(result === "atomics=true state=done value=500500 other_thread=true", `the page shows: ${result}`);
    assert(await page.evaluate("self.crossOriginIsolated") === true, "the page is not cross-origin isolated");
}

await run([
    ["a spawned thread sends its value back in a page isolated by the headers of the server", async () => {
        const page = await open(site, { isolated: true, width: 320, height: 200 });
        try {
            await expectThreadResult(page);
            assert(await page.evaluate(WORKERS) === 0, "a service worker was registered although the server sends the headers");
            assert(page.navigations.length === 1, `the page was loaded ${page.navigations.length} times: ${page.navigations.join(", ")}`);
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }],
    ["without the headers the service worker isolates the page after one reload", async () => {
        const page = await open(site, { width: 320, height: 200 });
        try {
            await expectThreadResult(page);
            assert(await page.evaluate(WORKERS) === 1, "the service worker of the threaded mode is not registered");
            assert(await page.evaluate("navigator.serviceWorker.controller?.scriptURL.endsWith('/ferroui-sw.js?coi=1')") === true,
                "the page is not controlled by ferroui-sw.js?coi=1");
            assert(page.navigations.length === 2, `the page was loaded ${page.navigations.length} times, not twice: ${page.navigations.join(", ")}`);
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }],
    ["a page that cannot be isolated says so and does not create the module", async () => {
        const page = await open(site, { query: "?ThreadsServiceWorker=false", width: 320, height: 200 });
        try {
            await page.waitFor("document.getElementById('ferroui-threads-message')", 30000);
            const message = await page.evaluate("document.getElementById('ferroui-threads-message').textContent");
            assert(message.includes("cross-origin isolated") && message.includes("Cross-Origin-Embedder-Policy"), `the message is: ${message}`);
            assert(await page.evaluate("self.crossOriginIsolated") === false, "the page is isolated without headers and without the worker");
            assert(await page.evaluate("typeof globalThis.threadSpawn") === "undefined", "the module was created in a page that is not isolated");
            assert(await page.evaluate(WORKERS) === 0, "a service worker was registered although the page turned it off");
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }]
]);
