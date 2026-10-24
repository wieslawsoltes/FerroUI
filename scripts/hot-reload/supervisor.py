#!/usr/bin/env python3
"""Experimental owning supervisor for the HotReload sample.

Run with the pinned Rust/dx toolchain, from a Git workspace. Only this process
builds/applies patches. Builds use private copies; the editor's checkout is never
compiled in place or modified. Ctrl-C stops the owned application and cleans up.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile
import time
from urllib.parse import quote

from probe import ROOT, events, patch_input, read, run, select_tip_artifacts

SAMPLE = 'samples/HotReload/'
PROGRAM = SAMPLE + 'program.rs'
LINK_DIRECTORY = 1 << 24


def input_names(root, extra_inputs=()):
    names = subprocess.check_output(['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z'], cwd=root)
    result = set(os.fsdecode(name) for name in names.split(b'\0') if name)
    for value in extra_inputs:
        relative = Path(value)
        if (relative.is_absolute() or relative.drive or '..' in relative.parts
                or not relative.parts or any(part.casefold() == '.git' for part in relative.parts)):
            raise ValueError(f'Explicit build input must be a workspace-relative path: {value}')
        if any((root / parent).is_symlink() for parent in relative.parents if parent.parts):
            raise ValueError(f'Select the input symlink and its target separately: {value}')
        pending = [relative]
        while pending:
            relative = pending.pop()
            path = root / relative
            if not path.resolve(strict=True).is_relative_to(root):
                raise ValueError(f'Build input escapes the private workspace: {relative}')
            result.add(relative.as_posix())
            if path.is_dir() and not path.is_symlink():
                children = list(path.iterdir())
                if any(child.name.casefold() == '.git' for child in children):
                    raise ValueError(f'Nested Git metadata is not a build input: {relative}')
                pending.extend(relative / child.name for child in children)
    return sorted(result)


def capture(root, extra_inputs=()):
    """Capture Git-visible files, preserving internal file symlinks.

    Ignored inputs must be explicitly selected. Selected directories include their
    entire contents, empty directories, permissions and internal symlinks.
    """
    root = root.resolve()
    result = {}
    for name in input_names(root, extra_inputs):
        path = root / name
        if not path.resolve(strict=True).is_relative_to(root):
            raise ValueError(f'Build input escapes the private workspace: {name}')
        info = path.lstat()  # A missing tracked file is an incomplete snapshot.
        if stat.S_ISLNK(info.st_mode):
            target = os.readlink(path)
            resolved = path.resolve(strict=True)
            if os.path.isabs(target) or not resolved.is_relative_to(root) or not (resolved.is_file() or resolved.is_dir()):
                raise ValueError(f'Build input link escapes the private workspace: {name}')
            result[name] = (os.fsencode(target), stat.S_IFLNK | stat.S_IMODE(info.st_mode) | (LINK_DIRECTORY if resolved.is_dir() else 0))
        elif stat.S_ISREG(info.st_mode):
            result[name] = (path.read_bytes(), stat.S_IMODE(info.st_mode))
        elif stat.S_ISDIR(info.st_mode):
            result[name] = (b'', stat.S_IFDIR | stat.S_IMODE(info.st_mode))
        else:
            raise ValueError(f'Unsupported build input: {name}')
    for name, (data, mode) in result.items():
        if stat.S_ISLNK(mode & 0xffff):
            target = (root / name).resolve().relative_to(root).as_posix()
            if target not in result and not (mode & LINK_DIRECTORY and any(path.startswith(target + '/') for path in result)):
                raise ValueError(f'Build input link target is not captured: {name} -> {target}')
    return result


def stable_capture(root, extra_inputs=()):
    first = capture(root, extra_inputs)
    if capture(root, extra_inputs) != first:
        raise ValueError('Source changed while capturing a revision')
    return first


def stamps(root, extra_inputs=()):
    root = root.resolve()
    result = {}
    for name in input_names(root, extra_inputs):
        info = (root / name).lstat()
        result[name] = (info.st_dev, info.st_ino, info.st_mtime_ns, info.st_ctime_ns, info.st_size, info.st_mode)
    return result


def watched_revision(root, previous, before, after, extra_inputs=()):
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    if changed and all(name.startswith(SAMPLE) and name.endswith('.xaml')
                       and all(stat.S_ISREG(snapshot[name][-1]) for snapshot in (before, after)
                               if name in snapshot) for name in changed):
        # Only runtime text changes: avoid rereading 200 MB of untouched compiler
        # inputs. Code admission still uses a fresh complete snapshot and final check.
        result = dict(previous)
        for name in changed:
            if name not in after:
                raise ValueError(f'Missing markup document: {name}')
            if not stat.S_ISREG(after[name][-1]):
                raise ValueError(f'Markup document must be a regular file: {name}')
            result[name] = ((root / name).read_bytes(), stat.S_IMODE(after[name][-1]))
    else:
        result = stable_capture(root, extra_inputs)
    if stamps(root, extra_inputs) != after:
        raise ValueError('Source changed while capturing a revision')
    return result


def stage(root, files):
    directories = []
    for name, (data, mode) in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        if stat.S_ISLNK(mode & 0xffff):
            path.symlink_to(os.fsdecode(data), target_is_directory=bool(mode & LINK_DIRECTORY))
        elif stat.S_ISDIR(mode & 0xffff):
            path.mkdir(exist_ok=True)
            directories.append((path, mode))
        else:
            path.write_bytes(data)
            path.chmod(mode)
    for path, mode in reversed(directories):
        path.chmod(stat.S_IMODE(mode))


def runtime_markup(name, value):
    return (name.startswith(SAMPLE) and name.endswith('.xaml')
            and stat.S_IFMT(value[1] & 0xffff) in (0, stat.S_IFREG))


def markup(files):
    return {'ferres://hot_reload_demo/' + quote(name[len(SAMPLE):], safe='/'): data.decode('utf-8')
            for name, (data, mode) in files.items() if runtime_markup(name, (data, mode))}


def rust_inputs(files):
    # Runtime XAML is not an input to a function patch; the staged baseline XAML
    # remains unchanged until a full rebuild establishes a new compiled baseline.
    return {name: value for name, value in files.items()
            if not runtime_markup(name, value)}


def admission_snapshot(files):
    result = {}
    for name, (data, mode) in files.items():
        key = name[len(SAMPLE):] if name.startswith(SAMPLE) else '@workspace/' + name
        result[key] = data.decode('utf-8') if name == PROGRAM else 'sha256:' + hashlib.sha256(data).hexdigest()
        result['@mode/' + key] = str(mode)
    return result


def verify_toolchain(source, dx):
    pinned = json.loads(Path(__file__).with_name('toolchain.json').read_text())
    if any(os.environ.get(key) for key in ['RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']):
        raise RuntimeError('Custom compiler/wrapper overrides are outside the tested patch contract')
    if run(['rustc', '--version'], source).split()[1] != pinned['rust'] or run([dx, '--version'], source).split()[1] != pinned['dioxus_cli']:
        raise RuntimeError(f'Requires Rust {pinned["rust"]} and dx {pinned["dioxus_cli"]}')


class StartupCleanupError(RuntimeError):
    def __init__(self, application):
        super().__init__('Application startup failed and its process could not be stopped')
        self.application = application


class Application:
    def __init__(self, client, root, files, directory, state=None):
        self.root = root
        self.client = client
        self.files = files
        self.directory = directory
        self.startup_state = state
        baseline = directory / 'baseline.json'
        baseline.write_text(json.dumps(admission_snapshot(files)))
        environment = dict(os.environ, FERROUI_PATCH_BASELINE=str(baseline))
        # Never inherit another session's source/restart channels.
        for key in ['FERROUI_HOT_VIEW', 'FERROUI_RESTART_STATE', 'FERROUI_RESTART_READY', 'FERROUI_SMOKE_EXIT_MS']:
            environment.pop(key, None)
        self.ready_path = directory / 'restored'
        if state is not None:
            state_path = directory / 'state.json'
            state_path.write_text(json.dumps(state))
            environment.update(FERROUI_RESTART_STATE=str(state_path), FERROUI_RESTART_READY=str(self.ready_path))
        self.log = (directory / 'application.log').open('w')
        self.process = None
        try:
            self.process = subprocess.Popen([client['exe']], cwd=root, stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=self.log, text=True, encoding='utf-8', env=environment)
            ready = self.response()
            if ready.get('status') != 'ready':
                raise RuntimeError(f'Application did not initialize its control channel: {ready}')
            self.aslr = ready['aslr']
            if state is not None:
                deadline = time.monotonic() + 20
                while not self.ready_path.exists():
                    if self.process.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError('Replacement did not acknowledge restored UI')
                    time.sleep(.025)
                if self.ready_path.read_bytes() != b'ferroui-ready-v1':
                    raise RuntimeError('Invalid startup acknowledgement')
        except BaseException:
            try:
                self.close()
            except BaseException as error:
                raise StartupCleanupError(self) from error
            raise

    def response(self):
        while True:
            line = read(self.process)
            if line.startswith('FERROUI_DEV='):
                return json.loads(line[len('FERROUI_DEV='):])

    def command(self, message):
        self.process.stdin.write(json.dumps(message) + '\n')
        self.process.stdin.flush()
        return self.response()

    def close(self):
        if self.process is not None and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        if self.process is not None:
            for pipe in (self.process.stdin, self.process.stdout):
                if pipe is not None:
                    pipe.close()
        if not self.log.closed:
            self.log.close()
            if os.environ.get('FERROUI_TRACE_CLICKS'):
                print(f'FERROUI_APPLICATION_LOG_BEGIN pid={self.process.pid if self.process else None}', flush=True)
                print((self.directory / 'application.log').read_text(encoding='utf-8', errors='replace'), end='', flush=True)
                print('FERROUI_APPLICATION_LOG_END', flush=True)


class Supervisor:
    def __init__(self, source, directory, dx, extra_inputs=()):
        verify_toolchain(source, dx)
        self.source, self.directory, self.dx = source.resolve(), directory.resolve(), dx
        self.extra_inputs = tuple(extra_inputs)
        self.application = None
        self.generation = 0
        self.active_directory = None
        self.retired = []

    def current(self, files):
        return stable_capture(self.source, self.extra_inputs) == files

    def rebuild(self, files):
        self.generation += 1
        directory = self.directory / str(self.generation)
        try:
            return self._rebuild(files, directory)
        finally:
            # _rebuild registers a generation before starting any application.
            # Unregistered directories have only completed/cancelled compilers.
            if directory != self.active_directory and not any(path == directory for _, path in self.retired):
                self.discard(directory)

    def discard(self, directory):
        if directory is None or not directory.exists():
            return
        try:
            shutil.rmtree(directory)
        except OSError as error:
            # Antivirus/file-indexer locks must not undo an accepted handoff.
            # The owning TemporaryDirectory retries cleanup at session shutdown.
            print(f'Could not remove retired build {directory}: {error}', flush=True)

    def _rebuild(self, files, directory):
        root = directory / 'workspace'
        stage(root, files)
        print(f'Building replacement {self.generation}; current application stays running', flush=True)
        output = run([self.dx, 'build', '--fat-binary', '--platform', 'server',
            '--package', 'hot-reload-demo', '--features', 'patch-control', '--locked', '--json-output'], root, timeout=1200, env=dict(os.environ, CARGO_TARGET_DIR=str(root / 'target')))
        client = next(event['BuildsFinished']['client'] for event in events(output) if 'BuildsFinished' in event)
        client = select_tip_artifacts(client, root, 'hot_reload_demo', PROGRAM)
        if not self.current(files):
            print('Discarded superseded replacement build', flush=True)
            return False
        previous = self.application
        previous_directory = self.active_directory
        candidate = None
        state = None
        try:
            if previous is not None:
                response = previous.command({'op': 'prepare_restart'})
                if response.get('status') != 'state':
                    raise RuntimeError(f'Cannot capture restart state: {response}')
                state = response['state']
            # Reserve the directory before startup. If construction fails while
            # stopping its process, retain files rather than deleting live code.
            self.retired.append((None, directory))
            try:
                candidate = Application(client, root, files, directory, state)
            except StartupCleanupError as error:
                self.retired[-1] = (error.application, directory)
                raise
            except BaseException:
                # Constructor guarantees successful cleanup for ordinary errors.
                self.retired.remove((None, directory))
                raise
            self.retired[-1] = (candidate, directory)
            if not self.current(files):
                print('Discarded superseded replacement startup', flush=True)
                return False
            self.application = candidate
            self.active_directory = directory
            self.retired.remove((candidate, directory))
            if previous is not None:
                self.retired.append((previous, previous_directory))
                previous.close()
                self.retired.remove((previous, previous_directory))
                self.discard(previous_directory)
            print(f'Running PID {candidate.process.pid}; replacement restored and acknowledged' if state else
                  f'Running PID {candidate.process.pid}', flush=True)
            return True
        finally:
            if candidate is not None and self.application is not candidate:
                candidate.close()
                self.retired.remove((candidate, directory))
            if previous is not None and self.application is previous and previous.process.poll() is None:
                previous.command({'op': 'resume'})

    def update(self, files):
        started = time.monotonic_ns()
        phase = started
        stages = {}
        def mark(name):
            nonlocal phase
            now = time.monotonic_ns()
            stages[name] = (now - phase) / 1000
            phase = now

        app = self.application
        if rust_inputs(files) == rust_inputs(app.files):
            result = app.command({'op': 'markup', 'markup': markup(files)})
            print(json.dumps(result), flush=True)
            if result['status'] == 'restart_required':
                self.rebuild(files)
            return
        # Keep compiled XAML baseline in the patch snapshot. Unsaved/runtime markup
        # accompanies the same admitted UI transaction without rebuilding assets.
        patch_files = dict(files)
        for name in set(patch_files) | set(app.files):
            if runtime_markup(name, patch_files.get(name, app.files.get(name))):
                if name in app.files:
                    patch_files[name] = app.files[name]
                else:
                    patch_files.pop(name, None)
        sources = admission_snapshot(patch_files)
        mark('input_preparation_us')
        admitted = app.command({'op': 'admit', 'sources': sources, 'markup': markup(files)})
        mark('admission_us')
        if admitted['status'] != 'admitted':
            print(f'Restart required: {admitted.get("error")}', flush=True)
            self.rebuild(files)
            return
        try:
            # Admission permits only this registered executable function's source.
            # The rest of the private build tree remains the original build inputs.
            data, mode = patch_files[PROGRAM]
            path = app.root / PROGRAM
            path.write_bytes(data)
            path.chmod(mode)
            mark('staging_us')
            output = run([self.dx, 'tools', 'hotpatch', '--aslr-reference', str(app.aslr),
                '--platform', 'server', '--package', 'hot-reload-demo', '--features', 'patch-control', '--json-output'], app.root, json.dumps(patch_input(app.client)), env=dict(os.environ, CARGO_TARGET_DIR=str(app.root / 'target')))
            mark('compiler_invocation_us')
            if not self.current(files):
                print('Discarded superseded patch build', flush=True)
                return
            mark('source_revalidation_us')
            table = next(event['Hotpatch']['jump_table'] for event in events(output) if 'Hotpatch' in event)
            mark('artifact_decode_us')
            result = app.command({'op': 'apply', 'request': admitted['request'], 'table': table, 'sources': sources})
            mark('application_ack_us')
            result = dict(result, patch_timing={
                'stages_us': stages,
                'update_to_ack_us': (time.monotonic_ns() - started) / 1000,
            })
            print(json.dumps(result), flush=True)
            if result['status'] == 'applied':
                app.files = patch_files
        finally:
            app.command({'op': 'cancel'})
        if result['status'] == 'restart_required':
            self.rebuild(files)
        return result

    def close(self):
        if self.application is not None:
            self.application.close()
            self.discard(self.active_directory)
            self.application = None
            self.active_directory = None
        for application, directory in list(self.retired):
            if application is not None:
                application.close()
                self.retired.remove((application, directory))
                self.discard(directory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT)
    parser.add_argument('--interval', type=float, default=.5)
    parser.add_argument('--input', action='append', default=[], metavar='PATH',
                        help='Include an ignored workspace-relative build input (file or whole directory); repeatable')
    args = parser.parse_args()
    if args.interval < .05:
        parser.error('--interval must be at least 0.05 seconds')
    source = args.source.resolve()
    dx = os.environ.get('FERROUI_DX', 'dx')
    with tempfile.TemporaryDirectory(prefix='ferroui-supervisor-') as directory:
        supervisor = Supervisor(source, Path(directory), dx, args.input)
        try:
            observed_stamps = stamps(source, args.input)
            observed = stable_capture(source, args.input)
            if stamps(source, args.input) != observed_stamps:
                raise RuntimeError('Initial source snapshot changed')
            if not supervisor.rebuild(observed):
                raise RuntimeError('Initial build was superseded; start again with a stable checkout')
            print('Watching source. Ctrl-C stops the session.', flush=True)
            while supervisor.application.process.poll() is None:
                time.sleep(args.interval)
                try:
                    latest_stamps = stamps(source, args.input)
                    if latest_stamps == observed_stamps:
                        continue
                    time.sleep(args.interval)
                    if stamps(source, args.input) != latest_stamps:
                        continue
                    current = watched_revision(source, observed, observed_stamps, latest_stamps, args.input)
                    # Remember failures too; retry only when an input changes again.
                    observed, observed_stamps = current, latest_stamps
                    supervisor.update(current)
                except Exception as error:
                    # Malformed compiler output must not tear down the last-good app.
                    # KeyboardInterrupt/SystemExit still stop the owned session.
                    print(f'Revision failed; retaining running application: {error}', flush=True)
        except KeyboardInterrupt:
            pass
        finally:
            supervisor.close()


if __name__ == '__main__':
    main()
