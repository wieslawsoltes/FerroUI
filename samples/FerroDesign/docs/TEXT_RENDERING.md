# Text rendering and browser observation

FerroDesign submits shaped text through public FerroUI text and drawing APIs.
The browser application uses the Vello Software2D or WebGL2 surface; it does not
select Skia or paint the editor with HTML controls.

## Translation reuse for unhinted outlines

An immutable Vello glyph run now owns a bounded cache for the flattened contours
of unhinted glyphs. The key contains the glyph index and the exact linear part of
the drawing transform. Translation reuses the prepared contour; scale, rotation
and shear use distinct entries. Each run retains at most 256 entries and 128 KiB
of path elements. Over-budget contours retain only a negative cache entry, so
they use the existing curve path without repeating speculative preparation.

The existing 0.05 curve tolerance is applied after the linear transform. Brush
placement, clipping and each glyph's source-over operation remain independent.
In particular, translucent overlapping glyphs are not merged into one fill.
The public scene-sink default still receives canonical curves; CPU, WebGL2,
hybrid and GPU sinks opt into the reusable preparation only for antialiased,
unhinted contours. Recording and custom sinks retain the original precision.

Tests cover translation reuse, transform/glyph misses, eviction, oversized
contours, canonical path precision, and rendered pixels under fractional affine
transforms, clips, translucent gradients and image brushes. Added assertions
are not a claim that their current source has completed CI; consult the exact
commit's native feedback and full validation runs.

## Browser pan benchmark version 2

`tests/text-render-benchmark.mjs` retains the seeded 150-label workload, twelve
warmups and 36 measured real pointer motions per surface. Its timed polling uses
the constant-size, read-only `ferroDesignFrameState()` bridge. It records host
dispatch, native frame observation, native presentation observation and the two
subsequent animation callbacks separately. Bridge duration/payload/poll counts,
bounded trusted input timestamps and browser long tasks are reported alongside
the timing distributions. Compact and full state are compared outside the
timed interval, and every measured motion must advance the expected camera and
native frame/presentation counters.

These observations include automation and browser scheduling. A presentation
counter is not a GPU-completion timestamp, and the benchmark requests
SwiftShader. It makes neither an FPS assertion nor a physical-GPU throughput
claim. Historical builds without the compact bridge are labeled as using the
legacy measurement method, and comparisons across those methods are suppressed.
The previous 1,366.94 ms WebGL2 p95 remains in the historical evidence; a cache
change alone does not establish its cause or demonstrate an improvement.

### Optional Chromium trace cohort

`FERRODESIGN_TRACE_TEXT_PAN=1` enables six additional real middle-button motions
after the ordinary measurements for each surface. The full browser workflow
enables this diagnostic. Its samples have separate labels and never enter the
ordinary timing distributions. Tracing is inactive during the ordinary samples;
the extra work between surfaces still changes the overall run and must not be
treated as an identical environment for a before/after performance claim.

Trace-enabled runs own Chromium through public Playwright `BrowserServer`
APIs and connect over WebSocket. The ordinary trace-off path retains
`chromium.launch()`. Reports record this transport and trace configuration,
and comparative deltas require matching configurations. Trace-enabled dispatch
measurements are therefore not transport-identical to the historical runs.
The owner first disconnects its connected `Browser`, then closes its
`BrowserServer`. These are separate public lifetimes: closing the explicitly
created contexts does not dispose the connected client. Playwright documents
[connected Browser.close](https://playwright.dev/docs/api/class-browser#browser-close)
as clearing its contexts and disconnecting from the browser server; the server
then owns process termination. The owner bounds client disconnection, graceful
process shutdown, force-kill and direct owned-child fallback to three seconds
each, observes process exit, and shares one shutdown result with the outer
finalizer. Every attempted phase remains recorded. A client timeout does not
prevent cleanup of its process and does not become a success merely because
that process subsequently exits.

A successful graceful-close promise must also have an observed normal process
exit. Disconnection alone, a resolved kill request without exit, or a previously
crashed process cannot report `closed`. Reports distinguish clean closure,
failed cleanup, forced termination and unconfirmed termination. Every result
other than `closed` fails benchmark finalization; fatal cleanup stops later
modes and persists its report before exit.

User Timing marks bracket pointer capture/bubble, observed native frame and
presentation, and two subsequent animation callbacks. The analyzer aligns the
page clock to Chromium's trace clock using marks bracketed by page timestamps,
reports alignment uncertainty, and rejects inconsistent clocks. It does not
subtract host timestamps from page timestamps. Overlapping synchronous slices
are combined as wall-time unions; asynchronous input/flow records are counted
separately. Groups can overlap and their durations must not be added.

The cohort, Chromium buffer, retained stream and analyzed event count are
bounded. Missing marks, buffer loss, truncation, unmatched slice boundaries and
incomplete input remain explicit in the diagnostic. Raw traces and detailed
observations stay in the normal browser report directory; the CI log contains
only a bounded derived summary. Nine pure analysis regressions cover nested
intervals, clock alignment, incompatible origins, missing/duplicate markers,
asynchronous input, event limits and log bounds. Eleven mock-only teardown tests
cover one-shot ownership, client-before-server disposal, stalled or falsely
successful client close, stalled graceful/force cleanup, exact-child fallback,
delayed and absent process exit, crashes and ownership changes, for twenty
tests total. They launch no browser or subprocess and do not establish actual
Chromium trace capture or resolution of a previous browser shutdown failure.

The exact-source [5cb7e884 browser evidence](../../../docs/ferrodesign-evidence/2026-10-22-5cb7e884-browser/README.md)
records completed ordinary and six-sample trace cohorts in both modes. Its final
server close exceeded three seconds and required a confirmed `SIGKILL`; the
benchmark step therefore failed despite those completed measurements. That log
does not establish why Chromium did not close promptly. Disposing the connected
client corrects the observed ownership omission; an actual run of the new source
must establish whether finalization completes normally.

This instrumentation can help locate recorded work around the high WebGL2
input tail. Recorded browser/compositor/GPU activity is not proof of scheduling
cause, CPU utilization, this page's GPU work, or GPU completion. The exact
source's CI run must establish whether Chromium captured usable evidence.

## Inline text-entry diagnosis

The typography scenario still performs one original two-press gesture with an
80 ms delay and the original 30-second readiness deadline. It checks the actual
native inline control's focus, layout and a published scene/frame. A fixed-size
native pointer ring and bounded browser observations record timestamps, click
counts, buttons and camera state on success or failure. They do not alter click
recognition or retry the gesture. The earlier WebGL2 failure remains a failure
until a new exact-source browser run establishes otherwise.
