# The framework comparison's FerroUI application, from this checkout and from the published crates

The public framework comparison measures a FerroUI application of its own: the gauntlet, the
still wall and the patchwork with its chat, written on FerroUI's declarative layer
(`ferroui-view`). It is not `samples/GauntletBenchmark`, which is the same screens written with
the controls themselves. What the comparison publishes for FerroUI is what that application
does, so this directory builds it against the FerroUI of this checkout and measures it with the
comparison's own harness, beside the same application built from the published crates.

| Path | What it is |
| --- | --- |
| `upstream/` | The comparison's repository, a git submodule. Nothing in it is changed. |
| `ferroui-local/Cargo.toml` | A manifest of this repository for the comparison's application (`upstream/.../ferro-app/src/main.rs` and its data crate `perf-data`), with the FerroUI crates of this checkout as path dependencies. |
| `compare.py` | Builds the two sides and runs the comparison's harness on each. |

The two sides:

- **local**: `ferroui-local/Cargo.toml`. FerroUI is this checkout.
- **official**: the comparison's own `ferro-app/Cargo.toml`. FerroUI is the published version
  that manifest names.

Both use the published Skia binding and the comparison's release profile, so they differ in
FerroUI alone.

## Using it

The submodule is not fetched by a clone (`update = none` in `.gitmodules`); fetch it once:

```sh
git submodule update --init --checkout benchmarks/comparison/upstream
```

Build and measure:

```sh
python3 benchmarks/comparison/compare.py build
python3 benchmarks/comparison/compare.py run --scenario gauntlet:16 --rounds 2
```

`run` prints, for a first start and a second start of each scenario, the frames a second over
the run, the time to the first frame and the CPU a frame, for the two sides beside each other.
The harness is the comparison's (`desktop.py`): it measures from outside the application, needs
macOS, a display at 60 Hz and its frame counter (`upstream/.../framecount/build.sh`),
and refuses a display on which something else moves. `--diagnostic` measures anyway, for work
in progress. Its other arguments (`--scenario NAME[:TIER]`, `--rounds N`, `--run SECONDS`) pass
through; the comparison's README describes the screens and the tiers.

To move the comparison's sources forward, check out the wanted commit in `upstream/` and commit
the submodule; when the comparison's manifest changes its dependencies, `ferroui-local/Cargo.toml`
follows it by hand.

## What it is not

It is not part of the workspace (the root manifest excludes this directory), not built by the
continuous integration, and not a suite of the local gate (`scripts/perf/gate.py`), whose scenes
run without a display. Findings made here are recorded in
`docs/porting/desktop-performance.md` (sections 9.81 and 9.82 are the first).

The comparison's sources are under the licences its repository states (`Apache-2.0 OR MIT` for
the application and its data crate).
