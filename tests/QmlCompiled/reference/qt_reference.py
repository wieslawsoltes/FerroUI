"""Compare public QQmlEngine behavior to both FerroUI backends on a shared fixture."""
import subprocess
import sys
from pathlib import Path

from PySide6.QtCore import QCoreApplication, QUrl, qVersion
from PySide6.QtQml import QQmlComponent, QQmlEngine

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CASES = tuple(line.split("\t", 1)[0] for line in (HERE / "qt-6.12.0.tsv").read_text().splitlines())


def qt_results():
    if qVersion() != "6.12.0":
        raise RuntimeError(f"expected Qt 6.12.0, found {qVersion()}")
    app = QCoreApplication.instance() or QCoreApplication([])
    engine = QQmlEngine()
    component = QQmlComponent(engine, QUrl.fromLocalFile(str(HERE / "Probe.qml")))
    obj = component.create()
    if obj is None:
        raise RuntimeError("\n".join(e.toString() for e in component.errors()))
    results = {case: getattr(obj, case)() for case in CASES}
    del obj, component, engine, app
    return results


def main():
    expected = qt_results()
    if sys.argv[1:] == ["--print-reference"]:
        for case, result in expected.items():
            print(f"{case}\t{result}")
        return
    run = subprocess.run(
        ["cargo", "run", "--locked", "--quiet", "-p", "qml-compiled-tests", "--example", "qt_reference"],
        cwd=ROOT, check=True, stdout=subprocess.PIPE, text=True
    )
    observed = {}
    for line in run.stdout.splitlines():
        mode, case, result = line.split("\t", 2)
        key = mode, case
        if key in observed:
            raise AssertionError(f"duplicate result {key!r}")
        observed[key] = result
    for mode in ("source", "compiled"):
        for case, reference in expected.items():
            actual = observed.pop((mode, case))
            if actual != reference:
                raise AssertionError(f"{mode}/{case}: FerroUI {actual!r}, Qt 6.12.0 {reference!r}")
            print(f"{mode}/{case}\t{actual}")
    if observed:
        raise AssertionError(f"unexpected results {observed}")


if __name__ == "__main__":
    main()
