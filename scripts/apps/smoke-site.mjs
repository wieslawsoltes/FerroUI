// Smoke run of a built browser site (the CI of an application, .github/workflows/app.yml): serves
// the site, opens it in headless Chrome (scripts/browser/harness.mjs) and waits until the view has
// drawn its first frame (the canvas is there and the splash is closed). Fails when that does not
// happen, when a request of the page fails or answers with an error status (a file missing from the
// site), when the page logs an error or throws, or when the splash screen is not closed and hidden
// after the first frame.
//
//   node scripts/apps/smoke-site.mjs <site> [--query ?a=b] [--screenshot file.png]
//        [--ignore <regular expression>]... [--timeout seconds]
//
// --ignore leaves out errors and failed requests whose text matches (for example a third-party
// service the page calls that CI cannot reach). CHROME=<path> chooses the browser.
import { open, sleep } from "../browser/harness.mjs";

const args = process.argv.slice(2);
let site, query = "", screenshot, timeout = 180;
const ignore = [];
for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
        case "--query": query = args[++i]; break;
        case "--screenshot": screenshot = args[++i]; break;
        case "--ignore": ignore.push(new RegExp(args[++i])); break;
        case "--timeout": timeout = Number(args[++i]); break;
        default:
            if (site || args[i].startsWith("--")) { console.error(`unknown argument: ${args[i]}`); process.exit(2); }
            site = args[i];
    }
}
if (!site) { console.error("usage: smoke-site.mjs <site> [--query ..] [--screenshot ..] [--ignore ..] [--timeout ..]"); process.exit(2); }

const page = await open(site, { query, width: 1280, height: 800, network: true, navigate: false });
const urls = new Map();
const failures = [];
page.on("Network.requestWillBeSent", (params) => urls.set(params.requestId, params.request.url));
page.on("Network.responseReceived", (params) => {
    if (params.response.status >= 400) { failures.push(`[http ${params.response.status}] ${params.response.url}`); }
});
page.on("Network.loadingFailed", (params) => {
    if (!params.canceled) { failures.push(`[failed: ${params.errorText}] ${urls.get(params.requestId) ?? params.requestId}`); }
});
await page.navigate();

let drawn = true;
try {
    await page.waitFor("document.querySelector('canvas') && (document.querySelector('.ferroui-splash') === null"
        + " || document.querySelector('.ferroui-splash').classList.contains('splash-close'))", timeout * 1000);
    await sleep(1500);
} catch (error) {
    drawn = false;
    failures.push(`[view] within ${timeout} s the page has no canvas, or the splash screen was not closed (the view drew no frame,`
        + " or the framework did not find the splash screen in the host element of the view)");
}
// Late errors of the first frames and of what the page loads after them.
await sleep(3000);
// The splash screen (`.ferroui-splash` in the host element, or `#ferroui-splash`) is closed by the
// framework at the first frame of the view (class `splash-close`); the stylesheet of the page then
// has to take it out of the view and of the accessibility tree (not displayed or not visible).
const splash = await page.evaluate(`(() => {
    const splash = document.querySelector(".ferroui-splash") ?? document.getElementById("ferroui-splash");
    if (!splash) { return null; }
    const style = getComputedStyle(splash);
    return { closed: splash.classList.contains("splash-close"), display: style.display, visibility: style.visibility };
})()`);
if (drawn && splash && !splash.closed) {
    failures.push("[splash] the view drew its first frame and the splash screen is not closed (no class splash-close)");
} else if (splash && splash.closed && splash.display !== "none" && splash.visibility !== "hidden") {
    failures.push(`[splash] the closed splash screen is still shown (display: ${splash.display}, visibility: ${splash.visibility})`);
}
if (screenshot) { await page.screenshot(screenshot); }
const problems = [...failures, ...page.errors].filter((line) => !ignore.some((pattern) => pattern.test(line)));
console.log(`requests: ${page.requests.length}, view drawn: ${drawn}`);
for (const line of page.log) { console.log(`  ${line}`); }
await page.close();
if (problems.length) {
    console.error(`::error::the site ${site} has ${problems.length} problem(s):`);
    for (const line of problems) { console.error(`  ${line}`); }
    process.exit(1);
}
console.log(`the site ${site} loads and draws without errors`);
