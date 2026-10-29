// Smoke run of a built browser site (the CI of an application, .github/workflows/app.yml): serves
// the site, opens it in headless Chrome (scripts/browser/harness.mjs) and waits until the view has
// drawn its first frame (the canvas is there and the splash is closed). Fails when that does not
// happen, when a request of the page fails or answers with an error status (a file missing from the
// site), or when the page logs an error or throws.
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
    failures.push(`[view] the view did not draw within ${timeout} s: ${String(error.message).split("\n")[0]}`);
}
// Late errors of the first frames and of what the page loads after them.
await sleep(3000);
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
