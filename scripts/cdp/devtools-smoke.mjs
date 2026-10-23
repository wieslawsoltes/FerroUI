// Against an already-running ControlCatalog CDP endpoint. CHROME may select an installed Chrome.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const endpoint = process.argv[2] || 'http://127.0.0.1:9222';
const output = process.argv[3] || 'target/cdp-chrome-devtools.png';
const response = await fetch(`${endpoint}/json/list`);
assert(response.ok, 'Discovery succeeded');
const targets = await response.json();
assert(targets.length, 'ControlCatalog has a target');
const chrome = await chromium.launch({ headless: true, ...(process.env.CHROME ? { executablePath: process.env.CHROME } : {}) });
try {
    const page = await chrome.newPage({ viewport: { width: 1400, height: 1000 } });
    await page.goto(targets[0].devtoolsFrontendUrl);
    const root = page.getByText('MainWindow', { exact: true }).first();
    await root.waitFor();
    await root.click();
    // A native property in the Styles panel proves matched/inline styles actually rendered.
    await page.getByText('ActualTransparencyLevel', { exact: true }).first().waitFor({ timeout: 15000 });
    await root.click();
    await page.keyboard.press('ArrowRight');
    await page.getByText('Panel', { exact: true }).first().waitFor({ timeout: 10000 });
    await fs.mkdir(path.dirname(output), { recursive: true });
    await page.screenshot({ path: output });
    console.log(`PASS: Chrome ${chrome.version()} expands the native tree and renders FerroUI properties in Styles (${output})`);
} finally {
    await chrome.close();
}
