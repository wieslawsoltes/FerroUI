// V8 reactions retain a revocable token. The native transaction and its
// mutable state are borrowed only while its owner thread pumps a checkpoint.
(() => {
    let latest;
    let active;

    function run(token, fn, receiver, args) {
        const previous = active;
        active = token;
        try { return Reflect.apply(fn, receiver, args); }
        finally { active = previous; }
    }

    function capture(callback) {
        if (typeof callback !== "function") return callback;
        const token = active && !active.revoked ? latest : active;
        return function (...args) { return run(token, callback, this, args); };
    }

    const nativeThen = Promise.prototype.then;
    Object.defineProperty(Promise.prototype, "then", {
        ...Object.getOwnPropertyDescriptor(Promise.prototype, "then"),
        value: function then(fulfilled, rejected) {
            return Reflect.apply(nativeThen, this, [capture(fulfilled), capture(rejected)]);
        }
    });
    const enqueue = globalThis.queueMicrotask;
    if (typeof enqueue === "function") {
        globalThis.queueMicrotask = callback => enqueue(capture(callback));
    }

    return {
        begin(command) {
            if (!command.turn) return; // Reentry uses the same transaction.
            if (command.revoke && latest?.id === command.revoke) latest.revoked = true;
            if (latest?.id !== command.turn) latest = {id:command.turn, revoked:false};
        },
        dispatch() { return latest?.id; },
        call(fn, command) { return run(latest, fn, undefined, [command]); },
        fail() { if (latest) latest.revoked = true; },
        check() {
            if (!active) throw new Error("No QML turn owns this asynchronous callback");
            if (active.revoked || latest?.revoked) {
                throw new Error("The QML transaction that scheduled or resumed this callback rolled back");
            }
        }
    };
})()
