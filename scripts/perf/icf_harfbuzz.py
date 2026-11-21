#!/usr/bin/env python3
"""Whether identical code folding touches the callback comparison of HarfBuzz (#1001).

`hb_font_t::has_func_set` (hb-font.cc) decides whether a font has a callback by comparing the
address in the slot of the font's function table with the address of the default callback of
that slot. If the linker folds a callback that is set with the default of the same slot, the
comparison says "not set" and shaping takes another path. The OpenType functions
(`hb_ot_font_set_funcs`, `hb_ot_get_*`) are the callbacks FerroUI sets.

Input: the output of the linker with its list of folded functions (`link.exe /VERBOSE:ICF`, or
`lld --print-icf-sections`) and, as a control, its map. Outcomes:

    none        no default callback is folded with anything
    harmless    a default callback is folded, but not with a callback set in its own slot
    path-change a default callback is folded with the OpenType callback of its slot
    inconclusive the log has no folded function, or the map has no default callback

    icf_harfbuzz.py --format msvc|lld --label TEXT --log FILE [--map FILE] [--json FILE]
    icf_harfbuzz.py --self-test
"""
import argparse
import json
import re
import sys

# The slots of `hb_font_funcs_t` (HB_FONT_FUNCS_IMPLEMENT_CALLBACKS, hb-font.hh).
SLOTS = (
    'font_h_extents', 'font_v_extents', 'nominal_glyph', 'nominal_glyphs', 'variation_glyph',
    'glyph_h_advance', 'glyph_v_advance', 'glyph_h_advances', 'glyph_v_advances',
    'glyph_h_origin', 'glyph_v_origin', 'glyph_h_kerning', 'glyph_v_kerning', 'glyph_extents',
    'glyph_contour_point', 'glyph_name', 'glyph_from_name', 'draw_glyph', 'paint_glyph',
)
_SLOT = '|'.join(sorted(SLOTS, key=len, reverse=True))
_FONT = re.compile(rf'hb_font_(?:get_)?({_SLOT})_(default|nil)(?![a-z0-9_])')
_OT = re.compile(rf'hb_ot_(?:get_)?({_SLOT})(?![a-z0-9_])')
_ANSI = re.compile(r'\x1b\[[0-9;]*m')
_LLD_SELECTED = re.compile(r'selected section (\S.*)$')
_LLD_REMOVED = re.compile(r'removing identical section (\S.*)$')
_MSVC_MAP = re.compile(r'^\s*([0-9a-fA-F]{4}:[0-9a-fA-F]{8})\s+(\S+)\s+[0-9a-fA-F]{16}')
_LLD_MAP = re.compile(r'^\s*([0-9a-f]+)\s+[0-9a-f]+\s+[0-9a-f]+\s+\d+\s+(\S.*)$')


def classify(name):
    """(kind, slot) of a HarfBuzz callback: default, nil or set; None for any other name."""
    match = _FONT.search(name)
    if match:
        return match.group(2), match.group(1)
    match = _OT.search(name)
    return ('set', match.group(1)) if match else None


def groups_msvc(lines):
    """The groups of `/VERBOSE:ICF`: a selected symbol and the symbols replaced by it."""
    groups, current, state = [], None, None
    for raw in lines:
        line = _ANSI.sub('', raw).strip()
        lower = line.lower()
        if lower.startswith('selected symbol'):
            current, state = [], 'name'
            groups.append(current)
            rest = line.split(':', 1)[1].strip() if ':' in line else ''
            if rest:
                current.append(rest.split()[0])
        elif lower.startswith('replaced symbol'):
            state = 'name' if current is not None else None
            rest = line.split(':', 1)[1].strip() if ':' in line else ''
            if rest and current is not None:
                current.append(rest.split()[0])
        elif state == 'name' and line and not line.startswith(('Starting', 'Finished', 'Invoking')):
            token = line.split()[0]
            if token.startswith(('?', '_', '.')) or re.match(r'[A-Za-z]', token):
                current.append(token)
            else:
                state = None
        elif not line:
            state = None
    return [group for group in groups if len(group) > 1]


def groups_lld(lines):
    """The groups of `--print-icf-sections`: a selected section and the sections removed."""
    groups, current = [], None
    for raw in lines:
        line = _ANSI.sub('', raw)
        selected = _LLD_SELECTED.search(line)
        if selected:
            current = [selected.group(1).strip()]
            groups.append(current)
            continue
        removed = _LLD_REMOVED.search(line)
        if removed and current is not None:
            current.append(removed.group(1).strip())
    return [group for group in groups if len(group) > 1]


def map_callbacks(lines, fmt):
    """{(kind, slot): set of addresses} of the callbacks the linker map names."""
    pattern = _MSVC_MAP if fmt == 'msvc' else _LLD_MAP
    found = {}
    for line in lines:
        if 'hb_' not in line:
            continue
        match = pattern.match(line)
        if match:
            callback = classify(match.group(2))
            if callback:
                found.setdefault(callback, set()).add(match.group(1))
    return found


def analyse(groups, callbacks=None):
    """The outcome and its evidence."""
    with_default, path_change, harfbuzz_groups = [], [], 0
    for group in groups:
        members = [(name, classify(name)) for name in group]
        known = [callback for _, callback in members if callback]
        harfbuzz_groups += bool(known) or any('hb_' in name for name in group)
        defaults = {slot for kind, slot in known if kind == 'default'}
        if not defaults:
            continue
        with_default.append(group)
        if defaults & {slot for kind, slot in known if kind == 'set'}:
            path_change.append(group)

    same_address = []
    defaults_in_map = None
    if callbacks is not None:
        defaults_in_map = sum(1 for kind, _ in callbacks if kind == 'default')
        for (kind, slot), addresses in callbacks.items():
            if kind == 'set' and addresses & callbacks.get(('default', slot), set()):
                same_address.append(slot)

    if not groups or defaults_in_map == 0:
        outcome = 'inconclusive'
    elif path_change or same_address:
        outcome = 'path-change'
    elif with_default:
        outcome = 'harmless'
    else:
        outcome = 'none'
    return {
        'outcome': outcome,
        'folded_groups': len(groups),
        'groups_with_harfbuzz': harfbuzz_groups,
        'default_callbacks_in_map': defaults_in_map,
        'set_callbacks_in_map': None if callbacks is None
        else sum(1 for kind, _ in callbacks if kind == 'set'),
        'groups_with_a_default': with_default,
        'path_change_groups': path_change,
        'same_address_in_map': sorted(same_address),
    }


def report(label, result):
    print(f'### {label}: {result["outcome"]}')
    print()
    print('```')
    for key in ('folded_groups', 'groups_with_harfbuzz', 'default_callbacks_in_map',
                'set_callbacks_in_map'):
        print(f'{key}: {result[key]}')
    print(f'groups with a default callback: {len(result["groups_with_a_default"])}')
    print(f'of them with the callback set in the same slot: {len(result["path_change_groups"])}')
    print(f'same address in the map (default and set, same slot): {result["same_address_in_map"]}')
    for group in result['groups_with_a_default'][:20]:
        print('  ' + ' = '.join(name[:70] for name in group[:6]))
    print('```')
    print()


def self_test():
    default = '?hb_font_get_glyph_name_default@@YAHPEAUhb_font_t@@@Z'
    nil = '?hb_font_get_glyph_name_nil@@YAHPEAUhb_font_t@@@Z'
    callback = '?hb_ot_get_glyph_name@@YAHPEAUhb_font_t@@@Z'
    other_slot = '?hb_ot_get_glyph_extents@@YAHPEAUhb_font_t@@@Z'
    msvc = lambda a, b: ['Starting pass 2', '    Selected symbol:', f'        {a} (int f(void))',
                         '    Replaced symbol(s):', f'        {b} (int g(void))', '']
    assert classify(default) == ('default', 'glyph_name')
    assert classify(nil) == ('nil', 'glyph_name')
    assert classify(callback) == ('set', 'glyph_name')
    assert classify('_ZL24hb_ot_get_glyph_h_originP9hb_font_tPvjPiS2_S1_') == ('set', 'glyph_h_origin')
    assert classify('_ZL35hb_font_get_glyph_h_advance_defaultP9hb_font_t') == \
        ('default', 'glyph_h_advance')
    assert classify('hb_font_draw_glyph_default') == ('default', 'draw_glyph')
    assert classify('hb_ot_layout_has_glyph_classes') is None
    assert analyse(groups_msvc(msvc('?a@@YAHXZ', '?b@@YAHXZ')))['outcome'] == 'none'
    assert analyse(groups_msvc(msvc(default, nil)))['outcome'] == 'harmless'
    assert analyse(groups_msvc(msvc(default, other_slot)))['outcome'] == 'harmless'
    assert analyse(groups_msvc(msvc(default, callback)))['outcome'] == 'path-change'
    assert analyse([])['outcome'] == 'inconclusive'
    lld = ['selected section a.o:(.text._ZL29hb_font_get_glyph_name_defaultP9hb_font_t)',
           '  removing identical section a.o:(.text._ZL20hb_ot_get_glyph_nameP9hb_font_t)']
    assert analyse(groups_lld(lld))['outcome'] == 'path-change'
    no_defaults = {('set', 'glyph_name'): {'1'}}
    assert analyse(groups_msvc(msvc('?a@@YAHXZ', '?b@@YAHXZ')), no_defaults)['outcome'] == \
        'inconclusive'
    table = map_callbacks([f' 0001:00000010       {default} 0000000140001010 f   a.obj',
                           f' 0001:00000010       {callback} 0000000140001010 f   a.obj'], 'msvc')
    assert analyse(groups_msvc(msvc('?a@@YAHXZ', '?b@@YAHXZ')), table)['outcome'] == 'path-change'
    print('self-test passed')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n', 1)[0])
    parser.add_argument('--format', choices=('msvc', 'lld'))
    parser.add_argument('--label', default='link')
    parser.add_argument('--log')
    parser.add_argument('--map')
    parser.add_argument('--json')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not (args.format and args.log):
        parser.error('--format and --log are required')
    with open(args.log, errors='replace') as log:
        groups = (groups_msvc if args.format == 'msvc' else groups_lld)(log)
    callbacks = None
    if args.map:
        with open(args.map, errors='replace') as linker_map:
            callbacks = map_callbacks(linker_map, args.format)
    result = analyse(groups, callbacks)
    report(args.label, result)
    if args.json:
        with open(args.json, 'w') as out:
            json.dump({'label': args.label, **result}, out)


if __name__ == '__main__':
    main()
