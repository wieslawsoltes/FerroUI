"""Inclusive sample counts per function of one thread, from a macOS `sample` call graph.

    /usr/bin/sample <pid> 10 -file run.sample
    python3 scripts/vello-perf/inclusive.py run.sample <thread name pattern> [function pattern]

With a benchmark: the thread of the test (for example frame_benchmark_buttons_page).
"""
import sys, re, collections
path, thread_pat = sys.argv[1], sys.argv[2]
pat = sys.argv[3] if len(sys.argv) > 3 else ""
lines = open(path, errors="replace").read().split("\n")
counts = collections.Counter(); total = 0; in_thread = False
for line in lines:
    m = re.match(r"^\s{4}(\d+) Thread_\S+\s*(.*)", line)
    if m:
        in_thread = thread_pat in line; 
        if in_thread: total = int(m.group(1))
        continue
    if line.startswith("Total number") or line.startswith("Sort by"): in_thread = False
    if not in_thread: continue
    m = re.match(r"^[\s+!:|]*?(\d+) (.+?)  \(in ", line)
    if not m: continue
    name = re.sub(r"::h[0-9a-f]{16}$", "", m.group(2))
    counts[name] = max(counts[name], 0)
    counts[name] += int(m.group(1))
print("thread total", total)
for name, c in counts.most_common(400):
    if pat in name:
        print(f"{c:6d} {name[:170]}")
