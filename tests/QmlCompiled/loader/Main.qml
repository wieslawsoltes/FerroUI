import QtQuick
Item {
    id: page
    property int activations: 0
    property int seed: 10
    // This inactive declaration makes the local URL part of the build graph.
    Loader { active: false; source: "Tile.qml" }
    Component { id: inner; Item { width: 40; height: 12; property int amount: 3 } }
    Loader { id: slot; onLoaded: page.activations++ }
    function inlineOn() { slot.sourceComponent = inner }
    function deactivate() { slot.active = false }
    function activate() { slot.active = true }
    function enqueue(value: int) { slot.setSource("Tile.qml", {amount: value}) }
    function missing() { slot.source = "Unavailable.qml" }
    function removeItem() { slot.source = "" }
    function report(): string {
        return slot.status + "/" + slot.width + "/" + slot.height + "/" +
            (slot.item ? slot.item.amount : 0) + "/" + activations
    }
}
