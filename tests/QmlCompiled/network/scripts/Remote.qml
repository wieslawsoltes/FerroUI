import QtQml
import "parts/logic.js" as Logic
import "parts/store.mjs" as Store
QtObject {
    property int seed: 4
    function compute(): string { return Logic.increment(seed) + "/" + Store.next() }
    function classicOrigin(): string { return Logic.origin() }
}
