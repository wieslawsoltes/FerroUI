import QtQml
import "math/index.mjs" as MathKit
import "Labels.js" as Labels

QtObject {
    id: app
    property int left: 2
    property int right: 8
    property bool alternate: false
    property string label: "root"
    property QtObject scopedChild: QtObject {
        property string label: "child"
        property string captured: Labels.scopeLabel()
    }
    readonly property int blockValue: { let v = left; if (v > 1) { v * 10 } else { v } }
    readonly property int score: MathKit.scale(alternate ? right : left)
    readonly property string caption: Labels.caption(score)
    property Counter first: Counter { }
    property Counter second: Counter { }
    property var callback: null

    signal recorded(value: int)
    onRecorded: function(value) {
        app.left = value
    }

    function scopeProbe(): string {
        Labels.local = 7
        return scopedChild.captured + "/" + Labels.nextLocal()
    }
    function cycle(n: int): int { return MathKit.cycle(n) }
    function modern(): string {
        class Sample {
            constructor(value) { this.value = value }
            render() { return `#${this.value}` }
        }
        const rows = [{amount: 2}, {amount: 4}, {}]
        return rows.map(({amount = 1}) => new Sample(amount ** 2).render())
                   .filter(text => /^#\d+$/.test(text)).join("|")
    }
    function saveClosure(base: int) {
        let current = base
        callback = step => current += step
    }
    function useClosure(step: int): int { return callback(step) }
    function nested(n: int): int { return n + 1 }
    function throughHost(n: int): int {
        recorded(nested(n))
        return score
    }
    function modules(): string {
        return [first.tick(), second.tick(), first.tick(), MathKit.tick(), MathKit.serial].join(":")
    }
    function fail() {
        left = 200
        throw new TypeError("fixture rollback")
    }
}
