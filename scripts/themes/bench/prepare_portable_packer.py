#!/usr/bin/env python3
"""Materialize an isolated archive build tool from unchanged production codecs.

This package is only an archive producer. It cannot replace the shipping Loader,
Base, runtime compiler, or a dependency in a measured application graph. The CLI,
binary_document modules and XamlX crate are copied verbatim; a small facade owns
the verbatim lexical parser method and development-namespace constant.

No Cargo or compiler process is launched here. After materialization, resolve
the copied lock seed offline with Cargo, run verify-lock, then build with --locked.
The builder must verify the resulting archives against the retained canonical
hashes before supplying this tool to the production PGO replay.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LOADER = Path("src/Markup/FerroUI.Markup.Xaml.Loader")
XAMLX = Path("external/XamlX/src/XamlX")
PACKAGE = "ferroui-portable-archive-tool"
METHOD = re.compile(
    r"^    pub\(crate\) fn parse_source_syntax\(xaml: &str\) -> XamlResult<XamlDocument> \{\n.*?^    \}\n",
    re.MULTILINE | re.DOTALL,
)
CONSTANT = re.compile(r'^pub const DEVELOPMENT_NAMESPACE: &str = "[^"\n]+";\n', re.MULTILINE)


def identity(path: Path) -> dict:
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path.resolve()), "bytes": path.stat().st_size, "sha256": digest}


def save(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def toml_value(value) -> str:
    if isinstance(value, dict):
        return "{ " + ", ".join(json.dumps(k) + " = " + toml_value(v) for k, v in value.items()) + " }"
    return json.dumps(value)


def unique(pattern: re.Pattern, text: str, label: str) -> str:
    found = list(pattern.finditer(text))
    if len(found) != 1:
        raise RuntimeError(f"require exactly one unchanged {label}, found {len(found)}")
    return found[0].group()


def materialize(args) -> None:
    source = args.source.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    sources = {}
    source_links = {}
    copies = {}

    def copy(relative: Path, destination: Path) -> None:
        original = source / relative
        if not original.is_file():
            raise RuntimeError(f"require a regular source file: {original}")
        if original.is_symlink():
            if not original.resolve(strict=True).is_relative_to(source):
                raise RuntimeError(f"source link escapes the frozen checkout: {original}")
            source_links[str(relative)] = str(original.readlink())
        before = identity(original)
        target = output / destination
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(original.read_bytes())
        if identity(target)["sha256"] != before["sha256"]:
            raise RuntimeError(f"copy changed source bytes: {relative}")
        sources[str(relative)] = before
        copies[str(destination)] = {"source": str(relative), "copied": identity(target)}

    for directory, target in [(LOADER / "binary_document", Path("binary_document")), (XAMLX, Path("xamlx"))]:
        for path in sorted((source / directory).rglob("*")):
            if path.is_file() or path.is_symlink():
                copy(path.relative_to(source), target / path.relative_to(source / directory))
    copy(LOADER / "bin/ferroui-fxd.rs", Path("bin/ferroui-fxd.rs"))
    for relative in [Path("Cargo.toml"), Path("Cargo.lock"), LOADER / "Cargo.toml",
                     LOADER / "compiler_extensions/ferro_xaml_il_compiler.rs",
                     LOADER / "compiler_extensions/transformers/development_key_transformer.rs"]:
        copy(relative, Path("original") / relative)

    workspace_text = (source / "Cargo.toml").read_text()
    workspace = tomllib.loads(workspace_text)["workspace"]
    loader = tomllib.loads((source / LOADER / "Cargo.toml").read_text())
    xamlx = tomllib.loads((source / XAMLX / "Cargo.toml").read_text())
    if set(xamlx["dependencies"]) != {"roxmltree"} or xamlx["dependencies"]["roxmltree"] != {"workspace": True}:
        raise RuntimeError("review changed XamlX dependency closure before isolating the tool")
    if loader["dependencies"]["xamlx"] != {"workspace": True, "features": ["testing"]}:
        raise RuntimeError("review changed Loader XamlX feature selection")
    if loader["dependencies"]["flate2"] != {"workspace": True, "optional": True}:
        raise RuntimeError("review changed Loader compression dependency selection")
    if workspace["dependencies"]["flate2"] != {"version": "1.1", "default-features": False, "features": ["rust_backend"]}:
        raise RuntimeError("review changed compression backend before archive reproduction")
    readme = Path(workspace["package"]["readme"])
    if readme.is_absolute() or ".." in readme.parts:
        raise RuntimeError("workspace readme must stay within the isolated package")
    copy(readme, readme)

    method_source = LOADER / "compiler_extensions/ferro_xaml_il_compiler.rs"
    constant_source = LOADER / "compiler_extensions/transformers/development_key_transformer.rs"
    method = unique(METHOD, (source / method_source).read_text(), "lexical parser method")
    constant = unique(CONSTANT, (source / constant_source).read_text(), "development namespace constant")
    facade = (
        "// Archive-tool facade; the method and constant below are verbatim source excerpts.\n"
        "use std::collections::HashMap;\n"
        "use xamlx::{ast::XamlDocument, exceptions::XamlResult, parsers::XDocumentXamlParser, XamlNamespaces};\n\n"
        + constant + "\npub(crate) struct FerroXamlIlCompiler;\n\nimpl FerroXamlIlCompiler {\n" + method + "}\n"
    )
    (output / "compiler_extensions.rs").write_text(facade)
    (output / "lib.rs").write_text("//! Isolated archive tooling; no framework runtime is linked.\npub mod binary_document;\nmod compiler_extensions;\n")
    metadata = unique(re.compile(r"^\[workspace\.package\]\n.*?(?=^\[)", re.MULTILINE | re.DOTALL), workspace_text, "workspace package metadata")
    manifest = (
        '[workspace]\nresolver = "2"\nmembers = ["xamlx"]\n\n[package]\n'
        f'name = "{PACKAGE}"\nversion = "0.0.0"\nedition = {toml_value(workspace["package"]["edition"])}\n'
        f'rust-version = {toml_value(workspace["package"]["rust-version"])}\npublish = false\nautobins = false\n\n'
        '[lib]\nname = "ferroui_markup_xaml_loader"\npath = "lib.rs"\n\n'
        '[[bin]]\nname = "ferroui-fxd"\npath = "bin/ferroui-fxd.rs"\n\n[dependencies]\n'
        f'xamlx = {{ path = "xamlx", version = "={workspace["package"]["version"]}", features = ["testing"] }}\n'
        f'flate2 = {toml_value(workspace["dependencies"]["flate2"])}\n\n'
        + metadata + '\n[workspace.dependencies]\n'
        + f'roxmltree = {toml_value(workspace["dependencies"]["roxmltree"])}\n'
    )
    tomllib.loads(manifest)
    (output / "Cargo.toml").write_text(manifest)
    (output / "Cargo.lock").write_bytes((output / "original/Cargo.lock").read_bytes())
    for relative, item in sources.items():
        if identity(source / relative) != item:
            raise RuntimeError("source changed during materialization")
    for relative, target in source_links.items():
        if str((source / relative).readlink()) != target:
            raise RuntimeError("source link changed during materialization")
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=source, text=True, capture_output=True)
    report = {
        "schema_version": 1, "scope": __doc__, "directory": str(output), "package": PACKAGE,
        "producer": identity(Path(__file__)), "source_directory": str(source),
        "source_revision": revision.stdout.strip() if revision.returncode == 0 else None,
        "source_inputs": sources, "source_links_materialized_as_regular_files": source_links,
        "verbatim_copies": copies,
        "facade_extraction": {"method_source": str(method_source), "method": method,
                              "constant_source": str(constant_source), "constant": constant,
                              "method_sha256": hashlib.sha256(method.encode()).hexdigest(),
                              "constant_sha256": hashlib.sha256(constant.encode()).hexdigest()},
        "xamlx_version": workspace["package"]["version"],
        "files": {str(path.relative_to(output)): identity(path) for path in sorted(output.rglob("*"))
                  if path.is_file() and path != output / "Cargo.lock"},
        "lock_seed": identity(output / "original/Cargo.lock"),
        "lock_state": "Cargo.lock is a copied seed; resolve offline, then run verify-lock before --locked build.",
        "shipping_graph_substitution_allowed": False,
    }
    save(output / "materialization.json", report)
    print(output / "materialization.json")


def verify_lock(args) -> None:
    manifest_path = args.manifest.resolve(strict=True)
    report = json.loads(manifest_path.read_text())
    for relative, expected in report["files"].items():
        if identity(Path(expected["path"])) != expected:
            raise RuntimeError(f"immutable packer input changed: {relative}")
    output = Path(report["directory"])
    source_lock = tomllib.loads(Path(report["lock_seed"]["path"]).read_text())
    resolved_path = output / "Cargo.lock"
    resolved = tomllib.loads(resolved_path.read_text())

    def external(lock):
        return {(p["name"], p["version"], p["source"], p.get("checksum"))
                for p in lock["package"] if p.get("source")}

    if not external(resolved) <= external(source_lock):
        raise RuntimeError("tool resolution introduced an external dependency absent from the source lock")
    local = {(p["name"], p["version"]) for p in resolved["package"] if not p.get("source")}
    if local != {(PACKAGE, "0.0.0"), ("xamlx", report["xamlx_version"])}:
        raise RuntimeError(f"isolated tool must contain only its package and local XamlX: {local}")
    if not {"flate2", "roxmltree"} <= {p[0] for p in external(resolved)}:
        raise RuntimeError("resolved tool is missing its compression or XML dependency")
    destination = args.out.resolve()
    if destination.exists():
        raise RuntimeError("preserve prior verification; choose a fresh --out")
    save(destination, {"schema_version": 1, "materialization": identity(manifest_path),
                       "resolved_lock": identity(resolved_path), "external_packages": [
                           {"name": n, "version": v, "source": s, "checksum": c}
                           for n, v, s, c in sorted(external(resolved))],
                       "immutable_inputs_verified": True, "external_lock_subset_verified": True,
                       "shipping_graph_substitution_allowed": False})
    print(destination)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="action", required=True)
    prepare = commands.add_parser("materialize")
    prepare.add_argument("--source", type=Path, default=ROOT)
    prepare.add_argument("--output", type=Path, required=True)
    prepare.set_defaults(handler=materialize)
    verify = commands.add_parser("verify-lock")
    verify.add_argument("--manifest", type=Path, required=True)
    verify.add_argument("--out", type=Path, required=True)
    verify.set_defaults(handler=verify_lock)
    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
