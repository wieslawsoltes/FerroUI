"""Run the dynamic object fixtures against the published Qt runtime."""
from pathlib import Path
from PySide6.QtCore import QCoreApplication, QEventLoop, QTimer, QUrl, qVersion
from PySide6.QtQml import QQmlComponent, QQmlEngine

def main():
    assert qVersion() == '6.12.0'
    app = QCoreApplication([])
    engine = QQmlEngine()
    folder = Path(__file__).resolve().parent
    def load(path):
        component = QQmlComponent(engine, QUrl.fromLocalFile(str(folder / path)))
        root = component.create()
        assert root is not None, [error.toString() for error in component.errors()]
        return component, root

    component, root = load("Main.qml")
    root.make(4)
    root.make(7)
    a, b = [item for item in root.children() if item.property("seed") is not None]
    assert a.property("score") == 9 and b.property("score") == 12
    a.setProperty("exposed", 5)
    root.setProperty("offset", 10)
    assert a.property("score") == 19 and b.property("score") == 20
    root.hold(a)
    root.removeLater(a, 0)
    assert root.property("retained") == a
    loop = QEventLoop()
    QTimer.singleShot(25, loop.quit)
    loop.exec()
    assert root.property("retained") is None and root.property("destructions") == 1
    for path, cases in [("Initial.qml", [("probe", "7/15/false/true/5,9")]), ("catalog/Main.qml", [("make",11), ("makeType","named"), ("makeViaString",13)])]:
        component, root = load(path)
        for method, expected in cases:
            assert getattr(root, method)() == expected
    engine.setBaseUrl(QUrl.fromLocalFile(str(folder / "origin") + "/"))
    component, root = load("origin/Main.qml")
    root.capture()
    assert root.run() == "73/84/95"
    print("Qt 6.12.0 dynamic component, factory, binding and lifetime probes passed")

if __name__ == "__main__":
    main()
