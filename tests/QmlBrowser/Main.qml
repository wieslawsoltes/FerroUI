import QtQml
import "math.mjs" as MathTools
QtObject {
    id: page
    property int seed: 3
    property int total: MathTools.sum([seed * 2, 5])
    function bump(n: int): int { seed += n; return total }
    function callNested(): int { return bump(2) + total }
    function makeBinding() { total = Qt.binding(() => MathTools.sum([page.seed, 40])) }
    function fail() { seed = 77; throw new Error("browser rollback") }
    function later() { return Promise.resolve(2).then(value => { page.seed += value; return page.total }) }
    function wide(text: string): string { return text + "!" }
    function spin() { while (true) {} }
    property QtObject child: QtObject { id: nested; property string caption: "native child" }
    property ListModel rows: ListModel { ListElement { n: 2 } }
    function uncopyable() { seed = 500; return () => 17 }
    function rowEdit() { rows.setProperty(0,"n",5) }
    function copied(input) { return {nested:input, numbers:[-0, NaN, Infinity], empty:null} }
    property int ticks: 0
    property Timer clock: Timer { interval: 20; onTriggered: page.ticks++ }
    function startTimer() { clock.start() }
}
