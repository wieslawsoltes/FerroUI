pragma ListPropertyAssignBehavior: Append
import QtQml
QtObject {
    property list<QtObject> entries: [QtObject {objectName:"base"}]
}
