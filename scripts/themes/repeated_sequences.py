#!/usr/bin/env python3
"""Counts repeated sequences in the deferred and value functions of generated themes (#854).

Two modes:

- `source <compiled_xaml.rs>...`: the statements of every function of the generated files whose
  name ends in `_deferred_<n>` (template content and deferred values), normalised (locals,
  positions, numbers and string literals replaced by placeholders, comments dropped), and the
  windows of `--window` consecutive statements that occur more than once: how many functions
  and statements they cover and the most frequent ones. A window is counted once for each place
  it starts at, and the statements covered are counted once however many windows cover them.
  This says which shapes repeat in the source; it does not say what the compiler made of them.
- `binary <unstripped executable>`: the instructions of the functions whose demangled names
  match `--grep` (default: the deferred functions of the theme crates), disassembled with
  `objdump`, normalised (addresses, immediates and symbol operands replaced), and the windows of
  `--window` consecutive instructions that occur more than once, with the bytes they cover (4
  bytes an instruction on arm64; on x86-64 the bytes of the instructions as listed). This is the
  count the issue asks for: the linked output, not the source.

Examples:

    python3 -I scripts/themes/repeated_sequences.py source src/FerroUI.Themes.WinUI/compiled_xaml.rs
    python3 -I scripts/themes/repeated_sequences.py binary <target>/dist/winui-gallery --window 16
"""
import argparse
import collections
import re
import subprocess
import sys

FUNCTION = re.compile(r'^(?:pub )?fn (\w+)\(')
DEFERRED = re.compile(r'_deferred_\d+$')
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')
LOCAL = re.compile(r'\b([a-z][a-z0-9]*(?:_[a-z0-9]+)*)_\d+\b')
NUMBER = re.compile(r'\b\d+(?:\.\d+)?(?:_(?:f64|f32|i32|u32|u8|usize|i64|u64))?\b')


def normalise_statement(text):
    text = STRING.sub('S', text)
    text = LOCAL.sub(lambda match: match.group(1) + '_N', text)
    text = NUMBER.sub('K', text)
    return text


def source_functions(paths):
    functions = {}
    for path in paths:
        with open(path, encoding='utf-8') as file:
            lines = file.read().split('\n')
        name = None
        body = []
        depth = 0
        for line in lines:
            match = FUNCTION.match(line)
            if match and depth == 0:
                name = match.group(1)
                body = []
                depth = 0
            if name is None:
                continue
            depth += line.count('{') - line.count('}')
            text = line.strip()
            if text and not text.startswith('//') and not FUNCTION.match(line):
                body.append(normalise_statement(text))
            if depth == 0 and line.startswith('}'):
                if DEFERRED.search(name):
                    functions[f'{path}:{name}'] = body[:-1]
                name = None
    return functions


def windows(functions, size):
    counts = collections.Counter()
    places = collections.defaultdict(list)
    for name, items in functions.items():
        for start in range(0, max(0, len(items) - size + 1)):
            key = tuple(items[start:start + size])
            counts[key] += 1
            places[key].append((name, start))
    return counts, places


def report(functions, size, weight, unit, top):
    counts, places = windows(functions, size)
    total = sum(weight(item) for items in functions.values() for item in items)
    covered = collections.defaultdict(set)
    for key, count in counts.items():
        if count < 2:
            continue
        for name, start in places[key]:
            covered[name].update(range(start, start + size))
    covered_weight = sum(weight(functions[name][index]) for name, indices in covered.items() for index in indices)
    repeated = sum(1 for count in counts.values() if count > 1)
    print(f'functions: {len(functions)}; {unit}: {total}; windows of {size}: {sum(counts.values())}, '
          f'{repeated} distinct repeated')
    print(f'{unit} inside a repeated window: {covered_weight} ({100 * covered_weight / max(total, 1):.1f}%), '
          f'in {len(covered)} functions')
    ranked = sorted(((count, key) for key, count in counts.items() if count > 1), key=lambda pair: (-pair[0] * sum(map(weight, pair[1])), pair[1]))
    for count, key in ranked[:top]:
        functions_with = len({name for name, _ in places[key]})
        print(f'\n{count} times in {functions_with} functions, {sum(map(weight, key))} {unit} each:')
        for item in key:
            print(f'    {item[:200]}')


def binary_functions(path, pattern):
    output = subprocess.run(['objdump', '--disassemble', '--demangle', '--no-show-raw-insn', path],
                            capture_output=True, text=True, check=True).stdout
    header = re.compile(r'^[0-9a-f]+ <(.*)>:$')
    instruction = re.compile(r'^\s*([0-9a-f]+):\s+(.*)$')
    functions = {}
    name = None
    for line in output.split('\n'):
        match = header.match(line)
        if match:
            name = match.group(1) if re.search(pattern, match.group(1)) else None
            if name is not None:
                functions[name] = []
            continue
        if name is None:
            continue
        match = instruction.match(line)
        if not match:
            continue
        text = match.group(2)
        text = re.sub(r'<[^>]*>', '<sym>', text)
        text = re.sub(r'\b0x[0-9a-f]+\b', 'IMM', text)
        text = re.sub(r'#-?\d+', '#IMM', text)
        text = re.sub(r'\b[0-9a-f]{5,}\b', 'ADDR', text)
        functions[name].append((int(match.group(1), 16), text.split(';')[0].strip()))
    # Sizes: the distance to the next instruction (the last one of a function counts 4 bytes).
    sized = {}
    for name, items in functions.items():
        rows = []
        for index, (address, text) in enumerate(items):
            size = items[index + 1][0] - address if index + 1 < len(items) else 4
            rows.append((text, max(size, 1)))
        sized[name] = rows
    return sized


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('mode', choices=['source', 'binary'])
    parser.add_argument('paths', nargs='+')
    parser.add_argument('--window', type=int, default=None, help='statements (source, default 4) or instructions (binary, default 12)')
    parser.add_argument('--grep', default=r'ferroui_themes_\w+::compiled_xaml::\w+_deferred_\d+', help='binary: the functions to read')
    parser.add_argument('--top', type=int, default=10)
    args = parser.parse_args()
    if args.mode == 'source':
        functions = source_functions(args.paths)
        report(functions, args.window or 4, lambda item: 1, 'statements', args.top)
    else:
        for path in args.paths:
            sized = binary_functions(path, args.grep)
            functions = {name: [text for text, _ in rows] for name, rows in sized.items()}
            sizes = {}
            for rows in sized.values():
                for text, size in rows:
                    sizes.setdefault(text, size)
            print(f'# {path}')
            report(functions, args.window or 12, lambda item: sizes.get(item, 4), 'bytes', args.top)
    return 0


if __name__ == '__main__':
    sys.exit(main())
