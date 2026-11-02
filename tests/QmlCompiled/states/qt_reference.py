"""Compare the shared state fixture with the released Qt Quick engine."""
from pathlib import Path
from PySide6.QtCore import QEventLoop, QObject, QTimer, QUrl, qVersion
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlComponent, QQmlEngine

def settle():
    loop = QEventLoop()
    QTimer.singleShot(250, loop.quit)
    loop.exec()

app = QGuiApplication([])
assert qVersion() == "6.12.0"
engine = QQmlEngine()
component = QQmlComponent(engine, QUrl.fromLocalFile(str(Path(__file__).with_name("Main.qml").resolve())))
root = component.create()
assert root is not None, [error.toString() for error in component.errors()]
root.open()
assert root.property("state") == "open" and root.property("value") == 50
settle()
assert root.property("width") == 100 and root.property("value") == 102
root.close()
settle()
assert root.property("width") == 20 and root.property("value") == 50
print("Qt 6.12.0 State, PropertyChanges and reversible Transition reference passed")

# Scrub Qt's animation tree with a public AnimationController. This exercises
# the same exact timestamps as the Rust owner clock without wall-clock sampling.
controller_type = QQmlComponent(engine)
controller_type.setData(b"import QtQuick\nAnimationController {}", QUrl())
controller = controller_type.create()
group = root.findChild(QObject, "group-under-test")
assert group is not None and controller.setProperty("animation", group)
for elapsed, gauge, color, latched in [
    (50, 5, "#000000", False), (150, 10, "#7f7f7f", False),
    (275, 10, "#ffffff", False), (350, 15, "#ffffff", True), (400, 20, "#ffffff", True),
]:
    controller.setProperty("progress", elapsed / 400)
    assert root.property("gauge") == gauge
    assert root.property("accent").name() == color
    assert root.property("latched") == latched
print("Qt 6.12.0 nested animation groups, color channels and discrete actions matched at five exact times")

# Serialize one independent Qt trace consumed by both FerroUI runtime paths.
component = QQmlComponent(engine, QUrl.fromLocalFile(str(Path(__file__).with_name("PropertyTracks.qml").resolve())))
root = component.create()
assert root is not None, [error.toString() for error in component.errors()]
controller = controller_type.create()
assert controller.setProperty("animation", root.findChild(QObject, "property-tracks"))
def fields(elapsed):
    controller.setProperty("progress", elapsed / 1000)
    scalar_names = ("straight", "clockwiseWrap", "counterWrap", "shortestWrap", "fullTurns",
                    "negativeTie", "angle", "rotation", "steps", "scalar")
    values = [format(root.property(name), ".12g") for name in scalar_names]
    color = root.property("tint")
    point, size, rect = [root.property(name) for name in ("location","extent","bounds")]
    geometry = [point.x(),point.y(),size.width(),size.height(),rect.x(),rect.y(),rect.width(),rect.height()]
    return "\t".join([str(elapsed), *values, color.name(color.NameFormat.HexArgb), *(format(v, ".12g") for v in geometry)])
# Seek once before sampling zero so the controller writes the declared start values.
controller.setProperty("progress", 0.001)
actual = "\n".join(fields(t) for t in (0, 250, 500, 750, 999, 1000)) + "\n"
expected = Path(__file__).with_name("property_tracks.tsv").read_text()
assert actual == expected, actual
print("Qt 6.12.0 rotation directions, integer/color interpolation and geometry tracks passed at six exact times")
