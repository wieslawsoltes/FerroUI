export function open(parent) { return Qt.createComponent("Part.qml").createObject(parent).marker }
export function asset() { return Qt.resolvedUrl("module.svg") }
