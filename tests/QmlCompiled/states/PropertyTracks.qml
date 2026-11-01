import QtQuick
QtObject {
    id: page
    property real straight: 0
    property real clockwiseWrap: 0
    property real counterWrap: 0
    property real shortestWrap: 0
    property real fullTurns: 0
    property real negativeTie: 0
    property real angle: 0
    property real rotation: 0
    property int steps: 0
    property real scalar: 0
    property color tint: "#2070ff30"
    property point location: Qt.point(-4, 5)
    property size extent: Qt.size(2, 3)
    property rect bounds: Qt.rect(-1, 2, 3, 4)
    property ParallelAnimation driver: ParallelAnimation {
        objectName: "property-tracks"
        RotationAnimation { target: page; property: "straight"; from: 10; to: 730; duration: 1000 }
        RotationAnimation { target: page; property: "clockwiseWrap"; from: 350; to: 10; direction: RotationAnimation.Clockwise; duration: 1000 }
        RotationAnimation { target: page; property: "counterWrap"; from: -725; to: 740; direction: RotationAnimation.Counterclockwise; duration: 1000 }
        RotationAnimation { target: page; property: "shortestWrap"; from: -725; to: 740; direction: RotationAnimation.Shortest; duration: 1000 }
        RotationAnimation { target: page; property: "fullTurns"; from: 730; to: 10; direction: RotationAnimation.Clockwise; duration: 1000 }
        RotationAnimation { target: page; property: "negativeTie"; from: 0; to: -540; direction: RotationAnimation.Shortest; duration: 1000 }
        RotationAnimation { target: page; from: 10; to: 350; direction: RotationAnimation.Shortest; duration: 1000 }
        PropertyAnimation { target: page; property: "steps"; from: 0; to: 5; duration: 1000 }
        PropertyAnimation { target: page; property: "scalar"; from: 1; to: 3; duration: 1000 }
        PropertyAnimation { target: page; property: "tint"; from: "#2070ff30"; to: "#a0a000f0"; duration: 1000 }
        PropertyAnimation { target: page; property: "location"; from: Qt.point(-4,5); to: Qt.point(6,0); duration: 1000 }
        PropertyAnimation { target: page; property: "extent"; from: Qt.size(2,3); to: Qt.size(8,10); duration: 1000 }
        PropertyAnimation { target: page; property: "bounds"; from: Qt.rect(-1,2,3,4); to: Qt.rect(1,4,7,12); duration: 1000 }
    }
    function play() { driver.start() }
}
