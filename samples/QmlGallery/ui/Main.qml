import QtQuick
import QtQuick.Controls

Column {
    id: page
    spacing: 12
    property int count: 0

    Text {
        text: "FerroUI QML"
        font.pixelSize: 28
        font.bold: true
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
            text: "Increment"
            onClicked: page.count = page.count + 1
        }
        Text {
            text: "Count: " + page.count
        }
    }
}
