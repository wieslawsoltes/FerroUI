import QtQml
import Ui.State as Store
QtObject {
    property QtObject first: Consumer {}
    property QtObject second: Consumer {}
    property int total: first.observed + second.observed + Store.Session.count
    property QtObject state: Store.Session
}
