#!/usr/bin/env python3
"""Counts the rows of the resource tables of generated theme files (#855).

For each generated file (`compiled_xaml.rs` of a theme crate, or the `$OUT_DIR/xaml` copy of a
build) it prints the row tables by shape: the number of tables and rows, and the bytes the rows
take as the present slices of row structs on a 64-bit target (the sizes of the structs, without
the strings they point to, which both forms keep). For packed tables (`rt::PackedRows`) it
prints the records, the pools of strings and the namespace tables.

With `--parts <theme_parts.txt>` (the units an application keeps, from its `$OUT_DIR/xaml`), it
counts only the tables that the runs of the kept units call, and the tables outside every unit:
those the function of a document calls directly (ambient), and the tables of template bindings,
which are called from the functions of templates and are not attributed to a unit here.

What the linked executable keeps is measured on the file itself (`size -m`, the `__const` and
`__data_const` sections); this count says which rows are there to be packed.
"""
import argparse
import collections
import re
import sys

# The size of each row struct on a 64-bit target: sized strings (16), optional namespace slices
# (16), 32-bit positions, values.
ROW_BYTES = {
    'rt::ResourceAliasRow': 64,
    'rt::LiteralBrushRow': 48,
    'rt::DynamicBrushRow': 64,
    'rt::StaticBrushRow': 64,
    'rt::ValueRow<f64>': 32,
    'rt::ValueRow<[f64; 4]>': 56,
    'rt::ValueRow<u32>': 24,
    "rt::ValueRow<&'static str>": 40,
    'rt::TemplateBindingRow': 24,
}

TABLE = re.compile(r'^static (\w+): &\[(rt::[^\]]+?)\] = &\[$')
PACKED = re.compile(r'^static (\w+): rt::PackedRows = rt::PackedRows \{$')
FUNCTION = re.compile(r'^(?:pub )?fn (\w+)')
UNIT = re.compile(r'^pub static THEME_UNIT_(\d+): rt::ThemeUnit = rt::ThemeUnit \{ name: "((?:[^"\\]|\\.)*)", index: \d+, runs: &\[(.*)\] \};$')
CALL = re.compile(r'rt::add_\w+\(&\w+, (?:&context, &\w+, )?&?(\w+_ROWS_\d+)\)\?;')


def kept_names(path):
    with open(path, encoding='utf-8') as file:
        return {line.strip() for line in file if line.strip() and not re.search(r': \d+ of \d+ units$', line.strip())}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('files', nargs='+')
    parser.add_argument('--parts', help="an application's theme_parts.txt")
    args = parser.parse_args()
    names = kept_names(args.parts) if args.parts else None
    for path in args.files:
        with open(path, encoding='utf-8') as file:
            lines = file.read().split('\n')
        tables = {}
        calls = collections.defaultdict(set)
        unit_runs = {}
        current_function = None
        index = 0
        while index < len(lines):
            line = lines[index]
            function = FUNCTION.match(line)
            if function:
                current_function = function.group(1)
            for call in CALL.finditer(line):
                calls[call.group(1)].add(current_function)
            unit = UNIT.match(line)
            if unit:
                runs = [run.strip() for run in unit.group(3).split(',') if run.strip()]
                for run in runs:
                    unit_runs[run] = unit.group(2)
            table = TABLE.match(line)
            packed = PACKED.match(line)
            if table:
                rows = 0
                index += 1
                while lines[index] != '];':
                    rows += 1
                    index += 1
                tables[table.group(1)] = (table.group(2), rows, 0, 0)
            elif packed:
                strings = records = 0
                index += 1
                while lines[index] != '};':
                    text = lines[index].strip()
                    if text.startswith('strings: '):
                        # The length of the literal, escapes counted as written: close enough.
                        strings = len(text[len('strings: '):-1]) - 2
                    elif re.match(r'^\d', text):
                        fields = len(re.findall(r'\d+', text))
                        records += 1
                        strings += 2 * fields
                    index += 1
                tables[packed.group(1)] = ('rt::PackedRows', records, strings, 0)
            index += 1
        by_kind = collections.Counter()
        rows_of = collections.Counter()
        bytes_of = collections.Counter()
        outside = 0
        for name, (kind, rows, strings, _) in tables.items():
            functions = calls.get(name, set())
            units = {unit_runs.get(function) for function in functions}
            if names is not None:
                in_unit = {unit for unit in units if unit is not None}
                if in_unit and not in_unit & names:
                    continue
                if not in_unit:
                    outside += 1
            by_kind[kind] += 1
            rows_of[kind] += rows
            bytes_of[kind] += rows * ROW_BYTES.get(kind, 0) + (strings if kind == 'rt::PackedRows' else 0)
        print(f'# {path}' + (' (kept units only)' if names is not None else ''))
        for kind in sorted(by_kind):
            note = 'bytes of the records and the pools' if kind == 'rt::PackedRows' else 'bytes of the row structs'
            print(f'{kind:34s} {by_kind[kind]:6d} tables {rows_of[kind]:7d} rows {bytes_of[kind]:9d} {note}')
        print(f'total {sum(by_kind.values())} tables, {sum(rows_of.values())} rows'
              + (f', {outside} tables outside every unit' if names is not None else ''))
    return 0


if __name__ == '__main__':
    sys.exit(main())
