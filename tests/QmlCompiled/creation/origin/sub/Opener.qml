import QtQml
import "./scripts/Open.js" as Classic
import "./scripts/Open.mjs" as ESM
QtObject {
    function capture() {
        const create = Qt.createComponent
        return parent => create("Part.qml").createObject(parent).marker
    }
    function classic(parent) { return Classic.open(parent) }
    function esm(parent) { return ESM.open(parent) }
}
