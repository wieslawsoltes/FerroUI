"""Where the GPU time of a process goes, from a Metal System Trace.

    xcrun xctrace record --template 'Metal System Trace' --time-limit 120s --output run.trace \
        --env FERROUI_TEST_RENDERER=vello-hybrid --env FERROUI_BENCH_SURFACE=metal --env FERROUI_BENCH_SCALING=2 \
        --launch -- <catalog test binary> frame_benchmark_buttons_page --ignored --nocapture --test-threads=1
    xcrun xctrace export --input run.trace \
        --xpath '/trace-toc/run[@number="1"]/data/table[@schema="metal-gpu-intervals"]' > gpu.xml
    python3 scripts/vello-perf/gpu_intervals.py gpu.xml <process name, e.g. control_catalog>

Prints the fragment time per second of the trace and, per encoder label and GPU channel, the count
and the mean and median duration: divide by the number of frames for the time per frame.
"""
import sys, re, collections, statistics
import xml.etree.ElementTree as ET
path, proc = sys.argv[1], sys.argv[2]
ids = {}
def val(el):
    if el is None: return None
    ref = el.get('ref')
    if ref is not None: return ids.get(ref)
    v = (el.get('fmt'), el.text, el)
    if el.get('id') is not None: ids[el.get('id')] = v
    for child in el: val(child)
    return v
rows = []
for row in ET.parse(path).getroot().iter('row'):
    vals = [val(c) for c in row]
    label = vals[6][0] if vals[6] else ''
    if proc not in label: continue
    rows.append((int(vals[0][1]), int(vals[1][1]), vals[2][0], re.sub(r'\s+\(.*$', '', label).strip()))
rows.sort()
t0 = rows[0][0]
# group by second for a timeline of GPU busy
busy = collections.Counter()
for s, d, ch, l in rows:
    if ch == 'Fragment': busy[(s - t0)//1_000_000_000] += d
print('fragment ms per second of trace:', [round(busy[k]/1e6,1) for k in sorted(busy)])
by = collections.defaultdict(list)
for s, d, ch, l in rows:
    by[(ch, l)].append(d)
for (ch, l), ds in sorted(by.items(), key=lambda kv: -sum(kv[1]))[:12]:
    print(f'{ch:9s} n={len(ds):5d} mean={statistics.mean(ds)/1e3:8.1f}us median={statistics.median(ds)/1e3:8.1f}us  {l}')
