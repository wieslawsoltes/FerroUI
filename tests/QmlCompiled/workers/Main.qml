import QtQml
import QtQml.WorkerScript
import "nested" as Nested
QtObject {
    id: app
    property string answer: ""
    property ListModel rows: ListModel { ListElement { number: 7 } }
    readonly property int nestedValue: pane.result
    function startNested() { pane.start() }
    property Nested.Pane pane: Nested.Pane { }
    property int replies: 0
    property int localValue: 0
    property WorkerScript actor: WorkerScript {
        source: "calculate.mjs"
        onMessage: function(message) { app.answer = message.text; app.replies++ }
    }
    property WorkerScript classic: WorkerScript {
        source: "counter.js"
        onMessage: function(n) { app.localValue = n }
    }
    property Component factory: Component {
        WorkerScript { source: "counter.js"; onMessage: function(n) { app.localValue = n } }
    }
    function start() {
        const input = {items: [2, 3]}
        actor.sendMessage(input)
        input.items[0] = 90
        classic.sendMessage(1)
        classic.sendMessage(2)
    }
    function syncRows() { actor.sendMessage({model: rows}) }
    function abortSend() { actor.sendMessage({items: [90]}); throw new Error("failed transaction") }
    function restart() { actor.source = ""; actor.source = "calculate.mjs" }
    function spin() { actor.sendMessage({loop: true}) }
    function selectClassic(url: string) { classic.source = url }
    function control() { classic.sendMessage(5) }
}
