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
}
