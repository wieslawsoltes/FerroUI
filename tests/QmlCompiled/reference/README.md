# Qt QML behavioral reference

The original `Probe.qml` component runs under the published Qt 6.12.0 engine and both FerroUI backends. Each engine loads a fresh root, then calls the same seven methods in order on that root. The returned strings record branch-dependent reads, removal of an imperative binding, direct signal delivery, private ids and aliases in two inline-component instances, loop control, short-circuit side effects, and synchronous property notifications.

The last probe writes a property twice and reads its change handler's accumulated value between the writes. Qt returns `3/8`. The shared transaction used by FerroUI's native and source scripts delivers the handler before each assignment completes. A separate Rust test exercises runaway feedback and verifies rollback.

The Cargo tests compare both backends to `qt-6.12.0.tsv`. To repeat the live comparison from the repository root on a system with Python, Cargo and a supported PySide6 wheel, run

```sh
python3 -m venv /tmp/ferroui-qt-reference
/tmp/ferroui-qt-reference/bin/python -m pip install PySide6-Essentials==6.12.0
/tmp/ferroui-qt-reference/bin/python tests/QmlCompiled/reference/qt_reference.py
```

The reference uses `QCoreApplication`, `QQmlEngine` and the dynamic object's public methods. It requires no display. Normal builds and Cargo tests run without Qt or Python.

```sh
cargo test --locked -p qml-compiled-tests
```
