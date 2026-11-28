#!/usr/bin/env python3
"""Count live macOS arm64 text symbols in the typed binding implementation."""
import collections
import json
from pathlib import Path
import re
import subprocess
import sys

live = Path(sys.argv[1]).read_text(errors='replace').split('# Dead Stripped Symbols:')[0]
sections, symbols = live.split('# Sections:')[1].split('# Symbols:')
address, length = re.search(r'^(0x\w+)\s+(0x\w+)\s+__TEXT\s+__text$', sections, re.M).groups()
start, length = int(address, 16), int(length, 16)
rows = [
    (int(size, 16), name[1:])
    for addr, size, name in re.findall(r'^(0x\w+)\s+(0x\w+)\s+\[\s*\d+\]\s+(.+)$', symbols, re.M)
    if start <= int(addr, 16) < start + length and name.startswith('__R') and 'typed_binding_expression' in name
]
names = subprocess.run(['xcrun', 'llvm-cxxfilt', '-n'], input='\n'.join(x[1] for x in rows),
                       capture_output=True, text=True, check=True).stdout.splitlines()
assert len(rows) == len(names)
total = 0
groups = collections.defaultdict(lambda: [0, 0])
sources, values = set(), set()
for (size, _), name in zip(rows, names):
    if not re.match(r'<?ferroui_base::data::core::typed_binding_expression::', name):
        continue
    total += size
    # A rustc-demangled free function can end in ::<module::Type>.
    # Remove that suffix before taking the function's last path segment.
    base = name
    if base.endswith('>'):
        depth = 0
        for index in range(len(base) - 1, -1, -1):
            depth += (base[index] == '>') - (base[index] == '<')
            if not depth:
                base = base[:index].rstrip(':')
                break
    symbol = base.rsplit('::', 1)[-1]
    groups[symbol][0] += size
    groups[symbol][1] += 1
    if symbol == 'write_source_value_to_target' and 'TypedBindingExpression<' in name:
        args = name.split('TypedBindingExpression<', 1)[1].rsplit('>>::', 1)[0]
        depth = 0
        for index, char in enumerate(args):
            if char == ',' and depth == 0:
                sources.add(args[:index])
                values.add(args[index+2:])
                break
            depth += (char == '<') - (char == '>')
print(json.dumps({'text': length, 'typed_binding_text': total,
                  'source_types_identified_in_write_source_value_to_target': len(sources),
                  'value_types_identified_in_write_source_value_to_target': len(values),
                  'families_bytes_count': dict(sorted(groups.items(), key=lambda x: -x[1][0])[:35])}, indent=2))
