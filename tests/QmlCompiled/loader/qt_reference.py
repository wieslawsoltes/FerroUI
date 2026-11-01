"""Exercise the shared Loader and Repeater fixtures on the published Qt Quick engine without a display."""
import os
from pathlib import Path

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

from PySide6.QtCore import QUrl, qVersion
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlComponent, QQmlEngine


def main():
    assert qVersion() == "6.12.0", qVersion()
    app = QGuiApplication([])
    engine = QQmlEngine()
    def load(path):
        component = QQmlComponent(engine, QUrl.fromLocalFile(str(path.resolve())))
        root = component.create()
        if root is None:
            raise RuntimeError("\n".join(e.toString() for e in component.errors()))
        return component, root

    folder = Path(__file__).parent
    component, root = load(folder / "Main.qml")
    assert root.report() == "0/0/0/0/0"
    root.inlineOn()
    assert root.report() == "1/40/12/3/1"
    root.deactivate()
    root.enqueue(12)
    assert root.report() == "0/40/12/0/1"
    root.activate()
    assert root.report() == "1/30/20/12/2"
    root.enqueue(13)
    assert root.report() == "1/30/20/13/3"
    root.missing()
    assert root.report() == "3/30/20/0/3"
    root.removeItem()
    assert root.report() == "0/30/20/0/3"
    print("Qt 6.12.0 Loader lifecycle and geometry reference passed")
    component, root = load(folder.parent / "delegates/Main.qml")
    assert root.summary() == "3/2/2/0=one/1=two"
    root.enableEdit()
    assert root.summary() == "30/4/2/0=one/1=two"
    root.changeSeed()
    assert root.summary() == "50/6/2/0=one/1=two"
    root.disableEdit()
    assert root.summary() == "5/2/2/0=one/1=two"
    root.remember()
    root.moveRow()
    assert root.savedAt(1) and root.summary() == "5/2/2/0=two/1=one"
    root.renameRow()
    assert root.savedAt(1) and root.summary() == "5/2/2/0=two/1=updated"
    root.eraseRow()
    assert root.savedAt(0) and root.summary() == "5/2/1/0=updated/-"
    print("Qt 6.12.0 multi-target Binding, Repeater order and delegate identity passed")
    del root, component, engine, app


if __name__ == "__main__":
    main()
