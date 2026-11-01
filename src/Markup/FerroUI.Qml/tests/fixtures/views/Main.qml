import QtQuick
import QtQml.Models

Item {
    width: 100
    height: 60
    ListView {
        id: list
        width: 100
        height: 60
        cacheBuffer: 0
        model: 1000000
        delegate: Item {
            required property int index
            width: 100
            height: 20
        }
    }

    ListModel { id: rows; ListElement { title: "A" } ListElement { title: "B" } }
    ListView {
        id: editable
        width: 100; height: 60
        model: rows
        delegateModelAccess: DelegateModel.ReadWrite
        delegate: Item {
            required property int index
            required property string title
            required property var model
            width: 100; height: 20
            function edit() { title = "updated" }
        }
    }
    function scrollMiddle() {
        list.forceLayout()
        list.positionViewAtIndex(500000, ListView.Beginning)
        list.forceLayout()
    }
    function report(): string {
        return list.count + "/" + list.contentHeight + "/" + list.contentY + "/" +
               list.indexAt(5,list.contentY + 4) + "/" + list.itemAtIndex(500000).index +
               "/" + (list.itemAtIndex(400000) === null)
    }
    function changeCurrent() { editable.currentIndex = 1; editable.currentItem.edit(); rows.move(1,0,1); editable.forceLayout() }
    function selection(): string {
        return editable.currentIndex + "/" + editable.currentItem.title + "/" + rows.get(0).title
    }
}
