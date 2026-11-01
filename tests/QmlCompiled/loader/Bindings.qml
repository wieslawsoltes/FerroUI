import QtQuick
Item {
    id: page
    property int seed: 7
    Loader { id: slot; active: false; source: "Tile.qml" }
    function bind() {
        slot.setSource("Tile.qml", { amount: Qt.binding(function(){ return page.seed * 2 }) })
        slot.active = true
    }
}
