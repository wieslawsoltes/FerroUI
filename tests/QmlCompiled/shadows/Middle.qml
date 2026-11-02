import QtQml
Base {
    id: middle
    override property int level: 9
    property alias middleLevel: middle.level
    property int middleTotal: 0
    onLevelChanged: { middleTotal += level }
    function middleWrite(next: int) { level = next }
}
