import QtQml
import "sub" as Sub
QtObject {
    id: app
    property Sub.Opener opener: Sub.Opener {}
    property var retained: null
    function capture() { retained = opener.capture() }
    function run(): string { return [retained(app),opener.classic(app),opener.esm(app)].join("/") }
}
