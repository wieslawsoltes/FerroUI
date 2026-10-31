import QtQml

QtObject {
    property int left: 2
    function ordered(z: int, a: int): string {
        return arguments[0] + "/" + arguments[1]
    }
    function fulfilledLater(z: int, a: int) {
        left = z
        return (async () => {
            await Promise.resolve()
            return z + a
        })()
    }
    function rejectedLater(value: int) {
        left = value
        return Promise.resolve().then(() => { throw new RangeError("expected rejection") })
    }
    function pendingLater() {
        return new Promise(resolve => { globalThis.finishQmlProbe = resolve })
    }
    function finishLater(n: int) { globalThis.finishQmlProbe(n) }
    function outOfTurn() {
        left = 15
        return Promise.resolve().then(() => { left = 89 })
    }
    function failedOuter() {
        globalThis.afterFailedOuter = Promise.resolve().then(() => { left = 91 })
        globalThis.afterFailedOuter.catch(() => {})
        left = 19
        throw new TypeError("outer failed")
    }
    function afterFailure() { return globalThis.afterFailedOuter }
    function importAndSettle(n: int) {
        return import("./math/promise-sum.mjs").then(module => module.delayedSum(n))
    }
}
