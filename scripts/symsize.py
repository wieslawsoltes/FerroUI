#!/usr/bin/env python3
"""Per-crate and per-symbol size attribution of an unstripped Mach-O or ELF binary.

Size of a symbol = distance to the next symbol in the same section (nm -n). Rust legacy-mangled
names are decoded here; a symbol is attributed to the crate named first in its path (for
`<A as B>::f`, the crate of A). Names mangled with the v0 scheme (`_R...`) are attributed to the
first crate root of their path and listed as they are. Other symbols are grouped as C, C++ and
Objective-C, and the private constants that hold embedded files as anonymous data.
"""
import argparse, collections, re, subprocess, sys

def decode_legacy(sym):
    s = sym
    if s.startswith('__ZN'):
        s = s[1:]
    if not s.startswith('_ZN'):
        return None
    i = 3
    parts = []
    while i < len(s) and s[i] != 'E':
        m = re.match(r'\d+', s[i:])
        if not m:
            return None
        n = int(m.group(0)); i += len(m.group(0))
        parts.append(s[i:i + n]); i += n
    if parts and re.fullmatch(r'h[0-9a-f]{16}', parts[-1]):
        parts = parts[:-1]
    else:
        return None
    out = []
    for p in parts:
        if p.startswith('_$'):
            p = p[1:]
        p = p.replace('$LT$', '<').replace('$GT$', '>').replace('$u20$', ' ').replace('$RF$', '&')
        p = p.replace('$C$', ',').replace('$BP$', '*').replace('$u7b$', '{').replace('$u7d$', '}')
        p = p.replace('$u5b$', '[').replace('$u5d$', ']').replace('$u27$', "'").replace('$LP$', '(').replace('$RP$', ')')
        p = p.replace('..', '::').replace('$u3b$', ';').replace('$u2b$', '+').replace('$u22$', '"').replace('$u21$', '!')
        out.append(p)
    return '::'.join(out)

def crate_of_v0(sym):
    """The crate of a symbol mangled with the v0 scheme (`_R...`, the default of newer rustc): the
    first crate root of its path (`C`, an optional disambiguator `s<base62>_`, then the length
    and the identifier), so `<core::ptr::drop_in_place<T>>` is `core` as in the legacy scheme."""
    s = sym[1:] if sym.startswith('__R') else sym
    if not s.startswith('_R'):
        return None
    m = re.search(r'C(?:s[0-9a-zA-Z]*_)?(\d+)_?', s[2:])
    if not m:
        return None
    start = m.end() + 2
    return s[start:start + int(m.group(1))] or None

def crate_of(name):
    n = name
    while n.startswith('<'):
        n = n[1:]
        n = n.lstrip('&*').replace('mut ', '').replace('dyn ', '').strip()
    m = re.match(r'[A-Za-z_][A-Za-z0-9_]*', n)
    return m.group(0) if m else '?'

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('binary')
    ap.add_argument('-n', type=int, default=50)
    ap.add_argument('--crates', type=int, default=40)
    ap.add_argument('--families', type=int, default=0, help='group generic families: strip <...> args')
    ap.add_argument('--grep', default=None, help='only list top symbols matching')
    a = ap.parse_args()
    out = subprocess.run(['nm', '-n', '-m', a.binary], capture_output=True, text=True).stdout.splitlines()
    rows = []
    for line in out:
        m = re.match(r'([0-9a-f]+) \(([^)]+)\) (\S+ )?(.*?)(external|non-external)( \([^)]*\))? (\S+)$', line)
        if not m:
            continue
        addr = int(m.group(1), 16); sect = m.group(2); name = m.group(7)
        if sect in ('undefined', 'absolute', 'indirect'):
            continue
        rows.append((addr, sect, name))
    rows.sort()
    sizes = collections.Counter(); crates = collections.Counter(); sects = collections.Counter()
    fam = collections.Counter(); famn = collections.Counter()
    syms = []
    for i, (addr, sect, name) in enumerate(rows):
        j = i + 1
        while j < len(rows) and rows[j][0] == addr:
            j += 1
        if j >= len(rows) or rows[j][1] != sect:
            continue
        size = rows[j][0] - addr
        if size <= 0 or size > 200_000_000:
            continue
        d = decode_legacy(name)
        v0 = crate_of_v0(name) if d is None else None
        if v0 is not None:
            cr = v0
            dn = name
        elif d is None:
            cr = '[anonymous data]' if name.startswith(('_anon.', 'l_anon.')) or '.llvm.' in name and name.startswith('_anon') else '[C/C++/ObjC]'
            dn = name
        else:
            cr = crate_of(d); dn = d
        crates[cr] += size; sects[sect] += size
        syms.append((size, cr, sect, dn))
        if a.families:
            f = re.sub(r'<[^<>]*>', '<_>', dn)
            for _ in range(6):
                f2 = re.sub(r'<[^<>]*>', '<_>', f)
                if f2 == f: break
                f = f2
            f = re.sub(r'\{\{closure\}\}', '{closure}', f)
            fam[f] += size; famn[f] += 1
    total = sum(crates.values())
    print(f'# attributed {total/1e6:.2f} MB')
    for s, v in sects.most_common():
        print(f'  section {s:28s} {v/1e6:8.2f} MB')
    print('# by crate')
    for c, v in crates.most_common(a.crates):
        print(f'{v/1e6:8.2f} MB {100*v/total:5.1f}%  {c}')
    print(f'# top {a.n} symbols')
    syms.sort(reverse=True)
    k = 0
    for size, cr, sect, dn in syms:
        if a.grep and not re.search(a.grep, dn):
            continue
        print(f'{size/1e3:9.1f} kB  {cr:28s} {dn[:180]}')
        k += 1
        if k >= a.n: break
    if a.families:
        print(f'# top {a.families} families')
        for f, v in fam.most_common(a.families):
            print(f'{v/1e6:8.3f} MB {famn[f]:6d}x  {f[:170]}')

main()
