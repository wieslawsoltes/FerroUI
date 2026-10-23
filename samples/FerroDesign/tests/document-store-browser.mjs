// Real IndexedDB validation. This only serves the host module and does not need
// a FerroUI/Wasm build. Full application reload coverage lives in browser.mjs.
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const source = await readFile(fileURLToPath(new URL("../web/document-store.js", import.meta.url)));
const server = createServer((request, response) => {
    if (request.url === "/document-store.js") {
        response.writeHead(200, { "content-type": "text/javascript" }).end(source);
    } else {
        response.writeHead(200, { "content-type": "text/html" }).end(
            '<!doctype html><title>FerroDesign storage validation</title><script type="module">import * as module from "/document-store.js"; globalThis.storageModule=module;</script>');
    }
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const base = `http://127.0.0.1:${server.address().port}`;
let browser;
const checks = [];
try {
    browser = await chromium.launch({ headless: true, executablePath: process.env.FERRODESIGN_CHROMIUM || undefined });
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(base);
    await page.waitForFunction(() => globalThis.storageModule);
    const migration = await page.evaluate(async () => {
        const { LEGACY_DOCUMENT_KEY, DOCUMENT_DATABASE, DOCUMENT_STORE, openDocumentStorage } = storageModule;
        localStorage.setItem(LEGACY_DOCUMENT_KEY, "legacy design");
        globalThis.store = await openDocumentStorage();
        store.save("new design");
        const flushed = await store.flush();
        const database = await new Promise((resolve, reject) => {
            const request = indexedDB.open(DOCUMENT_DATABASE);
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
        const records = await new Promise((resolve, reject) => {
            const transaction = database.transaction(DOCUMENT_STORE);
            const request = transaction.objectStore(DOCUMENT_STORE).getAll();
            transaction.oncomplete = () => resolve(request.result);
            transaction.onabort = () => reject(transaction.error);
        });
        database.close();
        return { flushed, state: store.status(), legacy: localStorage.getItem(LEGACY_DOCUMENT_KEY), records };
    });
    assert.equal(migration.flushed, true);
    assert.equal(migration.legacy, null);
    assert.equal(migration.state.backend, "indexedDB");
    assert.equal(migration.state.revision, 2);
    assert.equal(migration.records.find((record) => record.key === "current").text, "new design");
    assert.equal(migration.records.find((record) => record.key === "previous").text, "legacy design");
    checks.push("Legacy migration and current/previous snapshots commit atomically");

    const aborted = await page.evaluate(async () => {
        const original = IDBObjectStore.prototype.put;
        IDBObjectStore.prototype.put = function (value, ...arguments_) {
            const request = original.call(this, value, ...arguments_);
            if (value.key === "current" && value.text === "aborted draft") {
                request.addEventListener("success", () => this.transaction.abort());
            }
            return request;
        };
        try {
            store.save("aborted draft");
            const flushed = await store.flush();
            const reloaded = await storageModule.openDocumentStorage();
            return { flushed, state: store.status(), savedText: reloaded.load() };
        } finally { IDBObjectStore.prototype.put = original; }
    });
    assert.equal(aborted.flushed, false);
    assert.equal(aborted.state.phase, "error");
    assert.equal(aborted.state.dirty, true);
    assert.equal(aborted.savedText, "new design", "Request success followed by transaction abort must never replace the prior save");
    assert.equal(await page.evaluate(() => store.flush()), true);
    checks.push("A write request that succeeds before transaction abort is not acknowledged or persisted");

    const stale = await context.newPage();
    await stale.goto(base);
    await stale.waitForFunction(() => globalThis.storageModule);
    await stale.evaluate(async () => { globalThis.store = await storageModule.openDocumentStorage(); });
    assert.equal(await page.evaluate(async () => { store.save("current-tab edit"); return store.flush(); }), true);
    const conflict = await stale.evaluate(async () => {
        store.save("stale-tab draft");
        return { flushed: await store.flush(), state: store.status(), draft: store.load() };
    });
    assert.equal(conflict.flushed, false);
    assert.equal(conflict.state.phase, "conflict");
    assert.equal(conflict.draft, "stale-tab draft");
    await stale.close();
    checks.push("Two real browser tabs cannot overwrite a snapshot loaded at an older revision");

    const large = await page.evaluate(async () => {
        const text = JSON.stringify({ name: "Large portable design", data: "Orbit · café Ω ".repeat(800_000) });
        const started = performance.now();
        store.save(text);
        const flushed = await store.flush();
        return { flushed, state: store.status(), characters: text.length, saveMs: performance.now() - started };
    });
    assert.equal(large.flushed, true);
    assert.ok(large.characters > 10 * 1024 * 1024);
    assert.equal(large.state.dirty, false);
    await page.reload();
    await page.waitForFunction(() => globalThis.storageModule);
    const reopened = await page.evaluate(async () => {
        const store = await storageModule.openDocumentStorage();
        const document = JSON.parse(store.load());
        return { characters: store.load().length, name: document.name, dataLength: document.data.length, state: store.status() };
    });
    assert.equal(reopened.characters, large.characters);
    assert.equal(reopened.name, "Large portable design");
    assert.equal(reopened.dataLength, "Orbit · café Ω ".length * 800_000);
    assert.equal(reopened.state.revision, large.state.revision);
    checks.push("A document exceeding 10 MiB survives a real browser reload without localStorage");
    await context.close();

    const failedMigrationContext = await browser.newContext();
    const migrationPage = await failedMigrationContext.newPage();
    await migrationPage.goto(base);
    await migrationPage.waitForFunction(() => globalThis.storageModule);
    const failedMigration = await migrationPage.evaluate(async () => {
        localStorage.setItem(storageModule.LEGACY_DOCUMENT_KEY, "irreplaceable legacy design");
        const original = IDBObjectStore.prototype.put;
        IDBObjectStore.prototype.put = function (value, ...arguments_) {
            const request = original.call(this, value, ...arguments_);
            if (value.key === "current") request.addEventListener("success", () => this.transaction.abort());
            return request;
        };
        try {
            const store = await storageModule.openDocumentStorage();
            return { text: store.load(), state: store.status(), legacy: localStorage.getItem(storageModule.LEGACY_DOCUMENT_KEY) };
        } finally { IDBObjectStore.prototype.put = original; }
    });
    assert.equal(failedMigration.text, "irreplaceable legacy design");
    assert.equal(failedMigration.legacy, failedMigration.text);
    assert.equal(failedMigration.state.phase, "error");
    checks.push("An aborted migration keeps the original localStorage document readable and intact");
    await failedMigrationContext.close();
    console.log(JSON.stringify({ passed: true, browser: browser.version(), checks, largeDocument: large }, null, 2));
} finally {
    await browser?.close();
    await new Promise((resolve) => server.close(resolve));
}
