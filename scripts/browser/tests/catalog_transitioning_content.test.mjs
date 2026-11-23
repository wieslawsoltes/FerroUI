// Run against a ControlCatalog browser build with the site directory as the first argument.
import assert from "node:assert/strict";
import path from "node:path";
import { open, sleep } from "../harness.mjs";
import { drive, inContent, visit, waitUntilReady } from "../catalog-pages.mjs";

const site = path.resolve(process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? "target", "browser/control-catalog-browser"));
const screenshot = process.argv[3];
const scheme = process.env.FERROUI_CATALOG_COLOR_SCHEME ?? "dark";
const page = drive(await open(site, {
    query: "?RenderingMode=WebGL2&Threads=false&RenderThread=false",
    width: 1080,
    height: 960,
    angle: process.env.FERROUI_BROWSER_ANGLE ?? "metal",
    navigate: false,
}));
const click = async text => page.clickElement(await page.find(text, inContent));
const selectTransition = async text => {
    const label = await page.find("Transition", inContent);
    await page.click(Math.round(label.x + label.width - 20), Math.round(label.y + label.height + 20));
    await click(text);
};
const photos = state => state.elements.filter(e => e.type === "Image" && e.source?.width === 640);
const photo = async height => page.element("a rendered gallery photo", e => e.type === "Image" && e.source?.height === height);

function hasPhotoPixels(picture, bounds) {
    const colours = new Set();
    for (let y = 1; y < 8; y++) {
        for (let x = 1; x < 8; x++) {
            const px = Math.round(bounds.x + bounds.width * x / 8);
            const py = Math.round(bounds.y + bounds.height * y / 8);
            colours.add(picture.pixel(px, py).join(","));
        }
    }
    return colours.size > 25;
}

async function settledPhoto(height, transition) {
    await page.until(`${transition} leaves one photo`, s => {
        const visible = photos(s);
        return visible.length === 1 && visible[0].source.height === height;
    });
    const bounds = await photo(height);
    for (const deadline = Date.now() + 2000; Date.now() < deadline; await sleep(50)) {
        await page.drawn();
        if (hasPhotoPixels(await page.screenshot(), bounds)) return;
    }
    assert.fail(`${transition} left the preview blank`);
}

try {
    await page.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: scheme }] });
    await page.send("Page.navigate", { url: page.url });
    await waitUntilReady(page, 60000);
    await visit(page, { section: "Media & Graphics", name: "TransitioningContentControl" }, 960);
    const initial = await photo(426);
    await page.find("Delicate Arch · 1 of 3", inContent);
    const picture = await page.screenshot(screenshot);
    assert.ok(hasPhotoPixels(picture, initial), "the preview must contain rendered photo pixels");
    assert.ok(!(await page.state()).elements.some(e => e.text === "FerroUI.Media.IImage"));

    // Editing the duration keeps the same transition selected and updates the active animation.
    const duration = await page.element("duration editor", e => e.type === "TextBox" && e.text === "500");
    await page.click(Math.round(duration.x + 12), Math.round(duration.y + duration.height / 2));
    await page.press("Home");
    await page.press("End", ["shift"]);
    await page.type("1500");
    await page.press("Enter");
    await page.element("duration is 1500 ms", e => e.type === "TextBox" && e.text === "1500");
    await click("Next");
    await page.find("Red deer · 2 of 3", inContent);
    await sleep(500);
    assert.equal(photos(await page.state()).length, 2, "crossfade should still be running after the original duration");
    await settledPhoto(512, "CrossFade");

    await selectTransition("Slide horizontally");
    await click("Off");
    await page.find("On", inContent);
    await click("Previous");
    await page.find("Delicate Arch · 1 of 3", inContent);
    await settledPhoto(426, "reverse horizontal slide");

    for (const [transition, height] of [["Slide vertically", 512], ["Composite", 480], ["Custom", 426]]) {
        await selectTransition(transition);
        await click("Next");
        await settledPhoto(height, transition);
    }

    await selectTransition("None");
    await click("Previous");
    await page.find("Maple leaf · 3 of 3", inContent);
    assert.deepEqual(photos(await page.state()).map(e => e.source.height), [480], "the previous button wraps immediately with no transition");
    await settledPhoto(480, "None");
    assert.deepEqual(page.errors, []);
    console.log("CATALOG_TRANSITIONING_CONTENT_PASS", scheme);
} finally {
    await page.close();
}
