import QtQml
QtObject {
    id: base
    virtual property int level: 1
    property int baseTotal: 0
    onLevelChanged: { baseTotal += level }
    property alias baseLevel: base.level
    property QtObject child: QtObject { property int fromScope: level }
    function dynamicRead(obj, key): int { return obj[key] }
    function baseWrite(next: int) { base.level = next }
}
