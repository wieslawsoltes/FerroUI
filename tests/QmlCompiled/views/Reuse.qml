import QtQuick
ListView {
    id: list
    width: 40; height: 40; cacheBuffer: 0
    reuseItems: true; model: 200
    property int pooled: 0
    property int reused: 0
    property int invalid: 0
    delegate: Item {
        required property int index
        property int lastIndex: -1
        width: 20; height: 10
        ListView.onPooled: { lastIndex = index; list.pooled += 1 }
        ListView.onReused: {
            if (lastIndex < 0 || lastIndex === index || ListView.view !== list) {
                list.invalid += 1
            }
            lastIndex = -1
            list.reused += 1
        }
    }
    function jump(i: int) {
        positionViewAtIndex(i, ListView.Beginning)
        forceLayout()
    }
    function valid(): bool { return pooled >= reused && reused > 0 && invalid === 0 }
}
