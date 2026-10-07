# 09. Benchmarks and a regression gate

Status: design, not implemented. `scripts/browser/scroll-profile.mjs` (pull request 47) is its
first piece.

## Finding

- **[M]** The figures of this folder come from one script run by hand on a shared machine; runs of
  the same module differ by up to 10 %.
- The trace that started the analysis was of the published module, which has no function names;
  the function order of a local build differs from the published one, so the names could not be
  mapped and the scrolling had to be reproduced.
- There is no measurement of the same work on the desktop, so it is not known how much of the cost
  is specific to WebAssembly.

## Design

1. **A native benchmark of recycling**, without a window: a headless application (the test
   services of `ferroui-base` with the Skia and HarfBuzz backends, as the catalog tests use them)
   that builds a table view with the data of the catalog page, lays it out, then scrolls it by a
   fixed sequence of offsets and reports time per recycled row and allocations per row. It runs
   under `cargo bench` or as an ignored test with a release profile, on Linux and macOS. This
   gives the designs a number that does not depend on a browser and can be profiled with native
   tools.
2. **Counters behind a feature.** A feature `perf-counters` of `ferroui-base` that counts property
   changes raised, changes with listeners, virtual calls by member, styles evaluated and matched,
   resource lookups, bindings created, text layouts created and cache hits. Off by default, no
   code when off. The designs 02 to 06 each start by reading these.
3. **Scrolling scenarios for the browser script**: besides the wheel, a drag of the scroll bar
   thumb (the case of the original trace: a viewport per event) and other pages with virtualized
   lists (ListBox, TreeView, the data grid when it is ported).
4. **A named module from CI.** The Pages workflow uploads, next to the published module, the same
   link with `--profiling-funcs` as a workflow artifact, so that a trace of the published site can
   be given names. **[H]** Linked in the same job from the same objects, the two should have the same function
   indices; an earlier comparison found their code sections 172 bytes apart, so the step includes
   checking that the indices line up.
5. **A regression gate.** The performance workflow (`.github/workflows/perf.yml`) runs the native
   benchmark and, in the browser job, `scroll-profile.mjs` against the base ref and the measured
   ref alternately, and reports the change; as with start-up times, a slower median is reported,
   not failed, until the noise of the runners is known.

## Expected gain

None by itself. It makes the estimates of the other designs measurable and keeps what they gain.

## Risks

Low. The counters must compile to nothing without the feature, which a size comparison of the
module checks.

## Verification

- The benchmark reports the same per-row figures on repeated runs within a few percent on an idle
  machine.
- The module with the feature off is byte-identical in size to the module before the counters.
