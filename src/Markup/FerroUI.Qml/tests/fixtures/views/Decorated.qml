import QtQuick
ListView {
    id: list
    width: 160; height: 160; spacing: 3; cacheBuffer: 0
    verticalLayoutDirection: ListView.BottomToTop
    model: ListModel {
        id: rows
        ListElement { title: "AA" }
        ListElement { title: "AB" }
        ListElement { title: "BB" }
        ListElement { title: "BA" }
        ListElement { title: "XX" }
    }
    header: Item { width: 7; height: 7 }
    footer: Item { width: 9; height: 9 }
    section.property: "title"
    section.criteria: ViewSection.FirstCharacter
    section.delegate: Item {
        required property string section
        width: 11; height: 11
    }
    delegate: Item {
        required property int index
        property string neighbors: ListView.previousSection + ListView.section + ListView.nextSection
        width: 20; height: 20
    }
    function report(): string {
        forceLayout()
        return currentSection + "/" + contentHeight + "/" + headerItem.y + "/" +
               footerItem.y + "/" + itemAtIndex(0).y + "/" + itemAtIndex(2).y + "/" +
               indexAt(5,-12) + "/" + indexAt(5,-2) + "/" + itemAtIndex(1).neighbors + "/" + itemAtIndex(1).y
    }
    function regroup() { rows.setProperty(1,"title","BA"); forceLayout() }
}
