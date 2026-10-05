# Task: desktop binary size and start-up time

Branch: `desktop-perf` (it exists; continue on it, rebased on `main`). Pull request title: `Desktop: binary size and start-up pass`.

Read `docs/porting/desktop-performance.md` first: it is the measurement report of the previous worker (per-component size tables, the start-up timeline to the first presented frame, the theme-load split into parse, transform and interpret, and its ranked proposals). The branch carries that worker's first four changes, which were written but not verified when the work was handed over:

1. the release profile with fat LTO and one codegen unit;
2. non-generic insertions in the value type registry;
3. type-independent parts of generic helpers moved out of line;
4. `scripts/perf-report.py`, which builds the three applications and prints a size and start-up table.

Do:

1. Verify each of the four changes on its own: the suites of the standing brief stay green, and no behaviour changes. Drop or repair what does not hold.
2. Measurement off macOS. Sizes can be measured here for the Linux build of the crates that build on Linux; desktop start-up and the real binaries need macOS. Add a manually dispatched workflow `.github/workflows/perf.yml` that runs `scripts/perf-report.py` on a macOS runner for a given ref and writes the table to the job summary (stripped sizes of `hello_window`, `themed_window` and `control-catalog-desktop`; median warm time from process start to the window-opened print over at least seven launches, with the load average). Keep the script dependency-free. Use it to produce before and after numbers for the pull request.
3. Continue down the ranked proposals of the report, one commit each, each with its measured effect: unwind tables and the panic strategy only if nothing in the framework depends on unwinding (the report lists what to check); duplicated generic instantiations worth outlining; registration tables that keep dead code alive; start-up work that upstream also does lazily. Behaviour must stay identical; compare with upstream before making anything lazy.
4. Do not change the XAML loader or compiler (the compiler branch removes the run-time theme load, which the report measures at about half of start-up), the browser backend, or the Skia feature set.

Record the final before and after table in `docs/porting/desktop-performance.md`.
