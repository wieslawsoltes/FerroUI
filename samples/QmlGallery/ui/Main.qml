import QtQuick
import QtQuick.Controls
import FerroUI.Controls as Ferro

Column {
    id: page
    spacing: 12
    property int count: 0

    Ferro.TextBlock {
        text: "FerroUI QML"
        fontSize: 28
    }

    TextField {
        id: nameField
        width: 220
        placeholderText: "Your name"
    }

    Text {
        text: nameField.text === "" ? "Hello!" : "Hello, " + nameField.text
    }

    Row {
        spacing: 8
        Button {
            id: increment
            text: "Increment"
            onClicked: page.count = page.count + 1
        }
        Text {
            id: countText
            text: "Count: " + page.count
        }
    }
}
