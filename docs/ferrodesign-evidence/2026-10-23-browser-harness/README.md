# Browser harness corrections, 23 October 2026

These measurements validate the numerical corner reference and the bounded
browser ownership helper. They do **not** establish that the current editor
passes its complete browser scenarios or text benchmark.

## Corner coverage

The tiny PNG and SVG in this directory are actual native exports from a local
Software2D run of official browser artifact `12467852902`, source
`b7b0b8853dd47a3426537a6babc11fcc0e0e40cc`. Its
[full workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39869066118)
failed browser validation. In particular, the preceding 8×/16× SVG coverage pair
failed its four-alpha convergence gate with maximum difference `4.453125` in
both presentation modes; the native comparison was not reached.

`corner-coverage.json` records the input and tested-source SHA-256 hashes,
Chromium version, launch flags, and scalar measurements. The revised exact
helper from `tests/corners-browser.mjs` was evaluated in Chromium
`156.0.8078.4`, with the same SwiftShader flags, 1600×1000 viewport and scale 1
used by the editor harness. The editor was not loaded during this independent
decoder measurement.

| Fixed scale | Maximum native PNG alpha difference | Native pixels differing by >12 | Maximum strip/full difference | Strip/full differing samples |
| --- | ---: | ---: | ---: | ---: |
| 16× | 11 | 0 | 0.25 | 4 |
| 32× | 10 | 0 | 0.125 | 8 |

The strip references' 16×→32× maximum alpha difference was **1.96875**, below
the unchanged limit of 4. The absolute alpha-area difference was `0.855086`
pixels, with no opposite fully opaque/transparent pixels. The final 32×
reference has zero native differences over 12, against the unchanged limit of
48 pixels. No reference or native comparison threshold was widened.

Integer-translated SVG strips preserve the sampling grid, but their Chromium
rasterization is not byte-identical to a full canvas: a few partial-edge samples
change as recorded above. The convergence gate tests the actual strip
references. It is a numerical stability criterion, not an absolute mathematical
error proof. Explicit strip canvas/readback buffers are bounded to 6400×512
RGBA8; no two strips or scales run concurrently. The optional reproduction
below also allocates a bounded full canvas to measure the strip/full difference.

From the repository root, after installing `samples/FerroDesign`'s locked Node
dependencies and pinned Playwright browser:

```sh
node docs/ferrodesign-evidence/2026-10-23-browser-harness/reproduce-corner-coverage.mjs
```

`FERRODESIGN_CHROMIUM` can select the pinned Chromium executable, and
`FERRODESIGN_COVERAGE_REPORT` can write a fresh scalar report. The script reads
the actual current corner helper, enforces its original native and convergence
thresholds, and always closes its owned browser.

## Browser shutdown ownership

`trace-lifecycle.json` records a small real-browser lifecycle check after the
ownership correction in `3be16cb7`. It creates a BrowserServer, connects a
Browser, renders six Canvas2D and six WebGL2 frames, ends and drains CDP tracing,
detaches its session, and closes the context. Both the original unattached-client
variant and the corrected attached-client variant exited normally with code 0.
The corrected variant confirmed client disconnection before server shutdown.

This small workload does not reproduce the full benchmark's CI failure.
The previous benchmark completed both measured/trace cohorts and then exceeded
its graceful BrowserServer shutdown deadline; forced process exit remained an
explicit failure. The correction adds the omitted public connected-Browser
cleanup and requires observed owned-process exit. Its tests continue to reject
stalled or still-connected clients, unexpected process signals, and unconfirmed kills.

The pure regression command passed **26/26** tests locally (6 coverage tests and
20 trace-analysis/ownership tests):

```sh
cd samples/FerroDesign
node --test --test-isolation=none --test-reporter=spec tests/alpha-coverage.test.mjs tests/text-pan-trace-analysis.test.mjs
```

## Other corrected expectations, awaiting full browser execution

The paragraph export fixture now places the unchanged, natively authored text
inside an explicitly clipped 440×200 frame. Text selection exports deliberately
include conservative glyph-overhang paint bounds; they were 448×208 in the
previous run. Exact paragraph history checks remain, export must preserve the
entire fixture, and the original authored document is restored before inline
editing tests.

Width-handle expectations now use passive, trusted browser input observations
and the actual routed native press to account for fractional pointer delivery.
The independently calculated width/contour equations retain their `1e-7`
tolerance. They never infer the expected value from the resulting profile.
The test records its bounded observed-input evidence and still compares entire
documents exactly through undo, redo, cancel, persistence and lifecycle commit.
These two editor scenarios have not been rerun successfully at this checkpoint.
