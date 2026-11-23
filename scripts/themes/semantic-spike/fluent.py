"""Compare the separate Fluent theme after preparing the WinUI native and FXP probes."""
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[3]
source = ROOT / "src/FerroUI.Themes.Fluent"
win = ROOT / "target/full-opcode-spike"
out = ROOT / "target/semantic-spike/fluent"
theme = out / "ast/theme"
loader = out / "ast/loader"
out.mkdir(parents=True, exist_ok=True)
theme.mkdir(parents=True, exist_ok=True)
shutil.copytree(win / "ast/loader", loader, dirs_exist_ok=True)
shutil.copy(ROOT / "target/semantic-spike/fluent.bin", loader / "theme.bin")
shutil.copytree(source / "Accents", theme / "Accents", dirs_exist_ok=True)
for file in ("color_palette_resources.rs", "color_palette_resources_collection.rs", "fluent_theme.rs", "register_types.rs", "rust_paths.rs"):
    shutil.copy(source / file, theme / file)
(theme / "lib.rs").write_text((source / "lib.rs").read_text().replace("ferroui_markup_xaml::include_compiled_xaml!();", "mod compiled_xaml;"))
shutil.copy(win / "ast/theme/assets.rs", theme / "assets.rs")
spec = (win / "ast/theme/Cargo.toml").read_text().replace("themes-winui", "themes-fluent").replace(str(win / "ast/loader"), str(loader))
spec = "\n".join(line for line in spec.splitlines() if not line.startswith("ferroui-media-"))
(theme / "Cargo.toml").write_text(spec + "\n")
(theme / "compiled_xaml.rs").write_text((win / "ast/theme/compiled_xaml.rs").read_text().replace("WinUI", "Fluent").replace("winui", "fluent"))
(theme / "paths.rs").write_text("&" + json.dumps(sorted(str(p.relative_to(source)) for p in source.rglob("*.xaml"))))
template = (ROOT / "scripts/themes/compact-spike/template_bench.rs").read_text().replace("WinUI", "Fluent").replace("themes_winui", "themes_fluent")
a = template.index("    for (variant, target)")
b = template.index('    println!', a)
template = template[:a] + '''
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        assert!(theme.try_get_resource(&"ButtonBackground".into(), Some(&variant)).flatten().is_some());
    }
    assert!(theme.palettes().ptr_eq(&theme.palettes()));
    theme.set_density_style(ferroui_themes_fluent::DensityStyle::Compact);
    let height = theme.try_get_resource(&"ComboBoxMinHeight".into(), Some(&ThemeVariant::light())).flatten();
    assert_eq!(from_markup_value::<f64>(&height), Some(24.0));
''' + template[b:]
template = template.replace("use ferroui_base::data::core::ValueTypes;\n", "")
env = dict(os.environ, CARGO_TARGET_DIR=os.environ.get("FLUENT_SPIKE_TARGET", str(out / "cargo")))
results = {}
for mode in ("ast", "native"):
    project = out / mode
    project.mkdir(exist_ok=True)
    spec = (win / mode / "Cargo.toml").read_text().replace("full-opcode-probe", "fluent-probe").replace("themes-winui", "themes-fluent").replace("Themes.WinUI", "Themes.Fluent").replace(str(win / "ast/theme"), str(theme))
    (project / "Cargo.toml").write_text(spec)
    (project / "main.rs").write_text(template)
    shutil.copy(ROOT / "Cargo.lock", project / "Cargo.lock")
    subprocess.run(["cargo", "build", "--offline", "--profile", "dist", "--manifest-path", str(project / "Cargo.toml")], env=env, check=True)
    exe = project / "probe"
    shutil.copy(Path(env["CARGO_TARGET_DIR"]) / "dist/fluent-probe", exe)
    subprocess.run(["strip", exe], check=True)
    subprocess.run(["codesign", "-f", "-s", "-", exe], check=True)
    results[mode] = {"bytes": exe.stat().st_size, "samples_ns": []}
for i in range(7):
    for mode in (("native", "ast") if i % 2 == 0 else ("ast", "native")):
        raw = subprocess.check_output([out / mode / "probe"], text=True)
        results[mode]["samples_ns"].append([int(v) for v in raw.split()])
for row in results.values():
    row["median_ns"] = list(map(statistics.median, zip(*row["samples_ns"])))
(out / "results.json").write_text(json.dumps(results, indent=2) + "\n")
print(json.dumps(results))
