import assert from "node:assert/strict";
import test from "node:test";
import { setImmediate } from "node:timers/promises";
import { DocumentStorage, DocumentConflictError, LEGACY_DOCUMENT_KEY, openDocumentStorage } from "../web/document-store.js";

// An independently controlled durable backend lets tests separate request
// acceptance from transaction completion and inject failures at that boundary.
class ControlledBackend {
    name = "controlled";
    current = { text: "original", revision: 7 };
    previous = null;
    calls = [];
    pending = [];
    commit(text, expectedRevision, writer) {
        this.calls.push({ text, expectedRevision, writer });
        return new Promise((resolve, reject) => this.pending.push({ text, expectedRevision, resolve, reject }));
    }
    complete() {
        const request = this.pending.shift();
        assert.ok(request, "A durable transaction must be pending");
        assert.equal(request.expectedRevision, this.current.revision);
        this.previous = this.current;
        this.current = { text: request.text, revision: this.current.revision + 1 };
        request.resolve(this.current.revision);
    }
    fail(error) { this.pending.shift().reject(error); }
}
const tick = () => setImmediate();
const create = () => {
    const backend = new ControlledBackend();
    return { backend, storage: new DocumentStorage(backend, backend.current, { writer: "test-tab" }) };
};

test("queued acceptance never claims a document is durably saved", async () => {
    const { backend, storage } = create();
    assert.equal(storage.load(), "original");
    assert.equal(storage.save("edited"), true);
    let flushed = false;
    const flushing = storage.flush().then((result) => { flushed = true; return result; });
    await tick();
    assert.equal(flushed, false);
    assert.equal(storage.status().phase, "saving");
    assert.equal(storage.status().dirty, true);
    assert.equal(storage.status().committedSequence, 0);
    assert.equal(backend.current.text, "original");
    backend.complete();
    assert.equal(await flushing, true);
    assert.equal(storage.status().phase, "saved");
    assert.equal(storage.status().dirty, false);
    assert.equal(storage.status().revision, 8);
    assert.equal(backend.current.text, "edited");
    assert.equal(backend.previous.text, "original");
});

test("rapid edits coalesce and a slow old transaction cannot acknowledge newer text", async () => {
    const { backend, storage } = create();
    storage.save("a"); storage.save("b"); storage.save("c");
    await tick();
    assert.deepEqual(backend.calls.map((call) => call.text), ["c"]);
    storage.save("d"); storage.save("latest");
    let flushed = false;
    const flushing = storage.flush().then((result) => { flushed = true; return result; });
    backend.complete();
    await tick();
    assert.equal(flushed, false);
    assert.equal(storage.status().phase, "saving");
    assert.deepEqual(backend.calls.map((call) => call.text), ["c", "latest"]);
    assert.deepEqual(backend.calls.map((call) => call.expectedRevision), [7, 8]);
    assert.equal(backend.pending.length, 1, "Only one storage transaction runs at a time");
    backend.complete();
    assert.equal(await flushing, true);
    assert.equal(backend.current.text, "latest");
    assert.equal(storage.status().sequence, storage.status().committedSequence);
});

test("quota or transaction abort retains the prior snapshot and retries the newest edit", async () => {
    const { backend, storage } = create();
    storage.save("intermediate");
    await tick();
    storage.save("latest");
    const flushing = storage.flush();
    backend.fail(new DOMException("Storage quota exceeded", "QuotaExceededError"));
    assert.equal(await flushing, false);
    assert.equal(storage.status().phase, "error");
    assert.equal(storage.status().dirty, true);
    assert.equal(storage.load(), "latest");
    assert.equal(backend.current.text, "original");
    assert.equal(backend.calls.length, 1, "A failed transaction must not create a retry loop");
    const retry = storage.flush();
    await tick();
    assert.deepEqual(backend.calls.at(-1), { text: "latest", expectedRevision: 7, writer: "test-tab" });
    backend.complete();
    assert.equal(await retry, true);
    assert.equal(backend.current.text, "latest");
});

test("stale tabs keep their draft and never overwrite the newer committed revision", async () => {
    const { backend, storage } = create();
    storage.save("local draft");
    await tick();
    const flushing = storage.flush();
    backend.current = { text: "newer remote draft", revision: 8 };
    backend.fail(new DocumentConflictError());
    assert.equal(await flushing, false);
    assert.equal(storage.status().phase, "conflict");
    assert.equal(storage.save("more local work"), false);
    assert.equal(await storage.flush(), false);
    assert.equal(storage.load(), "more local work");
    assert.equal(backend.current.text, "newer remote draft");
    assert.equal(backend.calls.length, 1);
});

test("edits enqueued by a saved-status listener are drained before flush resolves", async () => {
    const { backend, storage } = create();
    storage.setStatusListener((status) => {
        if (status.phase === "saved" && status.sequence === 1) storage.save("listener edit");
    });
    storage.save("first edit");
    let flushed = false;
    const flushing = storage.flush().then((result) => { flushed = true; return result; });
    await tick();
    backend.complete();
    await tick();
    assert.equal(flushed, false);
    assert.equal(storage.status().dirty, true);
    assert.equal(backend.calls.at(-1).text, "listener edit");
    backend.complete();
    assert.equal(await flushing, true);
    assert.equal(storage.load(), backend.current.text);
});

test("unchanged documents do not create transactions or duplicate revisions", async () => {
    const { backend, storage } = create();
    assert.equal(storage.save("original"), true);
    assert.equal(await storage.flush(), true);
    assert.equal(backend.calls.length, 0);
    assert.equal(storage.save(""), false);
    assert.equal(storage.save(null), false);
    assert.equal(storage.load(), "original");
});

test("lifecycle guards exist only while actual edits are uncommitted", async () => {
    const { backend, storage } = create();
    const window = new EventTarget(), document = new EventTarget();
    document.visibilityState = "visible";
    storage.attachLifecycle(window, document);
    let event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    assert.equal(event.defaultPrevented, false);
    storage.save("uncommitted");
    event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    assert.equal(event.defaultPrevented, true);
    await tick();
    document.visibilityState = "hidden";
    document.dispatchEvent(new Event("visibilitychange"));
    window.dispatchEvent(new Event("pagehide"));
    assert.equal(backend.calls.length, 1);
    backend.complete();
    assert.equal(await storage.flush(), true);
    event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    assert.equal(event.defaultPrevented, false);
    storage.detachLifecycle();
});

test("an uncommitted native text draft attaches a guard without writing old JSON", async () => {
    const { backend, storage } = create();
    const window = new EventTarget(), document = new EventTarget();
    storage.attachLifecycle(window, document);
    storage.setDraftDirty(true);
    assert.equal(storage.status().phase, "editing");
    assert.equal(storage.status().dirty, true);
    assert.equal(storage.status().draftDirty, true);
    assert.equal(await storage.flush(), false, "An old saved snapshot must not acknowledge a native draft");
    assert.equal(backend.calls.length, 0, "Keystrokes must not serialize or write a full document");
    let commits = 0;
    storage.setDraftCommitter(() => {
        commits++;
        storage.setDraftDirty(false);
        storage.save("committed native Unicode draft · Ω 🚀");
    });
    const event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    assert.equal(commits, 1, "Unload must synchronously finish the native transaction first");
    assert.equal(event.defaultPrevented, true, "The durable transaction is still pending");
    await tick();
    assert.equal(backend.calls.length, 1);
    assert.equal(backend.calls[0].text, "committed native Unicode draft · Ω 🚀");
    backend.complete();
    assert.equal(await storage.flush(), true);
    assert.equal(storage.status().draftDirty, false);
    assert.equal(storage.status().dirty, false);
    storage.detachLifecycle();
});

test("hidden and pagehide commit a focused native draft once before autosave", async () => {
    const { backend, storage } = create();
    const window = new EventTarget(), document = new EventTarget();
    document.visibilityState = "visible";
    storage.attachLifecycle(window, document);
    let commits = 0;
    storage.setDraftCommitter(() => {
        commits++;
        storage.save("hidden-tab draft");
        storage.setDraftDirty(false);
    });
    storage.setDraftDirty(true);
    document.visibilityState = "hidden";
    document.dispatchEvent(new Event("visibilitychange"));
    window.dispatchEvent(new Event("pagehide"));
    await tick();
    assert.equal(commits, 1);
    assert.equal(backend.calls.length, 1);
    assert.equal(backend.calls[0].text, "hidden-tab draft");
    backend.complete();
    assert.equal(await storage.flush(), true);
    storage.detachLifecycle();
});

test("canceling or reverting a native draft clears the guard without a duplicate write", async () => {
    const { backend, storage } = create();
    const window = new EventTarget(), document = new EventTarget();
    storage.attachLifecycle(window, document);
    storage.setDraftCommitter(() => storage.setDraftDirty(false));
    storage.setDraftDirty(true);
    const event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    assert.equal(event.defaultPrevented, false);
    assert.equal(await storage.flush(), true);
    assert.equal(backend.calls.length, 0);
    storage.detachLifecycle();
});

test("a failed native draft commit keeps the unload guard and never claims success", async () => {
    const { backend, storage } = create();
    const window = new EventTarget(), document = new EventTarget();
    storage.attachLifecycle(window, document);
    storage.setDraftCommitter(() => { throw new Error("Native draft unavailable"); });
    storage.setDraftDirty(true);
    const event = new Event("beforeunload", { cancelable: true });
    assert.doesNotThrow(() => window.dispatchEvent(event));
    assert.equal(event.defaultPrevented, true);
    assert.equal(storage.status().phase, "error");
    assert.equal(storage.status().dirty, true);
    assert.equal(await storage.flush(), false);
    assert.equal(backend.calls.length, 0);
    storage.detachLifecycle();
});

test("discarding an invalid inspector draft clears validation without inventing a document write", async () => {
    const { backend, storage } = create();
    storage.setDraftCommitter(() => {});
    storage.setDraftDirty(true);
    assert.equal(storage.commitDraft(), false);
    assert.equal(storage.status().phase, "error");
    assert.match(storage.status().error, /inspector/);
    storage.setDraftDirty(false);
    assert.equal(storage.status().phase, "saved");
    assert.equal(storage.status().error, "");
    assert.equal(await storage.flush(), true);
    assert.equal(backend.calls.length, 0);
    assert.equal(backend.current.text, "original");
});

test("inspector validation cannot clear a stale writer conflict or enable an overwrite retry", async () => {
    const { backend, storage } = create();
    storage.save("local edit");
    await tick();
    backend.fail(new DocumentConflictError("A newer tab already saved this document."));
    assert.equal(await storage.flush(), false);
    assert.equal(storage.status().phase, "conflict");
    const conflict = storage.status().error;
    storage.setDraftCommitter(() => {});
    storage.setDraftDirty(true);
    assert.equal(storage.commitDraft(), false);
    assert.equal(storage.status().phase, "conflict");
    assert.equal(storage.status().error, conflict);
    storage.setDraftDirty(false);
    assert.equal(storage.status().phase, "conflict");
    assert.equal(storage.save("further local edit"), false);
    assert.equal(await storage.flush(), false);
    assert.equal(backend.calls.length, 1);
    assert.equal(backend.current.text, "original");
});

test("unavailable IndexedDB preserves the legacy document and never silently forks saves", async () => {
    const values = new Map([[LEGACY_DOCUMENT_KEY, "legacy document"]]);
    const storage = await openDocumentStorage({ localStorage: {
        getItem: (key) => values.get(key),
        setItem: (key, value) => values.set(key, value),
        removeItem: (key) => values.delete(key),
    } });
    assert.equal(storage.load(), "legacy document");
    assert.equal(storage.status().phase, "unavailable");
    assert.equal(storage.save("new draft"), false);
    assert.equal(await storage.flush(), false);
    assert.equal(storage.load(), "new draft");
    assert.equal(values.get(LEGACY_DOCUMENT_KEY), "legacy document");
});

test("storage security exceptions do not prevent the editor from opening", async () => {
    const environment = {};
    Object.defineProperty(environment, "localStorage", { get() { throw new DOMException("Denied", "SecurityError"); } });
    Object.defineProperty(environment, "indexedDB", { get() { throw new DOMException("Denied", "SecurityError"); } });
    const storage = await openDocumentStorage(environment);
    assert.equal(storage.load(), "");
    assert.equal(storage.status().phase, "unavailable");
    assert.equal(storage.save("editable draft"), false);
    assert.equal(storage.load(), "editable draft");
});


test("queued serialization is dirty before JSON exists and flush captures it once", async () => {
    const { backend, storage } = create();
    let captures = 0;
    storage.setDraftCommitter(() => {
        captures++;
        storage.save("latest coalesced document");
        storage.setSerializationPending(false);
    });
    storage.setSerializationPending(true);
    storage.setSerializationPending(true);
    assert.equal(storage.status().phase, "saving");
    assert.equal(storage.status().dirty, true);
    assert.equal(storage.load(), "original");
    assert.equal(backend.calls.length, 0);
    const flushing = storage.flush();
    await tick();
    assert.equal(captures, 1);
    assert.equal(backend.calls[0].text, "latest coalesced document");
    assert.equal(storage.status().dirty, true);
    backend.complete();
    assert.equal(await flushing, true);
    assert.equal(storage.status().dirty, false);
});

test("pagehide synchronously captures a queued non-text edit before its deferred timer", async () => {
    const { backend, storage } = create();
    const window = new EventTarget(), document = new EventTarget();
    storage.attachLifecycle(window, document);
    let captures = 0;
    storage.setDraftCommitter(() => {
        captures++;
        storage.save("confirmed pointer edit");
        storage.setSerializationPending(false);
    });
    storage.setSerializationPending(true);
    assert.equal(storage.unloadAttached, true);
    window.dispatchEvent(new Event("pagehide"));
    window.dispatchEvent(new Event("pagehide"));
    assert.equal(storage.load(), "confirmed pointer edit");
    assert.equal(captures, 1);
    await tick();
    backend.complete();
    assert.equal(await storage.flush(), true);
    assert.equal(storage.unloadAttached, false);
    storage.detachLifecycle();
});

test("failed queued serialization cannot acknowledge the old saved document", async () => {
    const { backend, storage } = create();
    storage.setDraftCommitter(() => {});
    storage.setSerializationPending(true);
    assert.equal(await storage.flush(), false);
    assert.equal(storage.status().phase, "error");
    assert.equal(storage.status().serializationPending, true);
    assert.equal(storage.status().dirty, true);
    assert.equal(backend.calls.length, 0);
    assert.equal(backend.current.text, "original");
});

test("flush includes a fresh deferred snapshot queued while its prior write completes", async () => {
    const { backend, storage } = create();
    let next = "new deferred edit";
    storage.setDraftCommitter(() => {
        storage.save(next);
        storage.setSerializationPending(false);
    });
    storage.save("first snapshot");
    const flushing = storage.flush();
    await tick();
    storage.setSerializationPending(true);
    backend.complete();
    await tick();
    assert.equal(backend.calls.at(-1).text, next);
    backend.complete();
    assert.equal(await flushing, true);
    assert.equal(backend.current.text, next);
    assert.equal(storage.status().dirty, false);
});
