# Browser input readiness: observed failures and source corrections

This folder preserves the diagnosis of failures in the verified `08e29e4f` WebAssembly runtime with the frozen `0f073278` harness. It also records the source corrections published in [c549f6bc](https://github.com/wieslawsoltes/FerroUI/commit/c549f6bcafe614a6373c743f5efbf2a3cb0eccd2). **The local checks below do not establish a native or browser pass for c549f6bc.** That checkpoint requires a rebuilt runtime and a new interaction run.

## Provenance and original results

| Role | Exact source |
| --- | --- |
| Executed WebAssembly runtime | `08e29e4f530c0ae65f6e2ac072f381605e23ed0c` |
| Executed browser harness | `0f0732784bd6fdf1cb2477a61dd332d9a007fec2` |
| Subsequent correction | `c549f6bcafe614a6373c743f5efbf2a3cb0eccd2` |

The original distribution was [build run 39945441130](https://github.com/wieslawsoltes/FerroUI/actions/runs/39945441130), artifact 12499117711. Its payload hashes were verified, and all 55 recorded harness inputs matched before and after execution. The browser was Chromium 156.0.8078.4 with Playwright 1.64.1 and Node 24.19.0; the observed WebGL2 device was SwiftShader. Exact trees, archive/WASM hashes and correction-source hashes are in [provenance.json](provenance.json).

The [original selected browser run](../browser-runtime08-harness0f/) failed overall. Software2D passed all 17 gradient check groups; WebGL2 stopped during initial gradient entry, before any gradient assertion. Both paragraph scenarios stopped after their first four check groups. The separate [text-pan measurement](../browser-text-pan-runtime08-harness0f/) preserves its strict Software2D console failure for the missing `/favicon.ico`. Neither run validates the subsequent source correction.

## Gradient entry: established observations and inference

The native button `PaintCanvasGradient1` reported valid, enabled bounds at `x=1001, y=366, width=92, height=30`. The harness requested exactly one click at its center, (1047,381). Two earlier 240-pixel wheel requests had moved its reported y-coordinate 846→606→366 while scene counters advanced 8→9→11.

After the timeout, the only difference in the complete recorded native state was `sceneFrames`, 11→13. Gradient node and fill remained null, the paint modal remained visible, and CanvasHost retained focus within. The document revision, history, selection, paint fields and chrome diagnostics were unchanged. Storage reported saved with `dirty=false` and `draftDirty=false`.

The trace's sampled frame immediately before the click still displayed the previous scroll position, with the entry button below the modal viewport. The first sampled frame showing the new position was presented approximately 522.6 ms after the click. These presentation estimates use the trace's wall/monotonic origin; they are not native pointer timestamps.

Source review explains why current layout bounds and a higher scene counter are insufficient together: bounds describe current UI layout, while native pointer routing and Button release use the compositor's rendered readback. Scene notifications are coalesced completions and do not identify which layout mutation a particular completed batch contains.

**Routing through an older rendered scene is the strongest supported explanation. The actual historical native press target was not recorded.** A transient capture or focus interruption cannot be conclusively excluded. The captured state supplies no evidence of dirty-draft or IME validation rejection. These limits are explicit in the [diagnosis](gradient-entry-diagnosis.json) and [independent native audit](gradient-entry-native-audit.json). The compact [timeline](gradient-entry-timeline.json) retains the exact trace calls, sampled-frame metadata, image hashes and presentation-clock conversion without copying the images.

The diagnosis JSON is preserved byte-for-byte. The audit retains its original source-byte hashes and conclusions; its compositor source location was corrected to lines 293–312 of the exact 08 revision, as recorded in provenance. Large traces, screenshots and full native-state dumps are not duplicated here.

## Paragraph insertion: the requested newline was absent in the result

Both renderer traces requested `A🦀\n` through `keyboard.insertText` at the established native caret zero. Both saved documents instead contained `A🦀A🦀\nA🦀\nA🦀\n`: 23 UTF-8 bytes, exactly the result of inserting five bytes without the requested newline, instead of six. The resulting character ranges 0..11, 11..17, 17..23 and paragraph ranges/indents are consistent with that shorter insertion.

The browser bridge uses a single-line HTML input and forwards `InputEvent.data`; the native inline TextBox accepts returns. **The historical DOM event payload was not recorded**, so newline filtering at the bridge is a source-backed explanation, not an observed `InputEvent.data` value. The [paragraph diagnosis](paragraph-insertion-diagnosis.json) retains both exact trace call IDs, failure-document hashes, the complete relevant rich-text values and this limitation.

## Changes published in c549f6bc

| Correction | Resulting source behavior and retained contract |
| --- | --- |
| Native gradient entry | Read-only `pointerOver` and Button `pressed` diagnostics expose actual FerroUI state. The gradient helper moves once, waits for native hover at unchanged measured bounds, presses once, waits for native pressed state, and releases once in `finally`. Passive before/hover/down/after evidence is retained, including held state on failure. The original entry predicate remains, with exact document, selection and history preservation checks. Capture or cleanup errors cannot replace the primary failure. |
| Complete multiline clipboard input | The paragraph helper writes and reads back the actual `text/plain` browser clipboard value, then issues one trusted Ctrl+V into the native TextBox. Existing exact rich ranges, actual line indentation, single native undo/redo and one document-session history step remain required. The public native paste path takes one history snapshot and inserts the complete value. No synthetic paste or native text assignment is introduced. |
| Actual Inspect tab | Polygon XAML export clicks the real `InspectTab`, verifies the native inspector visibility/layout, downloads through `ExportXamlButton`, then restores Design in `finally`. Original export failures survive a restoration failure. |
| Relative favicon | The page explicitly links `./favicon.svg`, and the asset is included by the existing recursive web-directory copy. This relative URL also resolves beside an index served under `/current/`. The strict empty-console benchmark assertion remains unchanged. |

The [source commit](https://github.com/wieslawsoltes/FerroUI/commit/c549f6bcafe614a6373c743f5efbf2a3cb0eccd2) contains all four corrections. Their exact file hashes are recorded in provenance. The gradient helper requires the new runtime diagnostics and has no compatibility fallback to an older binary. No activation retry, fixed sleep, increased tolerance, altered undo expectation or generalized change to every native click was added.

## Local checks completed

The following command completed 46 actual helper cases with 46 passes, zero failures, zero skips and exit code 0 under Node 24.19.0:

```bash
cd samples/FerroDesign
node --test --test-isolation=none --test-reporter=tap \
  tests/alpha-coverage.test.mjs \
  tests/corner-radius-input.test.mjs \
  tests/gradient-input.test.mjs \
  tests/input-observations.test.mjs \
  tests/isolated-scenario.test.mjs \
  tests/text-pan-trace-analysis.test.mjs
```

| Helper file | Passed cases |
| --- | ---: |
| Alpha coverage | 6 |
| Corner-radius delivered-input oracle | 3 |
| Gradient delivered-input oracle | 5 |
| Passive input observation | 7 |
| Scenario isolation and primary-error preservation | 5 |
| Text trace analysis and owned-process cleanup | 20 |

The changed gradient, paragraph, polygon and passive-observer modules also passed `node --check`. [local-checks.json](local-checks.json) records the exact commands, all 46 case names, source hashes, executor clock and results. These are pure helper and syntax checks; they did not launch the application or Chromium. **No c549f6bc native test execution or actual browser replay is recorded in this evidence snapshot.**

[files-sha256.json](files-sha256.json) lists the compact evidence-file sizes and SHA-256 values; it excludes its own checksum to avoid a circular manifest.
