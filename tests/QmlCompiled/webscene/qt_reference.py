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
    signals = QQmlComponent(engine, QUrl.fromLocalFile(str(Path(__file__).with_name("Signals.qml").resolve())))
    sender = signals.create()
    if sender is None:
        raise RuntimeError("\n".join(issue.toString() for issue in signals.errors()))
    sender.scheduleLater()
    assert sender.property("laterHistory") == ""
    app.processEvents()
    assert sender.property("laterHistory") == "g2f3"
    app.processEvents()
    assert sender.property("laterHistory") == "g2f3h"
    sender.install()
    sender.reading.emit(2)
    assert sender.property("history") == "M2M2S2"
    sender.removeAll()
    sender.reading.emit(8)
    assert sender.property("history") == "M2M2S2"
    assert sender.explicitReceiver() == "4"
    assert sender.mutation() == "A/AAC"
    assert sender.badArgs() == "2"
    sender.observeCount()
    sender.setProperty("count", 6)
    assert sender.property("history") == "M2M2S2C6"
    bindings = QQmlComponent(engine, QUrl.fromLocalFile(str(Path(__file__).with_name("Bindings.qml").resolve())))
    props = bindings.create()
    if props is None:
        raise RuntimeError("\n".join(issue.toString() for issue in bindings.errors()))
    assert props.marker() == "function/false/TypeError/Error/Error"
    assert props.installBranch() == 109
    props.setProperty("right", 11)
    assert props.property("mapped") == 109
    props.setProperty("first", False)
    assert props.property("mapped") == 216
    props.setProperty("left", 30)
    assert props.property("mapped") == 216
    assert props.installAlias() == 111
    assert props.installArrow() == 130
    props.overwrite(42)
    props.setProperty("left", 40)
    assert props.property("mapped") == 42
    props.installNumbers()
    assert props.values() == "40,11"
    props.setProperty("enabled", True)
    assert props.installControlled() == 111
    props.setProperty("right", 15)
    assert props.property("controlled") == 115
    props.setProperty("left", 4)
    assert props.property("controlled") == 6
    props.setProperty("right", 16)
    assert props.property("controlled") == 6
    props.setProperty("enabled", False)
    assert props.property("controlled") == 5
    print("Qt 6.12.0 JavaScript, module, signal and binding reference passed")
    del props, bindings
    del sender, signals
    del root, component, engine, app


if __name__ == "__main__":
    main()
