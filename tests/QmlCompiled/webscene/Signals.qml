import QtQml

QtObject {
    id: root
    signal reading(value: int)
    signal forwarded(value: int)
    signal edit()
    property string history: ""
    property int count: 0
    onForwarded: (value) => { history += "S" + value }

    function add(value: int) { history += "M" + value }
    function install() {
        reading.connect(add)
        root.reading.connect(root.add)
        reading.connect(forwarded)
    }
    function removeAll() {
        reading.disconnect(add)
        reading.disconnect(forwarded)
        reading.disconnect(() => {})
    }
    function mutation(): string {
        let log = ""
        const second = () => { log += "B" }
        const third = () => { log += "C" }
        const first = () => { log += "A"; edit.disconnect(second); edit.connect(third) }
        edit.connect(first)
        edit.connect(second)
        edit()
        const before = log
        edit()
        return before + "/" + log
    }
    function observeCount() { countChanged.connect(() => { history += "C" + count }) }
    function explicitReceiver(): string {
        const context = { value: 1 }
        function change(value) { this.value += value }
        reading.connect(context, change)
        reading(3)
        reading.disconnect(context, change)
        reading(4)
        return String(context.value)
    }
    function badArgs(): string {
        let invalid = 0
        try { reading.connect(5) } catch (err) { invalid++ }
        try { reading.disconnect() } catch (err) { invalid++ }
        return String(invalid)
    }

    function connectAndFail() { reading.connect(add); throw new Error("undo connection") }
    function disconnectAndFail() { removeAll(); throw new Error("undo disconnect") }
    function failAfterWrite(value) { history = "discarded"; throw new Error("callback failed") }
    function installFailure() { reading.connect(failAfterWrite) }
    function clearFailure() { reading.disconnect(failAfterWrite) }
    function spin() { while (true) {} }
}
