#!/usr/bin/env python3
"""Native Subsecond acceptance probe. Requires dx 0.7.10 and Cargo on PATH.

Builds an isolated executable, changes one synchronous function, builds a patch,
and checks that the same PID returns the new result. Never edits repository sources.
A failed patch build is a failed acceptance gate, not a successful dry run.
"""
import json
import os
from pathlib import Path
import queue
import signal
import threading
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SOURCE = '''use std::io::{self, BufRead, Write};
use std::collections::BTreeMap;
use ferroui_hot_patch::{FunctionRegistration, PatchSupervisor, inputs::BuildInputs};
fn snapshot() -> ferroui_hot_patch::SourceSnapshot {
    BuildInputs::capture(std::path::Path::new("."), &["target".into()],
        BTreeMap::from([("toolchain".into(), "rust-1.93.0-dx-0.7.10".into())]))
        .unwrap().snapshot().unwrap()
}
fn value(delta: i32) -> (i32, usize, usize, usize) { ferroui_patch_probe::evaluate(delta * 1) }
fn layout() -> i32 { 1 }
fn main() {
    let baseline = snapshot();
    assert_eq!(baseline["src/main.rs"], include_str!("main.rs"));
    let mut supervisor = PatchSupervisor::with_registrations(baseline, [
        FunctionRegistration::callback("src/main.rs", "value"),
        FunctionRegistration::view_builder("src/main.rs", "layout", ["fixture".to_string()]),
    ]).unwrap();
    let mut pending = None;
    let callback = || ferroui_hot_patch::hot_reloadable_with(value, 1);
    ferroui_patch_probe::subscribe(std::rc::Rc::new(callback));
    println!("ASLR={}", subsecond::aslr_reference());
    io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        if line == "value" { let (state, identity, object_identity, address) = ferroui_patch_probe::fire(); println!("VALUE={state},{identity},{object_identity},{address}"); }
        else if line == "admit" {
            match supervisor.admit(snapshot()) {
                Ok(ticket) => { pending = Some(ticket); println!("ADMITTED"); }
                Err(reason) => { pending = None; println!("RESTART={reason}"); }
            }
        }
        else {
            let message: serde_json::Value = serde_json::from_str(&line).unwrap();
            let table: subsecond::JumpTable = serde_json::from_value(message["table"].clone()).unwrap();
            let current = snapshot();
            let admitted = pending.take().expect("admit before building");
            assert_eq!(admitted.views_to_reapply(), &["fixture".to_string()]);
            // SAFETY: the parent builds exclusively from this admitted body-only edit,
            // against this running executable; this fixture has no other executing threads.
            let previous_revision = supervisor.revision();
            let mode = message["mode"].as_str().unwrap();
            // The candidate layout function has no side effects. No Subsecond
            // reload handlers are registered, and the parent retains artifacts.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                supervisor.apply_with(admitted, &current, table, || {
                    assert_eq!(ferroui_hot_patch::hot_reloadable(layout), message["layout"].as_i64().unwrap() as i32);
                    match mode {
                        "reject" => Err("candidate view rejected".into()),
                        "panic" => panic!("candidate view panicked"),
                        _ => Ok(()),
                    }
                })
            }));
            match mode {
                "reject" => { assert!(outcome.unwrap().is_err()); assert_eq!(supervisor.revision(), previous_revision); println!("ROLLED_BACK"); }
                "panic" => { assert!(outcome.is_err()); assert_eq!(supervisor.revision(), previous_revision); println!("ROLLED_BACK"); }
                _ => { outcome.unwrap().unwrap(); assert_eq!(supervisor.revision(), previous_revision + 1); println!("APPLIED"); }
            }
        }
        io::stdout().flush().unwrap();
    }
}
'''

# Durable framework objects and subscriptions belong to a stable library, never the tip.
LIBRARY = r'''use std::{cell::{Cell, RefCell}, rc::Rc};
use ferroui_base::{FerroObject, Ref, TypeInfo};
type Callback = Rc<dyn Fn() -> (i32, usize, usize, usize)>;
thread_local! {
    static COUNT: Cell<i32> = const { Cell::new(0) };
    static OBJECT: Ref<FerroObject> = FerroObject::new();
    static CALLBACK: RefCell<Option<Callback>> = const { RefCell::new(None) };
}
pub fn subscribe(callback: Callback) { CALLBACK.with(|slot| *slot.borrow_mut() = Some(callback)); }
pub fn fire() -> (i32, usize, usize, usize) { CALLBACK.with(|slot| slot.borrow().as_ref().unwrap()()) }
pub fn evaluate(delta: i32) -> (i32, usize, usize, usize) {
    let count = COUNT.with(|count| { count.set(count.get() + delta); count.get() });
    let identity = FerroObject::TYPE as *const TypeInfo as usize;
    let object_identity = OBJECT.with(|object| object.get_type() as *const TypeInfo as usize);
    let address = OBJECT.with(|object| &**object as *const FerroObject as usize);
    (count, identity, object_identity, address)
}
'''

def events(output):
    for line in output.splitlines():
        try:
            value = json.loads(line)
            yield json.loads(value['json']) if 'json' in value else value
        except (ValueError, TypeError):
            pass


def run(command, root, stdin=None, timeout=300, env=None):
    # A compiler launches Cargo, rustc, linkers and build scripts. Killing only
    # dx leaves those descendants writing into a workspace being discarded.
    options = dict(cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True, encoding='utf-8', env=env)
    job = None
    if os.name == 'nt':
        from windows_job import launch
        process, job, header = launch(command, **options)
        stdin = header + (stdin or '')
    else:
        process = subprocess.Popen(command, start_new_session=True, **options)
    try:
        stdout, stderr = process.communicate(stdin, timeout=timeout)
    except BaseException:
        if job is not None:
            job.close()
        else:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        if process.poll() is None:
            process.kill()
        process.wait()
        raise
    finally:
        if job is not None:
            job.close()
        for pipe in (process.stdin, process.stdout, process.stderr):
            pipe.close()
    if process.returncode:
        raise RuntimeError(f"{command[0]} failed ({process.returncode})\n{stderr}\n{stdout}")
    return stdout


def read(process):
    # Pipe select() is not portable to Windows; a single reader also avoids text-buffer
    # readiness races. It exits on EOF when the fixture is stopped in finally.
    if not hasattr(process, 'responses'):
        process.responses = queue.Queue()
        def pump():
            try:
                for line in process.stdout:
                    process.responses.put(line.strip())
            finally:
                process.responses.put(None)
        threading.Thread(target=pump, daemon=True).start()
    try:
        result = process.responses.get(timeout=15)
    except queue.Empty as error:
        raise RuntimeError('Probe did not reply within 15 seconds') from error
    if result is None:
        raise RuntimeError('Probe exited without a reply')
    return result


def patch_input(client):
    """Remove dx 0.7.10 process-local values before standalone rustc replay.

    dx constructs fresh linker output paths, then overlays the captured environment.
    Replaying the old DX_LINK values makes it read a different, empty argument file.
    Cargo jobserver descriptors also belong to the completed build process.
    Keep all actual compiler/build inputs unchanged.
    """
    client = dict(client)
    temporary = {"DX_LINK", "DX_LINK_ARGS_FILE", "DX_LINK_ERR_FILE",
                 "DX_LINK_TRIPLE", "DX_LINK_CUSTOM_LINKER", "CARGO_MAKEFLAGS"}
    client["rustc_envs"] = [(key, value) for key, value in client["rustc_envs"]
                            if key not in temporary]
    return client


def select_tip_artifacts(client, root, crate_name, source):
    """dx 0.7.10's structured output can select build_script_build.bin first.

    Require the exact captured application invocation; never replay another binary
    or reconstruct compiler arguments by guessing which flags were used.
    """
    matches = []
    for path in (root / 'target/dx/.captured-args').rglob(f'{crate_name}.bin.json'):
        captured = json.loads(path.read_text())
        args = captured['args']
        if '--crate-name' not in args or args[args.index('--crate-name') + 1] != crate_name:
            continue
        if not any((root / arg).resolve() == (root / source).resolve()
                   for arg in args if arg.endswith('.rs')):
            continue
        matches.append(captured)
    if len(matches) != 1:
        raise RuntimeError(f'Expected one captured invocation for {crate_name}/{source}; found {len(matches)}')
    client = dict(client)
    client['rustc_args'] = matches[0]['args']
    client['rustc_envs'] = matches[0]['envs']
    return client


def main():
    dx = os.environ.get('FERROUI_DX', 'dx')
    pinned = json.loads((Path(__file__).with_name('toolchain.json')).read_text())
    rust_version = run(['rustc', '--version'], ROOT).split()
    dx_version = run([dx, '--version'], ROOT).split()
    if rust_version[1] != pinned['rust'] or dx_version[1] != pinned['dioxus_cli']:
        raise RuntimeError(f"Required Rust {pinned['rust']} / dx {pinned['dioxus_cli']}; "
                           f"got {' '.join(rust_version)} / {' '.join(dx_version)}")
    with tempfile.TemporaryDirectory(prefix='ferroui-patch-probe-') as directory:
        root = Path(directory)
        (root / 'src').mkdir()
        (root / 'Cargo.toml').write_text('[package]\nname="ferroui-patch-probe"\nversion="0.1.0"\nedition="2021"\n'
            '[dependencies]\nferroui-hot-patch={path=' + json.dumps(str(ROOT / 'src/FerroUI.HotPatch')) + '}\n'
            'ferroui-base={path=' + json.dumps(str(ROOT / 'src/FerroUI.Base')) + '}\n'
            'subsecond="=0.7.10"\nserde_json="1"\n')
        source = root / 'src/main.rs'
        source.write_text(SOURCE)
        (root / 'src/lib.rs').write_text(LIBRARY)
        build_env = dict(os.environ, CARGO_TARGET_DIR=str(root / 'target'))
        built = run([dx, 'build', '--fat-binary', '--platform', 'server', '--json-output'], root, env=build_env)
        client = next(event['BuildsFinished']['client'] for event in events(built) if 'BuildsFinished' in event)
        process = subprocess.Popen([client['exe']], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=root)
        try:
            address = read(process).split('=', 1)[1]
            process.stdin.write('value\n'); process.stdin.flush()
            initial = read(process).removeprefix('VALUE=').split(',')
            assert initial[0] == '1' and initial[1] == initial[2], initial
            expected_state = 1
            current_delta = 1
            for value, mode in [(7, 'reject'), (2, 'apply'), (8, 'reject'), (9, 'panic'), (3, 'apply')]:
                changed = SOURCE.replace('ferroui_patch_probe::evaluate(delta * 1)', f'ferroui_patch_probe::evaluate(delta * {value})')
                changed = changed.replace('fn layout() -> i32 { 1 }', f'fn layout() -> i32 {{ {value} }}')
                source.write_text(changed)
                process.stdin.write('admit\n'); process.stdin.flush()
                assert read(process) == 'ADMITTED'
                patch = run([dx, 'tools', 'hotpatch', '--aslr-reference', address, '--platform', 'server', '--json-output'], root, json.dumps(patch_input(client)), env=build_env)
                table = next(event['Hotpatch']['jump_table'] for event in events(patch) if 'Hotpatch' in event)
                process.stdin.write(json.dumps({'table': table, 'mode': mode, 'layout': value}) + '\n'); process.stdin.flush()
                assert read(process) == ('APPLIED' if mode == 'apply' else 'ROLLED_BACK')
                if mode == 'apply':
                    current_delta = value
                process.stdin.write('value\n'); process.stdin.flush()
                expected_state += current_delta
                actual = read(process).removeprefix('VALUE=').split(',')
                assert actual == [str(expected_state), *initial[1:]], actual
            # Admission owns the gate: a changed stable-library type never reaches dx.
            library = root / 'src/lib.rs'
            library.write_text(LIBRARY + '\npub struct Incompatible;\n')
            process.stdin.write('admit\n'); process.stdin.flush()
            rejection = read(process)
            assert rejection.startswith('RESTART=') and 'src/lib.rs' in rejection, rejection
            library.write_text(LIBRARY)
            process.stdin.write('value\n'); process.stdin.flush()
            expected_state += 3
            assert read(process).removeprefix('VALUE=').split(',') == [str(expected_state), *initial[1:]]
            print(f'PASS: PID {process.pid} retained its framework object, type identity, subscription and counter through two accepted patches and three view-transaction rollbacks')
        finally:
            process.terminate()
            process.wait(timeout=10)

if __name__ == '__main__':
    main()
