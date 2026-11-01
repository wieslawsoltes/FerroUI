import QtQml
import "./scripts/Open.js" as Classic
import "./scripts/Open.mjs" as ESM
QtObject {
    function capture() {
        const create = Qt.createComponent
        return parent => create("Part.qml").createObject(parent).marker
    }
    function captureUrl() { return Qt.resolvedUrl }
    function classicUrl(): url { return Classic.asset() }
    function esmUrl(): url { return ESM.asset() }
    function classic(parent) { return Classic.open(parent) }
    function esm(parent) { return ESM.open(parent) }
}
