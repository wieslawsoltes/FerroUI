#!/usr/bin/env python3
"""Extract the frozen source coordinate method without replacing its body."""
from pathlib import Path
import argparse

folder = Path(__file__).resolve().parent
source = folder.parents[2] / 'src/Dock.Avalonia/Internal/DragPreviewHelper.cs'
text = source.read_text(encoding='utf-8-sig')
start = text.index('    private static Point GetManagedPosition(')
brace = text.index('{', start)
depth = 1
end = brace + 1
while depth:
    depth += (text[end] == '{') - (text[end] == '}')
    end += 1
method = text[start:end]
expected = '''// Generated from the frozen source GetManagedPosition body by generate.py.
// The parameter uses a Visual alias because the body reads only Visual APIs.
using Avalonia;
using Avalonia.Controls;
using Avalonia.VisualTree;
using ManagedWindowLayer = Avalonia.Visual;
partial class Program
{
''' + method + '\n}\n'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
target = folder / 'GeneratedPosition.cs'
if args.check:
    if not target.is_file() or target.read_text() != expected:
        raise SystemExit('Generated source coordinate method is stale; run generate.py.')
    print('Generated coordinate method matches the frozen source body.')
else:
    target.write_text(expected)
