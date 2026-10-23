// Host persistence for the native FerroUI application. No editor state is kept
// in the DOM. The Rust bridge reads this cache synchronously after initialize().
export const DOCUMENT_DATABASE = "ferrodesign.documents";
export const DOCUMENT_STORE = "snapshots";
export const LEGACY_DOCUMENT_KEY = "ferrodesign.document.v1";
const CURRENT = "current";
const PREVIOUS = "previous";

export class DocumentConflictError extends Error {
    constructor() {
        super("This design was saved in another tab. Export your changes before reloading the newer version.");
        this.name = "DocumentConflictError";
    }
}

/**
 * Coalesces synchronous Rust save requests while a durable transaction is in
 * flight. A successful save() means queued; only flush()/status() confirm that
 * the storage transaction committed. The backend owns the atomic revision check.
 */
export class DocumentStorage {
    constructor(backend, snapshot = null, options = {}) {
        this.backend = backend;
        this.text = snapshot?.text || "";
        this.revision = snapshot?.revision || 0;
        this.sequence = 0;
        this.committedSequence = 0;
        this.draftDirty = false;
        // Keep field validation separate from the durable writer state. A bad
        // native draft must never turn a stale writer's conflict into a retry.
        this.draftError = "";
        this.serializationPending = false;
        this.draftCommitter = null;
        this.phase = backend ? "saved" : "unavailable";
        this.error = options.error || "";
        this.writer = options.writer || globalThis.crypto?.randomUUID?.()
            || Date.now().toString(36) + "-" + Math.random().toString(36).slice(2);
        this.running = null;
        this.listener = null;
        this.notificationQueued = false;
        this.eventTarget = null;
        this.document = null;
        this.unloadAttached = false;
        this.onBeforeUnload = (event) => {
            const committed = this.commitDraft();
            if (!this.dirty) return;
            // An unloading page cannot await an IndexedDB transaction. Warn only
            // while there is an actual uncommitted edit; do not disable BFCache
            // for an already saved document.
            if (committed) this.flush();
            event.preventDefault();
            event.returnValue = "";
        };
        this.onPageHide = () => { if (this.commitDraft()) this.flush(); };
        this.onVisibilityChange = () => {
            if (this.document?.visibilityState === "hidden" && this.commitDraft()) this.flush();
        };
    }

    get pendingSave() { return this.sequence !== this.committedSequence; }
    get dirty() { return this.pendingSave || this.draftDirty || this.serializationPending; }
    load() { return this.text; }
    status() {
        return {
            phase: this.phase === "conflict" ? "conflict" : this.draftError ? "error"
                : this.draftDirty && ["saved", "saving"].includes(this.phase) ? "editing"
                : this.serializationPending && this.phase === "saved" ? "saving" : this.phase,
            backend: this.backend?.name || "memory",
            dirty: this.dirty,
            draftDirty: this.draftDirty,
            serializationPending: this.serializationPending,
            revision: this.revision,
            sequence: this.sequence,
            committedSequence: this.committedSequence,
            characters: this.text.length,
            error: this.phase === "conflict" ? this.error : this.draftError || this.error,
        };
    }
    setStatusListener(listener) {
        this.listener = listener;
        this.notify();
    }
    setDraftCommitter(commit) { this.draftCommitter = commit; }
    setSerializationPending(pending) {
        pending = Boolean(pending);
        if (pending === this.serializationPending) return;
        this.serializationPending = pending;
        if (!pending && !this.draftDirty) this.draftError = "";
        this.notify();
    }
    setDraftDirty(dirty) {
        dirty = Boolean(dirty);
        if (dirty === this.draftDirty) return;
        this.draftDirty = dirty;
        if (!dirty && !this.serializationPending) this.draftError = "";
        this.notify();
    }
    commitDraft() {
        if (!this.draftDirty && !this.serializationPending) return true;
        try {
            // The native TextBox keeps the live draft in Rust. Commit once at a
            // lifecycle boundary so no full document crosses the bridge for
            // each keystroke. The shell queues its ordinary save and clears the
            // draft marker before this synchronous callback returns.
            this.draftCommitter?.();
            if (this.serializationPending) throw new Error("The queued document snapshot could not be captured. Export the design before leaving.");
            if (this.draftDirty) throw new Error("The active inspector or text edit could not be committed. Correct the value or press Escape before leaving.");
            this.draftError = "";
            return true;
        } catch (error) {
            this.draftError = error?.message || String(error);
            this.notify();
            return false;
        }
    }
    attachLifecycle(eventTarget, document) {
        this.detachLifecycle();
        this.eventTarget = eventTarget;
        this.document = document;
        eventTarget.addEventListener("pagehide", this.onPageHide);
        document.addEventListener("visibilitychange", this.onVisibilityChange);
        this.updateUnloadListener();
    }
    detachLifecycle() {
        this.eventTarget?.removeEventListener("pagehide", this.onPageHide);
        this.eventTarget?.removeEventListener("beforeunload", this.onBeforeUnload);
        this.document?.removeEventListener("visibilitychange", this.onVisibilityChange);
        this.eventTarget = this.document = null;
        this.unloadAttached = false;
    }
    updateUnloadListener() {
        if (!this.eventTarget || this.unloadAttached === this.dirty) return;
        this.unloadAttached = this.dirty;
        this.eventTarget[this.dirty ? "addEventListener" : "removeEventListener"](
            "beforeunload", this.onBeforeUnload);
    }
    notify() {
        this.updateUnloadListener();
        if (this.notificationQueued) return;
        this.notificationQueued = true;
        queueMicrotask(() => {
            this.notificationQueued = false;
            this.listener?.(this.status());
        });
    }
    save(text) {
        if (typeof text !== "string" || !text.length) return false;
        if (text !== this.text || (!this.sequence && !this.revision)) {
            this.text = text;
            this.sequence++;
        }
        if (!this.pendingSave) return true;
        if (!this.backend) {
            this.phase = "unavailable";
            this.error ||= "Browser storage is unavailable. Export this design to keep your changes.";
            this.notify();
            return false;
        }
        // A stale tab must not turn the next keystroke into an overwrite. Its
        // current document remains available for a portable export.
        if (this.phase === "conflict") { this.notify(); return false; }
        this.phase = "saving";
        this.error = "";
        this.notify();
        this.drain();
        return true;
    }
    drain() {
        if (this.running) return this.running;
        this.running = Promise.resolve().then(async () => {
            while (this.pendingSave && this.backend && this.phase !== "conflict") {
                const sequence = this.sequence, text = this.text;
                try {
                    const revision = await this.backend.commit(text, this.revision, this.writer);
                    this.revision = revision;
                    this.committedSequence = sequence;
                    this.phase = this.pendingSave ? "saving" : "saved";
                    this.error = "";
                    this.notify();
                } catch (error) {
                    this.phase = error instanceof DocumentConflictError ? "conflict" : "error";
                    this.error = error?.message || String(error);
                    this.notify();
                    return false;
                }
            }
            return !this.pendingSave;
        }).finally(() => {
            this.running = null;
            // A status listener can enqueue an edit after the pump's final
            // iteration but before this promise settles. Do not strand it.
            if (this.pendingSave && this.phase === "saving") this.drain();
        });
        return this.running;
    }
    async flush() {
        if (this.serializationPending && !this.commitDraft()) return false;
        if (!this.pendingSave) return !this.draftDirty && !this.serializationPending;
        if (!this.backend || this.phase === "conflict") return false;
        do {
            this.phase = "saving";
            this.notify();
            const committed = await this.drain();
            if (!committed || ["error", "conflict", "unavailable"].includes(this.phase)) return false;
        } while (this.pendingSave);
        if (this.serializationPending) return this.flush();
        return !this.draftDirty;
    }
}

function openDatabase(indexedDB, timeoutMs) {
    return new Promise((resolve, reject) => {
        let request, settled = false;
        const finish = (error, database) => {
            if (settled) { database?.close(); return; }
            settled = true;
            clearTimeout(timer);
            if (error) reject(error); else resolve(database);
        };
        const timer = setTimeout(() => finish(new Error("Opening browser storage timed out.")), timeoutMs);
        try {
            request = indexedDB.open(DOCUMENT_DATABASE, 1);
            request.onupgradeneeded = () => {
                const database = request.result;
                if (!database.objectStoreNames.contains(DOCUMENT_STORE)) database.createObjectStore(DOCUMENT_STORE, { keyPath: "key" });
            };
            request.onerror = () => finish(request.error || new Error("Browser storage could not open."));
            request.onblocked = () => finish(new Error("Another tab is blocking the browser storage upgrade."));
            request.onsuccess = () => finish(null, request.result);
        } catch (error) { finish(error); }
    });
}

/** IndexedDB transactions serialize the read/check/write across every tab. */
class IndexedDocumentBackend {
    constructor(indexedDB, database, timeoutMs) {
        this.name = "indexedDB";
        this.indexedDB = indexedDB;
        this.timeoutMs = timeoutMs;
        this.use(database);
    }
    use(database) {
        this.database = database;
        database.onversionchange = () => {
            database.close();
            if (this.database === database) this.database = null;
        };
        database.onclose = () => { if (this.database === database) this.database = null; };
    }
    async connection() {
        if (!this.database) this.use(await openDatabase(this.indexedDB, this.timeoutMs));
        return this.database;
    }
    async load() {
        const database = await this.connection();
        return new Promise((resolve, reject) => {
            const transaction = database.transaction(DOCUMENT_STORE, "readonly");
            const request = transaction.objectStore(DOCUMENT_STORE).get(CURRENT);
            let snapshot;
            request.onsuccess = () => { snapshot = request.result; };
            transaction.oncomplete = () => resolve(snapshot || null);
            transaction.onabort = () => reject(transaction.error || new Error("The saved design could not be read."));
            transaction.onerror = () => {};
        });
    }
    async commit(text, expectedRevision, writer) {
        const database = await this.connection();
        return new Promise((resolve, reject) => {
            let transaction;
            // 'strict' asks the browser to flush to durable storage before it
            // reports completion. Older engines support the two-argument form.
            try { transaction = database.transaction(DOCUMENT_STORE, "readwrite", { durability: "strict" }); }
            catch (error) {
                if (!(error instanceof TypeError)) throw error;
                transaction = database.transaction(DOCUMENT_STORE, "readwrite");
            }
            const store = transaction.objectStore(DOCUMENT_STORE);
            const request = store.get(CURRENT);
            let failure, revision;
            request.onsuccess = () => {
                try {
                    const previous = request.result;
                    const actualRevision = previous?.revision || 0;
                    if (actualRevision !== expectedRevision) throw new DocumentConflictError();
                    revision = actualRevision + 1;
                    if (previous) store.put({ ...previous, key: PREVIOUS });
                    store.put({ key: CURRENT, text, revision, writer, savedAt: Date.now() });
                } catch (error) {
                    failure = error;
                    transaction.abort();
                }
            };
            // A put request can succeed and the transaction still abort later,
            // for example because its quota was exhausted. Acknowledge commit,
            // never individual requests, and retain the previous snapshot in
            // this very same transaction.
            transaction.oncomplete = () => resolve(revision);
            transaction.onabort = () => reject(failure || transaction.error || new Error("The design could not be saved."));
            transaction.onerror = () => {};
        });
    }
}

export async function openDocumentStorage(environment = globalThis, options = {}) {
    let localStorage, legacy = null, backend, snapshot, error = "";
    try {
        localStorage = environment.localStorage;
        legacy = localStorage?.getItem(LEGACY_DOCUMENT_KEY) ?? null;
    } catch (cause) { error = cause?.message || String(cause); }
    try {
        if (!environment.indexedDB) throw new Error("IndexedDB is unavailable in this browser.");
        const timeoutMs = options.timeoutMs ?? 5000;
        const database = await openDatabase(environment.indexedDB, timeoutMs);
        backend = new IndexedDocumentBackend(environment.indexedDB, database, timeoutMs);
        snapshot = await backend.load();
        error = "";
    } catch (cause) {
        error = cause?.message || String(cause);
        // Keep any legacy document readable, but do not silently save into a
        // second backend after IndexedDB fails. A subsequent successful open
        // could otherwise restore an older IndexedDB snapshot over newer work.
        try { backend?.database?.close(); } catch { /* The failed connection is discarded. */ }
        backend = null;
    }
    const storage = new DocumentStorage(backend, snapshot || { text: legacy }, { ...options, error });
    if (backend?.name === "indexedDB" && !snapshot && legacy) {
        storage.save(legacy);
        // Complete migration before the synchronous Rust load; never erase a
        // localStorage document when its replacement transaction failed.
        if (await storage.flush()) {
            try {
                if (localStorage.getItem(LEGACY_DOCUMENT_KEY) === legacy) localStorage.removeItem(LEGACY_DOCUMENT_KEY);
            } catch { /* The durable IndexedDB snapshot is already committed. */ }
        }
    }
    return storage;
}
