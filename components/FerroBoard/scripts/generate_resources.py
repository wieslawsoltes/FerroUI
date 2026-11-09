from pathlib import Path
import json,hashlib,subprocess,sys
root=Path(__file__).resolve().parents[1]
rows=json.loads((root/'tests/fixtures/resources.json').read_text())
assets=root/'src/FerroBoard.Core/resources/florisboard'
source=root/'reference/ScreenKeyboardControl/src/ScreenKeyboard.Core/Resources/florisboard'
assert {str(p.relative_to(assets)) for p in assets.rglob('*') if p.is_file() and p.suffix in ('.json','.gz')}=={r['path'] for r in rows}
out='// Generated in verified C# manifest order. See scripts/generate_resources.py.\nuse super::embedded_resources::Resource;\npub static RESOURCES: &[Resource] = &[\n'
for row in rows:
 path=row['path'];data=(assets/path).read_bytes()
 assert data==(source/path).read_bytes()
 assert hashlib.sha256(data).hexdigest()==row['rawSha']
 out+=f'Resource {{path: {json.dumps(path)}, logical_name: {json.dumps(row["name"])}, bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/resources/florisboard/{path}"))}},\n'
out+='];\n'
out=subprocess.run(['rustfmt','--edition','2024','--emit','stdout'],input=out,text=True,capture_output=True,check=True).stdout
p=root/'src/FerroBoard.Core/resources/resource_data.rs'
if '--check' in sys.argv:assert p.read_text()==out
else:p.write_text(out)
assert (assets/'LICENSE').read_bytes()==(source/'LICENSE').read_bytes()
print(len(rows),'embedded resource entries and copied license verified')
