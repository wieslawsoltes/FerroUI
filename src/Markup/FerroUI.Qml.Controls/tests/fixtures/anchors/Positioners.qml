import QtQuick

Item {
    id: page; width: 80; height: 60
    LayoutMirroring.enabled: true
    LayoutMirroring.childrenInherit: true
    Row {
        id: row; width: 80; height: 10; spacing: 2
        Item { id: first; width: 10; height: 5 }
        Item { id: second; width: 20; height: 5 }
    }
    Flow {
        id: flow; width: 50; height: 20; spacing: 2; y: 15
        Item { id: a; width: 20; height: 5 }
        Item { id: b; width: 10; height: 5 }
        Item { id: c; width: 40; height: 5 }
    }
    function positions(): string { return first.x + "/" + second.x + "/" + a.x + "/" + b.x + "/" + c.x }
    function reverseDirections() { row.layoutDirection = 1; flow.layoutDirection = 1 }
}
