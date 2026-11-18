"""Build and compare full WinUI, full Fluent, and Fluent's ambient + Button selection.

On macOS: python3 scripts/themes/full-opcode-spike/prepare.py, then run this script.
Optional CARGO_TARGET_DIR and TYPED_THEME_OUT select an existing cache and output folder.
Use "build MODE CASE" to build a single probe and "compare CASE" to validate and time it.
Use "behavior" to compare all four native/bytecode resource inventories and exercise
real Button templates on any desktop host, using debug builds without size or timing claims.
"""
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = Path(os.environ.get("TYPED_THEME_OUT", ROOT / "target/typed-style-bench")).resolve()
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", OUT / "cargo")).resolve()
WINUI = ROOT / "target/full-opcode-spike/native"
CASES = ["winui", "winui-button", "fluent", "fluent-button"]


def write_changed(path, contents):
    binary = contents.encode() if isinstance(contents, str) else contents
    if not path.exists() or path.read_bytes() != binary:
        path.write_bytes(binary)


def make_project(case, profile="dist", selected_inventory=False):
    if case == "winui":
        return WINUI, "full-opcode-probe"
    theme = case.split("-")[0]
    path = OUT / case
    path.mkdir(parents=True, exist_ok=True)
    binary = f"{case}-probe"
    spec = (WINUI / "Cargo.toml").read_text().replace("full-opcode-probe", binary)
    if theme == "fluent":
        spec = spec.replace("themes-winui", "themes-fluent").replace("Themes.WinUI", "Themes.Fluent")
    if case == "winui-button":
        spec += f'\n[build-dependencies]\nferroui-build = {{ path = {json.dumps(str(ROOT / "src/FerroUI.Build.Tasks"))} }}\n'
        spec += f'ferroui-build-scan = {{ path = {json.dumps(str(ROOT / "src/FerroUI.Build.Scan"))} }}\n'
        write_changed(path / "build.rs", (HERE / "button_units.rs").read_bytes())
    write_changed(path / "Cargo.toml", spec)
    write_changed(path / "Cargo.lock", (ROOT / "Cargo.lock").read_bytes())
    body = (ROOT / "scripts/themes/compact-spike/template_bench.rs").read_text()
    if theme == "fluent":
        body = body.replace("WinUI", "Fluent").replace("themes_winui", "themes_fluent")
    body = body.replace("use ferroui_base::data::core::ValueTypes;\n", "")
    start = body.index("    for (variant, target)")
    end = body.index("    println!", start)
    if case == "fluent":
        verify = (HERE / "verify.rs").read_text().replace("WinUI", "Fluent").replace("themes_winui", "themes_fluent")
        verify = verify.replace("key_count > 1000", "key_count > 0").replace("type_count > 70", "type_count > 0")
        body = body[:start] + """
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        assert!(theme.try_get_resource(&"ButtonBackground".into(), Some(&variant)).flatten().is_some());
    }
    assert!(theme.palettes().ptr_eq(&theme.palettes()));
    verify_full(&theme);
""" + body[end:] + "\n" + verify
    else:
        if theme == "winui":
            # Its ambient dictionaries and StaticResource setters need the compiler's complete
            # closure. Computing it in the build script keeps the unused unit array out of the app.
            parts = 'include!(concat!(env!("OUT_DIR"), "/button_units.rs"))'
        else:
            # A separate minimal Fluent probe uses only ambient and Button. All other style
            # and resource references encountered during this template build stay dynamic.
            units = source_file(theme, profile).read_text(encoding="utf-8")
            button = re.search(r'pub static (THEME_UNIT_\d+): rt::ThemeUnit = rt::ThemeUnit \{ name: "Controls/Button.xaml"', units)[1]
            parts = f"&[&xaml::THEME_UNIT_0, &xaml::{button}]"
        class_name = "WinUITheme" if theme == "winui" else "FluentTheme"
        body = body.replace(f"let theme = {class_name}::new();", f"""
    use ferroui_themes_{theme}::compiled_xaml as xaml;
    static PARTS: ferroui_markup_xaml::xaml_il::runtime::compiled::ThemeParts = {parts};
    let theme = {class_name}::with_parts(None, PARTS);""")
        body = body[:body.index("    for (variant, target)")] + """
    let calendar = ResourceKey::Type(<ferroui_controls::Calendar as ferroui_base::StaticType>::TYPE);
    let calendar_present = theme.try_get_resource(&calendar, Some(&ThemeVariant::light())).flatten().is_some();
    assert_eq!(calendar_present, EXPECT_CALENDAR);
""" + body[body.index("    println!"):]
        if selected_inventory:
            verify = (HERE / "verify.rs").read_text()
            if theme == "fluent":
                verify = verify.replace("WinUI", "Fluent").replace("themes_winui", "themes_fluent")
            body = body.replace("    println!", "    inventory(&theme);\n    println!") + "\n" + verify

    expected = 'include!(concat!(env!("OUT_DIR"), "/expect-calendar.rs"))' if theme == "winui" else "false"
    body = body.replace("EXPECT_CALENDAR", expected)
    write_changed(path / "main.rs", body)
    return path, binary


def source_file(theme, profile="dist"):
    files = list((TARGET / profile / "build").glob(f"ferroui-themes-{theme}-*/out/xaml/compiled_xaml.rs"))
    return max(files, key=lambda p: p.stat().st_mtime_ns)


def code_counts(source, source_path=None):
    counts = {}
    packed = None
    image = re.search(r'__FERROUI_XAML_IMAGE: &\[u8\] = include_bytes!\((".*?")\);', source)
    if image:
        if source_path is None:
            raise ValueError("packed XAML requires the generated Rust source path")
        packed = (source_path.parent / json.loads(image[1])).read_bytes()
        counts["packed_module_bytes"] = len(packed)
    programs = {}
    for name, token in [("resource", "TypedResourceProgram::new"), ("style", "TypedStyleProgram::new"), ("selector", "TypedSelectorProgram::new")]:
        tag = r'rt::' + re.escape(token) + r'\('
        streams = [bytes.fromhex(raw.replace("\\x", "")) for raw in re.findall(tag + r'b"((?:\\x[0-9a-f]{2})*)"', source)]
        for length, offset in re.findall(tag + r'&const \{ rt::program_bytes::<(\d+)>\(__FERROUI_XAML_IMAGE, (\d+)\) \}', source):
            length, offset = int(length), int(offset)
            assert packed is not None and offset + length <= len(packed)
            streams.append(packed[offset:offset + length])
        assert len(streams) == source.count(f"rt::{token}("), "unrecognized program source"
        programs[name] = streams
        counts[name + "_programs"] = len(streams)
        counts[name + "_instruction_bytes"] = sum(map(len, streams))
    resources = Counter()
    for data in programs["resource"]:
        value = 0
        for i, n in enumerate(data[2:]):
            value |= (n & 127) << (7 * i)
            if n < 128:
                break
        resources[data[1]] += value
    names = ["alias", "literal_brush", "dynamic_brush", "static_brush", "double", "thickness", "corner_radius", "color", "text"]
    counts["resources"] = {names[k - 1]: n for k, n in sorted(resources.items())}
    counts["native_style_tables"] = source.count(": rt::ControlThemeTable = ")
    counts["native_control_theme_constructors"] = source.count("::styling::ControlTheme::new();")
    tables = sorted(re.findall(r"static (\w+): rt::(?:ControlThemeTable|TypedStyleProgram) =", source))
    counts["style_factory_names_sha256"] = hashlib.sha256("\n".join(tables).encode()).hexdigest()
    if counts["native_style_tables"]:
        counts["style_members"] = dict(Counter(re.findall(r"rt::StyleMember::(\w+)", source)))
        counts["setter_values"] = dict(Counter(re.findall(r"rt::ThemeValue::(\w+)", source)))
        counts["selector_steps"] = dict(Counter(re.findall(r"rt::SelectorStep::(\w+)", source)))
    return counts


def packed_reachability(source, source_path, theme, linked, executable):
    image_ref = re.search(r'__FERROUI_XAML_IMAGE: &\[u8\] = include_bytes!\((".*?")\);', source)
    if not image_ref:
        return {}
    payload = (source_path.parent / json.loads(image_ref[1])).read_bytes()
    operand = r'&const \{ rt::program_bytes::<(\d+)>\(__FERROUI_XAML_IMAGE, (\d+)\) \}'
    spans = [(int(n), int(at)) for n, at in re.findall(operand, source)]
    assert all(at + n <= len(payload) for n, at in spans)
    used = sum(n for n, at in set(spans))
    assert used <= len(payload), "overlapping image runs"
    symbols = set(re.findall(r'ferroui_themes_' + theme + r'\d+compiled_xaml\d+([A-Z][A-Z0-9_]*)$', linked, re.M))
    groups = {}
    for name, class_name in [("resource", "TypedResourceProgram"), ("style", "TypedStyleProgram"), ("selector", "TypedSelectorProgram")]:
        pattern = r'^static (\w+): rt::' + class_name + r'[^\n]*? = rt::' + class_name + r'::new\(' + operand
        programs = [(sym, int(n), int(at)) for sym, n, at in re.findall(pattern, source, re.M)]
        selected = [(n, at) for sym, n, at in programs if sym in symbols]
        groups[name] = {"total_named_statics": len(programs), "linked_named_statics": len(selected),
                        "linked_logical_bytes": sum(n for n, at in selected),
                        "linked_distinct_bytes": sum(n for n, at in set(selected))}
    total_construction = [(int(n), int(at)) for n, at in re.findall(r'rt::ConstructionProgram \{ code: ' + operand, source)]
    binding_paths = [(int(n), int(at)) for n, at in re.findall(r'rt::BindingPathProgram \{ code: ' + operand, source)]
    setter_choices = [(int(n), int(at)) for n, at in re.findall(r'rt::TypedSetterChoice \{ code: ' + operand, source)]
    # Fixed-size const-evaluated arrays can be folded with identical arrays or point inside
    # another constant. A raw byte witness is a supplemental diagnostic, independent of symbols.
    long_programs = {payload[at:at+n] for n, at in set(spans) if n >= 32}
    present = [code for code in long_programs if code in executable]
    return {"image_bytes": len(payload), "image_sha256": hashlib.sha256(payload).hexdigest(),
            "contiguous_full_image_in_executable": payload in executable,
            "logical_program_count": len(spans), "unique_program_count": len(set(spans)),
            "bytes_outside_referenced_runs": len(payload) - used,
            "logical_bytes": sum(n for n, at in spans), "named": groups,
            "construction_programs": len(total_construction),
            "construction_logical_bytes": sum(n for n, at in total_construction),
            "binding_path_programs": len(binding_paths), "binding_path_logical_bytes": sum(n for n, at in binding_paths),
            "setter_choice_programs": len(setter_choices), "setter_choice_logical_bytes": sum(n for n, at in setter_choices),
            "object_document_programs": source.count("static PROGRAM: rt::ObjectDocumentProgram ="),
            "distinct_programs_at_least_32_bytes": len(long_programs),
            "distinct_programs_at_least_32_bytes_found_in_executable": len(present),
            "distinct_bytes_at_least_32_before_link": sum(map(len, long_programs)),
            "distinct_bytes_at_least_32_found_in_executable": sum(map(len, present))}


def build(mode, case):
    OUT.mkdir(parents=True, exist_ok=True)
    project, binary = make_project(case)
    label = case + "-" + mode
    link_map = OUT / (label + ".map")
    env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), FERROUI_XAML_BACKEND=mode)
    cmd = ["cargo", "rustc", "--offline", "--profile", "dist", "--manifest-path", str(project / "Cargo.toml"),
           "--", "-C", "link-arg=-Wl,-map," + str(link_map)]
    with (OUT / (label + ".log")).open("w") as log:
        subprocess.run(cmd, env=env, check=True, stdout=log, stderr=subprocess.STDOUT)
    exe = OUT / label
    shutil.copy(TARGET / "dist" / binary, exe)
    subprocess.run(["strip", exe], check=True)
    subprocess.run(["codesign", "-f", "-s", "-", exe], check=True)
    subprocess.run(["codesign", "--verify", exe], check=True)
    theme = case.split("-")[0]
    source_path = source_file(theme)
    code = source_path.read_text()
    census = code_counts(code, source_path)
    assert bool(census["style_programs"]) == (mode == "bytecode"), (label, census)
    units = dict(re.findall(r'pub static THEME_UNIT_(\d+): rt::ThemeUnit = rt::ThemeUnit \{ name: "([^"]+)"', code))
    linked = link_map.read_text(encoding="latin1").split("# Dead Stripped Symbols:")[0]
    retained = sorted(set(re.findall(r'ferroui_themes_' + theme + r'.*?THEME_UNIT_(\d+)', linked)), key=int)
    executable = exe.read_bytes()
    census["packed_reachability"] = packed_reachability(code, source_path, theme, linked, executable)
    census["mach_o_segment_bytes"] = {m[1]: int(m[2]) for m in re.finditer(
        r"^Segment (\w+): (\d+)", subprocess.check_output(["size", "-m", exe], text=True), re.M)}
    linked_tables = sorted(re.findall(r'ferroui_themes_' + theme + r'\d+compiled_xaml\d+([A-Z0-9_]+_DEFERRED_\d+_THEME)$', linked, re.M))
    census.update(linked_style_factories=len(linked_tables),
                  linked_style_names_sha256=hashlib.sha256("\n".join(linked_tables).encode()).hexdigest(),
                  bytes=exe.stat().st_size, sha256=hashlib.sha256(executable).hexdigest(),
                  full_unit_count=len(units), linked_units=[units[i] for i in retained])
    (OUT / (label + ".json")).write_text(json.dumps(census, indent=2) + "\n")
    print(label, census["bytes"], "bytes;", len(retained), "linked units", flush=True)


def compare(case):
    rows = {}
    for mode in ["native", "bytecode"]:
        label = case + "-" + mode
        rows[mode] = json.loads((OUT / (label + ".json")).read_text())
        rows[mode]["samples_ns"] = []
        with (OUT / (label + "-inventory.log")).open("w") as log:
            subprocess.run([OUT / label], env=dict(os.environ, FULL_SPIKE_KEYS=str(OUT / (label + "-keys.txt"))),
                           check=True, stdout=log, stderr=log)
    if not case.endswith("-button"):
        expected = (OUT / (case + "-native-keys.txt")).read_bytes()
        assert expected == (OUT / (case + "-bytecode-keys.txt")).read_bytes()
        rows["inventory_sha256"] = hashlib.sha256(expected).hexdigest()
    for i in range(11):
        for mode in (["native", "bytecode"] if i % 2 == 0 else ["bytecode", "native"]):
            proc = subprocess.run([OUT / (case + "-" + mode)], capture_output=True, text=True, check=True)
            rows[mode]["samples_ns"].append([int(v) for v in proc.stdout.split()])
    for mode in ["native", "bytecode"]:
        row = rows[mode]
        row["median_ns"] = list(map(statistics.median, zip(*row["samples_ns"])))
    assert rows["native"]["linked_units"] == rows["bytecode"]["linked_units"]
    assert rows["native"]["style_factory_names_sha256"] == rows["bytecode"]["style_factory_names_sha256"]
    if case == "winui-button":
        closure = max((TARGET / "dist/build").glob("winui-button-probe-*/out/button-units.txt"), key=lambda f: f.stat().st_mtime)
        assert rows["bytecode"]["linked_units"] == closure.read_text().splitlines()
        assert len(rows["bytecode"]["linked_units"]) < rows["native"]["full_unit_count"]
        expected_units = len(rows["bytecode"]["linked_units"])
    else:
        expected_units = 2 if case == "fluent-button" else rows["native"]["full_unit_count"]
    assert len(rows["native"]["linked_units"]) == expected_units
    (OUT / (case + "-results.json")).write_text(json.dumps(rows, indent=2) + "\n")
    print(case, [(m, rows[m]["bytes"], rows[m]["median_ns"]) for m in ["native", "bytecode"]], flush=True)


def catalog():
    """Measure matched shipping executables and compare the three pinned pages in each variant."""
    folder = OUT / "catalog"
    folder.mkdir(parents=True, exist_ok=True)
    rows = {}
    for mode in ["native", "bytecode"]:
        env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), FERROUI_XAML_BACKEND=mode)
        subprocess.run(["cargo", "build", "--offline", "--locked", "--profile", "dist", "-p",
                        "control-catalog-desktop", "--bin", "control-catalog-desktop"], check=True, env=env)
        exe = folder / mode
        shutil.copy(TARGET / "dist/control-catalog-desktop", exe)
        subprocess.run(["strip", exe], check=True)
        subprocess.run(["codesign", "-f", "-s", "-", exe], check=True)
        subprocess.run(["codesign", "--verify", exe], check=True)
        rows[mode] = {"bytes": exe.stat().st_size, "sha256": hashlib.sha256(exe.read_bytes()).hexdigest(),
                      "samples_ns": []}
    env = dict(os.environ, FERROUI_SMOKE_EXIT_MS="160", FERROUI_CATALOG_THEME_VARIANT="light")
    for i in range(7):
        for mode in (["native", "bytecode"] if i % 2 == 0 else ["bytecode", "native"]):
            start = time.perf_counter_ns()
            with subprocess.Popen([folder / mode], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True) as process:
                for line in process.stdout:
                    if "ControlCatalog: the theme variant Light" in line:
                        break
                else:
                    raise RuntimeError(f"{mode} did not reach the variant selection message")
                rows[mode]["samples_ns"].append(time.perf_counter_ns() - start)
                process.communicate(timeout=30)
                if process.returncode:
                    raise RuntimeError(f"{mode} exited with {process.returncode}")
    for row in rows.values():
        row["median_ns"] = statistics.median(row["samples_ns"])
    (folder / "results.json").write_text(json.dumps(rows, indent=2) + "\n")
    env = dict(os.environ, SPIKE_CATALOG_DIR=str(folder))
    subprocess.run([sys.executable, "-B", HERE / "catalog_smoke.py", "native", "bytecode"], env=env, check=True)


def selected_units(case):
    if case == "winui-button":
        closures = (TARGET / "debug/build").glob("winui-button-probe-*/out/button-units.txt")
        return max(closures, key=lambda path: path.stat().st_mtime_ns).read_text(encoding="utf-8").splitlines()
    if case == "fluent-button":
        return ["ambient", "Controls/Button.xaml"]
    source = source_file(case, "debug").read_text(encoding="utf-8")
    return re.findall(r'pub static THEME_UNIT_\d+: rt::ThemeUnit = rt::ThemeUnit \{ name: "([^"]+)"', source)


def behavior():
    OUT.mkdir(parents=True, exist_ok=True)
    results = {}
    for mode in ["native", "bytecode"]:
        for case in CASES:
            project, binary = make_project(case, profile="debug", selected_inventory=True)
            label = f"{case}-{mode}"
            env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), FERROUI_XAML_BACKEND=mode)
            with (OUT / (label + "-build.log")).open("w") as log:
                subprocess.run(["cargo", "build", "--offline", "--manifest-path", project / "Cargo.toml"],
                               env=env, check=True, stdout=log, stderr=subprocess.STDOUT)
            keys = OUT / (label + "-keys.txt")
            executable = TARGET / "debug" / (binary + (".exe" if os.name == "nt" else ""))
            run = subprocess.run([executable], capture_output=True, env=dict(env, FULL_SPIKE_KEYS=str(keys)))
            (OUT / (label + "-behavior.log")).write_bytes(run.stderr + run.stdout)
            run.check_returncode()
            counts = re.search(rb"(\d+) type keys and (\d+) distinct string keys", run.stderr)
            if not counts:
                raise ValueError(f"{label} did not report its inventory")
            results.setdefault(case, {})[mode] = {
                "type_keys": int(counts[1]), "string_keys": int(counts[2]),
                "inventory_sha256": hashlib.sha256(keys.read_bytes()).hexdigest(),
            }
            if mode == "bytecode":
                if keys.read_bytes() != (OUT / (case + "-native-keys.txt")).read_bytes():
                    raise ValueError(f"{case}: backend inventory mismatch")
                if results[case]["selected_units"] != selected_units(case):
                    raise ValueError(f"{case}: selected units differ between backends")
            else:
                results[case]["selected_units"] = selected_units(case)
            print(label, results[case][mode], flush=True)
    (OUT / "behavior-results.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    if sys.argv[1:] == ["behavior"]:
        behavior()
    elif sys.argv[1:] == ["catalog"]:
        catalog()
    elif sys.argv[1:2] == ["build"]:
        build(*sys.argv[2:])
    elif sys.argv[1:2] == ["compare"]:
        compare(sys.argv[2])
    else:
        for mode in ["native", "bytecode"]:
            for case in CASES:
                build(mode, case)
        for case in CASES:
            compare(case)
