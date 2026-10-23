# Gradient double-click correction after c549

This checkpoint records a harness correction and pure Node checks. **The
corrected insertion has not been rerun in a browser.** The original c549 failure
and its source proof remain unchanged in
[the retained diagnosis](../browser-runtime-c549/gradient-insertion-diagnosis.json).
Exact corrected file hashes, inspected dependency source, and verification scope
are in [gradient-insertion.json](gradient-insertion.json).

The failed WebGL2 action delivered two native single presses 994 ms apart. The
same source and fixture delivered native counts `1,2` at 175 ms in Software2D.
FerroUI's existing mouse limit is 500 ms. The original request used Playwright's
`delay: 80`; its WebGL2 action lasted 2,658.384 ms. Original DOM timestamps were
not recorded, so the diagnosis does not assign an unobserved CDP or renderer
delay to a specific command.

## Corrected input and required evidence

Both existing insertion calls in
[gradient-handles-browser.mjs](../../../../samples/FerroDesign/tests/gradient-handles-browser.mjs)
now use one `page.mouse.click` with `clickCount: 2` and no artificial delay. The
linear axis and transformed angular ring still use measured world coordinates.
The current radial fixtures contain no double-click insertion call.

Inspection of installed **Playwright 1.64.1** confirms that its no-delay path
issues ordered real move/down/up/down/up commands before awaiting completion.
Its delayed path instead awaits each response and adds three 80 ms waits for
this two-click request. The inspected `coreBundle.js` SHA-256 is
`829521b67ed7be90c7d6d6098ff39f2241562d47e4ad7ccab13710924a56001d`.
The record includes mouse, Chromium dispatch, session and transport line ranges.

Existing passive capture/bubble observers retain creation times, observation
times, pointer coordinates and trust flags. Native state is sampled outside the
two-press window. The harness requires ordered trusted down/up/down/up input at
one point, exactly two new native press serials, native click counts `1,2`, and
both creation-time and native intervals within the **unchanged 500 ms limit**.
It verifies the same native canvas/world point and records observer delays
separately. Diagnostics are bounded and retain the primary action error.
Missing evidence fails the action. The next execution writes
`<mode>-gradient-insertion-input.json` on success or failure.

All existing stop-count, offset, RGBA, paint metadata, selected-stop, full
document and exact undo/redo assertions remain. There are no retries, synthetic
JavaScript events, production changes or native command bypasses.

## Checks completed

| Check | Result |
|---|---:|
| `node tests/gradient-input.test.mjs` | 11 passed; six new evidence cases |
| `node tests/input-observations.test.mjs` | 7 passed |
| `node tests/isolated-scenario.test.mjs` | 5 passed |
| Gradient harness syntax and three-file diff checks | Passed |

The new cases cover the inclusive 500 ms boundary and 501 ms rejection, native
singles despite a requested double-click, incomplete/untrusted/cancelled input,
different hit points, changing viewports, and exact or Chromium float32 point
delivery. These are explicitly labelled validator fixtures. They establish no
native execution, browser rerun, rendering performance or visual-parity result.
