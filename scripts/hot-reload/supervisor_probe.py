#!/usr/bin/env python3
"""Exercise the owning supervisor against a private, editable copy of the workspace."""
import os
import json
from pathlib import Path
import subprocess
import tempfile
import time

from supervisor import ROOT, PROGRAM, SAMPLE, Supervisor, stable_capture, stage
from catalog_probe import frames, wait_for


def main():
    os.environ['FERROUI_RELOAD_TIMINGS'] = '1'
    os.environ['FERROUI_TRACE_CLICKS'] = '1'
    original = stable_capture(ROOT)
    # Skia invokes Windows curl with a cache path below the staged target tree.
    # Keep that path below MAX_PATH even with the default per-user temp directory.
    with tempfile.TemporaryDirectory(prefix='fr-supervisor-') as directory:
        directory = Path(directory)
        source = directory / 'editor'
        stage(source, original)
        subprocess.run(['git', 'init', '-q'], cwd=source, check=True)
        # The verification workspace must keep its byte-exact source baseline,
        # regardless of the developer's global Windows checkout settings.
        subprocess.run(['git', 'config', 'core.autocrlf', 'false'], cwd=source, check=True)
        subprocess.run(['git', 'config', 'core.eol', 'lf'], cwd=source, check=True)
        subprocess.run(['git', 'add', '-f', '.'], cwd=source, check=True)
        owner = Supervisor(source, directory / 'builds', os.environ.get('FERROUI_DX', 'dx'))
        try:
            assert owner.rebuild(stable_capture(source))
            app = owner.application
            initial = app.command({'op': 'click'})
            assert initial['counter'] == 1, initial
            frozen = app.command({'op': 'prepare_restart'})
            assert frozen['state']['application_state']['counter'] == 1
            assert frozen['interaction_enabled'] is False
            assert app.command({'op': 'click'})['status'] == 'rejected'
            assert app.command({'op': 'markup', 'markup': {}})['status'] == 'rejected'
            assert app.command({'op': 'cancel'})['status'] == 'cancelled'
            assert app.command({'op': 'state'})['interaction_enabled'] is True
            app.command({'op': 'prepare_restart'})
            assert app.command({'op': 'resume'})['status'] == 'resumed'
            assert app.command({'op': 'state'})['state']['application_state']['counter'] == 1
            view = source / SAMPLE / 'View.xaml'
            view_source = view.read_text(encoding='utf-8')
            view.write_text(view_source.replace('Spacing="12"', 'Spacing="16"'), encoding='utf-8')
            owner.update(stable_capture(source))
            current = app.command({'op': 'click'})
            assert current['spacing'] == 16 and current['counter'] == 2, current
            assert owner.generation == 1, 'Markup-only change launched a build'
            view.write_text('<', encoding='utf-8')
            owner.update(stable_capture(source))
            current = app.command({'op': 'click'})
            assert current['spacing'] == 16 and current['counter'] == 3, current
            program = source / PROGRAM
            program_source = program.read_text(encoding='utf-8')
            save_started = time.time_ns()
            counter_started = time.perf_counter_ns()
            program.write_text(program_source.replace('hot_reload_demo::increment(1)', 'hot_reload_demo::increment(2)'), encoding='utf-8')
            view.write_text(view_source.replace('Spacing="12"', 'Spacing="24"'), encoding='utf-8')
            capture_started = time.perf_counter_ns()
            revision = stable_capture(source)
            capture_us = (time.perf_counter_ns() - capture_started) / 1000
            result = owner.update(revision)
            assert result['status'] == 'applied', result
            log = app.directory / 'application.log'
            text = wait_for(app.process, log, lambda text: any(
                frame['revision'] == result['markup_revision'] for frame in frames(text)))
            frame = next(frame for frame in frames(text) if frame['revision'] == result['markup_revision'])
            wall_elapsed = time.time_ns() - save_started
            # One tick of the wall clock of the probe, as in catalog_probe.save_and_render.
            tick = int(time.get_clock_info('time').resolution * 1e9)
            if abs(wall_elapsed - (time.perf_counter_ns() - counter_started)) > 100_000_000 + tick:
                raise RuntimeError('Wall clock changed during patch measurement')
            rendered_elapsed = int(frame['rendered_unix_ns']) - save_started
            assert 0 <= rendered_elapsed <= wall_elapsed + tick, frame
            print('FERROUI_PATCH_MEASUREMENT=' + json.dumps({
                'scope': 'combined-rust-xaml-save-to-render',
                'source_capture_us': capture_us,
                'patch_timing': result['patch_timing'],
                'frame': frame,
                'save_to_render_us': rendered_elapsed / 1000,
                'watch_debounce_included': False,
                'compiler': 'rust-1.93.0-dx-0.7.10',
            }), flush=True)
            current = app.command({'op': 'click'})
            assert current['counter'] == 5 and current['spacing'] == 24, current
            assert (current['host'], current['type']) == (initial['host'], initial['type'])
            program.write_text('not valid Rust', encoding='utf-8')
            try:
                owner.update(stable_capture(source))
            except RuntimeError as error:
                print(f'EXPECTED_REPLACEMENT_BUILD_FAILURE={error}', flush=True)
            else:
                raise AssertionError('Invalid replacement unexpectedly built')
            assert owner.application is app and app.process.poll() is None
            assert not (owner.directory / '2').exists(), 'Failed build directory was retained'
            assert (owner.directory / '1').exists(), 'Running build files were removed'
            current = app.command({'op': 'click'})
            assert current['counter'] == 7, current
            # A library edit requires a full replacement. The old app keeps running
            # through compilation; its latest state is captured only after success.
            program.write_text(program_source, encoding='utf-8')
            library = source / SAMPLE / 'lib.rs'
            library.write_text(library.read_text(encoding='utf-8') + '\n// Supervisor restart acceptance revision.\n', encoding='utf-8')
            owner.update(stable_capture(source))
            replacement = owner.application
            assert replacement is not app and app.process.poll() is not None
            assert not (owner.directory / '1').exists(), 'Retired build directory was retained'
            assert (owner.directory / '3').exists(), 'Replacement build files were removed'
            # Keep click as the first command after startup; a preceding state
            # query must not hide a startup/order-dependent counter failure.
            current = replacement.command({'op': 'click'})
            restored = replacement.command({'op': 'state'})
            print('FERROUI_RESTART_OBSERVATION=' + json.dumps({
                'captured': replacement.startup_state,
                'first_click': current,
                'restored': restored,
            }), flush=True)
            expected = json.loads(json.dumps(replacement.startup_state))
            assert expected['application_state']['counter'] == 7, expected
            expected['application_state']['counter'] = 8
            assert restored['state'] == expected, restored
            assert current['counter'] == 8 and current['spacing'] == 24, current
            print('PASS: supervisor markup, admitted combined patch, failed build retention, acknowledged stateful restart and generation cleanup', flush=True)
        finally:
            owner.close()
        assert not list(owner.directory.iterdir()), 'Shutdown retained owned build generations'


if __name__ == '__main__':
    main()
