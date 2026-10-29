#!/usr/bin/env python3
"""Native root-binding replacement, rejection and recovery in ControlCatalog."""
import os
from pathlib import Path
import subprocess
import tempfile

from catalog_probe import ROOT, frames, save_and_render, wait_for
from resource_probe import assert_identity, states


def accepted(text, width, height):
    revision = frames(text)[-1]['revision']
    matches = [state for state in states(text)
               if state['revision'] == revision and state['status'] == 'Applied'
               and state.get('root_width') == width and state.get('root_height') == height]
    if len(matches) != 1:
        raise RuntimeError(f'Unexpected preserved root binding values: {states(text)[-6:]}')
    return matches[0]


def main():
    binary = Path(os.environ.get('FERROUI_CATALOG_DEMO', str(ROOT / 'target/debug' /
        ('control-catalog-desktop.exe' if os.name == 'nt' else 'control-catalog-desktop'))))
    original = (ROOT / 'samples/ControlCatalog/Pages/BorderPage.xaml').read_text(encoding='utf-8')
    if '<ContentPage ' not in original:
        raise RuntimeError('Expected ContentPage root in BorderPage fixture')

    def source(width, height):
        return original.replace('<ContentPage ', f'<ContentPage Width="{width}" Height="{height}" ', 1)

    binding = '{Binding Height, RelativeSource={RelativeSource Self}}'
    with tempfile.TemporaryDirectory(prefix='ferroui-root-binding-') as directory:
        directory = Path(directory)
        page = directory / 'Pages/BorderPage.xaml'
        page.parent.mkdir()
        page.write_text(original, encoding='utf-8')
        log = directory / 'catalog.log'
        env = dict(os.environ, FERROUI_HOT_RELOAD_ROOT=str(directory),
                   FERROUI_RELOAD_TIMINGS='1', FERROUI_RELOAD_TARGET_STATE='1',
                   FERROUI_SMOKE_SCREENSHOTS=str(directory / 'screenshots'),
                   FERROUI_SMOKE_SCREENSHOT_PAGES='Border', FERROUI_SMOKE_PAGES='15000',
                   FERROUI_SMOKE_EXIT_MS='45000')
        with log.open('w', encoding='utf-8') as output:
            process = subprocess.Popen([str(binary)], env=env, stdout=output, stderr=output)
            try:
                wait_for(process, log, lambda text: 'Catalog reload tracking: /Pages/BorderPage.xaml' in text)
                before = accepted(save_and_render(process, log, page, source('280', 360)), 280, 360)
                bound = accepted(save_and_render(process, log, page, source(binding, 360)), 360, 360)
                assert_identity(before, bound)
                changed = accepted(save_and_render(process, log, page, source(binding, 420), atomic=True), 420, 420)
                assert_identity(bound, changed)
                for invalid, status in [
                    (source('not a number', 420), 'Failed'),
                    (source('{Binding Height, RelativeSource={RelativeSource Self}, Mode=OneWayToSource}', 420),
                     'RestartRequired'),
                ]:
                    text = log.read_text(encoding='utf-8', errors='replace')
                    count = text.count(f'status: {status}')
                    page.write_text(invalid, encoding='utf-8')
                    text = wait_for(process, log, lambda text: text.count(f'status: {status}') > count)
                    rejected = [state for state in states(text)
                                if state['status'] == status and state['root'] == changed['root']][-1]
                    assert_identity(changed, rejected)
                    if (rejected['content'] != changed['content'] or rejected['root_width'] != 420
                            or rejected['root_height'] != 420):
                        raise RuntimeError(f'Rejected root binding damaged last-good view: {rejected}')
                recovered = accepted(save_and_render(process, log, page, source(binding, 430), atomic=True), 430, 430)
                assert_identity(changed, recovered)
                cleared = accepted(save_and_render(process, log, page, original), None, None)
                assert_identity(recovered, cleared)
                print(f'PASS: native root bindings PID {process.pid}: replacement, self source, rejection, recovery and removal preserve window/root/model', flush=True)
            except BaseException:
                print(log.read_text(encoding='utf-8', errors='replace'), flush=True)
                raise
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()


if __name__ == '__main__':
    main()
