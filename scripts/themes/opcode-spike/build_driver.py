import json
import os
import subprocess
from pathlib import Path

dist = Path(os.getenv("SPIKE_DIST", "target/dist"))
fingerprints = dist/".fingerprint"
lib = max((dist/"deps").glob("libferroui_markup_xaml-*.rlib"), key=lambda f: f.stat().st_mtime)
crate = "ferroui_markup_xaml"
meta = json.loads((fingerprints/lib.stem[3:].replace("_","-")/f"lib-{crate}.json").read_text())
args = ["--extern", f"{crate}={lib}"]
for _, crate, _, fingerprint in meta["deps"]:
    if crate not in ("ferroui_controls", "ferroui_base"):
        continue
    candidates = fingerprints.glob(crate.replace("_","-")+f"-*/lib-{crate}")
    dep = next(p for p in candidates if int.from_bytes(bytes.fromhex(p.read_text()),"little")==fingerprint)
    lib = dist/"deps"/f"lib{crate}-{dep.parent.name.split('-')[-1]}.rlib"
    args += ["--extern", f"{crate}={lib}"]
def build(source, mode, out, cache=None):
    cache_args = ["-Cincremental="+str(cache)] if cache else []
    subprocess.run(["rustc", "--edition=2021", "-Copt-level=z", "-Cpanic=abort",
                    "-Clto=thin", *cache_args, "-Cstrip=symbols", "-Ccodegen-units=1", f"-Clink-arg=-Wl,-map,{out/mode}.map", "--cfg", mode,
                    "-L", "dependency="+str(dist/"deps"), *args, source, "-o", out/mode],
                   env=dict(os.environ, SPIKE_OUT=str(out)), check=True)
    subprocess.run(["strip", out/mode], check=True)
    subprocess.run(["codesign", "-s", "-", "-f", out/mode], check=True)
    subprocess.run(["codesign", "--verify", out/mode], check=True)
