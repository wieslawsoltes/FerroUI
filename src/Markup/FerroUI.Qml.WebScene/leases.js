// No callback into a QML transaction from V8's finalizer. The next ordinary
// result carries changes. A new incarnation always has a distinct revision.
(() => {
    const cache = new Map(), changes = new Map();
    const Weak = globalThis.WeakRef, Finalizers = globalThis.FinalizationRegistry;
    let revision = 0, allocations = 0;
    function retired(ticket) {
        if (cache.get(ticket.id)?.revision !== ticket.revision) return;
        cache.delete(ticket.id);
        changes.set(ticket.id, -ticket.revision);
    }
    const finalizers = typeof Weak === "function" && typeof Finalizers === "function"
        ? new Finalizers(retired) : null;
    function get(id, create) {
        const entry = cache.get(id);
        const current = entry?.get();
        if (current !== undefined) return current;
        const value = create(), version = ++revision;
        if (typeof Weak === "function") {
            const ref = new Weak(value);
            cache.set(id, {revision:version, get:ref.deref.bind(ref)});
            finalizers?.register(value, {id, revision:version});
        } else {
            // Rust limits the number of simultaneous leases in either mode.
            cache.set(id, {revision:version, get:() => value});
        }
        allocations++;
        changes.set(id,version);
        return value;
    }
    function take() {
        const result=Array.from(changes);
        changes.clear();
        return result;
    }
    return {
        get,
        complete(result) {
            if (changes.size) result.hostLeases=take();
            if (typeof Weak === "function" && allocations>=256 && cache.size>=256) result.collectHosts=true;
            return result;
        },
        sweep() {
            allocations=0;
            for (const [id,entry] of cache) {
                if (entry.get() === undefined) retired({id,revision:entry.revision});
            }
            return take();
        },
    };
})()
