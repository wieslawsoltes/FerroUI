"""Build the stock desktop Catalog in a private workspace with just its WinUI backend replaced."""
from pathlib import Path
import re
import shutil

ROOT = Path.cwd()
OUT = ROOT / "target/full-opcode-spike"
DST = OUT / "catalog"
LOCAL = ["samples/ControlCatalog.Desktop", "samples/ControlCatalog", "src/FerroUI.Diagnostics", "src/FerroUI.Controls.Inking"]
OVERRIDES = {str(ROOT / path): str(DST / path) for path in LOCAL}
OVERRIDES[str(ROOT / "src/FerroUI.Themes.WinUI")] = str(DST / "src/FerroUI.Themes.WinUI")

def manifests(text, parent):
    def relocate(match):
        old = str((parent / match[1]).resolve())
        return 'path = "' + OVERRIDES.get(old, old) + '"'
    return re.sub(r'path\s*=\s*"([^"]+)"', relocate, text)

def setup():
    DST.mkdir(exist_ok=True)
    shutil.copy2(ROOT / "README.md", DST)
    shutil.copy2(ROOT / "Cargo.lock", DST)
    text = (ROOT / "Cargo.toml").read_text()
    text = text[text.index("[workspace.package]"):]
    text = manifests(text, ROOT)
    theme = "src/FerroUI.Themes.WinUI"
    members = ", ".join(f'"{member}"' for member in [*LOCAL, theme])
    (DST / "Cargo.toml").write_text(f'[workspace]\nresolver = "2"\nmembers = [{members}]\n' + text)
    for rel in LOCAL:
        src, dest = ROOT / rel, DST / rel
        shutil.copytree(src, dest, dirs_exist_ok=True)
        manifest = (src / "Cargo.toml").read_text()
        # Keep package-local targets in the copied tree, redirect dependency paths.
        head, tail = manifest.split("[dependencies]", 1)
        manifest = head + "[dependencies]" + manifests(tail, src)
        (dest / "Cargo.toml").write_text(manifest)
    (DST / "samples/media_desktop.rs").write_bytes((ROOT / "samples/media_desktop.rs").read_bytes())
    if not (DST / "scripts").exists():
        (DST / "scripts").symlink_to(ROOT / "scripts", target_is_directory=True)
    # Reuse the upstream native build script to supply its usual build-time metadata to the
    # Catalog and inking XAML compilers. No generated native WinUI theme module is linked.
    src, dest = ROOT / theme, DST / theme
    shutil.copytree(src, dest, dirs_exist_ok=True)
    for name in ["compiled_xaml.rs", "lib.rs", "paths.rs"]:
        shutil.copyfile(OUT / "ast/theme" / name, dest / name)
    # Generated Catalog code refers to its selected unit constants. The full binary archive
    # populates every unit. Empty runs meet the source ABI without referencing native factories.
    code = (dest / "compiled_xaml.rs").read_text()
    for idx, name in re.findall(r'pub static THEME_UNIT_(\d+): rt::ThemeUnit = rt::ThemeUnit \{ name: ("[^"]+")', (src / "compiled_xaml.rs").read_text()):
        code += f'\npub static THEME_UNIT_{idx}: rt::ThemeUnit = rt::ThemeUnit {{ name: {name}, index: {idx}, runs: &[] }};'
    (dest / "compiled_xaml.rs").write_text(code)
    (dest / "lib.rs").write_text((dest / "lib.rs").read_text().replace("mod compiled_xaml;", "pub mod compiled_xaml;"))
    # Distinct package name for the runtime decoder. The compiler used by build.rs stays stock.
    dec = DST / "decoder"
    shutil.copytree(OUT / "ast/loader", dec, dirs_exist_ok=True)
    p = dest / "Cargo.toml"
    p.write_text(p.read_text().replace("[dependencies]", f'[dependencies]\nferroui-markup-xaml-loader = {{ package = "ferroui-full-theme-decoder", path = "{dec}" }}'))
    # Its production runtime dependency differs from the original crate's test loader.
    p.write_text(p.read_text().replace('ferroui-markup-xaml-loader = { workspace = true, features = ["runtime", "testing", "emitter"] }', ''))

if __name__ == "__main__":
    setup()
