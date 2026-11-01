import QtQml
QtObject {
    id: scene
    property int offset: 2
    property int completions: 0
    property int destructions: 0
    property QtObject retained: null
    property list<QtObject> saved: []
    function hold(obj: QtObject) { retained = obj; saved = [obj] }
    function removeLater(obj: QtObject, ms: int) { obj.destroy(ms) }
    property Component factory: Component {
        QtObject {
            id: row
            required property int seed
            property QtObject content: QtObject { id: internal; property int extra: 3 }
            property alias exposed: internal.extra
            property int score: seed + scene.offset + internal.extra
            Component.onDestruction: scene.destructions += 1
            Component.onCompleted: scene.completions += 1
        }
    }
    function make(n: int): QtObject { return factory.createObject(scene, { seed: n }) }
}
