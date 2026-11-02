function open(parent) { return Qt.createComponent("Part.qml").createObject(parent).marker }
function asset() { return Qt.resolvedUrl("classic.svg") }
