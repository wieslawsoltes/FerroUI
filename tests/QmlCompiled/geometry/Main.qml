import QtQml

QtObject {
    id: root
    property point position: Qt.point(1.25, 2)
    property size extent
    property rect frame: "1,2,10x20"
    property point copy
    property size grouped
    grouped { width: root.position.x; height: root.frame.bottom }
    function offset(p: point): point { p.x += 10; return p }
    function show(): string { return frame.toString() }
    property real edge: frame.right + frame.bottom
    function edit(): real {
        let ref = position
        ref.x += 2
        copy = ref
        root.position.y = 5
        let box = Qt.rect(2, 4, 6, 8)
        box.width += 2
        frame = box
        extent = Qt.size(4, 7)
        return edge + copy.x + copy.y + position.y + extent.width
    }
    function fail() { frame.height = 100; root.unknownMethod() }
}
