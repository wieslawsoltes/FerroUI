import QtQuick
ListView {
    id: list; width: 100; height: 100
    model: [{title:""}, {title:"A"}, {title:"AB"}]
    section.property: "title"
    section.delegate: Item { width: 4; height: 4 }
    delegate: Item {
        required property int index
        property string label: ListView.section
        width: 10; height: 10
    }
    function report(): string {
        forceLayout()
        return section.property + "/" + currentSection + "/" + itemAtIndex(1).label +
               "/" + itemAtIndex(2).label + "/" + contentHeight
    }
    function regroup() { list.section.criteria = ViewSection.FirstCharacter; forceLayout() }
}
