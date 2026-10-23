// npm install --no-save playwright
// FERROUI_CDP_PORT=9222 cargo run -p control-catalog-desktop --features cdp
// node scripts/cdp/smoke.mjs http://127.0.0.1:9222 target/cdp-smoke.png
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const endpoint = process.argv[2] || 'http://127.0.0.1:9222';
const output = process.argv[3] || 'target/cdp-smoke.png';
const browser = await chromium.connectOverCDP(endpoint, { timeout: 15000 });
try {
    const page = browser.contexts()[0]?.pages()[0];
    assert(page, 'ControlCatalog target initialized as a Playwright page');
    const cdp = await page.context().newCDPSession(page);
    const { root } = await cdp.send('DOM.getDocument', { depth: 1 });
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector: '#SearchBox' });
    assert(nodeId, 'Found the ControlCatalog search box');
    const { attributes } = await cdp.send('DOM.getAttributes', { nodeId });
    const oldText = Object.fromEntries(Array.from({length: attributes.length / 2}, (_, i) => attributes.slice(i * 2, i * 2 + 2))).Text || '';
    try {
        await cdp.send('DOM.setAttributeValue', { nodeId, name: 'Text', value: '' });
        const { model } = await cdp.send('DOM.getBoxModel', { nodeId });
        const x = (model.content[0] + model.content[4]) / 2;
        const y = (model.content[1] + model.content[5]) / 2;
        await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
        await cdp.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
        await cdp.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 });
        await cdp.send('Input.insertText', { text: 'Button' });
        await cdp.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Backspace', code: 'Backspace' });
        await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Backspace', code: 'Backspace' });
        await cdp.send('Input.insertText', { text: 'n' });
        const updated = await cdp.send('DOM.getAttributes', { nodeId });
        const textIndex = updated.attributes.indexOf('Text');
        assert.equal(updated.attributes[textIndex + 1], 'Button', 'Raw click focused the textbox and raw text reached it');
        const { computedStyle } = await cdp.send('CSS.getComputedStyleForNode', { nodeId });
        assert(computedStyle.some(p => p.name === 'Text' && p.value === 'Button'));
        const { nodes } = await cdp.send('Accessibility.getFullAXTree');
        assert(nodes.length > 0);
        const { data } = await cdp.send('Page.captureScreenshot', { format: 'png' });
        const png = Buffer.from(data, 'base64');
        assert.equal(png.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
        assert(png.length > 100);
        await fs.mkdir(path.dirname(output), { recursive: true });
        await fs.writeFile(output, png);
        await assert.rejects(cdp.send('Runtime.evaluate', { expression: '1 + 1' }), /no JavaScript runtime/);
        console.log(`PASS: discovery, Playwright attachment, find, click, type, styles, accessibility, screenshot (${output})`);
    } finally {
        await cdp.send('DOM.setAttributeValue', { nodeId, name: 'Text', value: oldText });
        await cdp.detach();
    }
} finally {
    await browser.close();
}
