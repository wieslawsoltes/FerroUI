#!/usr/bin/env python3
"""Counts how the generated code of a theme applies the runs of its units (#853).

For each generated file (`compiled_xaml.rs` of a theme crate, or the `$OUT_DIR/xaml` copy of a
build) it prints, by function:

- the calls of `rt::apply_theme_unit` and of `rt::apply_theme_region`, and the pairs of the
  regions;
- the regions the calls would form: consecutive calls into one dictionary, with only comments
  between them;
- the key assignments of theme dictionaries (`IThemeVariantProvider::__markup_set_Key`) left in
  the function, which separate the calls of the runs of merged documents.

With `--parts <theme_parts.txt>` (the list of the units an application keeps, from its
`$OUT_DIR/xaml`), it also counts the calls and pairs that address a kept unit.

The bytes these calls cost in a linked executable come from its symbols, for example:

    python3 -I scripts/symsize.py <unstripped binary> -n 40 \\
        --grep 'populate_winuitheme_xaml|build_controls_winuicontrols_xaml|apply_theme_'
"""
import argparse
import collections
import re
import sys

CALL = re.compile(r'^\s*rt::apply_theme_unit\([^,]+, (\d+), (\d+), &context, &(\w+)\)\?;$')
REGION = re.compile(r'^\s*rt::apply_theme_region\([^,]+, &\[(.*)\], &context, &(\w+)\)\?;$')
PAIR = re.compile(r'\((\d+), (\d+)\)')
FUNCTION = re.compile(r'^(?:pub )?fn (\w+)')
SET_KEY = re.compile(r'IThemeVariantProvider>::__markup_set_Key\(')


UNIT = re.compile(r'^pub static THEME_UNIT_(\d+): rt::ThemeUnit = rt::ThemeUnit \{ name: "((?:[^"\\]|\\.)*)"')


def kept_names(path):
    """The names of the units a `theme_parts.txt` lists, under its `<class>: n of m units` lines."""
    with open(path, encoding='utf-8') as file:
        return {line.strip() for line in file if line.strip() and not re.search(r': \d+ of \d+ units$', line.strip())}


def count(path, names):
    functions = collections.OrderedDict()
    current = None
    with open(path, encoding='utf-8') as file:
        lines = file.read().split('\n')
    kept = None
    if names is not None:
        kept = {int(match.group(1)) for match in map(UNIT.match, lines) if match and match.group(2) in names}
    for line in lines:
        match = FUNCTION.match(line)
        if match:
            current = match.group(1)
        if current is None:
            continue
        stats = functions.setdefault(current, {
            'calls': 0, 'region_calls': 0, 'pairs': 0, 'keys': 0, 'kept': 0, 'regions': [], 'last': None,
        })
        call = CALL.match(line)
        region = REGION.match(line)
        if call or region:
            if call:
                units = [int(call.group(1))]
                dictionary = call.group(3)
                stats['calls'] += 1
            else:
                units = [int(unit) for unit, _ in PAIR.findall(region.group(1))]
                dictionary = region.group(2)
                stats['region_calls'] += 1
                stats['pairs'] += len(units)
            if kept is not None:
                stats['kept'] += sum(1 for unit in units if unit in kept)
            if stats['last'] == dictionary:
                stats['regions'][-1] += len(units)
            else:
                stats['regions'].append(len(units))
            stats['last'] = dictionary
            continue
        if SET_KEY.search(line):
            stats['keys'] += 1
        if not line.strip().startswith('//'):
            stats['last'] = None
    return functions


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('files', nargs='+', help='generated files of themes')
    parser.add_argument('--parts', help="an application's theme_parts.txt")
    args = parser.parse_args()
    names = kept_names(args.parts) if args.parts else None
    kept = names
    for path in args.files:
        functions = count(path, names)
        print(f'# {path}')
        total = collections.Counter()
        for name, stats in functions.items():
            if not stats['calls'] and not stats['region_calls']:
                continue
            sizes = collections.Counter(stats['regions'])
            line = (f"{name}: {stats['calls']} unit calls, {stats['region_calls']} region calls "
                    f"({stats['pairs']} pairs), {len(stats['regions'])} regions "
                    f"({sizes[1]} of one), {stats['keys']} key assignments")
            if kept is not None:
                line += f", {stats['kept']} addressing a kept unit"
            print(line)
            for key in ('calls', 'region_calls', 'pairs', 'keys', 'kept'):
                total[key] += stats[key]
            total['regions'] += len(stats['regions'])
        print(f"total: {total['calls']} unit calls, {total['region_calls']} region calls ({total['pairs']} pairs), "
              f"{total['regions']} regions, {total['keys']} key assignments"
              + (f", {total['kept']} addressing a kept unit" if kept is not None else ''))
    return 0


if __name__ == '__main__':
    sys.exit(main())
