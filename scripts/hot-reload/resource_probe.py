#!/usr/bin/env python3
"""Native application-resource revision and static-consumer acceptance in ControlCatalog."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

from catalog_probe import ROOT, frames, save_and_render, wait_for


def states(text):
    result = []
    for line in text.splitlines():
        if line.startswith('FERROUI_RELOAD_TARGET_STATE='):
            try:
                result.append(json.loads(line.split('=', 1)[1]))
            except json.JSONDecodeError:
                pass
    return result


def accepted_state(text, width):
    revision = frames(text)[-1]['revision']
    matches = [state for state in states(text)
               if state['revision'] == revision and state['status'] == 'Applied'
               and state['content_width'] == width]
    if len(matches) != 1:
        raise RuntimeError(f'Expected exactly one static consumer with width {width}: {states(text)[-6:]}')
    return matches[0]


def assert_identity(before, after):
    for field in ('root', 'window', 'model'):
        if before[field] != after[field]:
            raise RuntimeError(f'Resource revision changed {field}: {before} -> {after}')
    if after['window'] == '0x0':
        raise RuntimeError('Reload target is not attached to a native window')
    if after['model'] == '0x0':
        raise RuntimeError('Fixture has no view model to preserve')


def main():
    binary = Path(os.environ.get('FERROUI_CATALOG_DEMO', str(ROOT / 'target/debug' /
        ('control-catalog-desktop.exe' if os.name == 'nt' else 'control-catalog-desktop'))))
    original_page = (ROOT / 'samples/ControlCatalog/Pages/BorderPage.xaml').read_text(encoding='utf-8')
    original_resources = (ROOT / 'samples/ControlCatalog/CustomThemes.xaml').read_text(encoding='utf-8')
    page_source = original_page.replace('<ScrollViewer', '<ScrollViewer Width="{StaticResource HotReloadProbeWidth}"', 1)
    if page_source == original_page:
        raise RuntimeError('Border page fixture no longer contains the expected content root')

    def resources(width):
        head, tail = original_resources.rsplit('</ResourceDictionary>', 1)
        return head + f'<x:Double x:Key="HotReloadProbeWidth">{width}</x:Double></ResourceDictionary>' + tail

    with tempfile.TemporaryDirectory(prefix='ferroui-resource-reload-') as directory:
        directory = Path(directory)
        page = directory / 'Pages/BorderPage.xaml'
        page.parent.mkdir()
        page.write_text(original_page, encoding='utf-8')
        resource = directory / 'CustomThemes.xaml'
        resource.write_text(original_resources, encoding='utf-8')
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
                save_and_render(process, log, resource, resources(611))
                before = accepted_state(save_and_render(process, log, page, page_source), 611)
                changed = accepted_state(save_and_render(process, log, resource, resources(612), atomic=True), 612)
                assert_identity(before, changed)
                if before['theme'] == changed['theme'] or changed['theme'] == '0x0':
                    raise RuntimeError('Preserved root did not adopt the candidate static theme')
                if before['content'] == changed['content']:
                    raise RuntimeError('Static consumer was not rebuilt for the resource revision')
                failures = log.read_text(encoding='utf-8', errors='replace').count('status: Failed')
                resource.write_text(original_resources, encoding='utf-8')
                text = wait_for(process, log, lambda text: text.count('status: Failed') > failures)
                failed = [state for state in states(text) if state['status'] == 'Failed' and state['root'] == changed['root']][-1]
                assert_identity(changed, failed)
                if (failed['content'] != changed['content'] or failed['content_width'] != 612
                        or failed['theme'] != changed['theme']):
                    raise RuntimeError('Missing resource damaged the last-good static consumer')
                recovered = accepted_state(save_and_render(process, log, resource, resources(613), atomic=True), 613)
                assert_identity(changed, recovered)
                if recovered['theme'] == changed['theme']:
                    raise RuntimeError('Recovery did not refresh the root static theme')
                save_and_render(process, log, page, original_page)
                save_and_render(process, log, resource, original_resources)
                print('FERROUI_RESOURCE_OBSERVATION=' + json.dumps({
                    'before': before, 'changed': changed, 'failed': failed, 'recovered': recovered,
                }), flush=True)
                print(f'PASS: native resource reload PID {process.pid}: static consumer and root theme updated, missing key retained last-good view, atomic recovery preserved root/window/model', flush=True)
            except BaseException:
                print(log.read_text(encoding='utf-8', errors='replace'), flush=True)
                raise
            finally:
                if process.poll() is None:
                    process.terminate()
                process.wait(timeout=10)


if __name__ == '__main__':
    main()
