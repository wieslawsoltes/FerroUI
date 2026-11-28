"""Prepare full WinUI and Fluent timing binaries for this checkout. Run ../prepare.py first."""
from pathlib import Path
import shutil
import sys

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import typed_theme_probe as probe

if len(sys.argv) != 2:
    raise SystemExit("usage: python3 prepare.py OUTPUT_DIRECTORY")
output = Path(sys.argv[1]).resolve()
probe.OUT = output / "inputs"
for theme in ("winui", "fluent"):
    base, name = probe.make_project(theme)
    folder = output / theme
    folder.mkdir(parents=True, exist_ok=True)
    manifest = (base / "Cargo.toml").read_text().replace(name, f"template-{theme}-probe")
    (folder / "Cargo.toml").write_text(manifest)
    shutil.copy2(base / "Cargo.lock", folder / "Cargo.lock")
    source = (base / "main.rs").read_text()
    start = source.index("    let start = Instant::now();\n    for _ in 0..128")
    end = source.index("    let built = template", start)
    (folder / "main.rs").write_text(source[:start] + (here / "loop.rs").read_text() + source[end:])
