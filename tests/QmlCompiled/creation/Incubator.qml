import QtQml
QtObject {
    id: root
    property var handle
    property int called: 0
    property int last: -1
    property Component factory: Component {
        QtObject {
            required property int amount
            property int initial: -1
            Component.onCompleted: initial = amount
        }
    }
    function observe(status: int) {
        root.called++
        root.last = status
    }
    function begin() {
        root.handle = root.factory.incubateObject(root, {amount: 7}, Qt.Asynchronous)
        root.handle.onStatusChanged = root.observe
        return root.handle.status
    }
    function force() {
        root.handle.forceCompletion()
        return root.handle.status
    }
    function synchronous() {
        root.handle = root.factory.incubateObject(null, {amount: 9}, Qt.Synchronous)
        return root.handle.object.initial
    }
    function invalid() {
        root.handle = root.factory.incubateObject()
        root.handle.onStatusChanged = root.observe
        root.handle.forceCompletion()
        return root.handle.status
    }
}
