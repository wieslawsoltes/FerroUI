import QtQml

QtObject {
    id: root

    property bool chooseFirst: true
    property int left: 2
    property int right: 20
    property int selected: chooseFirst ? left : right
    property int delivered: 1
    property int watched: 0
    property int accumulated: 0
    onWatchedChanged: { accumulated += watched }

    signal adjusted(delta: int)
    onAdjusted: (delta) => { delivered += delta }

    component Counter: QtObject {
        id: unit
        property int count: 0
        property QtObject detail: QtObject {
            id: internalDetail
            property int doubled: unit.count * 2
        }
        property alias result: internalDetail.doubled

        function bump(amount: int): int {
            count += amount
            return result
        }
    }

    property Counter first: Counter { count: 3 }
    property Counter second: Counter { count: 11 }

    function branch(): string {
        let before = selected
        chooseFirst = false
        left = 8
        let switched = selected
        right = 24
        return before + "/" + switched + "/" + selected
    }

    function detach(): string {
        selected = 99
        right = 30
        chooseFirst = true
        return selected + "/" + left
    }

    function signalDelivery(): string {
        adjusted(4)
        adjusted(3)
        return "" + delivered
    }

    function componentScope(): string {
        let updated = first.bump(4)
        return updated + "/" + second.result + "/" + first.result
    }

    function localFlow(): string {
        let total = 0
        for (let i = 0; i < 8; ++i) {
            if (i == 2) continue;
            if (i > 5) break;
            total += i
        }
        do { total += 1 } while (total < 16);
        return "" + total
    }

    function propertyNotifications(): string {
        let start = accumulated
        watched = 3
        let during = accumulated - start
        watched = 5
        return during + "/" + (accumulated - start)
    }

    function lazyEval(): string {
        let skipped = false && first.bump(300)
        let present = true || first.bump(400)
        let val = 0 ?? first.bump(500)
        return skipped + "/" + present + "/" + val + "/" + first.count
    }

    property var recordValue: ({ inner: { n: 3 } })
    property int recordReading: recordValue.inner.n

    function recordIdentity(): string {
        let before = recordReading
        let same = recordValue
        same.inner["n"] = 7
        let stale = recordReading
        let distinct = same === ({ inner: { n: 7 } })
        recordValue = ({ inner: { n: 12 } })
        return before + "/" + same.inner.n + "/" + stale + "/" + distinct + "/" + recordReading
    }

    function arrayIdentity(): string {
        let data = [1, 2]
        let alias = data
        data.push(3)
        alias[5] = 6
        data.length = 4
        let copied = data.slice(1)
        let last = copied.pop()
        return (alias === data) + "/" + (copied === data) + "/" + data.join(",") + "/" + copied.join(":") + "/" + last
    }
}
