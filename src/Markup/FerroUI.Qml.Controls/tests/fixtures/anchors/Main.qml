import QtQuick

Item {
    id: page
    width: 200; height: 100
    property bool follow: true
    Item {
        id: label
        width: 10; height: 6
        anchors.horizontalCenter: follower.horizontalCenter
        anchors.bottom: marker.top
        anchors.bottomMargin: 1
    }
    Item {
        id: follower
        width: 8; height: 7
        anchors {
            left: page.follow ? marker.right : page.left
            right: page.right
            top: marker.bottom
            bottom: page.bottom
            margins: 5
            leftMargin: 3
            topMargin: 2
            bottomMargin: 6
        }
    }
    Item { id: marker; x: 20; y: 30; width: 40; height: 10 }
    function moveMarker() { marker.x = 40; marker.y = 40 }
    function resetLeft() { follow = false }
    function detachRight() { follower.anchors.right = undefined }
    function attachLeftToMarker() { follower.anchors.left = marker.right }
    function geometry(): string {
        return follower.x + "/" + follower.y + "/" + follower.width + "/" +
            follower.height + "/" + label.x + "/" + label.y
    }
}
