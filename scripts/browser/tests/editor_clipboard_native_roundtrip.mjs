import { assert, open } from "../harness.mjs";

const source = "abcdef\nahijkl\nanopqr";
const target = "12\n34\n56";
const state = "JSON.parse(globalThis.editorDemo.editorState())";

export async function checkNativeBrowserClipboard(site, mode, {
    browser, platform, command, rectangleModifier, keyOptions, nativePaste, withNativeCopy,
    refocus = false,
}) {
    const page = await open(site, {
        query: `?RenderingMode=${mode}`, width: 1180, height: 800,
        headless: false, chromeArgs: mode === "Software2D" ? ["--disable-gpu"] : [],
    });
    try {
        await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
        await page.setPermission("clipboard-read", "granted");
        await page.setPermission("clipboard-write", "granted");
        await page.waitFor("document.documentElement.dataset.editorReady === 'true'", 180_000);
        await page.waitForView();

        const press = (key, modifiers) => page.press(key, modifiers, keyOptions);
        const waitText = text => page.waitFor(`${state}.text.replace(/\\r\\n/g, "\\n") === ${JSON.stringify(text)}`, 15_000);
        const setText = async text => {
            const area = (await page.evaluate(state)).elements.find(element => element.hit && element.type.endsWith("TextArea"));
            assert(area, "Browser editor has no text area hit target");
            await page.click(area.x + area.width / 3, area.y + 12);
            await press("a", [command]);
            await page.waitFor(`${state}.selectedText === ${state}.text`, 10_000);
            const composition = await page.send("Input.imeSetComposition", { text, selectionStart: text.length, selectionEnd: text.length });
            assert(!composition.error, "Browser composition could not start");
            assert(await page.evaluate("globalThis.editorDemo.editorRequestFrame()"), "Editor has no compositor");
            await page.waitFor("globalThis.editorDemo.editorFrameDrawn()", 30_000);
            await page.send("Input.insertText", { text });
            await waitText(text);
        };
        const checkClipboard = async (marker, expected) => {
            const identifier = `FerroUI.Edit.${marker}`;
            const type = JSON.stringify(`web application/frn-fmt.text.${Buffer.from(identifier, "ascii").toString("hex")}`);
            await page.waitFor(`navigator.clipboard.read().then(async items => {
                const item = items.find(item => item.types.includes(${type}) && item.types.includes("text/plain"));
                return item && await (await item.getType(${type})).text() === "1" &&
                    (await (await item.getType("text/plain")).text()).replace(/\\r\\n/g, "\\n") === ${JSON.stringify(expected)};
            })`, 15_000);
        };
        const pressClipboard = async key => {
            let timeout;
            try {
                await Promise.race([
                    press(key, [command]),
                    new Promise((_, reject) => {
                        timeout = setTimeout(() => reject(new Error(`${browser} did not acknowledge ${command}+${key}`)), 10_000);
                    }),
                ]);
            } finally {
                clearTimeout(timeout);
            }
        };

        // Distinct payloads show that the native copy replaces the preceding
        // browser clipboard item.
        for (const [shape, marker, browserCopy, nativeCopy, browserPaste] of [
            ["rectangle", "RectangularSelection", "bcde\nhijk\nnopq", "vwxy\nijkl\nopqr", "1vwxy2\n3ijkl4\n5opqr6"],
            ["line", "WholeLineSelection", "abcdef\n", "uvwxyz\n", "12\nuvwxyz\n34\n56"],
        ]) {
            const rectangle = shape === "rectangle";
            await setText(source);
            await press("Home", [command]);
            if (rectangle) {
                await press("ArrowRight");
                for (let i = 0; i < 4; i++) await press("ArrowRight", [rectangleModifier, "shift"]);
                for (let i = 0; i < 2; i++) await press("ArrowDown", [rectangleModifier, "shift"]);
            }
            await pressClipboard("c");
            await checkClipboard(marker, browserCopy);
            await nativePaste(shape);
            console.log(`${browser} to native ${platform}: ${marker}, document and undo verified`);

            if (refocus) await page.send("Page.bringToFront");
            await setText(target);
            await withNativeCopy(shape, async () => {
                if (refocus) await page.send("Page.bringToFront");
                await checkClipboard(marker, nativeCopy);
                await press("Home", [command]);
                if (!rectangle) await press("ArrowDown");
                await press("ArrowRight");
                await pressClipboard("v");
                await waitText(browserPaste);
                await press("z", [command]);
                await waitText(target);
            });
            console.log(`Native ${platform} to ${browser}: ${marker}, document and undo verified`);
        }
    } finally {
        await page.close();
    }
}
