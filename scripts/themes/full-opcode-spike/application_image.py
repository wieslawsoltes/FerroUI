"""Count compiler-generated XAML instruction images for one Cargo executable's dependency graph."""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import re


INCLUDE = re.compile(r'const __FERROUI_XAML_IMAGE: &\[u8\] = include_bytes!\(("(?:\\.|[^"])*")\);')
SLICE = re.compile(r'rt::program_bytes::<(\d+)>\(__FERROUI_XAML_IMAGE, (\d+)\)')
PROGRAM = re.compile(r'rt::(ConstructionProgram|BindingPathProgram|TypedSetterChoice)\s*\{\s*code:\s*&const \{\s*$|'
                     r'rt::(TypedResourceProgram|TypedStyleProgram|TypedSelectorProgram)::new\(&const \{\s*$|'
                     r'rt::(apply_theme_run)\(&const \{\s*$')
IMPORT_TABLE = re.compile(r'^ *const CONSTRUCTION_IMPORTS: rt::ConstructionImports = rt::ConstructionImports \{ operations: &\[(.*)\] \};$', re.M)
IMPORT_TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|rt::ConstructionImport::\w+\(|\], strings: &\[')
ESCAPE = re.compile(r'\\(?:u\{([0-9a-fA-F]+)\}|(.))')


def construction_imports(text, counts, literals):
    """Count source-level factory imports before linker folding and dead stripping."""
    escapes = {'"': '"', '\\': '\\', 'n': '\n', 'r': '\r', 't': '\t', '0': '\0'}
    operations = set()
    for table in IMPORT_TABLE.finditer(text):
        counts['construction_import_tables'] += 1
        strings = False
        start = None
        for match in IMPORT_TOKEN.finditer(table[1]):
            token = match[0]
            if start is not None and (token.startswith('rt::ConstructionImport::') or token == '], strings: &['):
                operations.add(table[1][start:match.start()].rstrip(', '))
            if token == '], strings: &[':
                start = None
                strings = True
            elif token.startswith('rt::ConstructionImport::') and not strings:
                start = match.start()
                counts['construction_operation_entries'] += 1
                if token == 'rt::ConstructionImport::ObjectLiteral(':
                    counts['construction_object_literal_imports'] += 1
                if token == 'rt::ConstructionImport::RegisterSetter(':
                    counts['construction_register_setter_imports'] += 1
            elif strings and token.startswith('"'):
                value = ESCAPE.sub(lambda m: chr(int(m[1], 16)) if m[1] else escapes[m[2]], token[1:-1])
                literals.add(value)
                counts['construction_string_entries'] += 1
        if not strings:
            raise ValueError("Missing string slice in generated construction import table")
    counts['distinct_operation_initializers_within_modules'] += len(operations)


def measure(depfile):
    grouped = defaultdict(Counter)
    modules = Counter()
    construction_strings = defaultdict(set)
    # Cargo escapes spaces in dep-info paths. Other backslashes can be Windows
    # directory separators and must survive tokenization.
    depinfo = depfile.read_text(encoding="utf-8").replace("\\\n", " ")
    entries = re.findall(r"(?:\\ |[^\s])+", depinfo)
    for entry in sorted({entry.replace(r"\ ", " ") for entry in entries}):
        source = Path(entry)
        if source.suffix != ".rs" or source.parent.name != "xaml" or source.parent.parent.name != "out":
            continue
        package = source.parents[2].name.rsplit("-", 1)[0]
        text = source.read_text(encoding="utf-8")
        modules[package] += 1
        # The emitter marks each document function. A trimmable theme also exposes
        # a public populate_with_parts wrapper that selects its units.
        # A whole-document descriptor is emitted only if every statement of the
        # population body lowered and all external operands refer to its root.
        counts = grouped[package]
        construction_imports(text, counts, construction_strings[package])
        counts["class_document_methods"] += len(re.findall(r"^/// Generated from `[^\n]*`\.\npub fn populate_\w+\(", text, re.M))
        counts["whole_class_document_programs"] += text.count("static PROGRAM: rt::ObjectDocumentProgram")
        include = INCLUDE.search(text)
        if not include:
            continue
        image = (source.parent / json.loads(include[1])).read_bytes()
        counts["modules_with_image"] += 1
        counts["generated_image_bytes"] += len(image)
        intervals = set()
        for item in SLICE.finditer(text):
            length, offset = map(int, item.groups())
            if offset + length > len(image):
                raise ValueError(f"Invalid program slice at {source}:{offset}+{length}")
            prefix = text[max(0, item.start() - 100):item.start()]
            match = PROGRAM.search(prefix)
            kind = match[match.lastindex] if match else "Other"
            counts[kind + "_references"] += 1
            counts[kind + "_referenced_bytes"] += length
            intervals.add((offset, offset + length))
        end = covered = 0
        for a, b in sorted(intervals):
            covered += max(0, b - max(a, end))
            end = max(b, end)
        counts["source_referenced_unique_bytes"] += covered
        counts["unreferenced_generated_bytes"] += len(image) - covered
    if not modules:
        raise ValueError("No generated XAML modules in this Cargo dependency file")
    for package, count in modules.items():
        grouped[package]["generated_modules"] = count
    totals = sum(grouped.values(), Counter())
    for package, counts in grouped.items():
        counts['distinct_construction_string_utf8_bytes'] = sum(len(s.encode()) for s in construction_strings[package])
    unique = set().union(*construction_strings.values())
    totals['distinct_construction_string_utf8_bytes'] = sum(len(s.encode()) for s in unique)
    return {"packages": dict(sorted(grouped.items())), "total": totals}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cargo_depfile", type=Path)
    args = parser.parse_args()
    print(json.dumps(measure(args.cargo_depfile), indent=2))
