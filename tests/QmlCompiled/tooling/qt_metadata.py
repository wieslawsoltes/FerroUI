"""Run Qt's real static analyzer against modules exported by a Rust host."""
import subprocess
import tempfile
from pathlib import Path

from PySide6 import __file__ as pyside_file
from PySide6.QtCore import qVersion

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    assert qVersion() == "6.12.0", qVersion()
    qmllint = Path(pyside_file).parent / "qmllint"
    with tempfile.TemporaryDirectory() as directory:
        subprocess.run(
            ["cargo", "run", "--quiet", "--locked", "-p", "ferroui-qml", "--example",
             "export_qmltypes", "--", directory], cwd=ROOT, check=True,
        )
        def lint(name):
            return subprocess.run(
                [str(qmllint), "-I", directory, "--compiler", "warning",
                 "--max-warnings", "0", str(HERE / name)],
                capture_output=True, text=True,
            )
        good = lint("Good.qml")
        assert good.returncode == 0, good.stderr
        bad = lint("Invalid.qml")
        assert bad.returncode != 0, bad.stderr
        for expected in ["[read-only-property]", "[missing-property]", "Function expects 1 arguments"]:
            assert expected in bad.stderr, bad.stderr
    print("Qt 6.12.0 qmllint accepted native module metadata and rejected invalid member use")


if __name__ == "__main__":
    main()
