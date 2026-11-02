import QtQml
import Ui.State as Store
QtObject {
    property int contrast: Store.Palette.Tone.Dark - Store.Palette.Light
    property QtObject first: Consumer {}
    property QtObject second: Consumer {}
    property int total: first.observed + second.observed + Store.Session.count
    property QtObject state: Store.Session
}
