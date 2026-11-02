import QtQml

QtObject {
    id: counter
    required property int start
    property QtObject model: QtObject {
        id: privateModel
        property int count: start
    }
    property alias count: privateModel.count
    function increment(delta: int): int {
        counter.count += delta
        return counter.count
    }
}
