import { assert, sleep } from "../harness.mjs";

export async function checkClipboardRoundTrip(page, {
    platform, focusNative, nativeKeys, showBrowser = () => {}, copyFromBrowser = () => page.press("c", ["ctrl"]),
    seedClipboard, clipboardTypes, readClipboard, readClipboardText,
}) {
    const normalize = text => text.replace(/\r\n/g, "\n");
    await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
    await page.setPermission("clipboard-read", "granted");
    await page.setPermission("clipboard-write", "granted");
    await page.waitFor("document.documentElement.dataset.editorReady === 'true'", 180_000);
    await page.waitForView();
    const state = "JSON.parse(globalThis.editorDemo.editorState())";
    const settle = async () => {
        assert(await page.evaluate("globalThis.editorDemo.editorRequestFrame()"), "editor has no compositor");
        await page.waitFor("globalThis.editorDemo.editorFrameDrawn()", 30_000);
    };
    await settle();

    const focusBrowser = async () => {
        await showBrowser();
        await page.send("Page.bringToFront");
        const area = (await page.evaluate(state)).elements.find(element => element.hit && element.type.endsWith("TextArea"));
        assert(area, "browser editor has no text area hit target");
        await page.click(area.x + area.width / 3, area.y + 12);
        await page.waitFor(`${state}.focus?.endsWith("TextArea")`, 10_000);
    };
    const setNativeText = async text => {
        await focusNative();
        await seedClipboard(text);
        for (let attempt = 0; normalize(readClipboardText()) !== text; attempt++) {
            assert(attempt < 30, "The desktop clipboard did not receive the fixture text");
            await sleep(50);
        }
        nativeKeys("ctrl+a", "ctrl+v");
        await sleep(350);
        await checkNativeText(text);
    };
    const setBrowserText = async text => {
        await focusBrowser();
        await page.press("a", ["ctrl"]);
        await page.waitFor(`${state}.selectedText === ${state}.text`, 10_000);
        const started = await page.send("Input.imeSetComposition", { text, selectionStart: text.length, selectionEnd: text.length });
        assert(!started.error, "browser composition could not start");
        await settle();
        await page.send("Input.insertText", { text });
        await waitBrowserText(text);
    };
    const waitBrowserText = text => page.waitFor(`${state}.text === ${JSON.stringify(text)}`, 15_000);
    const checkNativeText = async expected => {
        nativeKeys("ctrl+a", "ctrl+c");
        let actual;
        let formats;
        for (let attempt = 0; attempt < 40; attempt++) {
            actual = normalize(readClipboardText());
            formats = clipboardTypes();
            if (actual === expected && formats.includes("text/html")) return;
            await sleep(100);
        }
        throw new Error(`Expected native text ${JSON.stringify(expected)}, received ${JSON.stringify(actual)} with formats ${formats.join(", ")}`);
    };
    const clipboardText = async identifier => {
        await showBrowser();
        const mime = `application/frn-fmt.text.${Buffer.from(identifier, "ascii").toString("hex")}`;
        const type = JSON.stringify(`web ${mime}`);
        await page.waitFor(`navigator.clipboard.read().then(async items => {
            const item = items.find(item => item.types.includes(${type}));
            return item && await (await item.getType(${type})).text() === "1";
        })`, 15_000);
        const map = JSON.parse(readClipboard('application/web;type="custom/formatmap"'));
        assert(/^application\/web;type="custom\/format\d+"$/.test(map[mime]),
            `Missing native slot for ${identifier}`);
        assert(readClipboard(map[mime]) === "1", `Unexpected native marker bytes for ${identifier}`);
        return normalize(readClipboardText());
    };

    const source = "abcdef\nahijkl\nanopqr";
    const target = "12\n34\n56";
    for (const [marker, rectangle, payload, pasted] of [
        ["RectangularSelection", true, "bcde\nhijk\nnopq", "1bcde2\n3hijk4\n5nopq6"],
        ["WholeLineSelection", false, "abcdef\n", "12\nabcdef\n34\n56"],
    ]) {
        const identifier = `FerroUI.Edit.${marker}`;
        const nativePosition = ["ctrl+Home", ...(rectangle ? [] : ["Down"]), "Right"];
        await setNativeText(target);
        await setBrowserText(source);
        await page.press("Home", ["ctrl"]);
        if (rectangle) {
            await page.press("ArrowRight");
            for (let i = 0; i < 4; i++) await page.press("ArrowRight", ["alt", "shift"]);
            for (let i = 0; i < 2; i++) await page.press("ArrowDown", ["alt", "shift"]);
        }
        await copyFromBrowser();
        const browserPayload = await clipboardText(identifier);
        assert(browserPayload === payload, `Unexpected browser copy ${JSON.stringify(browserPayload)}`);
        await focusNative();
        nativeKeys(...nativePosition, "ctrl+v");
        await sleep(350);
        await checkNativeText(pasted);
        nativeKeys("ctrl+z");
        await checkNativeText(target);
        console.log(`Chromium to native ${platform}: ${marker}, document and undo verified`);

        await setBrowserText(target);
        await setNativeText(source);
        nativeKeys("ctrl+Home", ...(rectangle ? [
            "Right", ...Array(4).fill("alt+shift+Right"), ...Array(2).fill("alt+shift+Down"),
        ] : []), "ctrl+c");
        const nativePayload = await clipboardText(identifier);
        let browserResult = pasted;
        if (rectangle) {
            // The native font can give the same pixel width different logical
            // end columns on each row. Use the independently read clipboard payload.
            const rows = nativePayload.split("\n");
            const sourceRows = source.split("\n");
            assert(rows.length === sourceRows.length &&
                rows.every((row, index) => row && sourceRows[index].startsWith(row, 1)),
                `Unexpected native rectangle ${JSON.stringify(nativePayload)}`);
            browserResult = target.split("\n").map((row, index) => row[0] + rows[index] + row[1]).join("\n");
        } else {
            assert(nativePayload === payload, `Unexpected native whole line ${JSON.stringify(nativePayload)}`);
        }
        await focusBrowser();
        await page.press("Home", ["ctrl"]);
        if (!rectangle) await page.press("ArrowDown");
        await page.press("ArrowRight");
        await page.press("v", ["ctrl"]);
        await waitBrowserText(browserResult);
        await page.press("z", ["ctrl"]);
        await waitBrowserText(target);
        console.log(`Native ${platform} to Chromium: ${marker}, document and undo verified`);
    }
}
