"""Compare the shared worker fixture against the released Qt engine."""
from pathlib import Path
from time import monotonic, sleep
from PySide6.QtCore import QCoreApplication, QUrl, qVersion
from PySide6.QtQml import QQmlComponent, QQmlEngine

def main():
    assert qVersion() == "6.12.0", qVersion()
    loop = QCoreApplication([])
    engine = QQmlEngine()
    component = QQmlComponent(engine, QUrl.fromLocalFile(str(Path(__file__).with_name("Main.qml").resolve())))
    root = component.create()
    if root is None:
        raise RuntimeError("\n".join(issue.toString() for issue in component.errors()))
    root.startNested()
    root.start()
    deadline = monotonic() + 8
    while root.property("answer") != "1:2/3" or root.property("localValue") != 3:
        assert monotonic() < deadline, "worker delivery timed out"
        loop.processEvents()
        sleep(0.001)
    while root.property("nestedValue") != 21:
        assert monotonic() < deadline, "relative nested worker did not respond"
        loop.processEvents()
        sleep(0.001)
    assert root.property("replies") == 1
    root.syncRows()
    while root.property("answer") != "synced-2":
        assert monotonic() < deadline, "model sync timed out"
        loop.processEvents()
        sleep(0.001)
    root.selectClassic("loose.js")
    root.control()
    while root.property("localValue") != 45:
        assert monotonic() < deadline, "source replacement timed out"
        loop.processEvents()
        sleep(0.001)
    print("Qt 6.12.0 WorkerScript module, copy and FIFO checks passed")

if __name__ == "__main__":
    main()
