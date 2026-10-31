import QtQml

QtObject {
    id: root
    property int amount: 2
    property bool active: true
    property int other: 7
    property int reading: active ? Math.max(amount, 4) : other
    signal calculate(limit: int)
    onCalculate: function(limit) {
        let value = 0
        for (let i = 0; i < limit; ++i) {
            if (i === 2) continue
            if (i > 4) break
            value += i
        }
        root.amount = value
    }
    function fold(limit: int): int {
        let n = 0
        do {
            n++
            if (n == limit) return n * 3
        } while (n < 4)
        return -1
    }
    function fail() {
        root.amount = 99
        while (true) {}
    }
    function lazy(): bool {
        let yes = true
        yes ||= missingName
        return yes && (active || missingName)
    }
}
