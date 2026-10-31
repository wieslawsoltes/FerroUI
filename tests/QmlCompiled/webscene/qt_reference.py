"""Check the shared JavaScript fixture using the published Qt engine."""
from pathlib import Path

from PySide6.QtCore import QCoreApplication, QUrl, qVersion
from PySide6.QtQml import QQmlComponent, QQmlEngine


def main():
    if qVersion() != "6.12.0":
        raise RuntimeError(f"expected Qt 6.12.0, found {qVersion()}")
    app = QCoreApplication([])
    engine = QQmlEngine()
    component = QQmlComponent(engine, QUrl.fromLocalFile(str(Path(__file__).with_name("App.qml").resolve())))
    root = component.create()
    if root is None:
        raise RuntimeError("\n".join(issue.toString() for issue in component.errors()))
    assert root.property("caption") == "qty-012"
    assert root.property("blockValue") == 20
    assert root.scopeProbe() == "root/8"
    assert root.cycle(3) == 8
    assert root.modern() == "#4|#16|#1"
    root.saveClosure(10)
    assert root.useClosure(2) == 12
    assert root.useClosure(3) == 15
    assert root.modules() == "1/1:1/2:2/3:1:1"
    assert root.throughHost(4) == 30
    assert root.property("caption") == "qty-030"
    print("Qt 6.12.0 JavaScript and module reference passed")
    del root, component, engine, app


if __name__ == "__main__":
    main()
