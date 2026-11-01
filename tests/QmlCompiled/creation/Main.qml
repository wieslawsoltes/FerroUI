import QtQml
QtObject {
    id: scene
    property int offset: 2
    property int completions: 0
    property Component factory: Component {
        QtObject {
            id: row
            required property int seed
            property QtObject content: QtObject { id: internal; property int extra: 3 }
            property alias exposed: internal.extra
            property int score: seed + scene.offset + internal.extra
            Component.onCompleted: scene.completions += 1
        }
    }
    function make(n: int): QtObject { return factory.createObject(scene, { seed: n }) }
}
