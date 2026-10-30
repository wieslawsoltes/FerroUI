#!/usr/bin/env python3
"""Audit the copied macOS theme inventory without claiming native visual fidelity.

    python3 scripts/macos-theme/audit.py           # print the summary, fail on contract errors
    python3 scripts/macos-theme/audit.py --json    # also print the full inventory as JSON

The baseline is immutable git content, not the working WinUI directory. XML and
include-graph checks complement the compiled/runtime Rust suite; they do not
resolve resource types, exercise interaction, or compare native screenshots.
The inventory is computed on demand and not checked in. Standard library only;
no network access.
"""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import io
import json
from pathlib import Path
import posixpath
import re
import subprocess
import sys
import tarfile
from urllib.parse import urlsplit
from xml.etree import ElementTree as ET

BASE_COMMIT = "98a56f09459c8ae02f07a2bcd235c0e87b4d44b3"
UPSTREAM_COMMIT = "7e4b0c2096be402f51cc8f7d203ebf67cff183d0"
BASE_ASSEMBLY = "FerroUI.Themes.WinUI"
THEME_ASSEMBLY = "FerroUI.Themes.MacOS"
BASE_DIRECTORY = f"src/{BASE_ASSEMBLY}"
THEME_DIRECTORY = f"src/{THEME_ASSEMBLY}"
XAML_KEY = "{http://schemas.microsoft.com/winfx/2006/xaml}Key"
MOTION_TOKENS = {
    "MacOSMotionFastDuration": "DynamicResource",
    "MacOSMotionStandardDuration": "DynamicResource",
    "MacOSMotionSlowDuration": "DynamicResource",
    # This period is immutable and must resolve before Animation interprets
    # keyframes. Reduce Motion deactivates its selector, not its duration.
    "MacOSProgressPulseDuration": "StaticResource",
}
# A necessary exception must identify its document, XML element path and reason.
# There are currently no exceptions to live central duration resources.
DURATION_EXCEPTIONS: dict[tuple[str, str], str] = {}
RESOURCE = re.compile(r"\{(DynamicResource|StaticResource)\s+(?:ResourceKey\s*=\s*)?([^{}\s,]+)\s*\}")
IMPLICIT_THEME = re.compile(r"\{x:Type\s+([^{}]+)\}")


def local_name(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def baseline_documents(repository: Path) -> dict[str, bytes]:
    result = subprocess.run(
        ["git", "archive", BASE_COMMIT, BASE_DIRECTORY],
        cwd=repository, capture_output=True, check=False,
    )
    if result.returncode:
        raise RuntimeError(
            f"Cannot read pinned baseline {BASE_COMMIT}: {result.stderr.decode(errors='replace').strip()}\n"
            "Fetch that commit or use a full checkout; the audit never substitutes current WinUI files."
        )
    prefix = BASE_DIRECTORY + "/"
    with tarfile.open(fileobj=io.BytesIO(result.stdout), mode="r:") as archive:
        return {
            member.name[len(prefix):]: archive.extractfile(member).read()
            for member in archive.getmembers()
            if member.isfile() and member.name.startswith(prefix) and member.name.endswith(".xaml")
        }


def current_documents(repository: Path) -> dict[str, bytes]:
    directory = repository / THEME_DIRECTORY
    return {p.relative_to(directory).as_posix(): p.read_bytes() for p in sorted(directory.rglob("*.xaml"))}


def walk(root: ET.Element, path: str = ""):
    """Yield stable element paths, independent of whitespace and comments."""
    current = path or f"/{local_name(root.tag)}"
    yield root, current
    counts: Counter[str] = Counter()
    for child in root:
        name = local_name(child.tag)
        counts[name] += 1
        yield from walk(child, f"{current}/{name}[{counts[name]}]")


def normalized(value: str) -> str:
    return value.replace(BASE_ASSEMBLY, THEME_ASSEMBLY).replace("WinUIControls.xaml", "MacOSControls.xaml")


def semantic_tree(root: ET.Element):
    """Ignore provenance comments, formatting and the intentional assembly rename."""
    return (
        normalized(root.tag),
        tuple(sorted((normalized(key), normalized(value)) for key, value in root.attrib.items())),
        normalized((root.text or "").strip()),
        tuple(semantic_tree(child) for child in root),
    )


def exported_themes(root: ET.Element) -> list[dict[str, str]]:
    """Collect public resource themes, excluding private templates and previews."""
    result = []
    def visit(node: ET.Element):
        name = local_name(node.tag)
        if name in {"ControlTemplate", "DataTemplate", "FocusAdornerTemplate", "Design.PreviewWith", "Setter"}:
            return
        if name == "ControlTheme":
            key = node.get(XAML_KEY, "")
            if IMPLICIT_THEME.fullmatch(key):
                result.append({"key": normalized(key), "target_type": normalized(node.get("TargetType", ""))})
            return
        for child in node:
            visit(child)
    visit(root)
    return sorted(result, key=lambda entry: entry["key"])


def parse_documents(documents: dict[str, bytes], label: str, errors: list[dict]) -> dict[str, ET.Element]:
    roots = {}
    for name, content in sorted(documents.items()):
        try:
            roots[name] = ET.fromstring(content)
        except ET.ParseError as exc:
            errors.append({"code": "invalid_xml", "document": f"{label}/{name}", "detail": str(exc)})
    return roots


def include_graph(roots: dict[str, ET.Element], entry: str, assembly: str, errors: list[dict]):
    reachable: set[str] = set()
    edges = []
    external = []
    pending = [entry]
    while pending:
        document = pending.pop()
        if document in reachable:
            continue
        if document not in roots:
            errors.append({"code": "missing_merged_document", "document": document, "detail": f"Required by {assembly}"})
            continue
        reachable.add(document)
        for node, location in walk(roots[document]):
            kind = local_name(node.tag)
            if kind not in {"MergeResourceInclude", "ResourceInclude", "StyleInclude"}:
                continue
            # A named ResourceInclude is stored as a value (e.g. CompactStyles),
            # not merged into the default resource scope by that declaration.
            if kind == "ResourceInclude" and node.get(XAML_KEY):
                continue
            source = node.get("Source", "")
            if not source:
                errors.append({"code": "missing_include_source", "document": document, "element": location})
                continue
            uri = urlsplit(source)
            if uri.scheme:
                if uri.scheme != "ferres" or uri.netloc != assembly:
                    external.append({"document": document, "source": source})
                    continue
                target = uri.path.lstrip("/")
            elif source.startswith("/"):
                target = source.lstrip("/")
            else:
                target = posixpath.normpath(posixpath.join(posixpath.dirname(document), source))
            if target.startswith("../"):
                errors.append({"code": "escaping_include", "document": document, "source": source})
                continue
            edges.append({"from": document, "to": target})
            pending.append(target)
    return reachable, sorted(edges, key=lambda e: (e["from"], e["to"])), sorted(external, key=lambda e: (e["document"], e["source"]))


def motion_consumers(roots: dict[str, ET.Element], errors: list[dict]) -> list[dict]:
    result = []
    for document, root in sorted(roots.items()):
        for node, location in walk(root):
            name = local_name(node.tag)
            durations = []
            if "Duration" in node.attrib:
                durations.append((location + "/@Duration", node.attrib["Duration"]))
            if name == "Setter" and node.get("Property", "").split(".")[-1] == "Duration":
                durations.append((location + "/@Value", node.get("Value", "")))
            if name.endswith(".Duration"):
                child = next(iter(node), None)
                value = (node.text or "").strip()
                if child is not None and local_name(child.tag) in {
                    "DynamicResource", "DynamicResourceExtension", "StaticResource", "StaticResourceExtension"
                }:
                    kind = local_name(child.tag).removesuffix("Extension")
                    value = "{" + kind + " " + child.get("ResourceKey", "") + "}"
                durations.append((location, value))
            timed = name in {"Animation", "CrossFade", "PageSlide"} or (
                name.endswith("Transition") and "." not in name and name != "CompositePageTransition"
            )
            if timed and "Duration" not in node.attrib and not any(local_name(child.tag).endswith(".Duration") for child in node):
                durations.append((location + "/@Duration", "<implicit framework default>"))
            for property_path, value in durations:
                match = RESOURCE.fullmatch(value)
                token = match.group(2) if match else None
                exception = DURATION_EXCEPTIONS.get((document, property_path))
                central = bool(match and match.group(1) == MOTION_TOKENS.get(token))
                entry = {"document": document, "element": property_path, "value": value, "resource_key": token}
                if exception:
                    entry["exception_reason"] = exception
                if not central and not exception:
                    errors.append({"code": "noncentral_motion_duration", "document": document, "element": property_path,
                                   "detail": f"{value!r}; use the central token with its declared resource lifetime"})
                result.append(entry)
    return result


def audit(base: dict[str, bytes], current: dict[str, bytes]) -> dict:
    errors: list[dict] = []
    base_roots = parse_documents(base, BASE_ASSEMBLY, errors)
    roots = parse_documents(current, THEME_ASSEMBLY, errors)
    base_reachable, _, _ = include_graph(base_roots, "WinUITheme.xaml", BASE_ASSEMBLY, errors)
    reachable, edges, external = include_graph(roots, "MacOSTheme.xaml", THEME_ASSEMBLY, errors)

    # Examine parsed markup and namespace declarations, never provenance comments.
    forbidden = re.compile(r"(?:ferres://|using:|assembly=|clr-namespace:)?FerroUI\.Themes\.WinUI(?:[/.;\s}]|$)", re.I)
    for document, root in sorted(roots.items()):
        for node, location in walk(root):
            for attribute, value in sorted(node.attrib.items()):
                if forbidden.search(value):
                    errors.append({"code": "winui_assembly_reference", "document": document, "element": location,
                                   "attribute": local_name(attribute), "value": value})
            if node.text and forbidden.search(node.text):
                errors.append({"code": "winui_assembly_reference", "document": document, "element": location, "value": node.text.strip()})
        for _, namespace in ET.iterparse(io.BytesIO(current[document]), events=("start-ns",)):
            if forbidden.search(namespace[1]):
                errors.append({"code": "winui_namespace_reference", "document": document, "value": namespace[1]})

    expected: dict[str, set[str]] = {}
    available: dict[str, set[str]] = {}
    for collection, documents, destination in [(base_roots, base_reachable, expected), (roots, reachable, available)]:
        for document in sorted(documents):
            for theme in exported_themes(collection[document]):
                destination.setdefault(theme["key"], set()).add(document)
    for key in sorted(expected.keys() - available.keys()):
        errors.append({"code": "missing_default_control_theme", "key": key, "baseline_documents": sorted(expected[key]),
                       "detail": "The baseline default theme is absent from the macOS merged include graph"})

    # The WinUI theme keeps growing after the fork: every default control theme of the current
    # WinUI tree needs a macOS counterpart too (or the WinUI addition is ported in the same change).
    winui_directory = Path(__file__).resolve().parents[2] / BASE_DIRECTORY
    if winui_directory.is_dir():
        winui_now = {p.relative_to(winui_directory).as_posix(): p.read_bytes() for p in sorted(winui_directory.rglob("*.xaml"))}
        winui_roots = parse_documents(winui_now, BASE_ASSEMBLY, errors)
        winui_reachable, _, _ = include_graph(winui_roots, "WinUITheme.xaml", BASE_ASSEMBLY, errors)
        winui_keys = {theme["key"] for document in winui_reachable for theme in exported_themes(winui_roots[document])}
        for key in sorted(winui_keys - available.keys()):
            errors.append({"code": "missing_current_winui_control_theme", "key": key,
                           "detail": "The current WinUI theme styles this control; the macOS theme does not"})

    controls = []
    for document in sorted(name for name in base_roots if name.startswith("Controls/") and name != "Controls/WinUIControls.xaml"):
        actual = roots.get(document)
        base_merged = document in base_reachable
        if actual is None:
            errors.append({"code": "missing_copied_control_document", "document": document})
            state = "missing"
        else:
            state = "unchanged" if semantic_tree(base_roots[document]) == semantic_tree(actual) else "changed"
            if base_merged and document not in reachable:
                errors.append({"code": "unmerged_copied_control_document", "document": document})
        controls.append({
            "document": document,
            "template_status": state,
            "baseline_merged": base_merged,
            "macos_merged": document in reachable,
            "default_themes": exported_themes(base_roots[document]),
            "baseline_sha256": sha256(base[document]),
            "macos_sha256": sha256(current[document]) if document in current else None,
            "native_fidelity": "unverified",
        })
    new_controls = sorted(name for name in roots if name.startswith("Controls/") and name not in base_roots and name != "Controls/MacOSControls.xaml")
    motion = motion_consumers(roots, errors)
    resource_keys = sorted({
        match.group(2) for root in roots.values() for node, _ in walk(root)
        for value in node.attrib.values() for match in RESOURCE.finditer(value)
        if match.group(2).startswith("MacOS")
    })
    counts = Counter(row["template_status"] for row in controls)
    input_digest = sha256("\n".join(f"{name}\0{sha256(content)}" for name, content in sorted(current.items())).encode())
    return {
        "schema_version": 1,
        "provenance": {
            "repository": "https://github.com/wieslawsoltes/FerroUI",
            "ferroui_base_commit": BASE_COMMIT,
            "copied_from": BASE_DIRECTORY,
            "theme_directory": THEME_DIRECTORY,
            "upstream_winui_commit": UPSTREAM_COMMIT,
            "upstream_license_notice": f"{THEME_DIRECTORY}/NOTICE.md",
            "template_comparison": "Parsed XML, ignoring comments/formatting and the intentional assembly/manifest rename",
            "unchanged_template_meaning": "The template structure is inherited; shared macOS resources and runtime settings can still change its appearance",
            "changed_template_meaning": "The template structure differs from the baseline; this is not a native fidelity measurement",
            "current_xaml_sha256": input_digest,
        },
        "static_contracts": {
            "status": "pass" if not errors else "fail",
            "xml_documents_parsed": len(roots),
            "documents_reachable_from_theme": len(reachable),
            "baseline_control_documents": len(controls),
            "merged_baseline_control_documents": sum(row["macos_merged"] for row in controls),
            "changed_control_documents": counts["changed"],
            "unchanged_control_documents": counts["unchanged"],
            "new_control_documents": new_controls,
            "baseline_default_theme_keys": len(expected),
            "preserved_default_theme_keys": len(expected.keys() & available.keys()),
            "duration_consumers": len(motion),
            "duration_resource_keys": sorted({entry["resource_key"] for entry in motion if entry["resource_key"]}),
            "documented_duration_exceptions": len(DURATION_EXCEPTIONS),
            "external_includes": external,
            "errors": sorted(errors, key=lambda entry: json.dumps(entry, sort_keys=True)),
        },
        "inherited_behavior_coverage": {
            "status": "default-theme inventory preserved" if not expected.keys() - available.keys() else "incomplete",
            "meaning": "Static template availability from the copied WinUI foundation; this is not an executed behavior result",
            "runtime_gate": "cargo test --locked -p ferroui-themes-macos --lib",
            "runtime_gate_status": "not evaluated by this audit",
            "default_theme_inventory": [
                {"key": key, "baseline_documents": sorted(expected[key]), "macos_documents": sorted(available.get(key, set()))}
                for key in sorted(expected)
            ],
        },
        "resource_validation": {
            "macos_resource_references": resource_keys,
            "status": "recorded for traceability; availability, types and live invalidation are runtime test gates",
            "implementation": f"{THEME_DIRECTORY}/settings_resources.rs",
            "tests": [f"{THEME_DIRECTORY}/tests/resource_dictionary_tests.rs", f"{THEME_DIRECTORY}/tests/accessibility_tests.rs"],
        },
        "native_fidelity": {
            "status": "unverified",
            "target": "macOS 26",
            "reference_capture_tool": "scripts/macos-theme/capture-native.swift",
            "measured_reference_evidence": [],
            "visual_comparison_evidence": [],
            "motion_comparison_evidence": [],
            "meaning": "Changed templates and passing structural/runtime tests do not establish native pixel, material or animation equivalence",
            "apple_design_references": ["https://developer.apple.com/videos/play/wwdc2025/310/", "https://developer.apple.com/videos/play/wwdc2025/219/"],
        },
        "control_documents": controls,
        "merged_include_graph": edges,
        "motion_duration_consumers": motion,
    }


def summary(report: dict) -> str:
    status = report["static_contracts"]
    return "\n".join([
        f"macOS 26 theme audit: {status['status'].upper()}",
        f"Pinned FerroUI baseline: {BASE_COMMIT}",
        f"XML: {status['xml_documents_parsed']} parsed, {status['documents_reachable_from_theme']} reachable from MacOSTheme.xaml",
        f"Copied controls: {status['merged_baseline_control_documents']}/{status['baseline_control_documents']} merged; "
        f"{status['changed_control_documents']} changed, {status['unchanged_control_documents']} unchanged",
        f"Inherited default themes: {status['preserved_default_theme_keys']}/{status['baseline_default_theme_keys']} preserved",
        f"Motion: {status['duration_consumers']} duration consumers; {status['documented_duration_exceptions']} documented exceptions",
        "Runtime behavior/resource checks: not executed by this static audit",
        "Native macOS 26 visual/material/motion fidelity: UNVERIFIED",
    ])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--json", action="store_true", help="print the full inventory as JSON")
    # Accepted for compatibility with earlier invocations; checking is the default.
    parser.add_argument("--check", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[2]
    try:
        report = audit(baseline_documents(repository), current_documents(repository))
    except (OSError, RuntimeError) as exc:
        print(f"macOS theme audit failed: {exc}", file=sys.stderr)
        return 1
    print(summary(report))
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    for error in report["static_contracts"]["errors"]:
        print("ERROR " + json.dumps(error, sort_keys=True), file=sys.stderr)
    return int(bool(report["static_contracts"]["errors"]))


if __name__ == "__main__":
    sys.exit(main())
