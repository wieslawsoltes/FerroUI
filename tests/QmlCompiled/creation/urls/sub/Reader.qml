import QtQml

QtObject {
    function fromDefinition(): url { return Qt.resolvedUrl("reader.svg") }
    function fromEngine(): url { return Qt.resolvedUrl("fallback.svg", null) }
}
