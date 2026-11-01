"""Compare the shared state fixture with the released Qt Quick engine."""
from pathlib import Path
from PySide6.QtCore import QEventLoop, QTimer, QUrl, qVersion
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
