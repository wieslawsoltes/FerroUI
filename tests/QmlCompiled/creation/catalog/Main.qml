import QtQml
QtObject {
    id: host
    function make(): int {
        let c = Qt.createComponent("Part.qml", Component.PreferSynchronous, host)
        return c.createObject(host, { amount: 11 }).seen
    }
    function makeType(): string { return Qt.createComponent("QtQml","QtObject").createObject(host,{ objectName:"named" }).objectName }
    function makeViaString(): int { return Qt.createQmlObject('import QtQml; import "." as Local; Local.Part { amount: 13 }', host, "Runtime.qml").seen }
    function missing(): Component { return Qt.createComponent("unknown.qml") }
    function startAsync(): Component { return Qt.createComponent("Part.qml", Component.Asynchronous) }
}
