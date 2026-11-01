import QtQuick
import QtQuick.Controls
Item {
    id:page
    property int moves:0
    property int edits:0
    Slider { id: level; from:10; to:0; value:6; stepSize:3; orientation:Qt.Vertical; onMoved:page.moves+=1 }
    ProgressBar { id: progress; from:level.from; to:level.to; value:level.value }
    SpinBox { id: spin; from:10; to:0; value:6; stepSize:3; editable:true; wrap:true; onValueModified:page.edits+=1 }
    function report():string {
        return level.value + "/" + level.position + "/" + level.visualPosition + "/" + progress.position + "/" +
            spin.value + "/" + spin.displayText + "/" + level.valueAt(0.5) + "/" + moves + "/" + edits
    }
    function step() { level.increase(); spin.increase() }
    function clamp() { level.value = 13; spin.value = 0; spin.increase() }
}
