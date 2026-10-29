#!/usr/bin/env python3
"""Native declarative-page reload in ControlCatalog; uses isolated source copies.

First build: cargo build -p control-catalog-desktop --features hot-reload
"""
import os
import json
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def frames(text):
    values = []
    for line in text.splitlines():
        if line.startswith('FERROUI_RELOAD_FRAME='):
            try:
                values.append(json.loads(line.split('=', 1)[1]))
            except json.JSONDecodeError:
                pass  # The render thread may still be writing this line.
    return values


# The wall clock brackets the render time the application reports from its own, precise clock.
# Before Python 3.13 the clock of Windows advances in ticks of 15.6 ms, so a render a few
# milliseconds before the end of the bracket can read as after it: the bracket is one tick wider.
WALL_CLOCK_TICK_NS = int(time.get_clock_info('time').resolution * 1e9)


def save_and_render(process, log, page, source, atomic=False):
    previous = frames(log.read_text(encoding='utf-8', errors='replace'))
    before = len(previous)
    started = time.time_ns()
    counter = time.perf_counter_ns()
    destination = page.with_suffix('.tmp') if atomic else page
    destination.write_text(source, encoding='utf-8')
    if atomic:
        destination.replace(page)
    text = wait_for(process, log, lambda text: len(frames(text)) > before)
    frame = frames(text)[before]
    if not any(uri.endswith('/' + page.name) for uri in frame['documents']):
        raise RuntimeError('Render completion belongs to another document')
    if previous and previous[-1]['batch'] == frame['batch']:
        raise RuntimeError('Coalesced revisions cannot be measured as separate frames')
    elapsed = time.perf_counter_ns() - counter
    wall_elapsed = time.time_ns() - started
    if abs(elapsed - wall_elapsed) > 100_000_000 + WALL_CLOCK_TICK_NS:
        raise RuntimeError('Wall clock changed during frame measurement')
    save_to_render = int(frame['rendered_unix_ns']) - started
    if save_to_render < 0 or save_to_render > wall_elapsed + WALL_CLOCK_TICK_NS:
        raise RuntimeError('Render completion does not belong to this save')
    result = dict(frame, page=page.stem, save_to_render_us=save_to_render / 1000,
                  atomic_save=atomic)
    print('FERROUI_RELOAD_MEASUREMENT=' + json.dumps(result), flush=True)
    return text


def wait_for(process, log, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = log.read_text(encoding="utf-8", errors="replace")
        if predicate(text):
            return text
        if process.poll() is not None:
            raise RuntimeError(f'Catalog exited ({process.returncode}):\n{text[-6000:]}')
        time.sleep(0.025)
    raise RuntimeError(f'Catalog reload timed out:\n{log.read_text(encoding="utf-8", errors="replace")[-6000:]}')


def run_page(name):
    binary = Path(os.environ.get('FERROUI_CATALOG_DEMO', str(ROOT / 'target/debug' /
        ('control-catalog-desktop.exe' if os.name == 'nt' else 'control-catalog-desktop'))))
    source = (ROOT / f'samples/ControlCatalog/Pages/{name}Page.xaml').read_text(encoding='utf-8')
    with tempfile.TemporaryDirectory(prefix='ferroui-catalog-reload-') as directory:
        directory = Path(directory)
        page = directory / f'Pages/{name}Page.xaml'
        page.parent.mkdir()
        page.write_text(source, encoding='utf-8')
        log = directory / 'catalog.log'
        env = dict(os.environ, FERROUI_HOT_RELOAD_ROOT=str(directory),
                   FERROUI_RELOAD_TIMINGS='1',
                   FERROUI_SMOKE_SCREENSHOTS=str(directory / 'screenshots'),
                   FERROUI_SMOKE_SCREENSHOT_PAGES=name, FERROUI_SMOKE_PAGES='15000',
                   FERROUI_SMOKE_EXIT_MS='45000')
        with log.open('w', encoding='utf-8') as output:
            process = subprocess.Popen([str(binary)], env=env, stdout=output, stderr=output)
            try:
                wait_for(process, log, lambda text: 'Catalog reload tracking:' in text)
                save_and_render(process, log, page, source.replace('<ContentPage ', '<ContentPage Width="700" ', 1))
                failures = log.read_text(encoding="utf-8", errors="replace").count('status: Failed')
                page.write_text('<ContentPage', encoding='utf-8')
                wait_for(process, log, lambda text: text.count('status: Failed') > failures)
                save_and_render(process, log, page, source.replace('<ContentPage ', '<ContentPage Width="710" ', 1), atomic=True)
                text = save_and_render(process, log, page, source)
                print('\n'.join(line for line in text.splitlines() if 'Catalog reload' in line), flush=True)
                print(f'PASS: ControlCatalog PID {process.pid} reloaded {name}, recovered from invalid markup, and removed an owned root attribute', flush=True)
            finally:
                if process.poll() is None:
                    process.terminate()
                process.wait(timeout=10)


if __name__ == '__main__':
    for name in os.environ.get('FERROUI_CATALOG_RELOAD_PAGES', 'Border,Slider,ProgressBar,CheckBox,Canvas,RadioButton,TextBlock,Viewbox,Expander,ListBox,AdornerLayer').split(','):
        run_page(name)
