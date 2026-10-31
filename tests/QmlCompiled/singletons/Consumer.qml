import QtQml
import Ui.State
QtObject {
    property int observed: Session.count
    function tick() { Session.increment() }
}
