import QtQml

QtObject {
    id: root
    property int left: 4
    property int right: 9
    property bool first: true
    property int bias: 100
    property QtObject child: QtObject {
        id: kid
        property int bias: 5
        property int target: 0
    }
    property alias mapped: kid.target
    property list<int> numbers: []
    readonly property int fixedValue: 13

    property bool enabled: false
    property int controlled: left + 1
    property Binding controller: Binding {
        target: root
        property: "controlled"
        value: left + 2
        when: root.enabled
    }

    function installBranch(): int {
        let runs = 0
        child.target = Qt.binding(function() {
            return 100 * ++runs + this.bias + (first ? left : right)
        })
        return child.target
    }
    function installAlias(): int {
        const stored = [Qt.binding(function() { return this.bias + right })]
        mapped = stored[0]
        return child.target
    }
    function installArrow(): int {
        child.target = Qt.binding(() => this.bias + left)
        return mapped
    }
    function installNumbers() { numbers = Qt.binding(() => [left, right]) }
    function values(): string { return numbers.join(",") }
    function installControlled(): int {
        controlled = Qt.binding(() => right + 100)
        return controlled
    }
    function overwrite(value: int) { mapped = value }
    function fail() {
        child.target = Qt.binding(() => 99)
        throw new Error("undo installed binding")
    }
    function cycle() { child.target = Qt.binding(() => mapped + 1) }
    function identityCycle() { child.target = Qt.binding(() => mapped) }
    function readOnly() { root.fixedValue = Qt.binding(() => 17) }
    function unsupported() { root.noSuchProperty = Qt.binding(() => 1) }
    function marker(): string {
        const fn = () => 3
        const binding = Qt.binding(fn)
        let outcome = [typeof binding, binding === fn]
        try { binding() } catch (err) { outcome.push(err.name) }
        try { Qt.binding(null) } catch (err) { outcome.push(err.name) }
        try { Qt.binding(fn, fn) } catch (err) { outcome.push(err.name) }
        return outcome.join("/")
    }
    function spin() { while (true) {} }
}
