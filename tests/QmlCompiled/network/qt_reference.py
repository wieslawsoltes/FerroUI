"""Exercise remote component readiness, relative dependencies and required initial values."""
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread

from PySide6.QtCore import QCoreApplication, QEventLoop, QTimer, QUrl, qVersion
from PySide6.QtQml import QQmlComponent, QQmlEngine


def main():
    assert qVersion() == "6.12.0"
    app = QCoreApplication([])
    engine = QQmlEngine()
    directory = Path(__file__).resolve().parent
    handler = partial(SimpleHTTPRequestHandler, directory=str(directory))
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            component = QQmlComponent(engine)
            component.loadUrl(QUrl(f"http://127.0.0.1:{server.server_port}/Card.qml"))
            assert component.isLoading()
            loop = QEventLoop()
            component.statusChanged.connect(lambda status: loop.quit() if status != QQmlComponent.Loading else None)
            QTimer.singleShot(10000, loop.quit)
            loop.exec()
            assert component.isReady(), [error.toString() for error in component.errors()]
            item = component.createWithInitialProperties({"amount": 41})
            assert item is not None and item.format() == "42 points"
            print("Qt 6.12 remote component, relative dependency and initial property fixture passed")
        finally:
            server.shutdown()
            thread.join()
    del item, component, engine, app


if __name__ == "__main__":
    main()
