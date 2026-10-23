# Common native text-entry timing

## Observed failure

The local radius replay used official browser artifact `12467852902`, application
commit `b7b0b8853dd47a3426537a6babc11fcc0e0e40cc`, with the corrected radius
input oracle based on harness commit `1637a093`. Software2D passed all seven
radius checks. WebGL2 stopped in the common text/IME setup before the radius
scenario. This was a later local replay of the b7 application, not the original
b7 CI failure and not a browser build of the latest application source.

[The compact extraction](native-text-entry-b7-failure.json) preserves the exact
runtime and executed harness hashes, report/trace digests, trace call IDs,
native presses and relevant control state. Its source report and trace remain
unchanged.

| Observed step | Evidence |
| --- | --- |
| Blank reset click | Native press 18 at world `(39.9999924223, 260.0000179110)`; selection changed from `[3]` to `[]` |
| Render and hit-test readiness | Both native waits completed; frames advanced from 97 to 98 and scenes from 67 to 68 before the next click |
| Requested text double-click | `mouseClick` with `clickCount: 2`, `delay: 80`; total command time 1,563.251 ms |
| Delivered native text presses | Presses 19 and 20 reached the same text interior at world `(420.0000055111, 88.0000084733)` |
| Native press timestamps | 92,542 and 93,178 monotonic ms: **636 ms apart** |
| Native click classification | Both presses had `clickCount: 1`; the platform double-click window is 500 ms |
| Final editing state | Text layer 3 selected, canvas focus retained, inline editor hidden, no modal overlay |

The preceding successful native pair was 171 ms apart and had click counts
`[1, 2]`. The failed pair's classification agrees with its recorded interval.
The saved trace rules out a missing blank reset, an unpublished selection
scene, a missed text hit or a modal interception as explanations of this
failure.

## Timestamp and automation semantics

The browser input bridge passes `performance.now() - event.timeStamp` as the
event's age. `BrowserInputHandler::event_timestamp` subtracts a valid age from
the monotonic elapsed clock before creating the raw pointer event. Dispatch
is synchronous and does not replace that timestamp. Ages from zero through
5,000 ms are valid; the complete failed click command lasted less than that
bound. The exact shipped b7 JavaScript implements its Emscripten monotonic
clock with `performance.now()`. This path is unchanged between b7 and
`08e29e4f`, and the existing native test
`a_pointer_event_is_stamped_with_the_time_the_page_raised_it` covers delayed
input delivery.

The pinned Playwright 1.64.1 implementation handles a nonzero click delay by
awaiting each CDP mouse down and up, with its requested waits between those
commands. It does not provide an explicit CDP event timestamp. A render or
host stall between awaited commands can therefore delay creation of the
second pointer event, even when the requested delay is 80 ms. `clickCount: 2`
does not override FerroUI's native timestamp-based classification.

The historical common setup did **not** record the DOM event creation and
observation timestamps. Consequently, the 636 ms native interval cannot be
directly compared with that pair's original `PointerEvent.timeStamp` values.
The source and shipped-clock inspection support faithful timestamp forwarding;
they do not identify the precise browser, renderer or host scheduling cause
of the historical delay. No production timestamp bug has been established.

## Harness correction and retained coverage

The first common text entry still requires actual trusted browser
double-click input, native press counts `[1, 2]`, and an arranged native
FerroUI TextBox. It has no retry or enlarged double-click window.

Subsequent commit/cancel and IME setup uses the existing public keyboard path:
a real blank reset, a real single click selecting the text layer, verified
native canvas focus and inactive editor state, then Enter. The existing native
selection test already verifies that Enter emits the text-edit event without
adding document history. All Unicode, rich-run, composition, exact undo/redo,
Escape, lifecycle, autosave and reload assertions remain in the shared suite.

Each entry now records bounded passive browser events, including event creation
and observation timestamps, plus the routed native presses, both press
intervals, delivery ages and native control state. The listeners do not call
the application bridge or change input defaults. Native state is sampled
outside the click window. Reports include `nativeTextEntryInput`, and the
separate `*-native-text-entry-input.json` file is collected even if entry fails.
Diagnostics have a five-second deadline and preserve the original input
failure if collection also fails. A diagnostic failure after successful input
still fails the run and is retained in its report.

The harness passed Node syntax validation and all 13 existing observer,
corner-radius input and angular input tests.

```sh
cd samples/FerroDesign
node --check tests/browser.mjs
node --test --test-isolation=none --test-reporter=spec \
  tests/input-observations.test.mjs \
  tests/corner-radius-input.test.mjs \
  tests/gradient-input.test.mjs
```

These commands used Node `24.19.0` and printed all 13 individual test names with
totals `tests 13`, `pass 13`, `fail 0`. `NODE_TEST_CONTEXT`, `NODE_OPTIONS` and
`NODE_TEST_REPORTER` were unset.

## Focused WebGL2 replay

[The completed replay extraction](native-text-entry-b7-replay.json) records
the runtime, executed harness and complete report/trace hashes. The standard
harness ran the common gates, `corner-radius-handles`, and its responsive and
persistence gates in WebGL2. It exited **0** with **all 26 checks passing**:
16 common checks, all 7 radius checks, and 3 responsive/persistence checks.
The radius scenario took 49,851 ms.

The mandatory first text entry used the real double-click and produced native
click counts `[1, 2]`. The independently observed intervals were:

| Clock / observation | Interval |
| --- | ---: |
| Original DOM pointer-event creation | 182.9000000004 ms |
| Native pointer-press timestamp | 183 ms |
| DOM listener observation | 170 ms |

The first and second pointer events arrived at the passive capture listener
14.1 ms and 1.2 ms after their respective creation timestamps. The native
interval agrees with creation time to its integer-millisecond precision,
despite the different delivery delays. This is direct evidence for the new
successful pair; the historical failed pair's missing DOM timestamps remain
unavailable.

All five later single-click-plus-Enter entries selected the intended text,
retained native canvas focus before Enter, and entered the actual editor.
Every native selection click had count 1. No browser input events or long-task
records were dropped. The existing composition, cancellation, late-IME-event,
pagehide, complete-document history and reload assertions all passed.

There were no page errors. The report retains one console 404, identified in
the trace as the local server's unused `/favicon.ico` request. Both owned
Node and Chrome processes exited, and the cgroup OOM counters stayed at their
pre-run values (`oom 73`, `oom_kill 5`).

The application was still the official **b7** runtime; the harness was based
on `08e29e4f` with the exact source correction hashed in the extraction. This
does not validate newer radial/angular or spacing-variable application code,
the other renderer in this run, or the complete feature suite. The earlier
failed report remains a separate, preserved result.

After that successful replay, the final source added the bounded diagnostics
guard described above. The browser input and all successful-path assertions
are unchanged. The extraction identifies both the executed harness hash and
this later source hash; the guard itself was not included in the b7 replay.
Node syntax validation and **18/18** named pure cases then passed, including
the five existing isolation/deadline tests:

```sh
node --test --test-isolation=none --test-reporter=spec \
  tests/input-observations.test.mjs \
  tests/corner-radius-input.test.mjs \
  tests/gradient-input.test.mjs \
  tests/isolated-scenario.test.mjs
```

```sh
cd samples/FerroDesign
FERRODESIGN_DIST=/path/to/extracted/browser-build \
FERRODESIGN_CHROMIUM=/path/to/chrome \
FERRODESIGN_REPORT=/path/to/report \
FERRODESIGN_SCENARIOS=corner-radius-handles \
FERRODESIGN_MODES=WebGL2 \
node tests/browser.mjs
```
