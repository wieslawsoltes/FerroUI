import QtQml

QtObject {
    id: page
    property int count: 1
    property int doubled: count * 2
    property int completed: 0
    property QtObject part: Part {}
    Component.onCompleted: completed = doubled
    function outer(arg: int): int { return nested(arg) + nested(arg + 1) }
    function nested(arg: int): int { return part.add(arg) }
    function mutate(): int { count += 1; return nested(count) }
}
