import QtQuick
import QtQml.Models

Item {
    id: page
    property int seed: 3
    property bool editing: false
    property int first: seed
    property int second: 2
    property Item saved

    Binding {
        when: page.editing
        page.first: page.seed * 10
        page { second: page.seed + 1 }
    }

    ListModel {
        id: rows
        ListElement { title: "one" }
        ListElement { title: "two" }
    }
    Repeater {
        id: entries
        model: rows
        Item {
            required property int index
            required property string title
            property string label: index + "=" + title
        }
    }

    function enableEdit() { editing = true }
    function changeSeed() { seed = 5 }
    function disableEdit() { editing = false }
    function remember() { saved = entries.itemAt(0) }
    function moveRow() { rows.move(0, 1, 1) }
    function renameRow() { rows.setProperty(1, "title", "updated") }
    function eraseRow() { rows.remove(0) }
    function summary(): string {
        return first + "/" + second + "/" + entries.count + "/" +
            (entries.itemAt(0) ? entries.itemAt(0).label : "-") + "/" +
            (entries.itemAt(1) ? entries.itemAt(1).label : "-")
    }
    function savedAt(pos: int): bool { return entries.itemAt(pos) === saved }
}
