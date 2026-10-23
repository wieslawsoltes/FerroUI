# Benchmarks and profiles of the render backends

The tools of the work recorded in `docs/porting/vello-performance.md` (section 4 has the method and
the rules: one build, every renderer in one session, Skia first and last).

| Script | What it does |
|---|---|
| `bench.sh <outdir> <renderers> [benchmarks]` | Builds the frame benchmarks of the catalog once and runs the test binary for each renderer and benchmark (`FERROUI_BENCH_SURFACE=metal` by default, `raster` for software) |
| `bench-bin.sh <outdir> <label>=<binary>=<renderer> ...` | Runs prebuilt binaries in one session: an A/B of two builds |
| `table.py <outdir> <labels>` | The logs of both as a Markdown table: median ms a frame with the device waited for / without |
| `inclusive.py <sample file> <thread> [function]` | Inclusive samples per function of a thread of a macOS `sample` profile |
| `gpu_intervals.py <exported xml> <process>` | GPU time per encoder from a Metal System Trace (the commands to record and export are in the script) |
| `vm-linux/build.sh <branch>`, `wait-build.sh`, `run.sh <name> [VAR=value ...]`, `times.sh` | In the Linux virtual machine: build the desktop catalog from a branch, run the catalog smoke run on X11 with frame times and a screenshot of the window, print the times |
| `vm-windows/build.ps1`, `vm-windows/run.ps1` | In the Windows virtual machine: build from sources synced by `scripts/windows/vm-sync.sh`, run the smoke run in the session of the logged-on user through a scheduled task |

Example, every renderer on the Metal surface:

```sh
scripts/vello-perf/bench.sh /tmp/bench skia,vello-hybrid,vello-gpu,vello-cpu,skia2
python3 scripts/vello-perf/table.py /tmp/bench skia,vello-hybrid,vello-gpu,vello-cpu,skia2
```

Build output belongs on a volume with room: set `CARGO_TARGET_DIR` before running `bench.sh`.
