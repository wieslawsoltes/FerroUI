import QtQml
QtObject {
    id: root
    property int input: 5
    property int called: 0
    property Component part: Component { QtObject { required property int value } }
    function run() {
        const handle = part.incubateObject(null, {value: Qt.binding(() => root.input * 2)})
        const pending = handle.status === Component.Loading && handle.object === null
        let observed
        handle.onStatusChanged = function(status) {
            root.called++
            observed = (this === handle) + "/" + status + "/" + handle.object.value
            handle.forceCompletion()
        }
        handle.forceCompletion()
        root.input = 8
        return [pending, observed, handle.object.value, root.called].join("|")
    }
}
