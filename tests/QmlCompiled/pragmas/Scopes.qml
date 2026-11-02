import QtQuick
Item {
    Bound {id: restricted}
    Unbound {id: dynamicSource}
    function edit():string { return dynamicSource.edit() }
    function report():string { return restricted.report() + ";" + dynamicSource.report() }
}
