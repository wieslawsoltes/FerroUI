import json
import os
import stat
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

from supervisor import Supervisor, StartupCleanupError, PROGRAM, SAMPLE, capture, stable_capture, stage, admission_snapshot, stamps, watched_revision, markup, rust_inputs


class SupervisorTests(unittest.TestCase):
    def test_explicit_ignored_inputs_are_staged_watched_and_fingerprinted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'source'
            root.mkdir()
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True)
            (root / '.gitignore').write_text('generated/\n', encoding='utf-8')
            (root / 'generated/empty').mkdir(parents=True)
            asset = root / 'generated/data.bin'
            asset.write_bytes(b'\x00\xffbefore')
            self.assertNotIn('generated/data.bin', capture(root))
            selected = ['generated']
            before = stamps(root, selected)
            files = stable_capture(root, selected)
            copied = Path(directory) / 'build'
            stage(copied, files)
            self.assertEqual((copied / 'generated/data.bin').read_bytes(), asset.read_bytes())
            self.assertTrue((copied / 'generated/empty').is_dir())
            self.assertTrue(stat.S_ISDIR(files['generated/empty'][1]))
            asset.write_bytes(b'\x00\xffafter')
            changed = watched_revision(root, files, before, stamps(root, selected), selected)
            self.assertNotEqual(admission_snapshot(files), admission_snapshot(changed))
            self.assertNotEqual(rust_inputs(files), rust_inputs(changed))
            with patch('supervisor.verify_toolchain'):
                owner = Supervisor(root, Path(directory) / 'builds', 'dx', selected)
            self.assertFalse(owner.current(files))
            self.assertTrue(owner.current(changed))
            (root / 'generated/new').write_bytes(b'new input')
            self.assertFalse(owner.current(changed))
            asset.unlink()
            self.assertNotIn('generated/data.bin', capture(root, selected))
            (root / 'generated/new').unlink()
            (root / 'generated/empty').rmdir()
            (root / 'generated').rmdir()
            with self.assertRaises(FileNotFoundError):
                capture(root, selected)

    def test_explicit_inputs_reject_external_paths_and_preserve_links(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'source'
            root.mkdir()
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True)
            (root / '.gitignore').write_text('generated/\n', encoding='utf-8')
            (root / 'generated/target').mkdir(parents=True)
            (root / 'generated/target/data').write_bytes(b'contents')
            for path in ['.', '../outside', str(root / 'generated'), '.git/config']:
                with self.subTest(path=path), self.assertRaises(ValueError):
                    capture(root, [path])
            if os.name == 'nt':
                return  # Windows link creation may require developer mode.
            (root / 'generated/link').symlink_to('target', target_is_directory=True)
            files = capture(root, ['generated'])
            copied = Path(directory) / 'build'
            stage(copied, files)
            self.assertTrue((copied / 'generated/link').is_symlink())
            self.assertEqual((copied / 'generated/link/data').read_bytes(), b'contents')
            with self.assertRaises(ValueError):
                capture(root, ['generated/link/data'])
            (root / 'generated/escape').symlink_to(directory, target_is_directory=True)
            with self.assertRaises(ValueError):
                capture(root, ['generated'])

    def test_markup_named_directories_remain_compiler_inputs(self):
        files = {SAMPLE + 'generated.xaml': (b'', stat.S_IFDIR | 0o755)}
        self.assertEqual(markup(files), {})
        self.assertEqual(rust_inputs(files), files)

    def test_changed_explicit_input_uses_admission_and_rebuild(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            files['generated/config'] = (b'old compiler input', 0o644)
            changed = dict(files, **{'generated/config': (b'new compiler input', 0o644)})
            app.command.return_value = {'status': 'restart_required', 'error': 'Unpatchable build input changed'}
            owner.rebuild = Mock()
            with patch('supervisor.run') as compiler:
                owner.update(changed)
                compiler.assert_not_called()
            request = app.command.call_args.args[0]
            self.assertEqual(request['op'], 'admit')
            self.assertEqual(request['sources'], admission_snapshot(changed))
            owner.rebuild.assert_called_once_with(changed)
            app.close.assert_not_called()

    def test_startup_cleanup_failure_retains_candidate_until_shutdown(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            app.process.poll.return_value = None
            app.command.return_value = {'status': 'state', 'state': {}}
            owner.current = lambda _: True
            candidate = Mock()
            with patch('supervisor.run', return_value=json.dumps({'BuildsFinished': {'client': {}}})), \
                 patch('supervisor.select_tip_artifacts', return_value={}), \
                 patch('supervisor.Application', side_effect=StartupCleanupError(candidate)):
                with self.assertRaises(StartupCleanupError):
                    owner.rebuild(files)
            self.assertIs(owner.application, app)
            staged = owner.directory / '1'
            self.assertTrue(staged.exists())
            self.assertEqual(app.command.call_args.args[0]['op'], 'resume')
            candidate.close.side_effect = lambda: self.assertTrue(staged.exists())
            owner.close()
            candidate.close.assert_called_once()
            self.assertFalse(staged.exists())

    def test_failed_and_superseded_builds_remove_only_candidate_files(self):
        for failure in ['compiler', 'superseded']:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                owner, app, files = self.exercise(directory)
                current = owner.directory / 'current'
                current.mkdir(parents=True)
                (current / 'loaded-library').write_bytes(b'keep')
                owner.active_directory = current
                owner.current = lambda _: False
                with patch('supervisor.run', side_effect=RuntimeError('build failed') if failure == 'compiler' else None,
                           return_value=json.dumps({'BuildsFinished': {'client': {}}})), \
                     patch('supervisor.select_tip_artifacts', return_value={}):
                    if failure == 'compiler':
                        with self.assertRaisesRegex(RuntimeError, 'build failed'):
                            owner.rebuild(files)
                    else:
                        self.assertFalse(owner.rebuild(files))
                self.assertFalse((owner.directory / '1').exists())
                self.assertEqual((current / 'loaded-library').read_bytes(), b'keep')
                app.close.assert_not_called()

    def test_retired_files_are_removed_only_after_process_closes(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            current = owner.directory / 'current'
            current.mkdir(parents=True)
            owner.active_directory = current
            app.close.side_effect = lambda: self.assertTrue(current.exists())
            app.command.return_value = {'status': 'state', 'state': {}}
            owner.current = lambda _: True
            candidate = Mock()
            with patch('supervisor.run', return_value=json.dumps({'BuildsFinished': {'client': {}}})), \
                 patch('supervisor.select_tip_artifacts', return_value={}), \
                 patch('supervisor.Application', return_value=candidate):
                self.assertTrue(owner.rebuild(files))
            self.assertFalse(current.exists())
            accepted = owner.directory / '1'
            self.assertTrue(accepted.exists())
            candidate.close.side_effect = lambda: self.assertTrue(accepted.exists())
            owner.close()
            self.assertFalse(accepted.exists())

    def test_failed_retirement_keeps_files_and_retries_stop_on_shutdown(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            current = owner.directory / 'current'
            current.mkdir(parents=True)
            owner.active_directory = current
            app.close.side_effect = [RuntimeError('stop failed'), None]
            app.command.return_value = {'status': 'state', 'state': {}}
            owner.current = lambda _: True
            candidate = Mock()
            with patch('supervisor.run', return_value=json.dumps({'BuildsFinished': {'client': {}}})), \
                 patch('supervisor.select_tip_artifacts', return_value={}), \
                 patch('supervisor.Application', return_value=candidate):
                with self.assertRaisesRegex(RuntimeError, 'stop failed'):
                    owner.rebuild(files)
            self.assertTrue(current.exists())
            self.assertIs(owner.application, candidate)
            owner.close()
            self.assertFalse(current.exists())
            self.assertEqual(app.close.call_count, 2)

    def test_private_snapshot_preserves_bytes_modes_and_internal_links(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'source'
            root.mkdir()
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True)
            (root / 'data').mkdir()
            (root / 'data/value').write_bytes(b'\x00\xffbinary')
            (root / 'data/value').chmod(0o755)
            try:
                (root / 'link').symlink_to('data', target_is_directory=True)
            except OSError:
                self.skipTest('Host cannot create symlinks')
            files = capture(root)
            copied = Path(directory) / 'build'
            stage(copied, files)
            self.assertEqual((copied / 'link/value').read_bytes(), b'\x00\xffbinary')
            self.assertTrue((copied / 'link').is_symlink())
            (root / 'escape').symlink_to(Path(directory))
            with self.assertRaises(ValueError):
                capture(root)

    def exercise(self, directory):
        root = Path(directory)
        files = {PROGRAM: (b'old body', 0o644), SAMPLE + 'View.xaml': (b'old markup', 0o644)}
        stage(root, files)
        with patch('supervisor.verify_toolchain'):
            owner = Supervisor(root, root / 'builds', 'dx')
        app = Mock()
        app.root, app.files, app.client, app.aslr = root, files, {'rustc_envs': []}, 123
        owner.application = app
        return owner, app, files

    def test_markup_changes_never_invoke_compiler(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            changed = dict(files, **{SAMPLE + 'View.xaml': (b'new markup', 0o644)})
            app.command.return_value = {'status': 'applied'}
            with patch('supervisor.run') as compiler:
                owner.update(changed)
                compiler.assert_not_called()
            self.assertEqual(app.command.call_args.args[0]['op'], 'markup')
            self.assertIs(app.files, files)

    def test_superseded_patch_is_cancelled_without_application(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            changed = dict(files, **{PROGRAM: (b'new body', 0o644), SAMPLE + 'View.xaml': (b'new markup', 0o644)})
            app.command.return_value = {'status': 'admitted', 'request': 1}
            owner.current = lambda _: False
            with patch('supervisor.run', return_value='unused'):
                owner.update(changed)
            messages = [call.args[0] for call in app.command.call_args_list]
            self.assertEqual([message['op'] for message in messages], ['admit', 'cancel'])
            self.assertEqual(messages[0]['sources']['View.xaml'], admission_snapshot(files)['View.xaml'])
            self.assertEqual(messages[0]['markup']['ferres://hot_reload_demo/View.xaml'], 'new markup')
            self.assertEqual(app.files, files)

    def test_build_failure_does_not_advance_running_baseline(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            changed = dict(files, **{PROGRAM: (b'new body', 0o644)})
            app.command.return_value = {'status': 'admitted', 'request': 1}
            with patch('supervisor.run', side_effect=RuntimeError('compiler failed')):
                with self.assertRaises(RuntimeError):
                    owner.update(changed)
            self.assertIs(owner.application, app)
            self.assertEqual(admission_snapshot(app.files), admission_snapshot(files))
            self.assertEqual(app.command.call_args.args[0]['op'], 'cancel')

    def test_incompatible_change_rebuilds_without_patch_compilation(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            changed = dict(files, **{SAMPLE + 'lib.rs': (b'new layout', 0o644)})
            app.command.return_value = {'status': 'rejected', 'error': 'layout changed'}
            owner.rebuild = Mock(side_effect=RuntimeError('replacement build failed'))
            with patch('supervisor.run') as compiler:
                with self.assertRaises(RuntimeError):
                    owner.update(changed)
                compiler.assert_not_called()
            owner.rebuild.assert_called_once_with(changed)
            self.assertIs(owner.application, app)

    def test_combined_markup_restart_cancels_patch_before_rebuild(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            changed = dict(files, **{PROGRAM: (b'new body', 0o644)})
            app.command.side_effect = [
                {'status': 'admitted', 'request': 1},
                {'status': 'restart_required'}, {'status': 'cancelled'},
            ]
            owner.current = lambda _: True
            def rebuild(revision):
                self.assertEqual(revision, changed)
                self.assertEqual(app.command.call_args.args[0]['op'], 'cancel')
            owner.rebuild = Mock(side_effect=rebuild)
            with patch('supervisor.run', return_value=json.dumps({'Hotpatch': {'jump_table': {}}})):
                owner.update(changed)
            owner.rebuild.assert_called_once()
            self.assertEqual(app.files, files)

    def test_only_registered_source_is_transported_as_source_text(self):
        files = {PROGRAM: (b'fn counter_click() {}', 0o644), SAMPLE + 'lib.rs': (b'stable type layout', 0o644)}
        snapshot = admission_snapshot(files)
        self.assertEqual(snapshot['program.rs'], 'fn counter_click() {}')
        self.assertTrue(snapshot['lib.rs'].startswith('sha256:'))
        changed = dict(files, **{SAMPLE + 'lib.rs': (b'edited type layout', 0o644)})
        self.assertNotEqual(admission_snapshot(changed)['lib.rs'], snapshot['lib.rs'])

    def test_markup_watch_avoids_compiler_input_reads_and_rejects_races(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True)
            files = {PROGRAM: (b'code', 0o644), SAMPLE + 'View.xaml': (b'old', 0o644)}
            stage(root, files)
            before = stamps(root)
            (root / SAMPLE / 'View.xaml').write_text('new markup')
            after = stamps(root)
            with patch('supervisor.stable_capture', side_effect=AssertionError('compiler inputs reread')):
                current = watched_revision(root, files, before, after)
            self.assertEqual(current[PROGRAM], files[PROGRAM])
            self.assertEqual(current[SAMPLE + 'View.xaml'][0], b'new markup')
            (root / SAMPLE / 'View.xaml').write_text('newer markup')
            with self.assertRaises(ValueError):
                watched_revision(root, files, before, after)

    def test_failed_and_superseded_startups_thaw_the_running_application(self):
        for failure in ['startup', 'superseded', 'verification']:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                owner, app, files = self.exercise(directory)
                app.process.poll.return_value = None
                app.command.return_value = {'status': 'state', 'state': {'counter': 7}}
                candidate = Mock()
                owner.current = Mock(side_effect=[True, False] if failure == 'superseded' else
                    [True, RuntimeError('source read failed')] if failure == 'verification' else [True])
                with patch('supervisor.stage'), patch('supervisor.run', return_value=json.dumps({'BuildsFinished': {'client': {}}})), patch('supervisor.select_tip_artifacts', return_value={}):
                    with patch('supervisor.Application', side_effect=RuntimeError('candidate failed') if failure == 'startup' else None, return_value=candidate):
                        if failure == 'superseded':
                            self.assertFalse(owner.rebuild(files))
                        else:
                            with self.assertRaises(RuntimeError):
                                owner.rebuild(files)
                self.assertIs(owner.application, app)
                self.assertEqual([call.args[0]['op'] for call in app.command.call_args_list], ['prepare_restart', 'resume'])
                app.close.assert_not_called()
                if failure != 'startup':
                    candidate.close.assert_called_once()

    def test_acknowledged_replacement_retires_the_frozen_application(self):
        with tempfile.TemporaryDirectory() as directory:
            owner, app, files = self.exercise(directory)
            app.command.return_value = {'status': 'state', 'state': {'counter': 7}}
            candidate = Mock()
            owner.current = lambda _: True
            with patch('supervisor.stage'), patch('supervisor.run', return_value=json.dumps({'BuildsFinished': {'client': {}}})), patch('supervisor.select_tip_artifacts', return_value={}), patch('supervisor.Application', return_value=candidate) as launch:
                self.assertTrue(owner.rebuild(files))
                self.assertEqual(launch.call_args.args[-1], {'counter': 7})
            self.assertIs(owner.application, candidate)
            self.assertEqual([call.args[0]['op'] for call in app.command.call_args_list], ['prepare_restart'])
            app.close.assert_called_once()
            candidate.close.assert_not_called()


if __name__ == '__main__':
    unittest.main()
