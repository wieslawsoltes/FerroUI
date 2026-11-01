import QtQml
import Ui.State
import "counter.mjs" as Counter

QtObject {
    function next() { return Counter.next() + Session.count }
}
