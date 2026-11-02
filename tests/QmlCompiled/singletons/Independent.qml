import QtQml
import Ui.State

QtObject {
    id: root
    property int base: 5
    property int observed: Session.count + inner.offset
    property QtObject state: Session
    property QtObject item: QtObject { id: inner; property int offset: root.base }
    property Consumer consumer
    property bool failOnLoad: false
    property bool failOnUnload: false
    Component.onCompleted: {
        if (failOnLoad) {
            Session.increment()
            unavailable()
        }
    }
    Component.onDestruction: {
        if (failOnUnload) {
            Session.increment()
            unavailable()
        }
    }
    function tick() { Session.increment() }
}
