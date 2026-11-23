#!/usr/bin/env python3
"""Sizes of the C samples of the C ABI (docs/binary-size.md, section 3.7; #978).

    python3 scripts/capi/size_report.py --source <tree> --profile release --json out.json \
        [--markdown summary.md] [--label measured] [--work <dir>]
    python3 scripts/capi/size_report.py --compare measured.json baseline.json [--markdown summary.md]

Builds `ferroui-capi` of the tree `--source` with the profile, in the full mode (the default
features) and, where the tree has it, the by-use mode (`--no-default-features --features
desktop`, docs/c-abi.md, section 16), and links `samples/CApi/hello_world.c` and `gauge.c`
against each library on macOS:

- static: the program with `libferroui_capi.a` and the native libraries rustc names
  (`--print native-static-libs`), `-Wl,-dead_strip`, with a link map;
- dynamic: the program against `libferroui_capi.dylib`, `-Wl,-dead_strip`.

For each it records the size as written and stripped, `size -m` of the stripped static program
(the `__TEXT` segment and the `__text` section), the stripped dynamic library, and the static
program by crate from the link map (`size_by_crate.py`). `--compare` writes the table of two
reports (the ref against the baseline). The script measures any tree, also one from before it
existed: the workflow runs the copy of its own commit with `--source`.

Cargo is `cargo`, or `$CARGO`; `--cargo-config` adds `--config` values to its builds; the target
directory is `$CARGO_TARGET_DIR`, else `<tree>/target`.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import size_by_crate  # noqa: E402

SAMPLES = ("hello_world", "gauge")
MODES = {
    "full": [],
    "by-use": ["--no-default-features", "--features", "desktop"],
}


def link_environment():
    """The environment of the C links: on macOS with `SDKROOT` set (from `xcrun`) when it is not,
    so that `cc` finds the libraries rustc names (`-lm`, `-liconv`) the way rustc's own link does."""
    env = dict(os.environ)
    if sys.platform == "darwin" and not env.get("SDKROOT"):
        sdk = subprocess.run(["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True, capture_output=True)
        if sdk.returncode == 0 and sdk.stdout.strip():
            env["SDKROOT"] = sdk.stdout.strip()
    return env


LINK_ENV = None


def run(command, cwd=None, capture=False):
    global LINK_ENV
    if LINK_ENV is None:
        LINK_ENV = link_environment()
    print("+ " + " ".join(command), flush=True)
    result = subprocess.run(command, cwd=cwd, env=LINK_ENV, text=True, stdout=subprocess.PIPE if capture else None,
                            stderr=subprocess.STDOUT if capture else None, check=False)
    if result.returncode != 0:
        if capture:
            print(result.stdout[-6000:])
        raise SystemExit(f"failed ({result.returncode}): {' '.join(command)}")
    return result.stdout if capture else ""


def has_by_use(source):
    """Whether the tree has the by-use mode (the feature `type-tables` of ferroui-capi)."""
    with open(os.path.join(source, "src", "FerroUI.CApi", "Cargo.toml"), encoding="utf-8") as file:
        return re.search(r"^type-tables\s*=", file.read(), re.M) is not None


def build(source, profile, mode, options):
    """Builds the library of a mode and returns its native libraries (the link line of a static
    program, after the archive)."""
    cargo = os.environ.get("CARGO", "cargo")
    # Touch the root of the crate so that rustc runs and prints the native libraries.
    os.utime(os.path.join(source, "src", "FerroUI.CApi", "lib.rs"))
    command = [cargo, "rustc", "--locked", "--profile", profile, "-p", "ferroui-capi", "--lib", *MODES[mode]]
    for value in options.cargo_config:
        command += ["--config", value]
    command += ["--", "--print", "native-static-libs"]
    output = run(command, cwd=source, capture=True)
    # Without the colours of a terminal (`CARGO_TERM_COLOR=always` on CI), whose escape sequences
    # would end the last library of the line (`-lm` followed by a reset).
    output = re.sub(r"\x1b\[[0-9;]*m", "", output)
    found = re.findall(r"native-static-libs: (.*)", output)
    if not found:
        raise SystemExit("rustc printed no native-static-libs line")
    return found[-1].split()


def text_sizes(path):
    """`__TEXT` and `__text` of a Mach-O file, from `size -m`."""
    output = subprocess.run(["size", "-m", path], text=True, capture_output=True, check=True).stdout
    segment = re.search(r"Segment __TEXT: (\d+)", output)
    section = re.search(r"Section __text: (\d+)", output)
    return int(segment.group(1)) if segment else 0, int(section.group(1)) if section else 0


def stripped_copy(path, keep_exports=False):
    out = path + ".stripped"
    subprocess.run(["strip", *(["-x"] if keep_exports else []), "-o", out, path], check=True)
    return os.path.getsize(out)


def measure_mode(source, profile, mode, work, options):
    native = build(source, profile, mode, options)
    target = os.environ.get("CARGO_TARGET_DIR") or os.path.join(source, "target")
    built = os.path.join(target, profile)
    directory = os.path.join(work, mode)
    dynamic_dir = os.path.join(directory, "dynamic")
    os.makedirs(dynamic_dir, exist_ok=True)
    archive = os.path.join(directory, "libferroui_capi.a")
    dylib = os.path.join(dynamic_dir, "libferroui_capi.dylib")
    shutil.copy(os.path.join(built, "libferroui_capi.a"), archive)
    shutil.copy(os.path.join(built, "libferroui_capi.dylib"), dylib)
    subprocess.run(["install_name_tool", "-id", "@rpath/libferroui_capi.dylib", dylib], check=True)
    result = {
        "library": {
            "archive": os.path.getsize(archive),
            "dylib": os.path.getsize(dylib),
            "dylib_stripped": stripped_copy(dylib, keep_exports=True),
        },
        "native_libs": " ".join(native),
        "samples": {},
    }
    include = ["-I", os.path.join(source, "src", "FerroUI.CApi", "include"), "-I", os.path.join(source, "samples", "CApi")]
    for sample in SAMPLES:
        program = os.path.join(source, "samples", "CApi", f"{sample}.c")
        if not os.path.exists(program):
            continue  # a tree from before the sample
        static = os.path.join(directory, f"{sample}-static")
        link_map = static + ".map"
        run(["cc", "-std=c11", "-O2", *include, program, archive, *native, "-Wl,-dead_strip", f"-Wl,-map,{link_map}",
             "-o", static])
        stripped = stripped_copy(static)
        segment, section = text_sizes(static + ".stripped")
        with open(link_map, encoding="utf-8", errors="replace") as file:
            crates = size_by_crate.read_ld64(file)
        dynamic = os.path.join(dynamic_dir, sample)
        run(["cc", "-std=c11", "-O2", *include, program, "-L", dynamic_dir, "-lferroui_capi", "-Wl,-dead_strip",
             "-Wl,-rpath,@executable_path", "-o", dynamic])
        dynamic_stripped = stripped_copy(dynamic)
        result["samples"][sample] = {
            "static": {
                "raw": os.path.getsize(static),
                "stripped": stripped,
                "text_segment": segment,
                "text_section": section,
                "crates": dict(sorted(crates.items(), key=lambda item: -item[1])),
            },
            "dynamic": {
                "raw": os.path.getsize(dynamic),
                "stripped": dynamic_stripped,
                "with_library": dynamic_stripped + result["library"]["dylib_stripped"],
            },
        }
        os.remove(link_map)
    return result


def mb(value):
    return f"{value / 1e6:.2f}" if isinstance(value, (int, float)) else "n/a"


def report_markdown(report):
    lines = [f"### C samples: `{report['label']}` at `{report['commit'][:12]}`, profile `{report['profile']}`", ""]
    lines.append("| Sample | Mode | Static, stripped | `__TEXT` | `__text` | Dynamic, stripped (with the library) |")
    lines.append("| --- | --- | ---: | ---: | ---: | ---: |")
    for mode, data in report["modes"].items():
        for sample, sizes in data["samples"].items():
            static, dynamic = sizes["static"], sizes["dynamic"]
            lines.append(f"| `{sample}.c` | {mode} | {mb(static['stripped'])} | {mb(static['text_segment'])} | "
                         f"{mb(static['text_section'])} | {mb(dynamic['stripped'])} ({mb(dynamic['with_library'])}) |")
    lines.append("")
    lines.append("MB. Libraries: " + "; ".join(
        f"{mode}: archive {mb(data['library']['archive'])}, dylib {mb(data['library']['dylib'])}, "
        f"stripped {mb(data['library']['dylib_stripped'])}" for mode, data in report["modes"].items()))
    lines.append("")
    for mode, data in report["modes"].items():
        if "hello_world" not in data["samples"]:
            continue
        crates = data["samples"]["hello_world"]["static"]["crates"]
        top = list(crates.items())[:10]
        lines.append(f"`hello_world.c`, {mode}, static, by crate (MB of symbols, total "
                     f"{mb(sum(crates.values()))}): " + ", ".join(f"{name} {mb(size)}" for name, size in top))
        lines.append("")
    if "by-use" not in report["modes"]:
        lines.append("The tree has no by-use mode (the feature `type-tables` of ferroui-capi).")
        lines.append("")
    return "\n".join(lines) + "\n"


def compare_markdown(measured, baseline):
    lines = [f"### C samples: `{measured['label']}` against `{baseline['label']}`, profile `{measured['profile']}`", "",
             "| Sample | Mode | Link | Ref | Baseline | Change |", "| --- | --- | --- | ---: | ---: | ---: |"]
    for mode in MODES:
        for sample in SAMPLES:
            for link, key in (("static", "stripped"), ("dynamic + library", "with_library")):
                group = "static" if link == "static" else "dynamic"
                values = []
                for report in (measured, baseline):
                    found = report["modes"].get(mode, {}).get("samples", {}).get(sample)
                    values.append(found[group][key] if found else None)
                ref, base = values
                change = f"{(ref - base) / 1e6:+.2f} ({100 * (ref - base) / base:+.1f} %)" if ref and base else "n/a"
                lines.append(f"| `{sample}.c` | {mode} | {link} | {mb(ref)} | {mb(base)} | {change} |")
    # The full mode of the baseline is the before of the by-use mode of the ref.
    full, by_use = baseline["modes"].get("full"), measured["modes"].get("by-use")
    if full and by_use:
        common = [sample for sample in SAMPLES if sample in full["samples"] and sample in by_use["samples"]]
        lines += ["", "By use (ref) against the full mode (baseline), static, stripped: " + ", ".join(
            f"`{sample}.c` {mb(by_use['samples'][sample]['static']['stripped'])} against "
            f"{mb(full['samples'][sample]['static']['stripped'])}" for sample in common)]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--source", help="the tree to measure")
    parser.add_argument("--profile", default="release")
    parser.add_argument("--label", default="measured")
    parser.add_argument("--work", help="where the libraries and programs go (default: <tree>/capi-size)")
    parser.add_argument("--json", help="write the report here")
    parser.add_argument("--markdown", help="append the table here")
    parser.add_argument("--cargo-config", action="append", default=[], help="a --config value for cargo")
    parser.add_argument("--compare", nargs=2, metavar=("REF", "BASELINE"), help="compare two reports")
    options = parser.parse_args()
    if options.compare:
        reports = []
        for path in options.compare:
            with open(path, encoding="utf-8") as file:
                reports.append(json.load(file))
        text = compare_markdown(*reports)
    else:
        if not options.source:
            parser.error("--source is required")
        source = os.path.abspath(options.source)
        work = os.path.abspath(options.work or os.path.join(source, "capi-size"))
        commit = subprocess.run(["git", "-C", source, "rev-parse", "HEAD"], text=True, capture_output=True).stdout.strip()
        modes = [mode for mode in MODES if mode == "full" or has_by_use(source)]
        report = {"label": options.label, "commit": commit, "profile": options.profile,
                  "modes": {mode: measure_mode(source, options.profile, mode, work, options) for mode in modes}}
        if options.json:
            with open(options.json, "w", encoding="utf-8") as file:
                json.dump(report, file, indent=1)
        text = report_markdown(report)
    print(text)
    if options.markdown:
        with open(options.markdown, "a", encoding="utf-8") as file:
            file.write(text + "\n")


if __name__ == "__main__":
    main()
