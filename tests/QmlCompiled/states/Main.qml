import QtQuick
Item {
    id: scene
    width: 20
    property bool expanded: false
    property real goal: 100
    property real value: goal / 2
    property real offset: 2
    states: [
        State {
            name: "open"
            when: scene.expanded
            PropertyChanges { scene { width: scene.goal; value: scene.goal + scene.offset } }
        }
    ]
    transitions: Transition {
        from: ""; to: "open"; reversible: true
        NumberAnimation { properties: "width"; duration: 100; easing.type: Easing.InOutQuad }
    }
    function open() { expanded = true }
    function close() { expanded = false }
}
