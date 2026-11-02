import QtQuick
Item {
    width: 30
    height: 20
    required property int amount
    property int completed: -1
    Component.onCompleted: completed = amount
}
