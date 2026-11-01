import QtQml
import QtQml.WorkerScript
QtObject {
    id: pane
    property int result: 0
    property WorkerScript actor: WorkerScript {
        onMessage: function(value) { pane.result = value }
    }
    function start() {
        actor.source = "nested-only.js"
        actor.sendMessage(7)
    }
}
