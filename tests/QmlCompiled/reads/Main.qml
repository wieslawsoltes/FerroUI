import QtQml
import Probe.Types

Surface {
    id: root
    width: 7

    component BasePane: Surface {
        virtual property int level: 3
        property int baseView: this.level
    }
    component Pane: BasePane {
        override property int level: 9
        property int derivedView: this.level
        property real fromParent: parent.width
    }

    property QtObject nested: Pane { id: pane; width: 8 }
    property real byId: pane.width
    property real own: this.width
    property real bare: width
    property real globalValue: Math.PI
    property var dynamic: null
    property real dynamicValue: dynamic.width
    property var functionValue: this.operation
    function operation() { return width }
}
