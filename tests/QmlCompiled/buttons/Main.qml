import QtQuick
import QtQuick.Controls
Item {
    id: root
    property string events: ""
    ButtonGroup { id: choices; onClicked: button => root.events += "G" }
    RadioButton {
        id: first; text: "First"; ButtonGroup.group: choices
        onClicked: root.events += "C"
        onToggled: root.events += "T"
    }
    RadioButton { id: second; text: "Second"; ButtonGroup.group: choices }
    CheckBox {
        id: mixed; text: "Include"; tristate: true
        onClicked: root.events += "K"
        onToggled: root.events += "X"
    }
    Switch { id: power; text: "Power"; Accessible.name: "Main power"; onToggled: root.events += "S" }
    function report(): string {
        return choices.buttons.length + "/" + (choices.checkedButton === first) + "/" + choices.checkState +
            "/" + first.checked + "/" + second.checked + "/" + mixed.checked +
            "/" + mixed.checkState + "/" + power.position + "/" + events
    }
    function choose() { first.click() }
    function setSecond() { second.checked = true }
    function groupAll() { choices.checkedButton = null; choices.exclusive = false; choices.checkState = Qt.Checked }
    function nextCheck() { mixed.click() }
    function flip() { power.click() }
    function detach() { first.ButtonGroup.group = null; power.Accessible.name = "Backup" }
    function attached(): string { return (first.ButtonGroup.group === choices) + "/" + power.Accessible.name }
}
