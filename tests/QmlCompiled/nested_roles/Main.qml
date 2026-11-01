import QtQuick
import QtQml.Models
Item {
    id: page
    property var saved
    ListModel {
        id: groups
        ListElement {
            title: "first"
            entries: [
                ListElement { label: "A"; tags: ListElement { number: 3 } },
                ListElement { label: "B"; tags: [] }
            ]
        }
        ListElement { title: "second"; entries: [] }
    }
    Repeater {
        id: outer
        model: groups
        Item {
            id: groupItem
            required property var entries
            Repeater {
                id: inner
                model: groupItem.entries
                Item { required property string label }
            }
            function labels(): string {
                return inner.count + "/" + (inner.count > 0 ? inner.itemAt(0).label : "-") +
                    "/" + (inner.count > 1 ? inner.itemAt(1).label : "-")
            }
        }
    }
    function remember() { saved = groups.get(0).entries }
    function edit() {
        saved.get(0).tags.append({number:7})
        saved.setProperty(1,"label","BB")
        groups.move(0,1,1)
    }
    function replace() { groups.set(1,{entries:[{label:"X",tags:[{number:9}]}]}) }
    function appendGroup() { groups.append({title:"third",entries:[{label:"C",tags:[]}]}) }
    function addNested() { groups.get(1).entries.append({label:"Y",tags:[]}) }
    function nestedEdit(): string {
        return outer.itemAt(1).labels() + "/" + (saved === groups.get(1).entries) + "/" +
            saved.get(0).tags.get(1).number
    }
    function replaced(): string {
        return outer.itemAt(1).labels() + "/" + (saved === null) + "/" +
            groups.get(1).entries.get(0).tags.get(0).number
    }
    function rollbackEdit() { groups.set(1,{entries:[{label:"Oops",tags:[]}]}); nonexistent() }
    function clearData() { groups.clear() }
}
