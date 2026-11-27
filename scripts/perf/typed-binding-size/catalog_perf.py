"""Time matched desktop Catalog processes through the after-setup marker or real page Loaded events."""
import argparse
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import time
import threading

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('mode', choices=['startup', 'navigation', 'frames'])
parser.add_argument('stock', type=Path)
parser.add_argument('candidate', type=Path)
parser.add_argument('output', type=Path)
parser.add_argument('--pairs', type=int, default=21)
args = parser.parse_args()
if args.pairs < 1:
    parser.error('--pairs must be positive')
args.output.mkdir(parents=True, exist_ok=True)
binaries = {'stock': args.stock.resolve(), 'candidate': args.candidate.resolve()}
base_env = {k: v for k, v in os.environ.items() if not k.startswith(('FERROUI_CATALOG_', 'FERROUI_SMOKE_'))}
base_env.update(FERROUI_CATALOG_THEME='winui', FERROUI_CATALOG_THEME_VARIANT='light', FERROUI_SMOKE_RENDERING='metal')
pages = ['ListBox', 'TreeDataGrid', 'Data Validation']
sequence = [p for visit in range(2) for page in pages for p in (page, 'Home')]
if args.mode in ('navigation', 'frames'):
    base_env.update(FERROUI_CATALOG_NAV_TIMING='1', FERROUI_SMOKE_PAGE_SEQUENCE=','.join(sequence),
                    FERROUI_SMOKE_PAGES='350', FERROUI_SMOKE_EXIT_MS=str(350 * (len(sequence) + 2) + 850))
else:
    base_env['FERROUI_SMOKE_EXIT_MS'] = '180'
if args.mode == 'frames':
    base_env['FERROUI_SLOW_FRAME_MS'] = '0'
frame_pattern = re.compile(r'slow frame of composition target (\d+): frame (\d+) took ([0-9.]+) ms$')
pattern = re.compile(r'Navigation timing: (.*?) \| (.*?) \| ([0-9.]+) ms$')
rows = []
for pair in range(-1, args.pairs):
    arms = ['stock', 'candidate'] if pair % 2 == 0 else ['candidate', 'stock']
    for arm in arms:
        start = time.perf_counter_ns()
        marker = None
        with subprocess.Popen([binaries[arm]], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              env=base_env, text=True) as proc:
            lines = []
            deadline = threading.Timer(45, proc.kill)
            deadline.start()
            try:
                for line in proc.stdout:
                    if marker is None and 'ControlCatalog: the theme variant Light' in line:
                        marker = time.perf_counter_ns() - start
                    lines.append(line)
                assert proc.wait() == 0
            finally:
                deadline.cancel()
        (args.output / f'{pair:02}-{arm}.log').write_text(''.join(lines))
        assert marker is not None, (pair, arm)
        visits = []
        for line in lines:
            # Compositor diagnostics on stderr can precede an intact stdout marker
            # on the same line when both streams are captured into one pipe.
            m = pattern.search(line.rstrip())
            if m:
                page, phase, ms = m.groups()
                if phase == 'assets-ready':
                    visits.append({'page': page})
                next(v for v in reversed(visits) if v['page'] == page)[phase] = float(ms)
        if args.mode in ('navigation', 'frames'):
            assert [v['page'] for v in visits] == ['Home', *sequence], (pair, arm, visits)
            assert all('loaded' in v for v in visits), (pair, arm, visits)
        if pair >= 0:
            row = {'arm': arm, 'pair': pair, 'spawn_to_after_setup_ns': marker, 'visits': visits}
            if args.mode == 'frames':
                selected = False
                draws = []
                for line in lines:
                    if line.startswith('Selecting '):
                        selected = True
                    if line.startswith('Timer fired:'):
                        break
                    frame = frame_pattern.fullmatch(line.strip()) if selected else None
                    if frame:
                        draws.append(float(frame[3]))
                assert draws, (pair, arm)
                row['post_selection_render_frame_ms'] = draws
            rows.append(row)
    (args.output / 'samples.json').write_text(json.dumps(rows, separators=(',', ':'))+'\n')
summary = {}
for arm in binaries:
    arm_rows = [r for r in rows if r['arm'] == arm]
    summary[arm] = {'spawn_to_after_setup_ms': statistics.median(r['spawn_to_after_setup_ns'] for r in arm_rows) / 1e6}
    if args.mode in ('navigation', 'frames'):
        summary[arm]['navigation_ms'] = {}
        for page in pages:
            visits = [[v for v in r['visits'] if v['page'] == page] for r in arm_rows]
            summary[arm]['navigation_ms'][page] = {str(i + 1): {
                phase: statistics.median(v[i][phase] for v in visits) for phase in ['constructed','host-returned','loaded']
            } for i in range(2)}
    if args.mode == 'frames':
        draws = [r['post_selection_render_frame_ms'] for r in arm_rows]
        summary[arm]['total_draws'] = sum(map(len, draws))
        summary[arm]['median_of_process_frame_medians_ms'] = statistics.median(statistics.median(v) for v in draws)
        summary[arm]['median_of_process_frame_p95_ms'] = statistics.median(
            statistics.quantiles(v, n=100, method='inclusive')[94] for v in draws)
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2)+'\n')
print(json.dumps(summary))
