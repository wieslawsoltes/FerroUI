import QtQml
import QtQml.Models
import "math/async-host.mjs" as AsyncModule
import "AsyncCode.js" as Classic

QtObject {
    id: root
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
    property int checksum: 0
    signal advanced(int value)
    onAdvanced: function(value) { checksum = 3 * value }
    property QtObject child: QtObject {
        property int left: 100
        function suspendChild() {
            return (async () => {
                await new Promise(resolve => { globalThis.resumeChild = resolve })
                left += 5
                return left
            })()
        }
    }
    property ListModel rows: ListModel { ListElement { title: "retained" } }
    function retainRow() {
        return Promise.resolve().then(() => {
            globalThis.retainedQmlRow = rows.get(0)
            return globalThis.retainedQmlRow.title
        })
    }
    function readRetainedRow() { return globalThis.retainedQmlRow.title }
    function moduleContinuations() {
        return (async () => {
            const total = await AsyncModule.compute(root)
            return total + await Classic.throughClassic(root)
        })()
    }
    function captureThis(value: int) {
        return (async () => { await 0; return this.left + arguments[0] })()
    }
    function immediate(value: int) {
        return (async () => {
            await Promise.resolve()
            left += value
            advanced(left)
            await Promise.resolve()
            return checksum
        })()
    }
    function rejectAfterAwait() {
        return (async () => { await 0; left = 301; advanced(left); throw new Error("asynchronous failure") })()
    }
    function resumeChild() { globalThis.resumeChild() }
    function abortWakeChild() { globalThis.resumeChild(); throw new Error("resolver aborted") }
    function abandonPending() {
        globalThis.abandoned = (async () => {
            await new Promise(resolve => { globalThis.resumeAbandoned = resolve })
            left = 777
        })()
        globalThis.abandoned.catch(() => {})
        throw new Error("cancel suspended work")
    }
    function wakeAbandoned() { globalThis.resumeAbandoned(); return globalThis.abandoned }
    function spinAfterAwait() {
        left = 800
        return (async () => { await 0; while (true) {} })()
    }
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
    function moduleOrigin() { return import("./math/café data.mjs").then(module => module.url) }
    function importAndSettle(n: int) {
        return import("./math/promise-sum.mjs").then(module => module.delayedSum(n))
    }
}
