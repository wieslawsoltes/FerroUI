"""Inventory the document and deferred factory bodies emitted by one or more Build::run invocations."""
import argparse
import csv
import gzip
import hashlib
import io
import json
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
import re


MODULE = re.compile(r"^/// The compiled documents of the group `(\w+)`\.$", re.M)
DOCUMENT = re.compile(r"/// Generated from `([^`]+)`\.")
DEFERRED = re.compile(r"/// Builds the deferred content at `(.+)\((\d+),(\d+)\)` \(a template or a deferred resource\)\.")
PART = re.compile(r"/// Part (\d+) of `(\w+)` \(`(.+)`\)\.")
FUNCTION = re.compile(r"(?:pub(?:\(crate\))? )?fn (\w+)\(")
FACTORY_OR_PART = re.compile(r"^(?:pub(?:\(crate\))? )?fn (\w+_(?:deferred|part)_\d+)\(", re.M)
PART_CALL = re.compile(r"(\w+_part_\d+)\((?:\w+\.clone\(\)(?:, )?)*\)\?;")
STYLE_CALL = re.compile(r"(\w+)\.build\(&context\)")
STYLE_TABLE = re.compile(r"^static (\w+): rt::TypedStyleProgram =", re.M)
RESOURCE_TABLE = re.compile(r"^static (\w+): rt::TypedResourceProgram<", re.M)
RESOURCE_CALL = re.compile(
    r"(\w+)\.(add_(?:(?:color|corner_radius|double|string|thickness)_resources|"
    r"resource_aliases|(?:dynamic|literal|static)_brushes))\(&\w+(?:, &context, &\w+)?\)\?;"
)
IMAGE = r"&const \{ rt::program_bytes::<\d+>\(__FERROUI_XAML_IMAGE, \d+\) \}"
IMPORTS = r"(?:&CONSTRUCTION_IMPORTS|&rt::ConstructionImports::EMPTY)"
IMPORT_TABLE = re.compile(r"const CONSTRUCTION_IMPORTS: rt::ConstructionImports = rt::ConstructionImports \{ operations: &\[.*\], strings: &\[.*\] \};")
DOCUMENT_PROGRAM = re.compile(
    rf"static PROGRAM: rt::ObjectDocumentProgram = rt::ObjectDocumentProgram \{{ document: &\w+, "
    rf"construction: rt::ConstructionProgram \{{ code: {IMAGE}, imports: {IMPORTS} \}}, "
    r"root_value: \|root\| Ok\(rt::to_value\(rt::construction_object::<[\w:]+>\(Some\(root.clone\(\)\)\)\?\)\) \};"
)
INPUT = r"rt::ConstructionInput::(?:Value\(&\w+\)|Object\(\w+\.upcast_ref::<::ferroui_base::FerroObject>\(\)\))"
PROGRAM = re.compile(
    rf"(?:let (?:mut construction_results|_construction_objects) = )?rt::ConstructionProgram "
    rf"\{{ code: {IMAGE}, imports: {IMPORTS} \}}\.(?:apply|hold|results)\("
    rf"&\[(?:{INPUT}(?:, {INPUT})*)?\], (?:None|name_scope\.as_ref\(\)), (?:None|Some\(&context\))\)\?;"
)
SIMPLE = r"(?:\w+(?:\.clone\(\))?|::core::clone::Clone::clone\(&\w+\))"
GLUE = [
    re.compile(r"let root = root\.clone\(\);"),
    re.compile(r"let context = rt::populate_context\(service_provider, &\w+, rt::to_value\(\w+\.clone\(\)\)\);"),
    re.compile(r"let _?context = rt::deferred_context\(service_provider, &(?:\w+|rt::DocumentInfo \{ base_uri: \w+\.base_uri, namespaces: \w+ \})\);"),
    re.compile(r"let name_scope = context\.name_scope_field\(\);"),
    re.compile(r"let \((?:\w+, ?)+\) = \{ let mut created = construction_results\.reader\(\); \((?:created\.(?:hold\(\d+\)|next_(?:object|value)::<[^;{}]+?>\(\))\?, ?)+\) \};"),
    re.compile(rf"::core::result::Result::Ok\((?:\(\)|{SIMPLE}|rt::(?:to_value|to_object)\({SIMPLE}\))\)"),
]
FIELDS = ("arm", "module", "kind", "document", "xaml_site", "function", "classification",
          "construction_regions", "parts", "helpers", "first_residual_function", "generated_line", "first_residual")


@dataclass
class Body:
    name: str
    kind: str
    document: str
    site: str
    lines: list


def bodies(source):
    lines = source.splitlines()
    result = {}
    for index, line in enumerate(lines):
        document = DOCUMENT.fullmatch(line)
        deferred = DEFERRED.fullmatch(line)
        part = PART.fullmatch(line)
        if not (document or deferred or part):
            continue
        declaration = FUNCTION.match(lines[index + 1])
        if not declaration:
            raise ValueError(f"missing function after {line}")
        name = declaration[1]
        if document:
            if name.startswith("populate_"):
                kind = "class"
            elif name.startswith("build_"):
                kind = "build"
            else:
                raise ValueError(f"unrecognized document function {name}")
            path, site = document[1], ""
        elif deferred:
            kind, path, site = "factory", deferred[1], f"{deferred[2]},{deferred[3]}"
        else:
            kind, path, site = "part", part[3], ""
        start = next(i for i in range(index + 1, len(lines)) if lines[i].endswith(" {"))
        end = lines.index("}", start + 1)  # Nested generated Rust blocks are indented.
        if name in result:
            raise ValueError(f"duplicate function {name}")
        result[name] = Body(name, kind, path, site, list(enumerate(lines[start + 1:end], start + 2)))
    if missed := set(FACTORY_OR_PART.findall(source)) - result.keys():
        raise ValueError(f"unaccounted generated functions: {sorted(missed)}")
    return result


def inventory_module(arm, path, source):
    functions = bodies(source)
    styles = set(STYLE_TABLE.findall(source))
    resources = set(RESOURCE_TABLE.findall(source))
    reached_parts = set()

    def operations(function):
        """Follow split bodies in call order and preserve the location of each native residual."""
        for number, line in function.lines:
            text = line.strip()
            call = PART_CALL.fullmatch(text)
            if call and call[1] in functions and functions[call[1]].kind == "part":
                reached_parts.add(call[1])
                yield "parts", call[1], None
                yield from operations(functions[call[1]])
            elif not text or text.startswith("//") or IMPORT_TABLE.fullmatch(text):
                continue
            elif DOCUMENT_PROGRAM.fullmatch(text):
                yield "document_descriptor", 1, None
            elif text == "PROGRAM.populate(service_provider, root.upcast_ref::<::ferroui_base::FerroObject>())":
                yield "document_call", 1, None
            elif PROGRAM.fullmatch(text):
                yield "region", 1, None
            elif any(pattern.fullmatch(text) for pattern in GLUE):
                continue
            elif (style := STYLE_CALL.fullmatch(text)) and style[1] in styles:
                yield "helper", "TypedStyleProgram.build", None
            elif (resource := RESOURCE_CALL.fullmatch(text)) and resource[1] in resources:
                yield "helper", f"TypedResourceProgram.{resource[2]}", None
            elif re.fullmatch(r"rt::build_control_theme\(&context, &\w+\)", text):
                yield "helper", "build_control_theme", None
            elif re.fullmatch(r'rt::static_resource_content\(service_provider, &\w+, "(?:\\.|[^"\\])*", \d+, \d+\)', text):
                yield "helper", "static_resource_content", None
            else:
                yield "residual", function.name, (number, text)

    for body in functions.values():
        if body.kind == "part":
            continue
        counts = Counter()
        helpers = []
        first = ("", "", "")
        for category, entry, location in operations(body):
            counts[category] += 1
            if category == "helper":
                helpers.append(entry)
            if category == "residual" and not first[0]:
                first = (entry, *location)
        if counts["residual"]:
            has_program = counts["document_descriptor"] or counts["region"] or helpers
            classification = "mixed-residual" if has_program else "native-residual"
        elif counts["document_descriptor"] == counts["document_call"] == 1:
            classification = "whole-document"
        elif helpers:
            classification = "helper-with-regions" if counts["region"] else "shared-helper"
        elif counts["region"]:
            classification = "regions-and-glue"
        else:
            classification = "glue-only"
        if counts["document_descriptor"] != counts["document_call"]:
            raise ValueError(f"unmatched document descriptor in {path.name}:{body.name}")
        yield (arm, path.name, body.kind, body.document, body.site, body.name, classification,
               counts["region"], counts["parts"], ",".join(dict.fromkeys(helpers)), *first)
    if missed := {name for name, body in functions.items() if body.kind == "part"} - reached_parts:
        raise ValueError(f"unaccounted generated parts in {path.name}: {sorted(missed)}")


def inventory(arm, directory):
    counts = {}
    factory_sites = defaultdict(set)
    digest = hashlib.sha256()
    rows = []
    # The Cargo output directory can retain obsolete groups after a rebuild.
    # mod.rs lists the groups in this build and contains machine-local absolute paths.
    manifest = (directory / "mod.rs").read_text(encoding="utf-8")
    modules = sorted(MODULE.findall(manifest))
    if len(set(modules)) != len(modules):
        raise ValueError(f"duplicate module in {directory / 'mod.rs'}")
    for module in modules:
        path = directory / f"{module}.rs"
        raw = path.read_bytes()
        digest.update(path.name.encode() + b"\0" + hashlib.sha256(raw).digest())
        for row in inventory_module(arm, path, raw.decode("utf-8")):
            counts.setdefault(row[2], Counter())[row[6]] += 1
            if row[2] == "factory":
                factory_sites[row[3], row[4]].add(row[6])
            rows.append(row)
    if not rows:
        raise ValueError(f"no generated XAML documents or factories in {directory}")
    # Report disagreements between copies explicitly if lowering depends on their module.
    site_counts = Counter(" / ".join(sorted(classes)) for classes in factory_sites.values())
    return rows, {"modules": len(modules), "module_sources_sha256": digest.hexdigest(),
                  "counts": counts, "distinct_factory_sites": site_counts}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", action="append", required=True, metavar="ARM=DIRECTORY")
    parser.add_argument("--output", type=Path, required=True, help="inventory TSV or .tsv.gz")
    args = parser.parse_args()
    table = io.StringIO(newline="")
    writer = csv.writer(table, delimiter="\t", lineterminator="\n")
    writer.writerow(FIELDS)
    summary = {}
    for item in args.input:
        arm, directory = item.split("=", 1)
        if not arm or arm in summary:
            parser.error("each input needs a unique nonempty arm name")
        rows, summary[arm] = inventory(arm, Path(directory))
        writer.writerows(rows)
    data = table.getvalue().encode("utf-8")
    args.output.write_bytes(gzip.compress(data, mtime=0) if args.output.suffix == ".gz" else data)
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
