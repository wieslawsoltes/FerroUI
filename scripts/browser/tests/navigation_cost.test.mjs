// What a page switch of the ControlCatalog site costs the GPU process: the masks its frames upload
// (docs/porting/desktop-performance.md, section 9.56). The drawer entries Buttons, CheckBox and
// Home are clicked in turn with real pointer events, three times over, and the WebGL work of the
// frames of every switch is counted on the thread of the page (../navigation-cost.mjs). A frame
// drawn under a clip of several rectangles has Skia's WebGL backend rasterise a mask on the
// processor and upload it for every draw, which made a page switch several times as expensive as
// the same frame under one rectangle; the counts do not depend on the machine, so they are checked
// against a budget here (`BUDGET` in ../navigation-cost.mjs), and the times are only printed.
//
// Run against the ControlCatalog site built from this checkout, with or without threads:
//   scripts/build-browser.sh control-catalog-browser
//   node scripts/browser/tests/navigation_cost.test.mjs [site directory]
// or against a deployed site, with the query that keeps its frames on the thread of the page:
//   node scripts/browser/tests/navigation_cost.test.mjs "https://<host>/<path>/?Threads=false"
import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { run, assert } from "../harness.mjs";
import { measure, overBudget, table, BUDGET, ROUTE } from "../navigation-cost.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const address = /^https?:/.test(process.argv[2] ?? "");
const site = address ? process.argv[2] : path.resolve(process.argv[2]
    ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser", "control-catalog-browser"));
if (!address && !fs.existsSync(path.join(site, "index.html"))) {
    console.error(`no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser`);
    process.exit(2);
}
const measured = (text) => console.log(`      measured: ${text}`);

let result;
const switches = async () => {
    if (!result) {
        result = await measure(site, { rounds: 3 });
        for (const line of table(result)) { measured(line); }
    }
    return result;
};

await run([
    ["the frames of a page switch are drawn with WebGL on the thread of the page, where they are counted", async () => {
        const { rendering, navigations } = await switches();
        assert(rendering.kind === "webgl", `the site renders with ${JSON.stringify(rendering)}`);
        assert(navigations.length === 3 * ROUTE.length, `${navigations.length} switches were measured`);
        for (const navigation of navigations) {
            assert(navigation.frames >= 1 && navigation.draws >= 1,
                `the switch to ${navigation.page} drew nothing: ${JSON.stringify(navigation)}`);
        }
    }],
    ["a page switch uploads the masks of its paths and glyphs, not one for every draw", async () => {
        const { first, again } = await switches();
        for (const [visit, pages] of Object.entries(BUDGET)) {
            for (const text of Object.keys(pages)) {
                assert((visit === "first" ? first : again)[text], `the switch to ${text} (${visit}) was not measured`);
            }
        }
        const over = overBudget(await switches());
        assert(over.length === 0, `over the budget:\n        ${over.join("\n        ")}`);
    }],
    ["every single switch keeps the budget of its page, not only their median", async () => {
        const { navigations } = await switches();
        const over = navigations.flatMap((navigation, index) => overBudget({
            first: navigation.visit === "first" ? { [navigation.page]: navigation } : {},
            again: navigation.visit === "again" ? { [navigation.page]: navigation } : {},
        }).map((line) => `switch ${index + 1}: ${line}`));
        assert(over.length === 0, `over the budget:\n        ${over.join("\n        ")}`);
    }],
    ["a page shown again links no program and uploads no more than its first visit", async () => {
        const { first, again } = await switches();
        for (const [text, numbers] of Object.entries(again)) {
            assert(numbers.programs === 0, `the switch back to ${text} linked ${numbers.programs} programs`);
            if (first[text]) {
                assert(numbers.masks <= first[text].masks + 4,
                    `the switch back to ${text} uploaded ${numbers.masks} masks, its first visit ${first[text].masks}`);
            }
        }
    }],
]);
