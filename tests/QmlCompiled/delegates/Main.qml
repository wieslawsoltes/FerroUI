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

    ListModel {
        id: stock
        ListElement { title: "A"; cost: 3 }
    }
    Repeater {
        id: multi
        model: stock
        Item {
            required property var modelData
            required property var model
            function rename() { model.title = "Z" }
            function describe() { return modelData.title + "/" + model.cost + "/" + (modelData === model) }
        }
    }
    Repeater {
        id: implicitMulti
        model: stock
        Item { property bool implicitDataAbsent: modelData === undefined }
    }
    ListModel { id: evolving; ListElement { title: "A" } }
    Repeater {
        id: stable
        model: evolving
        Item {
            required property var modelData
            required property var model
            function describe() { return (model.other === undefined ? modelData : modelData.title) + "/" + model.other }
        }
    }
    function multipleRoles(): string {
        return multi.itemAt(0).describe() + "/" + implicitMulti.itemAt(0).implicitDataAbsent
    }
    function renameThroughDelegate() { multi.itemAt(0).rename() }
    function growRoles() { evolving.setProperty(0, "other", 17); evolving.append({title:"B", other:19}) }
    function reattach() { stable.model = null; stable.model = evolving }
    function rowsSeen(): string {
        return stable.itemAt(0).describe() + "/" + stable.itemAt(1).describe() + "/" + evolving.get(0).other
    }
}
