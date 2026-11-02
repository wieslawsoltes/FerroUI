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
    property real gauge: 0
    property color accent: "#000000"
    property bool latched: false
    property alias playback: sequence
    SequentialAnimation {
        id: sequence
        objectName: "group-under-test"
        NumberAnimation { target: scene; property: "gauge"; from: 0; to: 10; duration: 100 }
        ParallelAnimation {
            PauseAnimation { duration: 200 }
            ColorAnimation { target: scene; property: "accent"; from: "#000000"; to: "#ffffff"; duration: 100 }
        }
        PropertyAction { target: scene; property: "latched"; value: true }
        NumberAnimation { target: scene; property: "gauge"; from: 10; to: 20; duration: 100 }
    }
    function play() { sequence.start() }
    function open() { expanded = true }
    function close() { expanded = false }
}
