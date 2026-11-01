pragma ListPropertyAssignBehavior: ReplaceIfNotDefault
pragma FunctionSignatureBehavior: Ignored
import QtQml
QtObject {
    default property list<QtObject> choices: [QtObject {objectName:"base"}]
    property list<QtObject> extras: [QtObject {id: original; objectName:"old"}]
    property list<int> sequence: [1,2]
    function passthrough(input: int): int { return input }
    function oldName(): string { return original.objectName }
}
