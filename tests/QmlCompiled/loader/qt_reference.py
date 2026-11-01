"""Exercise the shared Loader fixture on the published Qt Quick engine without a display."""
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
    component = QQmlComponent(engine, QUrl.fromLocalFile(str(Path(__file__).with_name("Main.qml").resolve())))
    root = component.create()
    if root is None:
        raise RuntimeError("\n".join(e.toString() for e in component.errors()))
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
    del root, component, engine, app


if __name__ == "__main__":
    main()
