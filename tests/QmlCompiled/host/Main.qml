import QtQml
import Host

QtObject {
    id: root
    property Meter first: Meter { input: 2 }
    property Meter second: Meter { input: 5 }
    property bool chooseFirst: true
    property int selected: (chooseFirst ? first : second).percent()
    property int edits: 0
    property Connections observer: Connections { target: first; function onCommitted() { root.edits++ } }
    function edit() { first.commitValue(8) }
    function fail() { first.abortWrite(99) }
}
