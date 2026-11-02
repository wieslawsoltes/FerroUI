import QtQml
QtObject {
    required property int amount
    property int seen: -1
    Component.onCompleted: seen = amount
}
