import QtQml
import "sub" as Sub
QtObject {
    id: app
    property Sub.Opener opener: Sub.Opener {}
    property var retained: null
    function capturedUrl(): url { const resolve = opener.captureUrl(); return resolve("captured.svg") }
    function classicUrl(): url { return opener.classicUrl() }
    function esmUrl(): url { return opener.esmUrl() }
    function capture() { retained = opener.capture() }
    function run(): string { return [retained(app),opener.classic(app),opener.esm(app)].join("/") }
}
