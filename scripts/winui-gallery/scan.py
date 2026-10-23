#!/usr/bin/env python3
"""Reproducible, conservative source accounting for the WinUI Gallery port.

This is a source scanner, not a WinMD/Roslyn compiler or a behavioral oracle.
Declaration candidates, reviewed mappings and executed samples are separate axes.
No source-name match is promoted to functional parity. Python 3.11+, stdlib only.
"""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from dataclasses import dataclass
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import xml.etree.ElementTree as ET

REPO = Path(__file__).resolve().parents[2]
DATA = REPO / "docs/porting/winui-gallery/data"
sys.path.insert(0, str(REPO / "scripts/port-status"))
from rustscan import blank_comments_and_literals, scan_rust  # noqa: E402

SCHEMA = 1
PRESENTATION = "http://schemas.microsoft.com/winfx/2006/xaml/presentation"
XAML = "http://schemas.microsoft.com/winfx/2006/xaml"
XML = "http://www.w3.org/XML/1998/namespace"
MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
DESIGN = "http://schemas.microsoft.com/expression/blend/2008"
NS_MACROS = {
    "MU_X_NAMESPACE": "Microsoft.UI.Xaml",
    "MU_XC_NAMESPACE": "Microsoft.UI.Xaml.Controls",
    "MU_XCP_NAMESPACE": "Microsoft.UI.Xaml.Controls.Primitives",
    "MU_XCAV_NAMESPACE": "Microsoft.UI.Xaml.Controls.AnimatedVisuals",
    "MU_X_XTI_NAMESPACE": "Microsoft.UI.Xaml.XamlTypeInfo",
    "MU_XAP_NAMESPACE": "Microsoft.UI.Xaml.Automation.Peers",
    "MU_XA_NAMESPACE": "Microsoft.UI.Xaml.Automation",
    "MU_XM_NAMESPACE": "Microsoft.UI.Xaml.Media",
    "MU_XMA_NAMESPACE": "Microsoft.UI.Xaml.Media.Animation",
    "MU_XI_NAMESPACE": "Microsoft.UI.Xaml.Input",
    "MU_XD_NAMESPACE": "Microsoft.UI.Xaml.Documents",
    "MU_PRIVATE_CONTROLS_NAMESPACE": "Microsoft.UI.Private.Controls",
    "MU_PRIVATE_MEDIA_NAMESPACE": "Microsoft.UI.Private.Media",
}
API_DIRS = (
    "controls/dev",
    "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model",
    "dxaml/xcp/tools/XCPTypesAutoGen/Modules",
)
GENERATED = ("api-map.json", "gallery-page-map.json", "gallery-xaml-map.json", "xaml-feature-map.json", "summary.json", "REPORT.md")
SNAPSHOTS = ("upstream-api.json", "gallery-pages.json", "gallery-usage.json", "source-files.json")


def canonical(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def tracked(root: Path, *directories: str) -> list[Path]:
    return [root / p for p in git(root, "ls-files", "--", *directories).splitlines() if p]


def verify_checkout(root: Path, commit: str) -> None:
    actual = git(root, "rev-parse", "HEAD")
    if actual != commit:
        raise ValueError(f"{root.name}: expected pinned {commit}, found {actual}")
    result = subprocess.run(["git", "-C", str(root), "diff", "--quiet", "HEAD", "--"])
    if result.returncode:
        raise ValueError(f"{root.name}: tracked upstream files differ from the pinned commit")


def source_location(source: str, path: str, start: int) -> dict:
    return {"path": path, "line": source.count("\n", 0, start) + 1}


def matching(text: str, start: int, opening: str = "{", closing: str = "}") -> int:
    depth = 1
    for i in range(start + 1, len(text)):
        if text[i] == opening:
            depth += 1
        elif text[i] == closing:
            depth -= 1
            if not depth:
                return i
    raise ValueError(f"Unterminated {opening!r} at offset {start}")


def mask(source: str) -> str:
    # Keep offsets so reported locations refer to original files. Preprocessor
    # conditions are retained in source metadata, not evaluated as one SDK build.
    return re.sub(r"(?m)^\s*#.*$", lambda m: " " * len(m.group()), blank_comments_and_literals(source))


def without_attributes(text: str) -> str:
    output = list(text)
    i = 0
    while i < len(text):
        if text[i] == "[":
            end = matching(text, i, "[", "]")
            output[i:end + 1] = " " * (end + 1 - i)
            i = end
        i += 1
    return "".join(output)


def attributes(source: str) -> list[str]:
    clean = mask(source)
    result = []
    i = 0
    while i < len(clean):
        if clean[i] == "[":
            end = matching(clean, i, "[", "]")
            result.append(signature(source[i + 1:end]))
            i = end
        i += 1
    return result


@dataclass
class Part:
    start: int
    header_end: int
    end: int
    body_start: int | None


def parts(text: str, start: int, end: int) -> list[Part]:
    """Top-level declarations, respecting attributes, parameter and body braces."""
    answer = []
    first = start
    i = start
    parens = 0
    brackets = 0
    while i < end:
        c = text[i]
        if c == "(":
            parens += 1
        elif c == ")":
            parens -= 1
        elif c == "[":
            brackets += 1
        elif c == "]":
            brackets -= 1
        elif not parens and not brackets and c == "{":
            close = matching(text, i)
            answer.append(Part(first, i, close + 1, i + 1))
            i = close
            first = i + 1
        elif not parens and not brackets and c == ";":
            if text[first:i].strip():
                answer.append(Part(first, i, i + 1, None))
            first = i + 1
        i += 1
    return answer


def signature(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip().rstrip(";")


def audience(attributes: str, namespace: str) -> str:
    if ".Private." in namespace or "PrivateApiContract" in attributes or "MUX_PRIVATE" in attributes:
        return "private"
    return "public-source"


def parse_members(source: str, text: str, start: int, end: int, owner: str,
                  kind: str, language: str, path: str, inherited_audience: str) -> list[dict]:
    members = []
    if kind == "enum":
        body = without_attributes(text[start:end])
        for m in re.finditer(r"(?:^|,)\s*([A-Za-z_]\w*)\s*(?:=\s*([^,]+))?", body):
            members.append({"name": m.group(1), "kind": "enum-value",
                            "signature": signature(source[start + m.start():start + m.end()]).lstrip(", "),
                            "audience": inherited_audience,
                            "source": source_location(source, path, start + m.start())})
        return members
    for part in parts(text, start, end):
        raw = source[part.start:part.header_end]
        head = without_attributes(text[part.start:part.header_end]).strip()
        attrs = raw[:max(0, len(raw) - len(raw.lstrip()))] + raw
        member_audience = "private" if audience(attrs, "") == "private" else inherited_audience
        if not head and part.body_start is not None:
            # MIDL 3 version/feature blocks: [MUX_PUBLIC_Vn] { ... }.
            members.extend(parse_members(source, text, part.body_start, part.end - 1,
                                         owner, kind, language, path, member_audience))
            continue
        if language == "csharp-model" and kind != "interface" and not re.search(r"\bpublic\b", head):
            continue
        if not head or re.search(r"\b(class|interface|enum|struct)\b", head):
            continue
        if "(" in head:
            before = head.split("(", 1)[0].strip()
            names = re.findall(r"[A-Za-z_]\w*", before)
            if not names:
                continue
            name = names[-1]
            member_kind = "constructor" if name == owner else "method"
        else:
            before = head.split("=", 1)[0].strip()
            names = re.findall(r"[A-Za-z_]\w*", before)
            if len(names) < 2:
                continue
            name = names[-1]
            member_kind = "event" if re.search(r"\bevent\b", head) else "property"
            if re.search(r"\bconst\b", head):
                member_kind = "constant"
        clean_raw = without_attributes(mask(raw)).strip()
        sig = signature(clean_raw)
        if member_kind == "property" and part.body_start is not None:
            accessors = signature(text[part.body_start:part.end - 1])
            sig += " { " + accessors + " }"
        projected = re.search(r'\bDXamlOverloadName\s*\(\s*"([^"]+)"', raw)
        members.append({"name": name, "projected_name": projected.group(1) if projected else name,
                        "attributes": attributes(raw),
                        "kind": member_kind, "signature": sig,
                        "audience": member_audience,
                        "source": source_location(source, path, part.start + len(raw) - len(raw.lstrip()))})
    return members


def parse_api(source: str, path: str, language: str) -> list[dict]:
    text = mask(source)
    result = []
    for nm in re.finditer(r"\bnamespace\s+([A-Za-z_][\w.]*)\s*\{", text):
        namespace = NS_MACROS.get(nm.group(1), nm.group(1))
        if not namespace.startswith(("Microsoft.UI.", "Windows.UI.Xaml")):
            continue
        end = matching(text, nm.end() - 1)
        for part in parts(text, nm.end(), end):
            raw = source[part.start:part.header_end]
            head = without_attributes(text[part.start:part.header_end]).strip()
            if part.body_start is None:
                delegate = re.search(r"\bdelegate\s+.+?\s+([A-Za-z_]\w*)\s*\(", head)
                if delegate and (language != "csharp-model" or re.search(r"\bpublic\b", head)):
                    name = delegate.group(1)
                    visibility = audience(raw, namespace)
                    result.append({"id": f"{namespace}.{name}", "namespace": namespace, "name": name,
                                   "kind": "delegate", "base": None, "audience": visibility, "language": language,
                                   "attributes": attributes(raw),
                                   "source": source_location(source, path, part.start + len(raw) - len(raw.lstrip())),
                                   "members": [{"name": "Invoke", "kind": "method", "signature": signature(head),
                                                "audience": visibility, "source": source_location(source, path, part.start)}]})
                continue
            match = re.search(r"\b(runtimeclass|class|struct|interface|enum)\s+([A-Za-z_]\w*)(?:\s*:\s*([^\n{]+))?", head)
            if match is None:
                continue
            if language == "csharp-model" and not re.search(r"\bpublic\b", head):
                continue
            kind, name, base = match.groups()
            visibility = audience(raw, namespace)
            item = {
                "id": f"{namespace}.{name}", "namespace": namespace, "name": name,
                "kind": kind, "base": signature(base).split(",")[0] if base else None,
                "attributes": attributes(raw),
                "audience": visibility, "language": language,
                "source": source_location(source, path, part.start + len(raw) - len(raw.lstrip())),
                "members": parse_members(source, text, part.body_start, part.end - 1,
                                         name, kind, language, path, visibility),
            }
            result.append(item)
    return result


def collect_api(root: Path) -> tuple[dict, list[dict]]:
    by_id = {}
    files = []
    namespaces = Counter()
    for path in tracked(root, *API_DIRS, "controls/idl/Microsoft.UI.Xaml.Controls.idl"):
        relative = path.relative_to(root).as_posix()
        is_idl = relative.startswith("controls/dev/") and path.suffix == ".idl"
        is_model = not relative.startswith("controls/") and path.suffix == ".cs"
        namespace_macros = relative == "controls/idl/Microsoft.UI.Xaml.Controls.idl"
        if not (is_idl or is_model or namespace_macros):
            continue
        data = path.read_bytes()
        files.append({"repository": "winui", "path": relative, "sha256": digest(data), "bytes": len(data)})
        if namespace_macros:
            definitions = re.findall(r"(?m)^#define\s+(MU_\w+_NAMESPACE)\s+([\w.]+)", data.decode("utf-8-sig"))
            for name, value in definitions:
                if name in NS_MACROS and NS_MACROS[name] != value:
                    raise ValueError(f"Upstream namespace macro changed: {name}: {NS_MACROS[name]} -> {value}")
            continue
        for name in re.findall(r"\bnamespace\s+(MU_\w+_NAMESPACE)", data.decode("utf-8-sig")):
            namespaces[name] += 1
        for item in parse_api(data.decode("utf-8-sig"), relative, "midl3" if is_idl else "csharp-model"):
            previous = by_id.get(item["id"])
            if previous is None:
                by_id[item["id"]] = item
            else:
                previous.setdefault("additional_sources", []).append(item["source"])
                previous["members"].extend(item["members"])
                if not previous["base"]:
                    previous["base"] = item["base"]
    for item in by_id.values():
        unique = {(x["kind"], x["name"], x["signature"]): x for x in item["members"]}
        item["members"] = sorted(unique.values(), key=lambda x: (x["kind"], x["name"], x["signature"]))
    unknown_namespaces = sorted(set(namespaces) - set(NS_MACROS))
    if unknown_namespaces:
        raise ValueError("Unmapped upstream namespace macros: " + ", ".join(unknown_namespaces))
    return {"schema_version": SCHEMA, "scope": list(API_DIRS),
            "namespace_macros": {name: NS_MACROS[name] for name in sorted(namespaces)},
            "limitations": ["Lexical source model; preprocessor and SDK contract conditions are not evaluated.",
                            "Public-source includes preview contracts; private contracts are retained separately.",
                            "Not a WinMD ABI, signature compatibility, or behavior certification."],
            "types": sorted(by_id.values(), key=lambda x: x["id"])}, files


def split_xml_name(name: str) -> tuple[str, str]:
    if name.startswith("{"):
        namespace, name = name[1:].split("}", 1)
        return namespace, name
    return "", name


def resolve_xml_type(namespace: str, name: str, type_index: dict[str, list[str]]) -> list[str]:
    if namespace == PRESENTATION or not namespace:
        return sorted(x for x in type_index.get(name, []) if x.startswith("Microsoft.UI.Xaml."))
    if namespace.startswith("using:"):
        return [namespace[6:] + "." + name]
    if namespace.startswith("clr-namespace:"):
        return [namespace[14:].split(";")[0] + "." + name]
    return ["{" + namespace + "}" + name]


def parse_xaml(source: str, path: str, type_index: dict[str, list[str]]) -> dict:
    types = Counter()
    members = Counter()
    features = Counter()
    namespaces = Counter()
    diagnostics = []
    try:
        pending = []
        stack = []
        contexts = []
        for event, value in ET.iterparse(io.StringIO(source), events=("start-ns", "start", "end")):
            if event == "start-ns":
                pending.append(value)
                namespaces[value[1]] += 1
            elif event == "start":
                context = dict(contexts[-1]) if contexts else {}
                context.update(pending)
                pending.clear()
                contexts.append(context)
                namespace, local = split_xml_name(value.tag)
                if "." in local:
                    owner, member = local.rsplit(".", 1)
                    for target in resolve_xml_type(namespace, owner, type_index):
                        members[target + "." + member] += 1
                    stack.append(None)
                else:
                    target_types = resolve_xml_type(namespace, local, type_index)
                    if not target_types:
                        target_types = ["unresolved:" + local]
                    for target in target_types:
                        types[target] += 1
                    stack.append(target_types)
                for attribute, attribute_value in value.attrib.items():
                    attr_ns, attr_name = split_xml_name(attribute)
                    if attr_ns in (XAML, XML):
                        features[("x:" if attr_ns == XAML else "xml:") + attr_name] += 1
                    elif attr_ns in (MC, DESIGN):
                        features[("mc:" if attr_ns == MC else "design:") + attr_name] += 1
                    elif "." in attr_name:
                        owner, member = attr_name.rsplit(".", 1)
                        # Unprefixed attached owners use the in-scope default
                        # namespace, not the custom element's own prefix.
                        targets = resolve_xml_type(attr_ns or context.get("", namespace), owner, type_index)
                        for target in targets or ["unresolved:" + owner]:
                            members[target + "." + member] += 1
                    elif stack[-1]:
                        for target in stack[-1]:
                            members[target + "." + attr_name] += 1
                    trimmed = attribute_value.lstrip()
                    extension_value = trimmed if trimmed.startswith("{") and not trimmed.startswith("{}") else ""
                    for extension in re.finditer(r"(?<!\{)\{(?![}\{])\s*([A-Za-z_][\w:.]*)", extension_value):
                        features["markup:" + extension.group(1)] += 1
                        prefix, _, local_extension = extension.group(1).rpartition(":")
                        if prefix and prefix in context and context[prefix] not in (XAML,):
                            for target in resolve_xml_type(context[prefix], local_extension, type_index):
                                types[target] += 1
            else:
                stack.pop()
                contexts.pop()
    except ET.ParseError as error:
        diagnostics.append({"path": path, "kind": "xml-parse", "detail": str(error)})
    return {"types": dict(sorted(types.items())), "members": dict(sorted(members.items())),
            "features": dict(sorted(features.items())), "namespaces": dict(sorted(namespaces.items())),
            "diagnostics": diagnostics}


def collect_gallery(root: Path, api: dict) -> tuple[dict, dict, list[dict]]:
    catalog_path = root / "catalog/windows-samples.json"
    catalog = load(catalog_path)
    data = load(root / "WinUIGallery/SampleSupport/Data/ControlInfoData.json")
    info = {x["UniqueId"]: x for group in data["Groups"] for x in group.get("Items", [])}
    type_index = defaultdict(list)
    for item in api["types"]:
        type_index[item["name"]].append(item["id"])
    files = []
    xaml = []
    code = []
    for path in tracked(root, "WinUIGallery", "catalog/windows-samples.json"):
        if path.suffix not in (".xaml", ".cs", ".txt", ".json", ".csproj"):
            continue
        relative = path.relative_to(root).as_posix()
        content = path.read_bytes()
        files.append({"repository": "gallery", "path": relative, "sha256": digest(content), "bytes": len(content)})
        text = content.decode("utf-8-sig")
        if path.suffix == ".xaml":
            xaml.append({"path": relative, **parse_xaml(text, relative, type_index)})
        elif path.suffix == ".cs":
            clean = mask(text)
            imports = sorted(set(re.findall(r"(?m)^\s*using\s+(?:static\s+)?((?:Microsoft|Windows|CommunityToolkit|WinRT|System)\.[\w.]+)\s*;", clean)))
            # Explicit qualified expressions are evidence; receiver.member on an
            # unbound C# expression is not guessed as an API identifier.
            qualified = sorted(set(re.findall(r"\b(?:Microsoft|Windows|CommunityToolkit|WinRT)\.[A-Za-z_][\w.]*", clean)))
            code.append({"path": relative, "imports": imports, "qualified_references": qualified,
                         "member_accesses": [{"expression": m.group(), "line": text.count("\n", 0, m.start()) + 1,
                                              "resolution": "requires-semantic-binding"}
                                             for m in re.finditer(r"\b[A-Za-z_]\w*(?:\??\.[A-Za-z_]\w*)+", clean)
                                             if not clean[:m.start()].rsplit("\n", 1)[-1].lstrip().startswith(("using ", "namespace "))],
                         "analysis": "lexical; unqualified receivers and overloads require semantic analysis"})
    file_map = {x["path"]: x for x in files}
    xaml_map = {x["path"]: x for x in xaml}
    pages = []
    for item in catalog["controls"]:
        gallery = item["gallery"]
        uid = gallery["uniqueId"]
        page = gallery["page"]
        code_behind = gallery.get("codeBehind")
        sample_dir = str(Path(page).parent) + "/"
        snippets = sorted(x for x in file_map if x.startswith(sample_dir) and x.endswith(".txt"))
        markup_examples = []
        page_source = root / page
        if page_source.exists():
            for element in ET.fromstring(page_source.read_text(encoding="utf-8-sig")).iter():
                if split_xml_name(element.tag)[1] == "ControlExample":
                    definition = element.attrib.get("SampleDefinition")
                    source = "WinUIGallery/Samples/" + definition.replace("\\", "/") if definition else None
                    markup_examples.append({"index": len(markup_examples), "name": element.attrib.get("{" + XAML + "}Name"),
                                            "title": element.attrib.get("HeaderText"), "sample_definition": source,
                                            "sha256": file_map.get(source, {}).get("sha256")})
        examples = []
        for number, example in enumerate(item.get("samples", [])):
            source = example["gallery"].get("source")
            examples.append({"index": number, "title": example.get("header", ""), "source": source,
                             "sha256": file_map.get(source, {}).get("sha256"),
                             "language": example.get("language", "xaml" if "xaml" in example else "unknown"),
                             "catalog_transformations": {k: v for k, v in example["gallery"].items()
                                                         if k not in ("snippet", "source", "name")}})
        requirements = xaml_map.get(page, {})
        pages.append({"id": uid, "catalog_id": item["id"], "title": item["name"],
                      "category": gallery["group"], "description": item.get("description", ""),
                      "page": {"path": page, "sha256": file_map.get(page, {}).get("sha256")},
                      "code_behind": {"path": code_behind, "sha256": file_map.get(code_behind, {}).get("sha256")},
                      "snippets": [{"path": x, "sha256": file_map[x]["sha256"]} for x in snippets],
                      "examples": examples, "markup_examples": markup_examples,
                      "api_namespace": info.get(uid, {}).get("ApiNamespace"),
                      "required_types": sorted(requirements.get("types", {})),
                      "required_members": sorted(requirements.get("members", {})),
                      "markup_features": sorted(requirements.get("features", {})),
                      "docs": item.get("docs", [])})
    if len(pages) != catalog["controlCount"]:
        raise ValueError("Catalog controlCount does not match its controls")
    catalog_ids = {p["id"] for p in pages}
    unlisted = sorted(set(info) - catalog_ids)
    pages_result = {"schema_version": SCHEMA, "catalog": "catalog/windows-samples.json",
                    "catalog_sha256": digest(catalog_path.read_bytes()),
                    "unlisted_control_info_ids": unlisted,
                    "pages": sorted(pages, key=lambda x: x["id"])}
    return pages_result, {"schema_version": SCHEMA, "xaml": xaml, "code": code}, files


def rust_namespace(crate: str, relative: Path, mappings: list[tuple[str, str]]) -> str | None:
    components = list(relative.with_suffix("").parts)
    if components and components[-1] in ("lib", "mod"):
        components.pop()
    module = "::".join([crate, *components])
    choices = [(p, n) for p, n in mappings if module == p or module.startswith(p + "::")]
    return max(choices, key=lambda x: len(x[0]))[1] if choices else None


def collect_rust(repo: Path) -> dict:
    result = {}
    # Per-crate namespace tables are the source of identity, not global short-name
    # matching. Unregistered helpers may be recorded but are not public candidates.
    for manifest in sorted((repo / "src").rglob("Cargo.toml")):
        crate_dir = manifest.parent
        toml = tomllib.loads(manifest.read_text())
        crate = toml.get("lib", {}).get("name", toml["package"]["name"].replace("-", "_"))
        registry_path = crate_dir / "register_types.rs"
        if not registry_path.exists():
            continue
        registry = registry_path.read_text()
        mappings = re.findall(r'\(\s*"([\w:]+)"\s*,\s*"([\w.]+)"\s*\)', registry)
        for path in sorted(crate_dir.rglob("*.rs")):
            relative = path.relative_to(crate_dir)
            if "tests" in relative.parts or path.name.endswith("_tests.rs") or path.name.startswith("compiled_"):
                continue
            # Do not attribute a nested crate's source to the outer crate.
            if any((parent / "Cargo.toml").exists() for parent in path.parents
                   if parent != crate_dir and parent.is_relative_to(crate_dir)):
                continue
            namespace = rust_namespace(crate, relative, mappings)
            if namespace is None:
                continue
            source = path.read_text()
            scanned = scan_rust(str(path), source)
            clean = blank_comments_and_literals(source)
            bases = dict(re.findall(r"\bferro_class!\s*[({]\s*(\w+)\s*:\s*([\w:]+)", clean))
            for name, item in scanned.types.items():
                if not item.defined:
                    continue
                identity = namespace + "." + name
                current = result.setdefault(identity, {"id": identity, "namespace": namespace, "name": name,
                                                       "base": None, "sources": [], "members": set(),
                                                       "registered": False})
                # Registered by class table or by MarkupTyped table. This remains
                # static evidence; runtime and compiled loader tests are separate.
                current["registered"] |= bool(re.search(r"\b" + re.escape(name) + r"\s*(?:::|,)", registry))
                current["sources"].append({"path": path.relative_to(repo).as_posix(), "line": item.line or 1})
                current["base"] = bases.get(name, current["base"])
                current["members"].update(item.fns - item.private_fns)
                current["members"].update(item.variants)
                current["members"].update(item.consts)
                current["members"].update(k for k, public in item.fields.items() if public)
    return result


def snake(name: str) -> str:
    name = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name).lower()


def expected_namespace(namespace: str) -> list[str]:
    if namespace == "Microsoft.UI.Xaml":
        return ["FerroUI", "FerroUI.Controls", "FerroUI.Styling", "FerroUI.Animation"]
    suffix = namespace.removeprefix("Microsoft.UI.Xaml.")
    if suffix.startswith(("Documents", "Shapes")):
        suffix = "Controls." + suffix
    elif suffix == "Input":
        return ["FerroUI.Input", "FerroUI.Interactivity"]
    elif suffix == "Media.Animation":
        return ["FerroUI.Animation", "FerroUI.Animation.Easings"]
    return ["FerroUI." + suffix]


def inherited_member(rust: dict, identity: str, name: str, seen: set[str] | None = None) -> str | None:
    seen = set() if seen is None else seen
    if identity in seen or identity not in rust:
        return None
    seen.add(identity)
    item = rust[identity]
    if name in item["members"]:
        return identity
    base = item["base"]
    if not base:
        return None
    base = base.split("::")[-1]
    candidates = [x for x in rust if rust[x]["name"] == base]
    local = [x for x in candidates if rust[x]["namespace"] == item["namespace"]]
    # Ambiguous inheritance must be resolved explicitly; never pick the first type.
    choices = local if len(local) == 1 else candidates
    if len(choices) != 1:
        return None
    return inherited_member(rust, choices[0], name, seen)


def compare_api(api: dict, rust: dict, mappings: dict) -> dict:
    rows = []
    for upstream in api["types"]:
        mapping = mappings.get("types", {}).get(upstream["id"])
        if mapping:
            target = mapping.get("target")
            candidates = [target] if target in rust else []
            mapping_kind = mapping.get("kind", "reviewed-adaptation")
        else:
            candidates = [ns + "." + upstream["name"] for ns in expected_namespace(upstream["namespace"])
                          if ns + "." + upstream["name"] in rust]
            mapping_kind = "same-name-source-candidate"
        selected = candidates[0] if len(candidates) == 1 else None
        members = []
        for member in upstream["members"]:
            overrides = (mapping or {}).get("members", {})
            mapped = overrides.get(member["name"], snake(member["name"]))
            if member["kind"] == "constructor":
                mapped = "new"
            elif member["kind"] == "enum-value":
                mapped = overrides.get(member["name"], member["name"])
            # Overloads can have different Rust entry points. A type/name-only
            # map must not make SymbolIcon(Symbol) look like the zero-arg new().
            mapped = (mapping or {}).get("overloads", {}).get(member["signature"], mapped)
            declaring = inherited_member(rust, selected, mapped) if selected else None
            identity = upstream["id"] + "::" + member["kind"] + ":" + member["signature"]
            members.append({"id": identity, "name": member["name"], "kind": member["kind"], "signature": member["signature"],
                            "audience": member["audience"], "rust_member": mapped,
                            "declaring_type": declaring,
                            "status": "source-candidate" if declaring else "missing-source-candidate",
                            "behavior": "unverified"})
        rows.append({"upstream": upstream["id"], "audience": upstream["audience"],
                     "candidates": candidates, "mapping_kind": mapping_kind,
                     "status": "source-candidate" if selected else "ambiguous" if candidates else "missing-source-candidate",
                     "registered": bool(selected and rust[selected]["registered"]),
                     "sources": rust[selected]["sources"] if selected else [],
                     "notes": (mapping or {}).get("notes"), "behavior": "unverified", "members": members})
    return {"schema_version": SCHEMA, "meaning": "Source declaration candidates only; no functional parity percentage.", "types": rows}


def upstream_member(api_types: dict, identity: str, member: str, seen: set[str] | None = None) -> dict | None:
    seen = set() if seen is None else seen
    if identity in seen or identity not in api_types:
        return None
    seen.add(identity)
    item = api_types[identity]
    matches = [x for x in item["members"] if x["name"] == member or x.get("projected_name") == member]
    if matches:
        return {"declaring_type": identity, "members": matches}
    base = item["base"]
    if not base:
        return None
    if "." not in base:
        local = item["namespace"] + "." + base
        choices = [local] if local in api_types else [x for x in api_types if api_types[x]["name"] == base]
        if len(choices) != 1:
            return None
        base = choices[0]
    return upstream_member(api_types, base, member, seen)


def compare_xaml(api: dict, usage: dict, rust: dict, api_mapping: dict, mappings: dict) -> dict:
    api_types = {x["id"]: x for x in api["types"]}
    type_mapping = {x["upstream"]: x for x in api_mapping["types"]}
    by_member = defaultdict(list)
    for document in usage["xaml"]:
        for member, count in document["members"].items():
            by_member[member].append({"path": document["path"], "occurrences": count})
    rows = []
    for identity, occurrences in sorted(by_member.items()):
        owner, name = identity.rsplit(".", 1)
        declaration = upstream_member(api_types, owner, name)
        mapping = type_mapping.get(owner)
        target = mapping["candidates"][0] if mapping and len(mapping["candidates"]) == 1 else None
        target_name = mappings.get("types", {}).get(owner, {}).get("members", {}).get(name, snake(name))
        found = inherited_member(rust, target, target_name) if target else None
        if owner.startswith("WinUIGallery."):
            state = "gallery-owned-contract"
        elif not declaration:
            state = "unresolved-upstream-member"
        elif found:
            state = "source-candidate"
        else:
            state = "missing-source-candidate"
        rows.append({"upstream": identity, "declared_by": declaration["declaring_type"] if declaration else None,
                     "target": target, "rust_member": target_name, "target_declared_by": found,
                     "status": state, "behavior": "unverified", "uses": occurrences})
    return {"schema_version": SCHEMA,
            "meaning": "XAML member spellings joined through WinUI and FerroUI inheritance; candidates are not semantic compatibility.",
            "member_statuses": dict(sorted(Counter(x["status"] for x in rows).items())), "members": rows}


def compare_xaml_features(repo: Path, usage: dict, mappings: dict) -> dict:
    uses = defaultdict(list)
    for document in usage["xaml"]:
        for feature, count in document["features"].items():
            uses[feature].append({"path": document["path"], "occurrences": count})
    evidence = mappings.get("xaml_features", {})
    unknown = set(evidence) - set(uses)
    if unknown:
        raise ValueError("Unknown XAML features in api-mappings.json: " + ", ".join(sorted(unknown)))
    rows = []
    for feature, sites in sorted(uses.items()):
        entry = evidence.get(feature, {})
        disposition = entry.get("disposition", "unreviewed")
        if disposition not in ("unreviewed", "shared-syntax-candidate", "implemented-subset", "rejected"):
            raise ValueError(f"Invalid XAML feature disposition: {disposition}")
        for path in entry.get("implementation", []):
            evidence_path(repo, path)
        for test in entry.get("tests", []):
            source = evidence_path(repo, test["path"]).read_text()
            if not re.search(r"\b(?:fn|def)\s+" + re.escape(test["name"]) + r"\b", source):
                raise ValueError(f"Missing XAML feature test anchor: {test}")
        if disposition == "implemented-subset" and not (entry.get("implementation") and entry.get("tests")):
            raise ValueError(f"Implemented XAML feature subset requires source and test anchors: {feature}")
        rows.append({"feature": feature, "disposition": disposition,
                     "occurrences": sum(s["occurrences"] for s in sites), "documents": len(sites),
                     "notes": entry.get("notes"), "implementation": entry.get("implementation", []),
                     "tests": entry.get("tests", []), "uses": sites, "behavior": "unverified"})
    return {"schema_version": SCHEMA, "meaning": "Audited syntax adaptations and explicit rejections, separate from behavior.", "features": rows}


def evidence_path(repo: Path, value: str) -> Path:
    path = (repo / value).resolve()
    if not path.is_relative_to(repo.resolve()) or not path.is_file():
        raise ValueError(f"Invalid or missing evidence file: {value}")
    return path


def verify_execution(repo: Path, evidence: dict, implementation: list[str], tests: list[dict]) -> dict:
    """Validate a durable result record and its exact-source hashes.

    The result producer is trusted like the repository's test harness. A plain
    label, stale record, empty test set, or test-name-only manifest is rejected.
    """
    if not isinstance(evidence, dict) or not evidence.get("record"):
        raise ValueError("Verified evidence requires an execution record path")
    path = evidence_path(repo, evidence["record"])
    record = load(path)
    if record.get("schema_version") != SCHEMA or record.get("result") != "passed" or not record.get("command"):
        raise ValueError(f"Execution record does not describe a successful test run: {path}")
    passed = set(record.get("passed_tests", []))
    if not tests or not passed or any(test["name"] not in passed for test in tests):
        raise ValueError(f"Execution record does not cover the declared tests: {path}")
    hashes = record.get("source_sha256", {})
    required = set(implementation) | {t["path"] for t in tests}
    if not required or not required <= set(hashes):
        raise ValueError(f"Execution record lacks required source hashes: {path}")
    for source, expected in hashes.items():
        if digest(evidence_path(repo, source).read_bytes()) != expected:
            raise ValueError(f"Execution evidence is stale for {source}")
    return {"record": evidence["record"], "sha256": digest(path.read_bytes()), "result": "passed"}


def apply_api_evidence(repo: Path, mapping: dict, manifest: dict) -> None:
    contracts = {m["id"]: m for t in mapping["types"] for m in t["members"]}
    for identity, evidence in manifest.get("members", {}).items():
        if identity not in contracts:
            raise ValueError(f"Unknown API contract in api-evidence.json: {identity}")
        state = evidence.get("status", "partial")
        if state not in ("partial", "verified"):
            raise ValueError(f"Invalid API evidence status: {state}")
        implementations = evidence.get("implementation", [])
        tests = evidence.get("tests", [])
        for path in implementations:
            evidence_path(repo, path)
        for test in tests:
            text = evidence_path(repo, test["path"]).read_text()
            if not re.search(r"\b(?:fn|def)\s+" + re.escape(test["name"]) + r"\b", text):
                raise ValueError(f"Missing API test anchor: {test}")
        if state == "verified":
            if not implementations or not tests:
                raise ValueError("Verified API contract requires implementation and tests")
        if state == "verified" or evidence.get("execution"):
            contracts[identity]["execution"] = verify_execution(repo, evidence.get("execution"), implementations, tests)
        contracts[identity]["behavior"] = state
        contracts[identity]["evidence"] = evidence
    types = {t["upstream"]: t for t in mapping["types"]}
    for identity, evidence in manifest.get("types", {}).items():
        if identity not in types:
            raise ValueError(f"Unknown type in api-evidence.json: {identity}")
        item = types[identity]
        state = evidence.get("status", "partial")
        if state not in ("partial", "verified"):
            raise ValueError(f"Invalid API type evidence status: {state}")
        implementations, tests = evidence.get("implementation", []), evidence.get("tests", [])
        for path in implementations:
            evidence_path(repo, path)
        for test in tests:
            text = evidence_path(repo, test["path"]).read_text()
            if not re.search(r"\b(?:fn|def)\s+" + re.escape(test["name"]) + r"\b", text):
                raise ValueError(f"Missing API type test anchor: {test}")
        if state == "verified":
            if any(m["behavior"] != "verified" for m in item["members"] if m["audience"] != "private"):
                raise ValueError(f"Verified API type has unverified members: {identity}")
            if not implementations or not tests:
                raise ValueError(f"Verified API type requires implementation and tests: {identity}")
        if state == "verified" or evidence.get("execution"):
            item["execution"] = verify_execution(repo, evidence.get("execution"), implementations, tests)
        item["behavior"] = state
        item["evidence"] = evidence


def page_map(repo: Path, pages: dict, api_map: dict, manifest: dict) -> dict:
    entries = manifest.get("pages", {})
    known = {x["id"] for x in pages["pages"]}
    if set(entries) - known:
        raise ValueError("Unknown page IDs in port-status.json: " + ", ".join(sorted(set(entries) - known)))
    by_api = {x["upstream"]: x for x in api_map["types"]}
    rows = []
    for page in pages["pages"]:
        evidence = entries.get(page["id"], {})
        status = evidence.get("status", "pending")
        if status not in ("pending", "partial", "ported", "verified"):
            raise ValueError(f"Invalid page status {page['id']}: {status}")
        implementations = evidence.get("implementation", [])
        tests = evidence.get("tests", [])
        for path in implementations:
            evidence_path(repo, path)
        for test in tests:
            text = evidence_path(repo, test["path"]).read_text()
            if not re.search(r"\b(?:fn|def)\s+" + re.escape(test["name"]) + r"\b", text):
                raise ValueError(f"Missing test anchor: {test}")
        if status in ("ported", "verified") and not implementations:
            raise ValueError(f"{page['id']}: {status} requires implementation evidence")
        examples = evidence.get("examples", [])
        if status == "verified":
            expected = {x["source"] for x in page["examples"]}
            actual = {x["source"] for x in examples if x.get("status") == "verified"}
            if not tests or expected != actual:
                raise ValueError(f"{page['id']}: verified requires tests and all catalog examples")
            expected_markup = {x["index"] for x in page.get("markup_examples", [])}
            actual_markup = {x["index"] for x in evidence.get("markup_examples", []) if x.get("status") == "verified"}
            if expected_markup != actual_markup:
                raise ValueError(f"{page['id']}: verified requires all ControlExample instances, including those absent from the catalog")
            if not evidence.get("execution"):
                raise ValueError(f"{page['id']}: verified requires an execution record, not only test names")
            verify_execution(repo, evidence["execution"], implementations, tests)
        requirements = []
        for name in page["required_types"]:
            mapped = by_api.get(name)
            requirements.append({"type": name, "status": mapped["status"] if mapped else "external-or-unresolved"})
        rows.append({"id": page["id"], "title": page["title"], "category": page["category"]["title"],
                     "status": status, "implementation": implementations, "tests": tests,
                     "examples_total": len(page["examples"]),
                     "examples_verified": sum(x.get("status") == "verified" for x in examples),
                     "notes": evidence.get("notes"), "requirements": requirements})
    return {"schema_version": SCHEMA, "pages": rows}


def summarize(api: dict, pages: dict, usage: dict, mapping: dict, page_mapping: dict) -> dict:
    public = [x for x in mapping["types"] if x["audience"] != "private"]
    public_members = [m for x in public for m in x["members"] if m["audience"] != "private"]
    def behavior_counts(entries: list[dict]) -> dict:
        counts = Counter(entry.get("behavior", "unverified") for entry in entries)
        states = ("verified", "partial", "unverified")
        if set(counts) - set(states):
            raise ValueError("Invalid behavioral parity status in comparison")
        return {state: counts[state] for state in states}

    enum_values = [member for member in public_members if member["kind"] == "enum-value"]
    other_members = [member for member in public_members if member["kind"] != "enum-value"]
    api_kinds = {item["id"]: item["kind"] for item in api.get("types", [])}
    verified_types = [{"id": item["upstream"], "kind": api_kinds.get(item["upstream"], "unknown"),
                       "public_members": sum(member["audience"] != "private" for member in item["members"])}
                      for item in public if item.get("behavior") == "verified"]
    verified_enum_values = [{"type": item["upstream"], "count": sum(
        member["audience"] != "private" and member["kind"] == "enum-value"
        and member.get("behavior") == "verified" for member in item["members"])} for item in public]
    verified_enum_values = [item for item in verified_enum_values if item["count"]]
    imports = sorted({x for entry in usage["code"] for x in entry["imports"]})
    platform = [x for x in imports if x.startswith(("Windows.", "Microsoft.Windows.", "Microsoft.Graphics.",
                                                   "Microsoft.Web.WebView2.", "WinRT.", "CommunityToolkit."))]
    return {
        "schema_version": SCHEMA,
        "upstream": {"types_public_source": len(public), "types_private_source": len(mapping["types"]) - len(public),
                     "members_public_source": len(public_members),
                     "type_candidates": dict(sorted(Counter(x["status"] for x in public).items())),
                     "member_candidates": dict(sorted(Counter(x["status"] for x in public_members).items())),
                     "types_behavior": behavior_counts(public),
                     "members_behavior": behavior_counts(public_members),
                     "enum_values_behavior": behavior_counts(enum_values),
                     "non_enum_members_behavior": behavior_counts(other_members),
                     "verified_types": sorted(verified_types, key=lambda item: item["id"]),
                     "verified_enum_values_by_type": sorted(verified_enum_values, key=lambda item: item["type"])},
        "gallery": {"pages": len(pages["pages"]), "catalog_examples": sum(len(p["examples"]) for p in pages["pages"]),
                    "markup_examples": sum(len(p.get("markup_examples", [])) for p in pages["pages"]),
                    "snippet_files": len({s["path"] for p in pages["pages"] for s in p["snippets"]}),
                    "page_statuses": {state: sum(page["status"] == state for page in page_mapping["pages"])
                                      for state in ("pending", "partial", "ported", "verified")},
                    "xaml_files": len(usage["xaml"]), "code_files": len(usage["code"]),
                    "csharp_member_accesses_requiring_semantic_binding": sum(len(x["member_accesses"]) for x in usage["code"]),
                    "xaml_parse_diagnostics": [d for x in usage["xaml"] for d in x["diagnostics"]],
                    "markup_features": dict(sorted(sum((Counter(x["features"]) for x in usage["xaml"]), Counter()).items())),
                    "unlisted_control_info_ids": pages["unlisted_control_info_ids"]},
        "platform_dependencies": [{"namespace": x, "status": "requires-platform-or-library-adapter"} for x in platform],
        "behavioral_api_parity": "not-certified",
        "limits": api["limitations"] + ["Rust candidates use namespace tables and inherited member names; signatures and defaults are not proven.",
                                         "C# usage is lexical; aliases, receiver types, extension methods and overload resolution are not inferred.",
                                         "Ported pages and reviewed test anchors do not count as executed/verified pages."],
    }


def report(summary: dict, page_mapping: dict, lock: dict) -> str:
    public = summary["upstream"]
    gallery = summary["gallery"]
    lines = ["# WinUI / Gallery source comparison", "", "Generated by `python3 scripts/winui-gallery/scan.py`. Do not edit.", "",
             "**Declaration candidates are not behavioral parity.** The strict completeness gate remains closed until every required contract and Gallery example has execution evidence.", "",
             "## Pinned sources", ""]
    for key, item in sorted(lock["repositories"].items()):
        lines.append(f"- {key}: [{item['commit'][:12]}]({item['url']}/tree/{item['commit']})")
    lines.extend(["", "## Source accounting", "", "| Measure | Count |", "|---|---:|",
                  f"| Public-source WinUI type declarations | {public['types_public_source']} |",
                  f"| Public-source WinUI member declarations | {public['members_public_source']} |",
                  f"| Private-source types (separate) | {public['types_private_source']} |",
                  f"| Gallery catalog pages | {gallery['pages']} |", f"| Catalog examples | {gallery['catalog_examples']} |",
                  f"| ControlExample instances in catalog pages | {gallery['markup_examples']} |",
                  f"| Page-directory snippet files | {gallery['snippet_files']} |",
                  f"| Gallery XAML files scanned | {gallery['xaml_files']} |", f"| Gallery C# files scanned | {gallery['code_files']} |",
                  "", "## Candidate comparison", "", "| Level | Source candidate | Missing candidate |", "|---|---:|---:|",
                  f"| Types | {public['type_candidates'].get('source-candidate', 0)} | {public['type_candidates'].get('missing-source-candidate', 0)} |",
                  f"| Members | {public['member_candidates'].get('source-candidate', 0)} | {public['member_candidates'].get('missing-source-candidate', 0)} |",
                  "", "Candidates use namespace-scoped declarations and base-class lookup. A same-name method can still differ in type, lifetime, defaults, scheduling, errors, or rendering.", "",
                  "## Behavioral contract evidence", "",
                  "Only explicit evidence changes these states. Public-source declarations are counted; private types and private members are excluded. A declaration candidate remains unverified until its own contract evidence is supplied.", "",
                  "| Public-source scope | Verified | Partial | Unverified |", "|---|---:|---:|---:|"])
    for label, key in [("Type contracts", "types_behavior"), ("All member contracts", "members_behavior"),
                       ("Enum values only", "enum_values_behavior"), ("Members excluding enum values", "non_enum_members_behavior")]:
        counts = public[key]
        lines.append(f"| {label} | {counts['verified']} | {counts['partial']} | {counts['unverified']} |")
    lines.extend(["", "Enum values are also included in the all-member total; they are shown separately so numeric enum coverage cannot be mistaken for control, XAML or Gallery behavior.", ""])
    if public["verified_enum_values_by_type"]:
        lines.extend(["Verified enum-value scope:", ""])
        lines.extend(f"- `{item['type']}`: {item['count']} exact public values."
                     for item in public["verified_enum_values_by_type"])
        lines.append("")
    if public["verified_types"]:
        lines.extend(["Types with complete recorded evidence for their public source declarations:", ""])
        lines.extend(f"- `{item['id']}` ({item['kind']}; {item['public_members']} public member declarations)."
                     for item in public["verified_types"])
        lines.append("")
    else:
        lines.extend(["No complete type contract has been verified.", ""])
    lines.extend(["A verified enum does not verify its consuming controls, installed fonts, inherited platform APIs or rendered appearance. Partial controls retain the boundaries documented in `CONTROL-CONTRACTS.md` and their per-signature evidence.", "",
                  "## Gallery port state", "", "| Page state | Count |", "|---|---:|"])
    for state in ("pending", "partial", "ported", "verified"):
        lines.append(f"| {state} | {gallery['page_statuses'][state]} |")
    lines.extend(["", "Partial pages exercise reviewed native subsets. Ported pages still require execution evidence; only verified pages satisfy the completeness gate.", "",
                  "| Page | Category | State | Catalog examples |", "|---|---|---|---:|"])
    for page in page_mapping["pages"]:
        lines.append(f"| {page['id']} | {page['category']} | {page['status']} | {page['examples_total']} |")
    lines.extend(["", "## Platform and external dependencies", "", "These imported namespaces require separate adapters or ports; an import is not an implemented feature.", ""])
    lines.extend(f"- `{x['namespace']}`" for x in summary["platform_dependencies"])
    lines.extend(["", "## Scanner limits", ""])
    lines.extend(f"- {x}" for x in summary["limits"])
    if gallery["xaml_parse_diagnostics"]:
        lines.extend(["", "## Parse diagnostics", ""])
        lines.extend(f"- `{d['path']}`: {d['detail']}" for d in gallery["xaml_parse_diagnostics"])
    return "\n".join(lines) + "\n"


def refresh(args: argparse.Namespace, lock: dict) -> None:
    if not args.winui_root or not args.gallery_root:
        raise ValueError("--refresh-reference requires --winui-root and --gallery-root")
    winui = Path(args.winui_root).resolve()
    gallery = Path(args.gallery_root).resolve()
    verify_checkout(winui, lock["repositories"]["winui"]["commit"])
    verify_checkout(gallery, lock["repositories"]["gallery"]["commit"])
    api, api_files = collect_api(winui)
    pages, usage, gallery_files = collect_gallery(gallery, api)
    values = {"upstream-api.json": api, "gallery-pages.json": pages, "gallery-usage.json": usage,
              "source-files.json": {"schema_version": SCHEMA, "files": sorted(api_files + gallery_files, key=lambda x: (x["repository"], x["path"]))}}
    for name, value in values.items():
        (DATA / name).write_text(canonical(value), encoding="utf-8")
    lock["snapshots"] = {name: digest((DATA / name).read_bytes()) for name in SNAPSHOTS}
    (DATA / "sources.lock.json").write_text(canonical(lock), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--refresh-reference", action="store_true", help="Regenerate pinned upstream snapshots from clean checkouts")
    parser.add_argument("--winui-root")
    parser.add_argument("--gallery-root")
    parser.add_argument("--check", action="store_true", help="Offline: verify snapshot hashes and generated reports without writing")
    parser.add_argument("--require-complete", action="store_true", help="Fail unless all API contracts and all Gallery pages are verified")
    args = parser.parse_args()
    try:
        if args.check and args.refresh_reference:
            raise ValueError("--check and --refresh-reference are mutually exclusive")
        lock = load(DATA / "sources.lock.json")
        if args.refresh_reference:
            refresh(args, lock)
        for name in SNAPSHOTS:
            if digest((DATA / name).read_bytes()) != lock["snapshots"].get(name):
                raise ValueError(f"Pinned snapshot changed: {name}; refresh from the pinned clean checkouts")
        api = load(DATA / "upstream-api.json")
        pages = load(DATA / "gallery-pages.json")
        usage = load(DATA / "gallery-usage.json")
        rust = collect_rust(REPO)
        mappings = load(DATA / "api-mappings.json")
        mapping = compare_api(api, rust, mappings)
        api_evidence_file = DATA / "api-evidence.json"
        apply_api_evidence(REPO, mapping, load(api_evidence_file) if api_evidence_file.exists() else {"members": {}})
        xaml_mapping = compare_xaml(api, usage, rust, mapping, mappings)
        xaml_features = compare_xaml_features(REPO, usage, mappings)
        evidence_file = REPO / "samples/WinUIGallery/port-status.json"
        evidence = load(evidence_file) if evidence_file.exists() else {"pages": {}}
        page_mapping = page_map(REPO, pages, mapping, evidence)
        summary = summarize(api, pages, usage, mapping, page_mapping)
        outputs = {"api-map.json": canonical(mapping), "gallery-page-map.json": canonical(page_mapping),
                   "gallery-xaml-map.json": canonical(xaml_mapping),
                   "xaml-feature-map.json": canonical(xaml_features),
                   "summary.json": canonical(summary), "REPORT.md": report(summary, page_mapping, lock)}
        stale = []
        for name, contents in outputs.items():
            destination = DATA / name
            if args.check:
                if not destination.exists() or destination.read_text(encoding="utf-8") != contents:
                    stale.append(name)
            else:
                destination.write_text(contents, encoding="utf-8")
        print(json.dumps({"upstream": summary["upstream"], "gallery": {k: v for k, v in summary["gallery"].items()
                                                                      if k not in ("markup_features", "xaml_parse_diagnostics")}}, sort_keys=True))
        if stale:
            print("Stale generated reports: " + ", ".join(stale), file=sys.stderr)
            return 1
        if args.require_complete:
            types_unverified = sum(t["audience"] != "private" and t["behavior"] != "verified" for t in mapping["types"])
            api_unverified = sum(1 for t in mapping["types"] if t["audience"] != "private"
                                 for m in t["members"] if m["audience"] != "private" and m["behavior"] != "verified")
            pages_unverified = sum(p["status"] != "verified" for p in page_mapping["pages"])
            if types_unverified or api_unverified or pages_unverified:
                print(f"Completeness gate: {types_unverified} API types, {api_unverified} API members and {pages_unverified} Gallery pages lack behavioral execution evidence.", file=sys.stderr)
                return 2
        return 0
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"WinUI/Gallery scan failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
