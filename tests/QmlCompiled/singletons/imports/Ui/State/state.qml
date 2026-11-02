pragma Singleton
import QtQml
QtObject {
    property int count: 2
    function increment() { count += 1 }
}
