import QtQml
import "parts" as Parts

QtObject {
    required property int amount
    property Parts.Badge badge: Parts.Badge {}
    property int total: badge.value + amount
    function format() { return total + " points" }
}
