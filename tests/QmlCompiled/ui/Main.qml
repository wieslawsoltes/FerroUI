import QtQml
import "widgets" as Widgets

QtObject {
    component DoubleCounter: Widgets.Counter {
        readonly property int twice: count * 2
    }
    property DoubleCounter first: DoubleCounter { id: left; start: 3 }
    property Widgets.Counter second: Widgets.Counter { id: right; start: 11 }
    property int sum: left.twice + right.count
    property string caption: "Count: " + sum
}
