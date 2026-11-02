import QtQml
Middle {
    id: finalView
    final property int level: 15
    property int ownTotal: 0
    onLevelChanged: { ownTotal += level }
    function observe(): string {
        baseWrite(2)
        middleWrite(7)
        level = 25
        return baseLevel + "/" + middleLevel + "/" + level + "/" + child.fromScope + "/" + dynamicRead(finalView, "level") + "/" + baseTotal + "/" + middleTotal + "/" + ownTotal
    }
}
