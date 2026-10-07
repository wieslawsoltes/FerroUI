// Processor time of scrolling a page of the ControlCatalog site in headless Chrome, with a CPU profile.
//
//   node scripts/browser/scroll-profile.mjs <site directory> [options]
//
//   --group <text>          entry of the navigation drawer that holds the page (default "Collections & Data")
//   --page <text>           entry of the page below it, and its header (default "TableView")
//   --steps <n>             wheel events, the first half down and the second half up (default 60)
//   --delta <pixels>        distance of one wheel event (default 120)
//   --cpu-profile <file>    write the CPU profile (.cpuprofile) of the scrolling
//   --screenshot <file>     write a PNG of the page after the scrolling
//   --json                  print the result as JSON
//
// The site is the one scripts/build-browser.sh control-catalog-browser writes. The page is opened through
// the drawer with real pointer events (the `catalogState` export of the host names what is shown), the
// profiler of the browser samples while the wheel events are sent 50 ms apart at the middle of the content,
// and the time the main thread was neither idle nor inside the browser itself is printed: the time of the
// module and of its script. For a profile with the names of the functions, link the module with its name
// section (`cargo rustc --profile browser --target wasm32-unknown-emscripten -p control-catalog-browser
// --bin control-catalog-browser -- -Clink-arg=--profiling-funcs`) and put it, with its script, in place
// of the module of the site. The figures depend on the machine and its load: compare two sites by
// measuring them alternately. No dependencies.
import fs from "node:fs";
import path from "node:path";
import { open, sleep } from "./harness.mjs";

const args = process.argv.slice(2);
const valued = new Set(["--group", "--page", "--steps", "--delta", "--cpu-profile", "--screenshot"]);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const site = args.find((a, i) => !a.startsWith("--") && !valued.has(args[i - 1]));
if (!site || !fs.existsSync(path.join(site, "index.html"))) {
    console.error("usage: node scripts/browser/scroll-profile.mjs <site directory> [--group g] [--page p] [--steps n] [--delta d] [--cpu-profile f] [--screenshot f] [--json]");
    process.exit(2);
}
const group = option("--group", "Collections & Data");
const name = option("--page", "TableView");
const steps = Number(option("--steps", "60"));
const delta = Number(option("--delta", "120"));
const profileFile = option("--cpu-profile");
const screenshotFile = option("--screenshot");

const SIZE = { width: 1280, height: 900 };
// The drawer is 260 pixels wide.
const DRAWER_EDGE = 260;

const page = await open(site, { query: "?RenderingMode=WebGL2", ...SIZE });
try {
    await page.waitFor(`(() => {
        const canvas = document.querySelector("#out canvas");
        const splash = document.querySelector("#out .ferroui-splash");
        return canvas && canvas.width > 0 && (!splash || splash.classList.contains("splash-close"))
            && globalThis.controlCatalog && controlCatalog.catalogState() !== "null";
    })()`, 180_000);
    const state = async () => JSON.parse(await page.evaluate("controlCatalog.catalogState()"));
    const until = async (description, predicate) => {
        let last;
        for (const end = Date.now() + 30_000; Date.now() < end;) {
            last = await state();
            if (predicate(last)) { return last; }
            await sleep(100);
        }
        throw new Error(`timed out waiting until ${description}; the page shown is ${JSON.stringify(last?.page)}`);
    };
    const click = async (text) => {
        const shown = (s) => s.elements.find((e) => e.text === text && e.hit && e.x + e.width <= DRAWER_EDGE);
        const element = shown(await until(`"${text}" is in the drawer`, (s) => shown(s)));
        await page.click(Math.round(element.x + element.width / 2), Math.round(element.y + element.height / 2));
    };
    await click(group);
    await until(`the page "${group}" is shown`, (s) => s.page === group && !s.navigating);
    await click(name);
    await until(`the page "${name}" is shown`, (s) => s.page === name && !s.navigating);
    await sleep(1500);

    await page.send("Profiler.enable");
    await page.send("Profiler.setSamplingInterval", { interval: 200 });
    await page.send("Profiler.start");
    const x = Math.round((DRAWER_EDGE + SIZE.width) / 2), y = Math.round(SIZE.height / 2);
    for (let i = 0; i < steps; i++) {
        await page.send("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY: i < steps / 2 ? delta : -delta, pointerType: "mouse" });
        await sleep(50);
    }
    await sleep(300);
    const { profile } = await page.send("Profiler.stop");
    if (profileFile) { fs.writeFileSync(profileFile, JSON.stringify(profile)); }
    if (screenshotFile) { await page.screenshot(screenshotFile); }

    // The time of every node of the profile but the idle time and the time inside the browser itself.
    const names = new Map(profile.nodes.map((node) => [node.id, node.callFrame.functionName]));
    let total = 0, busy = 0;
    profile.samples.forEach((id, i) => {
        const microseconds = profile.timeDeltas[i];
        total += microseconds;
        if (!["(idle)", "(program)", "(root)"].includes(names.get(id))) { busy += microseconds; }
    });
    const result = { page: name, steps, delta, busyMs: Math.round(busy / 100) / 10, perStepMs: Math.round(busy / steps / 10) / 100, totalMs: Math.round(total / 1000) };
    if (args.includes("--json")) { console.log(JSON.stringify(result)); }
    else { console.log(`${site}: ${name}, ${steps} wheel events of ${delta} px: ${result.busyMs} ms of processor time (${result.perStepMs} ms per event) in ${result.totalMs} ms`); }
} finally {
    await page.close();
}
