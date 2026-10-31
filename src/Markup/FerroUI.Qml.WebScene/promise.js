// Promise observations do not extend the host transaction. V8 can run pure
// microtasks between worker requests. Every QML host access still goes through
// the synchronous invocation gate in bridge.js.
(() => {
    const then = Promise.prototype.then;
    const observations = new WeakMap();
    function message(reason) {
        try { return String(reason?.stack || reason); }
        catch { return "unprintable JavaScript rejection"; }
    }
    return function observe(value) {
        if (value === null || (typeof value !== "object" && typeof value !== "function"))
            return {state:"other"};
        const existing = observations.get(value);
        if (existing) return existing;
        const status = {state:"pending", value:undefined};
        // Use the intrinsic brand check and bypass an overridden .then getter.
        // Attaching once leaves at most one observer per pending Promise.
        try {
            Reflect.apply(then,value,[
                result => { status.state="fulfilled"; status.value=result; },
                reason => { status.state="rejected"; status.value=message(reason); }
            ]);
        } catch { return {state:"other"}; }
        observations.set(value,status);
        return status;
    };
})()
