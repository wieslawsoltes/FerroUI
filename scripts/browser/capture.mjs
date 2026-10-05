// Captures a screenshot of a built browser site in headless Chrome.
//
//   node scripts/browser/capture.mjs <site directory> <out.png> [--query "?a=b"] [--size 460x520] [--scale 2]
//
// Prints the console of the page, the size of its canvas and which context renders it.
import { open } from "./harness.mjs";

const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : fallback; };
const query = option("--query", "");
const [width, height] = option("--size", "460x520").split("x").map(Number);
const scale = Number(option("--scale", "1"));
const [site, out] = args;
if (!site || !out) { console.error("usage: capture.mjs <site directory> <out.png> [--query \"?a=b\"] [--size WxH] [--scale n]"); process.exit(2); }

const page = await open(site, { query, width, height, scale });
try {
    await page.waitForView();
    const info = await page.evaluate(`(c => JSON.stringify({ canvas: [c.width, c.height, c.clientWidth, c.clientHeight], webgl2: !!c.getContext("webgl2"), canvas2d: !!c.getContext("2d"), dpr: devicePixelRatio }))(document.querySelector("canvas"))`);
    await page.screenshot(out);
    for (const line of page.log) { console.log(line.slice(0, 400)); }
    console.log(info);
    console.log(`written ${out}`);
} finally {
    await page.close();
}
