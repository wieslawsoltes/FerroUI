import QtQuick

Item {
    id: page
    width: 101; height: 60
    LayoutMirroring.enabled: true
    LayoutMirroring.childrenInherit: true
    Item { id: marker; x: 10; y: 11; width: 20; height: 10; baselineOffset: 7 }
    Item {
        id: label
        width: 10; height: 7; baselineOffset: 2
        anchors.left: marker.right; anchors.leftMargin: 2
        anchors.baseline: marker.baseline; anchors.baselineOffset: 3
    }
    Item {
        id: inset
        anchors.fill: parent; anchors.leftMargin: 2; anchors.rightMargin: 7
    }
    Item {
        id: middle; width: 10; height: 8
        anchors.centerIn: parent; anchors.horizontalCenterOffset: 4
    }
    Item {
        id: local; LayoutMirroring.enabled: false
        Item { id: leaf }
    }
    function frames(): string {
        return label.x + "/" + label.y + "/" + inset.x + "/" + inset.width + "/" + middle.x
    }
    function inherited(): string {
        return label.LayoutMirroring.enabled + "/" + local.LayoutMirroring.enabled + "/" + leaf.LayoutMirroring.enabled
    }
    function change() {
        page.LayoutMirroring.enabled = false
        marker.baselineOffset = 9
    }
    function inheritLocal() { local.LayoutMirroring.childrenInherit = true }
    function resetLocal() { local.LayoutMirroring.enabled = undefined }
}
