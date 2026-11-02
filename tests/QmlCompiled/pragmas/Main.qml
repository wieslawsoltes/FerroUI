pragma FunctionSignatureBehavior: Enforced
import QtQml
Base {
    id:root
    QtObject {objectName:"sub"}
    extras:[QtObject {objectName:"new"}]
    sequence:[3,4]
    property point p: ({ x: 3, y: 4 })
    property size dimensions: ({ width: 7 })
    property rect frame: ({ x: 1, height: 9 })
    property QtObject appended: AppendList { entries:[QtObject {objectName:"appended"}] }
    property QtObject replaced: ReplaceList { entries:[QtObject {objectName:"replaced"}] }
    function policies(): string {
        return appended.entries.length + "/" + appended.entries[0].objectName + "/" +
               replaced.entries.length + "/" + replaced.entries[0].objectName
    }
    function numeric(n: int): int { return n }
    function arities(): string { return numeric("39") + "/" + numeric("not numeric") + "/" + numeric() + "/" + passthrough("kept",9) }
    function report(): string {
        return choices.length + "/" + choices[0].objectName + "/" + choices[1].objectName + "/" +
               extras.length + "/" + extras[0].objectName + "/" + sequence[0] + "/" +
               oldName() + "/" + passthrough("retained") + "/" +
               p.x + "/" + p.y + "/" + dimensions.width + "/" + dimensions.height +
               "/" + frame.x + "/" + frame.height
    }
    function change() { p = {y:6}; dimensions = Qt.rect(3,4,5,6); frame = null }
}
