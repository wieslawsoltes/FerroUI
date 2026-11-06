#!/usr/bin/env python3
"""Validate this detachable keyboard workspace; no .NET or native-device certification."""
from pathlib import Path
import hashlib, json, subprocess
ROOT = Path(__file__).resolve().parents[1]
def run(*args):
    subprocess.run(args, cwd=ROOT, check=True)
for row in json.loads((ROOT/'docs/upstream-files.json').read_text()):
    path=ROOT/'reference/ScreenKeyboardControl'/row['source']
    assert hashlib.sha256(path.read_bytes()).hexdigest()==row['sha256'], row['source']
for row in json.loads((ROOT/'docs/resources.json').read_text()):
    assert hashlib.sha256((ROOT/row['target']).read_bytes()).hexdigest()==row['sha256'], row['target']
for generator in ['resources', 'icon_data', 'builtin_themes']:
    run('python3', 'scripts/generate_'+generator+'.py', '--check')
run('cargo','fmt','-p','ferroboard-core','-p','ferroboard-core-ferro','-p','ferroboard-controls','-p','ferroboard-sample','--','--check')
run('cargo','fmt','--manifest-path','tests/FerroBoard.Native.Consumer/Cargo.toml','--','--check')
run('cargo','test','--locked','--workspace','--no-fail-fast')
run('cargo','test','--locked','--manifest-path','tests/FerroBoard.Native.Consumer/Cargo.toml')
run('cargo','clippy','--locked','--workspace','--all-targets','--','-D','warnings')
run('cargo','clippy','--locked','--manifest-path','tests/FerroBoard.Native.Consumer/Cargo.toml','--all-targets','--','-D','warnings')
print('Standalone workspace, independent consumer and pinned reference/resources verified. Native devices remain separate.')
