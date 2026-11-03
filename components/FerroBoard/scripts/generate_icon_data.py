"""Migrate authoritative generated vector paths without fetching or regenerating SVG inputs."""
from pathlib import Path
import hashlib, json, re, subprocess, sys

root = Path(__file__).resolve().parents[1]
source_path = Path('reference/ScreenKeyboardControl/src/ScreenKeyboard.Uno/Icons/IconData.g.cs')
generator_path = Path('reference/ScreenKeyboardControl/build/scripts/generate_icons.py')
source = (root / source_path).read_text()
entries = re.findall(r'^\s*\[KeyIcon\.(\w+)\] = ("[^"]*"),$', source, re.M)
names = re.findall(r'^    pub const (\w+): Self = Self\((\d+)\);', (root / 'src/FerroBoard.Core/keys/key_icon.rs').read_text(), re.M)
assert len(entries) == 66 and len(names) == 68
assert [int(number) for _, number in names] == list(range(68))
paths = {name: json.loads(path) for name, path in entries}
assert len(paths) == len(entries)
assert set(paths) == {name for name, _ in names} - {'None', 'Emoticon'}
code = '// Generated from unchanged Icons/IconData.g.cs (Material Symbols, Google, Apache-2.0).\n'
code += '// See ../NOTICE.md. Regenerate with scripts/generate_icon_data.py.\n'
code += 'pub(super) const PATHS: [Option<&str>; 68] = [\n'
for name, number in names:
    code += ('    Some(' + json.dumps(paths[name]) + '),' if name in paths else '    None,') + f' // {number}: {name}\n'
code += '];\n'
code += 'pub(super) const NAMES: [&str; 68] = [\n'
code += ''.join('    '+json.dumps(name)+',\n' for name,_ in names)
code += '];\n'
code = subprocess.run(['rustfmt', '--edition', '2024', '--emit', 'stdout'], input=code,
                      text=True, capture_output=True, check=True).stdout
manifest = dict(source=str(source_path), sha256=hashlib.sha256((root/source_path).read_bytes()).hexdigest(),
                generator=str(generator_path), generator_sha256=hashlib.sha256((root/generator_path).read_bytes()).hexdigest(),
                provenance='Material Symbols by Google via pinned ScreenKeyboardControl generated output',
                license='Apache-2.0', runtime_loading='static strings; parsed commands lazily cached',
                absent_builtins=['None','Emoticon'], entries=[
                    dict(name=name, value=int(number), logical_name='KeyIcon.'+name,
                         compression='none', bytes=len(paths[name].encode()),
                         sha256=hashlib.sha256(paths[name].encode()).hexdigest())
                    for name,number in names if name in paths])
outputs = {root/'src/FerroBoard.Controls/icons/icon_data.rs': code,
           root/'docs/icons.json': json.dumps(manifest, indent=2)+'\n'}
for path, content in outputs.items():
    if '--check' in sys.argv:
        assert path.read_text() == content, path
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
print('66 unchanged vector paths, logical names, hashes and generator provenance verified')
