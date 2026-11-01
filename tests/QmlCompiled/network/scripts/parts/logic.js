.import "math.js" as MathLib
var counter = 0
function increment(n) { return MathLib.add(n, ++counter) }
function origin() { return Qt.resolvedUrl("relative.qml").toString() }
