import QtQml
import "sub" as Sources

QtObject {
    id: app
    property Sources.Reader reader: Sources.Reader {
        function fromSite(): url { return Qt.resolvedUrl("site.svg") }
    }
    function local(): url { return Qt.resolvedUrl("icons/../logo.svg?mode=dark#mark") }
    function relative(): url { return Qt.url("../icons/logo.svg") }
    function display(): url { return Qt.resolvedUrl("icons/large%20caf%C3%A9%2Fdark.svg") }
    function absolute(): url { return Qt.resolvedUrl("qrc:/app/icons/logo.svg") }
    function empty(): url { return Qt.resolvedUrl("") }
    function definition(): url { return reader.fromDefinition() }
    function site(): url { return reader.fromSite() }
    function objectContext(): url { return Qt.resolvedUrl("parent.svg", reader) }
    function engineBase(): url { return reader.fromEngine() }
}
