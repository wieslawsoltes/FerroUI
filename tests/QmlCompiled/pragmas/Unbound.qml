pragma ComponentBehavior: Unbound
import QtQuick
Item {
    id: owner
    QtObject { id: duplicate; objectName: "outer" }
    ListModel { id: entries; ListElement {title:"A"} }
    Repeater { id: implicitRows; model:entries
        delegate: Item {
            property var duplicate: "local"
            Item {
                id: nested
                function edit():string { duplicate="changed"; return duplicate }
                function resolve():string {return duplicate === "local" ? duplicate : duplicate.objectName}
            }
            function edit():string {return nested.edit()}
            function report(): string {
                return (duplicate === "local" ? duplicate : duplicate.objectName) + "/" +
                    (typeof index) + "/" + (typeof title) + "/" + this.duplicate + "/" + nested.resolve()
            }
        }
    }
    ListView { id: list; width:30; height:30; model:entries
        delegate: Item { required property string title; required property int index; width:10; height:10
            function report():string {return title + index}
        }
    }
    property Component factory: Component {
        Item {
            property var duplicate:"local"
            function resolve():string {return duplicate === "local" ? duplicate : duplicate.objectName}
        }
    }
    function edit():string {return implicitRows.itemAt(0).edit()}
    function report(): string {
        let object=factory.createObject(owner)
        return implicitRows.itemAt(0).report() + "/" + list.itemAtIndex(0).report() + "/" + object.resolve()
    }
}
