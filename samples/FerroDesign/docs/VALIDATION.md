# Validation and measurements

This file distinguishes implemented paths, completed checks and measurements. Each observation belongs to its stated source, runtime and run; older successful runs do not validate later changes. The branch incorporates FerroUI `ab7a15b8` / `0.4.0-preview.1` and uses Vello for every application target through public FerroUI APIs. [PR #254](https://github.com/wieslawsoltes/FerroUI/pull/254) collects the complete source and current review status. Current continuation evidence is recorded first; earlier checkpoints remain below with their original scope.

## Current continuation after `76d3744c` (2026-10-22)

The continuation contains production framework and application changes after the complete predecessor runs below. The newest completed core suite passed **511 tests at `7f813672`**, including all **five gradient / eight padding** core cases. The complete native feedback job at **`5cb7e884` passed**, including **217 application / 16 paragraph / 174 TextBox / 5 rich-history tests**, plus **8 Vello / 15 Base**. Its eight radius-handle cases and independent dense-curve coverage check passed. Its separate full-workflow native job also passed, including **498 core / 217 application tests** and all configured host/framework/renderer gates. Actual WASM and IndexedDB passed at `40dafdab`, whose editor suite failed after **24 checks in each mode** on a later corner silhouette comparison. The `124ffdb7` full workflow then failed application compilation; the explicit-type correction is now validated natively at `5cb7e884`. Its browser job remains a separate gate. The newer gradient/padding native controls and browser scenarios still require execution; these source-specific outcomes do **not** establish full Figma parity.

| Current source addition | Implemented scope and explicit boundary |
| --- | --- |
| [Native linear-gradient handles](GRADIENT_HANDLES.md) | Per-fill native entry, affine endpoint and axis drags, stable-index stop editing/insertion/deletion, separated tied markers, keyboard navigation and owned transaction/lifecycle integration. Public FerroUI drawing and shared eligibility preserve the bounded supported-box contract. Five core cases passed; seven native and ten browser checks are authored and await execution |
| [Four-side auto-layout padding](LAYOUT_PADDING.md) | Native independent/linked Top/Right/Bottom/Left fields, empty-frame authoring, atomic grouped draft validation, exact untouched-side precision, shared stale-dialog guards and save/pagehide integration. Nested reflow and instance layout overrides use one complete transaction. All eight core cases passed; five native and six browser checks await execution |
| [Automatic network preparation](VECTOR_FACES.md) | Native Fill all closed regions and bucket authoring split certified line/cubic intersections before face discovery. Curves retain exact subdivision geometry and stable identities; hover remains read-only and a changed bucket stroke remains one transaction. Overlaps, tangencies, unresolved close roots and exhausted limits reject atomically |
| [Continuous corners](CORNERS.md) and [polygon/star fillets](POLYGON_CORNERS.md) | Shared vector outlines drive artwork, picking, clips/masks and supported exports. Box smoothing and signed polygon/star circular fillets retain their authored parameters. Arbitrary vector-anchor fillets and numerical equality to every Figma construction remain outside the claim |
| [Native corner-radius handles](CORNER_RADIUS_HANDLES.md) | Select-mode hover, fractional affine drags and Alt/Option Rectangle-corner edits use the same core geometry. Distinct screen-space markers remain accessible at maximum radius. Owned transactions preserve exact metadata/redo on no-op or canceled input; save/pagehide commits once. Five core regressions and all eight native cases passed; seven browser checks require their own execution |
| [Stroke profiles](STROKE_PROFILES.md) and [canvas handles](STROKE_WIDTH_HANDLES.md) | Six presets, ordered custom points, stack-entry editing, captured width gestures and lifecycle/history integration use the same bounded contour preparation and filled geometry. Dashed/branched nonuniform strokes, independent left/right widths and per-contour profiles are unsupported |
| [Guide interactions and bindings](LAYOUT_GUIDES.md) | Typed per-guide variables, mode/alias validation, move/draw/resize snapping and stretch-guide-relative constraints extend the existing guide stacks and styles. Candidate search and first-compatible-guide rules are bounded and documented |
| [Native paragraphs and rich text history](NATIVE_TEXT.md) | Shared paragraph layout, exact native UTF-16 replacement metadata mapped to UTF-8 ranges, character/paragraph rebasing, application snapshots in existing TextBox undo, and guarded IME cancellation/commit preserve text ownership. The editing overlay retains its one-foreground-paint-per-run appearance subset |
| [Shaped-text XAML export](NATIVE_TEXT.md#ferroui-xaml-artwork-export) | Public drawing-sink/glyph APIs emit positioned solid-color glyph and decoration paths with affine transforms and once-only layer opacity. Unsupported paints, compositing or incomplete outlines reject the export; editable JSON remains the complete document format |
| [Text preparation and measurement](TEXT_RENDERING.md) | Bounded unhinted glyph-contour reuse, scoped confirmed typing without auto layout, and an exact suppressed-text-leaf renderer fast path reduce repeated preparation work. Auto-layout documents retain full arrangement. Browser benchmark phases now separate dispatch, frame/presentation observation and settling; no new end-to-end speedup is established by source inspection |

### Completed source-specific gates

The following terminal results come from the preserved decoded logs and API statuses. A successful substep remains distinct from its workflow's failed outcome.

| Gate | Exact source/run | Recorded result |
| --- | --- | --- |
| Gradient and padding document-core additions | `7f813672`, [run 39864431287 / job 120122942802](https://github.com/wieslawsoltes/FerroUI/actions/runs/39864431287/job/120122942802) | **511 passed; 0 failed, ignored or filtered out**, **1.65 s**, including all five gradient and eight padding cases. The separate doc-test target contains zero tests; [exact log, hashes and scope](../../../docs/ferrodesign-evidence/2026-10-22-7f813672-core/README.md) |
| Complete native job in the full workflow | `5cb7e884`, [run 39862047469 / job 120117259866](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047469/job/120117259866) | **Native job success**: **498 core**, **2.10 s**; **217 app**, **20.21 s**; **19 host / 4 input-host / 13 trace-analysis**, all configured preedit/compositing/geometry/glyph gates and five Vello/no-Skia target graphs passed. This is not the browser sibling or overall full-workflow result; [preserved native evidence](../../../docs/ferrodesign-evidence/2026-10-22-5cb7e884-native/README.md) |
| Complete native feedback after radius type correction | `5cb7e884`, [run 39862047515 / job 120114911173](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047515/job/120114911173) | **Overall success**: **217/217 application**, **25.49 s**; **8 Vello / 15 Base / 16 paragraph / 174 TextBox / 5 rich-history** passed. Includes all eight radius-handle cases and independent dense-curve native alpha coverage, PNG round-trip and history/fresh-render equality; [decoded log, hashes and exact scope](../../../docs/ferrodesign-evidence/2026-10-22-5cb7e884-native/README.md) |
| Latest complete core suite and subsequent full build | `124ffdb7`, [run 39858027509](https://github.com/wieslawsoltes/FerroUI/actions/runs/39858027509) | **498 core tests passed**, **2.07 s**. Both native and browser jobs then failed application compilation with **E0282 at `canvas/corner_radius.rs:206`**. Application tests, browser interactions and optional Chromium tracing did not execute |
| Same-value text/Redo correction | `58deaeee`, [run 39857371121 / job 120100682612](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857371121/job/120100682612) | **16/16 paragraph**, **174/174 TextBox**, **5/5 rich-history**, **8 Vello / 15 Base** passed. Application compilation failed with the corner-radius **E0282** error, so no app tests ran and the job **failed overall**; [decoded logs and exact scope](../../../docs/ferrodesign-evidence/2026-10-22-58deaeee-ci/README.md) |
| Official generated references after same-value text correction | `58deaeee`, [run 39857370963 / job 120098973929](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857370963/job/120098973929) | **All three generators and three drift/recompile gates passed**, empty generated patch, five identical source/output hashes; [exact provenance](../../../docs/ferrodesign-evidence/2026-10-22-58deaeee-ci/generated-provenance.json) |
| Radius-handle core geometry and history | `38713369`, [run 39857152118 / job 120098216085](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857152118/job/120098216085) | **498 passed, 0 failed, 0 ignored**, **2.45 s**, including all five new radius-handle geometry/atomic-history tests. The [decoded log](../../../docs/ferrodesign-evidence/2026-10-22-38713369-core/README.md) does not substitute for native/browser execution |
| Hosted native rich-history fixture | `a867795a`, [run 39855458955 / job 120095169276](https://github.com/wieslawsoltes/FerroUI/actions/runs/39855458955/job/120095169276) | **All 5 history tests passed**, **0.06 s**; **208/208 app**, **25.11 s**, and **8 Vello / 15 Base** passed. Controls remained **15/16 paragraph / 172/173 TextBox** on the same final redo assertion. Overall **failure**; [decoded log and exact scope](../../../docs/ferrodesign-evidence/2026-10-22-a867795a-native/README.md) |
| Native lifecycle and application feedback | `cdee5f26`, [run 39854329204 / job 120091228339](https://github.com/wieslawsoltes/FerroUI/actions/runs/39854329204/job/120091228339) | **208/208 application tests passed**, **15.32 s**; **8 Vello / 15 Base** passed. Controls paragraph **15 passed / 1 failed**, complete TextBox **172 passed / 1 failed**, history **4 passed / 1 failed**. Overall **failure**; [exact boundary and logs](../../../docs/ferrodesign-evidence/2026-10-22-cdee5f26-ci/README.md) |
| Official generated references after null-template handling | `cdee5f26`, [run 39854329141 / job 120090095921](https://github.com/wieslawsoltes/FerroUI/actions/runs/39854329141/job/120090095921) | **All three generators and three drift/recompile gates passed**, no generated patch, five identical source/output hashes matching `c88ad28f` and `ed4f03dd`; [provenance](../../../docs/ferrodesign-evidence/2026-10-22-cdee5f26-ci/generated-provenance.json) |
| Late-presenter subscription correction | `c88ad28f`, [run 39852882829 / job 120084104777](https://github.com/wieslawsoltes/FerroUI/actions/runs/39852882829/job/120084104777) | **14/14 paragraph** and **171/171 TextBox tests passed**, including the previously failing fixtures. **8 Vello / 15 Base** passed. History **4/5** and app **207/208** failed on separate fixture issues. Its generator run **39852882875 passed** with no drift; [logs and follow-ups](../../../docs/ferrodesign-evidence/2026-10-22-c88ad28f-ci/README.md) |
| Core document/editor suite and host checks | `ee1999b9`, [full run 39845598638 / native job 120061876291](https://github.com/wieslawsoltes/FerroUI/actions/runs/39845598638/job/120061876291) | **493 core tests passed** in **2.43 s**, **19 persistence-host / 4 input-host tests** passed, and all **five Vello/no-Skia target graphs** passed. Native preedit then had **11 passed / 1 failed**; later application/framework steps in this job were skipped |
| Official generated references | `ed4f03dd`, [run 39843397843 / job 120050185897](https://github.com/wieslawsoltes/FerroUI/actions/runs/39843397843/job/120050185897) | Official Rust-path and both compiled-theme generators, followed by all three drift/recompile gates, **passed with zero drift**. The five generated files kept identical hashes; [decoded log and provenance](../../../docs/ferrodesign-evidence/2026-10-22-ed4f03dd-generated/README.md) record the configured scope |
| Independent native framework/application feedback | `ee1999b9`, [run 39845598687 / job 120059769833](https://github.com/wieslawsoltes/FerroUI/actions/runs/39845598687/job/120059769833) | **204/204 application tests passed** in **18.13 s**; **8 Vello / 15 Base paragraph** tests passed. Controls paragraph tests had **11 passed / 2 failed**, so the job **failed overall** |
| Independent native framework/application feedback | `2b0fe4e1`, [run 39846931439 / job 120064759737](https://github.com/wieslawsoltes/FerroUI/actions/runs/39846931439/job/120064759737) | **205/205 application tests passed** in **24.35 s**, including the active-background XAML regression. **8 Vello / 15 Base paragraph** tests passed; Controls paragraph tests again had **11 passed / 2 failed**. The job **failed overall** |
| Actual WASM build and Chromium persistence | `ee1999b9`, [full run 39845598638 / browser job 120061875782](https://github.com/wieslawsoltes/FerroUI/actions/runs/39845598638/job/120061875782) | The Vello WebAssembly application **built successfully**; all **five real Chromium IndexedDB checks passed**, including reload of a **12,000,042-character** document |
| Editor interactions and pixels | Same exact `ee1999b9` browser job | The full suite was requested, but **both modes failed** in `corners-browser.mjs:34` after **23 Software2D / 23 WebGL2 checks**. Later feature scenarios did not run. The separate text-pan step then passed; it does not change the failed editor/job result |
| Later actual WASM, persistence and editor execution | `40dafdab`, [run 39851782871 / job 120079908067](https://github.com/wieslawsoltes/FerroUI/actions/runs/39851782871/job/120079908067) | Vello WASM **built** and all **five IndexedDB checks passed**. Editor **24 Software2D / 24 WebGL2 checks** completed before both modes failed the SVG/native PNG silhouette assertion with **288** differences at `corners-browser.mjs:77`. The separate benchmark passed; overall job **failure**; [decoded log and printed results](../../../docs/ferrodesign-evidence/2026-10-22-40dafdab-browser/README.md) |

The [ee1999b9 evidence index](../../../docs/ferrodesign-evidence/2026-10-22-ee1999b9-ci/README.md) contains the three decoded logs, failed printed browser summary, phase records and full-run API/artifact metadata. The [2b0fe4e1 native record](../../../docs/ferrodesign-evidence/2026-10-22-2b0fe4e1-native/README.md) preserves its decoded feedback log and exact extraction locations. The application counts are complete, unfiltered `ferrodesign-app --lib` runs. Test-suite durations are not interactive performance measurements. The earlier **493-test core pass at `bbcd8b5e`** remains separately [preserved](../../../docs/ferrodesign-evidence/2026-10-22-bbcd8b5e-core/README.md).

Generated-reference passes verify official outputs after the native text-history API work; they are not comprehensive framework or editor runs. Native feedback permits the independent application test step after a framework-test failure when toolchain setup succeeded and the job was not cancelled. The preceding failure still fails the job. Since `c88ad28f`, the shared-editing step executes all four configured filters before returning failure; older jobs stopped within that step after their first failed command. No `continue-on-error` or assertion suppression converts these failed workflows into passes.

The previously recorded **20:08 UTC pending** full workflow and **20:15 UTC failed-paragraph/application-running** feedback observations were intermediate statuses for `ee1999b9`. The terminal results above replace those pending statuses without erasing the failed substeps.

### Failure boundaries and later source corrections

In both feedback jobs, the presenter lookup succeeded but the TextBox retained its original text after `set_preedit_text` instead of reflecting provisional selection removal. `paragraph_preedit_replacement_uses_confirmed_ranges_and_blur_restores_the_collection` observed `A🦀\nB\nC` instead of `A\nC`; `repeated_paragraph_ime_replacement_uses_owned_selection_and_restores_on_blur` observed `A\nA\nA\n` instead of `A\n`. The former is also the full workflow's single preedit failure. A subsequent source investigation identified a production subscription gap for a TextPresenter created after TextBox attachment.

Checkpoint [`c88ad28f`](https://github.com/wieslawsoltes/FerroUI/commit/c88ad28f4fad60e6739fafbb18355c659a0a417b) fixes that production lifecycle gap through one presenter-property subscription helper used on attachment, attached template application and detachment. Replacing a live template disposes the old listener. The existing focus path reconnects a cached presenter after reattachment. Its regression checks exact Unicode selection removal/cancellation, collection identity, stale presenter isolation, actual-focus reattachment and unchanged native history. The two previously failing paragraph fixtures retain their assertions and **passed in the complete 14-test paragraph and 171-test TextBox filters**. Its exact smoothing `60` → rejected `101` → Escape regression also passed. The terminal native and generator results above replace the earlier 20:55 UTC pending observation; the full browser workflow was superseded while pending.

That run's rich-history test reached its final presenter lookup after its history/text/payload assertions, then found no presenter on an unhosted TextBox. [`a867795a`](https://github.com/wieslawsoltes/FerroUI/commit/a867795a8f03780f295c3edd2164b112443583d3) attaches a real TestRoot, executes initial layout before editing, and asserts both initial and restored style-binding identities. It retains every original history assertion. **All five native rich-history tests passed at that exact source**, including actual Undo/Redo, application snapshot identities, restoration-before-callback ordering and presenter bindings. Its complete application suite also passed all 208 tests; its older Controls redo failure remained.

[`cdee5f26`](https://github.com/wieslawsoltes/FerroUI/commit/cdee5f26916c7bc61305b99540c203e469cdfc21) also disconnects IME/presenter ownership when an attached TextBox removes its template. Its reentrant template-installation regression passed. The other new test passed initial removal, confirmed Unicode/style/selection restoration, stale-client isolation and an actual redo/undo, then failed its final history assertion after installing another template and canceling a second composition. Source tracing found that a TwoWay template binding echoes unchanged Text through coercion before the property store checks equality. The external-replacement branch cleared redo despite no value change. [`58deaeee`](https://github.com/wieslawsoltes/FerroUI/commit/58deaeeeae2226ece8220d4abcda9d0a3678ca20) resets history only for a different external value, adds a direct no-change regression, and strengthens the lifecycle case with intermediate history/payload identities and another actual redo/undo. Its exact-source execution now **passes all 16 paragraph, 174 TextBox and 5 rich-history tests**, including the previously failing final assertion and actual native Redo/Undo after the later cancellation. The overlapping filters are not added as unique test counts; the preceding failures remain preserved.

That `58deaeee` job subsequently failed before any application test ran: Rust could not infer the collected `handles` type in the new `canvas/corner_radius.rs:206` before `handles.clone()` at line 257. [The saved error](../../../docs/ferrodesign-evidence/2026-10-22-58deaeee-ci/native-feedback-job.log#L3060-L3076) is **E0282**, exit code **101**. The later full [`124ffdb7` run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39858027509) encountered the same compilation failure in both native and browser jobs. Its preceding 498-test core pass and focused framework passes do not establish an application or browser pass. The [preserved full and feedback logs](../../../docs/ferrodesign-evidence/2026-10-22-124ffdb7-ci/README.md) retain those exact outcomes; no radius-gesture runtime conclusion follows from a build that did not complete.

Published follow-up [`5cb7e884`](https://github.com/wieslawsoltes/FerroUI/commit/5cb7e8847746a2bfae45bf46cafed40889f1affb) supplies the explicit `Vec<Handle>` type and adds an independent native dense-curve coverage check plus bounded browser alpha diagnostics. Its [complete native feedback job passed](../../../docs/ferrodesign-evidence/2026-10-22-5cb7e884-native/README.md), including all **217 application tests**. The dense reference samples the cubic Bernstein equations independently of renderer flattening, checks convergence from 256 to 512 samples per cubic, and passes the original alpha threshold alongside PNG round-trip and exact retained/fresh/history comparisons. The existing browser `>12` alpha / at-most-48-pixel criterion stays enforced. Both requested browser surfaces export through the same CPU offscreen renderer, so the earlier repeated `288` result does not establish two independent export rasterizers. The [full browser workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047469) requires its own completed evidence; the native pass does not resolve that browser failure.

The [cache investigation](../../../docs/ferrodesign-evidence/2026-10-22-124ffdb7-ci/NATIVE_CACHE.md) found a successful restore followed by recompilation of unchanged repository crates, but the outdated logging target emitted no reasons. An isolated dependency-free probe confirms the pinned Cargo logger is `cargo::compiler::fingerprint`; the native workflow now enables its trace level. This changes diagnostic output only. No freshness-policy change or build-speed claim follows from this probe.

The browser runner now retains an assertion failure and its per-scenario evidence while allowing later independent scenarios only after the existing unchanged-draft, committed-storage, native-panic and exact-reload checks succeed. A failed boundary aborts that renderer session. Failure capture is bounded to 20 seconds; its primary error and sticky failed result are recorded first, and a capture timeout aborts rather than continuing an unresponsive session. The final result fails if any scenario failed, even when later features pass. Five pure orchestration tests pass locally, including failed boundary, retained partial counts, capture errors and a hanging capture. These tests validate collection behavior, not browser feature execution, and the smooth-corner assertion remains unchanged.

The failed `ee1999b9` browser summary has `scope.full: true`, both modes and `scenarios: "all"`; this records the requested scope, not completed coverage. The common gates and vector-network/native-authoring scenarios passed in both modes, including automatic transverse-crossing preparation and atomic overlapping-edge rejection. Each mode then failed after a valid smoothing edit, rejected `101` draft and Escape: the field text was `""` instead of `"60"` at `corners-browser.mjs:34`. The preceding assertion had verified the actual invalid field text as `"101"` and unchanged document/history, so this is not a completed corner check. No later corner export or remaining feature success is inferred. WebGL2 reported ANGLE/Vulkan/SwiftShader.

Checkpoint `40dafdab` corrects rejected inspector Enter so the field retains focus and Escape restores its confirmed text instead of clearing canvas selection. It also makes explicit same-value radius entry detach a variable binding while passive blur remains unchanged. Its [completed native feedback](../../../docs/ferrodesign-evidence/2026-10-22-40dafdab-native/README.md) had **206 passed / 1 failed** application tests: restored `"12.50"` did not match the fixture's `"12.5"`. [`54742a5e`](https://github.com/wieslawsoltes/FerroUI/commit/54742a5ef53fbcdde7e9e5f48ccbda8325ee6b7f) uses the existing two-decimal formatter in native/browser expectations and submits identical displayed bytes to exercise real no-value-change binding detachment. The complete **208-test application suite passed at `cdee5f26`**.

The terminal [40dafdab browser evidence](../../../docs/ferrodesign-evidence/2026-10-22-40dafdab-browser/README.md) now replaces its intermediate status. The actual Vello WASM build and all five real IndexedDB checks passed. The unfiltered editor invocation (`scope.full: true`, both modes, all scenarios requested) completed **24 checks per mode**: common gates, `vector-network`, `native-authoring`, and the first `corners` smoothing check, including invalid Enter/Escape and exact Undo/Redo. Both modes then failed the independent SVG-decoder/native PNG silhouette assertion with **288** alpha differences at `corners-browser.mjs:77`. Later feature scenarios were not reached. The separate text benchmark completed successfully after the editor failure. This is a distinct comparison failure, not the canonical numeric-display mismatch; the decoded log alone does not establish a geometry/export/decoder cause or a fix. That source predates the later Controls and radius-handle changes.

Checkpoint `2b0fe4e1` makes generic XAML background selection skip inactive fills (`visible && opacity > 0`). Its native regression `text_xaml_ancestor_background_ignores_visible_zero_opacity_fills` **passed** within the 205-test application suite. That result covers the correction at `2b0fe4e1`; it does not validate subsequent inspector/framework changes.

### Text-pan phase measurements at `40dafdab`

The [later benchmark evidence](../../../docs/ferrodesign-evidence/2026-10-22-40dafdab-browser/README.md#text-pan-measurements) preserves **36 measured native pan observations per mode** using the `compact-v1` bridge. Its [phase JSON](../../../docs/ferrodesign-evidence/2026-10-22-40dafdab-browser/text-pan-phases.json) contains the unchanged objects printed at browser-log lines **1407 and 1410**; rounded primary durations are at **1405 and 1408**. The separate benchmark pass does not alter the failed editor/job outcome.

| Observation | Software2D | WebGL2 |
| --- | ---: | ---: |
| Dispatch through expected camera/new native frame, median / p95 | **15.61 / 17.43 ms** | **16.12 / 1,310.39 ms** |
| Host pointer dispatch p95 | **13.89 ms** | **1,304.93 ms** |
| Additional frame-observation wait p95 | **5.46 ms** | **9.10 ms** |
| Cumulative dispatch through native-presentation p95 | **103.07 ms** | **1,397.97 ms** |

The three printed slow WebGL2 samples show approximately **1297.4–1307.8 ms** from browser event timestamp to capture observation, versus **0.3–0.4 ms** from capture to bubble, using one page clock. Their preparation deltas are zero and each advances native rendering/presentation. Those samples locate recorded delay before capture but do not identify its scheduler, compositor, GPU or transport cause. Node-clock phase durations and page-clock timestamps remain separate. Presentation timing includes dispatch, and phase quantiles cannot be added. These software-GPU observations establish neither physical-GPU timing nor a cross-version speedup; the high WebGL2 tail remains. The optional trace diagnostic introduced later at `124ffdb7` did not execute in its failed browser build.

### Text-pan phase measurements at `ee1999b9`

After the failed editor suite, the browser job completed the independent version-2 text-pan benchmark with **150 visible Inter labels, 12 warmups and 36 measured pointer motions per surface**. It used the `compact-v1` frame-state bridge. The [phase records](../../../docs/ferrodesign-evidence/2026-10-22-ee1999b9-ci/text-pan-phases.json) match the JSON printed at browser-log lines **1352 and 1355**; the rounded overall aggregates are at lines **1350 and 1353**. [Evidence and measurement boundaries](../../../docs/ferrodesign-evidence/2026-10-22-ee1999b9-ci/README.md#text-pan-measurements) are retained with the source.

| Observation | Software2D median / p95 | WebGL2 median / p95 |
| --- | ---: | ---: |
| Dispatch through expected camera/new native-frame observation | **15.13 / 21.51 ms** | **15.64 / 1,275.36 ms** |
| Host pointer dispatch | **12.84 / 13.78 ms** | **11.42 / 1,265.21 ms** |
| Additional frame-observation wait after dispatch | **2.23 / 6.27 ms** | **4.08 / 10.15 ms** |
| Cumulative dispatch through native-presentation observation | **76.61 / 109.21 ms** | **127.01 / 1,358.52 ms** |

Presentation timing starts at the same dispatch origin as the first row; it includes protocol work and observation of a newer public presentation counter. It is not an extra phase to add to the primary duration, and quantiles from separate distributions are not additive. Settling includes two later animation callbacks and is recorded separately. Large WebGL2 delays appear in the dispatch phase itself, but these observations do not identify their cause or isolate glyph, raster or GPU costs. Automation, input dispatch, bridge serialization and scheduling remain included; a presentation counter is not GPU completion. The historical method differs, so these values establish neither a cross-version speedup nor FPS, physical-GPU throughput or Figma parity. The high WebGL2 tail remains part of the result.

Official artifact metadata is preserved, but the ZIP bytes and complete raw editor/benchmark reports were not retrieved. The logs establish the successful build, failed editor checks, passing benchmark command and printed values. They do not support additional screenshot-total/CRC claims, full page/console-error-array inspection or binary comparisons with other builds.

### Earlier continuation feedback and corrections

Native [run 39843802451](https://github.com/wieslawsoltes/FerroUI/actions/runs/39843802451), source [`605f04c36e7a0ebebb0ed9afbe4eab4518ecb356`](https://github.com/wieslawsoltes/FerroUI/commit/605f04c36e7a0ebebb0ed9afbe4eab4518ecb356), passed **8 Vello glyph tests** and **15 FerroUI.Base paragraph tests**. The filtered Controls paragraph suite passed **11 of 13 tests**; two fixtures failed after a presenter lookup returned `None`. The three new IME regressions passed, but the later native history and application suites were not reached in that run. A follow-up fixture correction attaches the host before searching the visual tree. The later `ee1999b9` and `2b0fe4e1` runs found the presenter and reached the post-preedit text assertions, which still failed as recorded above.

The complete workflow's [browser job 120053553887 in run 39843802496](https://github.com/wieslawsoltes/FerroUI/actions/runs/39843802496/job/120053553887) at `605f04c3` also failed: its decoded log reports Rust `E0659`, an ambiguous `core` import at `canvas/vector_width.rs:4`. Chromium did not launch. The same import blocked an earlier attempt at source `a060`; checkpoint `b8a8ce9e` qualifies it. Those unsuccessful builds remain failures. The later actual WASM success at `ee1999b9`, recorded above, establishes that later build without converting its failed editor suite into a pass.

The source includes regressions for the new geometry, exports, guide bindings, rich history/IME and scoped renderer behavior. The 493 core and 204/205 application passes execute their stated suites. They are not substituted for the failed framework attachment path or the browser scenarios that were never reached. No measurement here demonstrates recovery of the historical WebGL2 tail or a general frame-rate improvement.

### Remaining parity boundary

[FEATURES.md](FEATURES.md) inventories implementation limits. There is no `.fig`/Figma clipboard interoperability, multiplayer/cloud account or permission service, remote library publishing, collaborative branches, plugin/widget runtime, full Dev Mode, complete prototype timeline/physics/video system or separate Figma product suite. Artwork retains bounded/declined geometry cases, explicit native text-overlay and export subsets, and incomplete color-management/effects interchange. No matched Figma document/reference capture establishes complete visual parity. Vello/public FerroUI rendering and successful tests of this editor cannot substitute for those missing capabilities.

## Complete predecessor checkpoint: `76d3744c`

[Run 39825808707](https://github.com/wieslawsoltes/FerroUI/actions/runs/39825808707), exact source [`76d3744cca13ab1580d0dc4478d403fff8b3239c`](https://github.com/wieslawsoltes/FerroUI/commit/76d3744cca13ab1580d0dc4478d403fff8b3239c), **completed successfully**. Its [native job 119988671222](https://github.com/wieslawsoltes/FerroUI/actions/runs/39825808707/job/119988671222) passed **394 core / 155 application tests** and **19 persistence-host tests**. Its [browser job 119988670854](https://github.com/wieslawsoltes/FerroUI/actions/runs/39825808707/job/119988670854) built the actual Vello WebAssembly application and passed **141 Software2D / 138 WebGL2 checks**, all **28 scenarios per mode**, with `passed: true` and `scope.full: true`. Real Chromium persistence checks also passed.

The [preserved decoded logs, printed summary and API/artifact metadata](../../../docs/ferrodesign-evidence/2026-10-22-76d3744c-ci/README.md) establish this exact predecessor's result. Two guarded compositor capture retries occurred for native local styles and native grid style before the successful terminal result. Official build/validation ZIP bytes were not retrieved, so their raw reports, capture contents, CRCs and runtime binary equality were not independently checked. This is not an all-first-attempt capture claim.

The separate historical 150-label pan benchmark recorded **14.49 ms median / 18.53 ms p95** for Software2D and **20.18 ms median / 1,412.95 ms p95** for WebGL2, with **36 observations per mode**. It used the older observation method and the CI software GPU stack. The high WebGL2 tail remains part of the result; these are not FPS or physical-GPU measurements and do not measure the subsequent glyph-cache or phase-separated benchmark changes.

This complete checkpoint follows the earlier matching-harness correction but predates the new production features listed above. Its full result supersedes no preserved failure and makes no current-head parity claim.

## Historical continuation: complete `f2bb1d61` CI and `f8e0471e` matching correction

This predecessor introduced native stacked-guide authoring, matching/inverse selection and transactional face-bucket painting. The source and run identities below preserve the evidence for that earlier implementation; its newer extensions belong to the current continuation above.

The original browser harness checkpoint is [`f2bb1d618843bf777b76859d2e06bf361bef009b`](https://github.com/wieslawsoltes/FerroUI/commit/f2bb1d618843bf777b76859d2e06bf361bef009b). Its independent CI workflow completed successfully, including the full browser suite: **141 Software2D / 138 WebGL2 checks and all 28 scenarios in each mode**. The [comparison through `e0d3cee3`](evidence/2026-10-22-f2bb1d61/successful-ci-input-equivalence.json) confirms unchanged application, framework, XAML, assets, Cargo and build inputs only over that specified commit range. Its sole non-documentation change was the test-only [`f8e0471e` matching correction](https://github.com/wieslawsoltes/FerroUI/commit/f8e0471eb4301a9b893e8185810550ebba5ddafc), whose focused run passed **25 Software2D / 22 WebGL2** checks. The later complete `76d3744c` result is recorded separately above; neither comparison extends to the current production changes.

The earlier local attempts used a recovered [`3a0bf1f1`](https://github.com/wieslawsoltes/FerroUI/commit/3a0bf1f13c6ede9abe5360bc3691085a3bc981f0) runtime. Both full attempts failed after Software2D passed 141 checks and WebGL2 passed 21, then 27 checks. A fresh WebGL2-only attempt failed after 82 checks and 16 completed scenarios at the matching-dialog opener. The subsequent 11-scenario attempt with the corrected matching harness was blocked before Chromium started. Those outcomes are preserved below and are not combined to infer the independent successful CI result.

The [`f2bb1d61` evidence index](evidence/2026-10-22-f2bb1d61/README.md) records these outcomes separately. Its input-equivalence manifest also identifies documentation-only checkpoint `177f92cd` and verifies that the frozen harness/package files are unchanged by that documentation commit.

### Complete browser CI at `f2bb1d61`

[FerroDesign run 39799180117](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117) was observed **completed/success at 16:50:41 UTC on 22 October 2026**. The [browser job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117/job/119894677332) completed successfully at **16:43:12 UTC**. Its decoded log records the exact checkout, the real Vello WASM build and a printed editor summary with `passed: true`, `scope.full: true`, both modes and `scenarios: "all"`.

| CI browser result | Software2D | WebGL2 |
| --- | --- | --- |
| Passed checks | **141** | **138** |
| Feature scenarios | All **28** | All **28** |
| Reported native surface | Software2D | WebGL2, ANGLE/Vulkan/SwiftShader |

The [source-tagged summary](evidence/2026-10-22-f2bb1d61/successful-ci-browser-summary.json) and [decoded-log archive](evidence/2026-10-22-f2bb1d61/successful-ci-browser-log.zip) preserve the evidence. The printed JSON was extracted from log lines 1243–1554 by removing only the timestamp prefix; all 28 ordered scenario starts per mode also appear in the log. The complete run includes the matching, stacked-guide, regional paint, bucket, prototype and bounded-history checks. Two WebGL2 capture warnings identify the existing guarded retry for **native local styles** and **native grid style**; the suite subsequently passed. This is not an all-first-attempt capture claim.

The archive is a locally packaged copy of the connector-decoded log and exact printed JSON with checked member hashes, not the official GitHub validation artifact. [Artifact/API metadata](../../../docs/ferrodesign-evidence/2026-10-22-f2bb1d61/successful-ci-artifact-metadata.json) and the [build-download record](evidence/2026-10-22-f2bb1d61/successful-ci-build-download-status.json) identify the published artifacts and retrieval limits. Their bytes, CRCs, raw `report.json`, capture sidecars, detailed page/console error arrays and runtime-member hashes were not independently retrieved. The successful build and complete gate are established by the exact-source log and API status; binary equality with the recovered local runtime is not asserted.

The [source-input comparison](evidence/2026-10-22-f2bb1d61/successful-ci-input-equivalence.json) finds 84 changed paths across four commits from tested `f2bb1d61` through `e0d3cee3`: README/evidence documentation and one matching-test module. No production/build input changed within that comparison. Its equivalence does not cover the newer production changes. At the **16:50:41 UTC** historical observation, [`e0d3cee3` run 39814628155](https://github.com/wieslawsoltes/FerroUI/actions/runs/39814628155) was in progress; that was an intermediate status rather than a terminal result. SwiftShader is not physical-GPU performance evidence, and neither the complete predecessor passes nor this input comparison establishes full Figma feature or visual parity.

### Completed native, build and framework gates

| Gate | Source-specific completed observation |
| --- | --- |
| Document core | **394** tests passed at exact source `f2bb1d61`; no failed or ignored tests. The earlier `f36dabf7` and `3a0bf1f1` jobs passed the same count |
| Native application | **155** tests passed at `f2bb1d61`, including actual Vello pixels, public-API rendering, guide clipping, gesture ownership and parser regressions; the two earlier jobs also passed all 155 |
| Persistence host | All **19** tests passed in the successful native jobs at `f2bb1d61` and `3a0bf1f1` |
| Actual Chromium IndexedDB | All **5** checks passed at `f2bb1d61` ([exact log extraction](evidence/2026-10-22-f2bb1d61/successful-ci-persistence-summary.json)) and previously at `3a0bf1f1`, including migration/rollback/conflict handling and reload of **12,000,042** document characters |
| Framework browser bridge | Type checking, all **10** input/composition cases and bundling passed during the `3a0bf1f1` WASM build |
| Actual Vello WebAssembly application | The exact `f2bb1d61` CI build and complete two-mode suite passed. Earlier builds completed at `f36dabf7` and `3a0bf1f1`, whose later editor suites failed |
| Resolved application dependencies | The exact `f2bb1d61` native job verified locked normal/build graphs containing Vello and no active Skia packages for Emscripten, Linux x64, Windows x64, macOS ARM64 and macOS x64 |
| Repository framework CI | All **14** jobs passed at `f36dabf7` in [run 39765413203](https://github.com/wieslawsoltes/FerroUI/actions/runs/39765413203), and all **14** passed at `3a0bf1f1` in [run 39771211233](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771211233) |

The source-tagged [native job 119894677519 in run 39799180117](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117/job/119894677519) completed successfully at 15:05:07 UTC, with **549 combined core/application tests** and the separate 19 host tests. Its [summary](evidence/2026-10-22-f2bb1d61/native-ci-summary.json) and decoded-log archive are linked in the `f2bb1d61` evidence index. The [earlier continuation index](evidence/2026-10-22-f36dabf7/README.md) retains the previous native logs, framework summaries, dependency graphs and extraction provenance under their actual source identities. Dependency checks establish backend selection, and the standard framework workflow establishes its own platform coverage. Neither substitutes for executing the FerroDesign native GUI on every supported desktop platform. The IndexedDB test exercises a real browser but is separate from the actual FerroUI editor interaction suite.

### Browser failures retained with their actual scope

| Run | Software2D | WebGL2 | Overall result |
| --- | --- | --- | --- |
| First local full attempt, harness `f36dabf7` | 141 checks passed | 74 checks passed, then the explicit layer-selection menu assertion failed | Failed |
| Palette diagnostic, harness `3a0bf1f1` | 38 checks passed | 35 checks passed | Passed only selection, matching and Flatten plus common gates; `full: false` |
| Second local full attempt, harness `3a0bf1f1` | 141 checks passed | 118 checks passed, then entering vector editing in the regional-image fixture failed | Failed |
| [FerroDesign CI 39765407889](https://github.com/wieslawsoltes/FerroUI/actions/runs/39765407889), `f36dabf7` | 88 checks passed, then the parent-resize constraint assertion failed | 95 checks passed, then a native local-style PNG capture exceeded its deadline and retry | Failed; native job succeeded |
| [FerroDesign CI 39771204240](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771204240), `3a0bf1f1` | 88 checks passed, then the same parent-resize assertion failed | 118 checks passed, then the regional-image fixture failed to enter vector editing | Failed; native job succeeded |
| First full recovered-runtime attempt, original `f2bb1d61` harness / `3a0bf1f1` runtime | 141 checks passed | 21 checks passed, then the alias-deletion document predicate timed out | Failed; `full: true` |
| Focused native-authoring diagnostic, instrumented copy of `f2bb1d61` / same runtime | Not requested | 21 checks passed, including both native-authoring cases and common gates | Passed only its requested scope; `full: false` |
| Second full recovered-runtime attempt, original `f2bb1d61` harness / same runtime | 141 checks and all 28 scenarios passed | 27 checks passed, including native-authoring and typography, then `ComponentValue2` readiness failed in component authoring | Failed; `full: true` |
| Fresh WebGL2-only attempt, original `f2bb1d61` harness / same runtime, all 28 scenarios requested | Not requested | 82 checks and 16 scenarios passed; matching was the 17th scenario entered and its second dialog cycle failed | Failed; `full: false` |
| Focused matching validation, corrected `f8e0471e` harness / same runtime | 25 checks passed, including all three matching cases | 22 checks passed, including all three matching cases | Passed only matching plus common gates; `full: false` |
| Additional `f8e0471e` WebGL2 coverage, remaining 11 feature scenarios | Not requested | Zero checks or scenarios; Chromium did not start | Environment blocked before validation |

Both earlier CI runs at `f36dabf7` and `3a0bf1f1` completed as failures; neither is treated as cancelled by a later push. The [preserved reports, traces, failure documents and decoded CI logs](evidence/2026-10-22-f36dabf7/README.md#preserved-browser-outcomes) retain those failures. A completed mode inside an overall failed run is not a passing complete run. Earlier focused counts also do not cover scenarios they skipped.

The first local failure exposed native palette publication timing. The `3a0bf1f1` tests wait after filtering/recentering and require the actual outcome of one native action click. The later geometry and vector-entry failures exposed keyboard-selection transitions where the new selection model preceded the published native input scene. In the CI geometry failure, the child rectangle became `[20, 10, 40, 160]` instead of the asserted `[20, 40, 140, 30]`. The `f2bb1d61` tests now wait for both a new native frame and input scene after selecting the exact parent or network root, then assert actual parent dimensions and unchanged child-constraint expectations. Vector tests require entry into vector editing and canvas focus before proceeding. The document, history, error-outcome and pixel assertions remain enforced; these changes do not add action retries or change application behavior.

The `f36dabf7` screenshot failed its original 30-second attempt and existing 15-second guarded retry; the failure capture also timed out. The retained trace showed loaded fonts, responsive native state, unchanged document/view state, an intact rendering context and continuing frame delivery. Some trace frames arrived 27–32 seconds after their recorded swap times. This supports the bounded `f2bb1d61` change to a **30-second first attempt and one 60-second guarded retry**, but does not identify the exact queue responsible or establish that the longer retry recovers that failure. The retry retains state and context-health checks and actual PNG pixel validation. No screenshot timeout occurred in the later `3a0bf1f1` CI run, which instead failed vector entry.

Two subsequent focused local runs passed before the workspace was lost, but their latest raw reports were not recovered. Their counts are recorded as contextual observations in the evidence index and are not linked as available raw proof. The later historical recovered-runtime attempts are recorded separately below.

The later recovered-runtime failure is separately preserved in the [new evidence directory](evidence/2026-10-22-f2bb1d61/README.md#original-full-browser-attempt). Its alias-deletion wait had a 10-second deadline but returned its timeout after 34.294 seconds; the subsequent failure-state evaluation took 118.002 seconds. A trace image received about 391 ms after the wait began already showed the intended deletion, and the eventual exported document confirmed it with one history entry. The original trace does not record predicate invocation times or identify the cause of the scheduling/protocol delay. Cleanup-time cgroup observations include new OOM events, but do not identify the killed process or attribute the earlier stall.

The [focused diagnostic](evidence/2026-10-22-f2bb1d61/README.md#focused-diagnostic-with-read-only-instrumentation) added read-only timestamps only in a separate copy. It retained the original one-click path, 10-second deadline, default animation-frame polling and all document/history/pixel assertions. All nine predicates returned true on their first evaluation; the final alias-deletion wait took 6 ms. Its 21-check pass and three successful first-attempt captures do not replace the full gate or prove the earlier stall's cause. The diagnostic logging itself can affect scheduling. No application or published harness change was made for this failure; the second full attempt used the original uninstrumented files.

The [second full attempt](evidence/2026-10-22-f2bb1d61/README.md#second-full-browser-attempt) completed at **15:39:17 UTC** with a failed result. Software2D passed all 28 scenarios and 141 checks. WebGL2 passed vector-network, native-authoring and typography, then stopped at `ComponentValue2` readiness in `components-browser.mjs`. All **53** shared native captures—46 Software2D and 7 WebGL2—passed on their first attempt. There were no page errors; Software2D retained one resource-404 console message, and WebGL2 reported no console errors. Post-run integrity checks found all **31 source files and 16 runtime files unchanged**. The cause remains under investigation; no application or test correction is attributed to this result.

The second attempt's resource samples reached the shared cgroup memory limit of **8,589,934,592 bytes**. `memory.events.max` increased by **1,631,582**, while `oom` and `oom_kill` did not increase. The peak summed RSS of PID/path-matched processes was **2,373,701,632 bytes**. The [sampler audit](evidence/2026-10-22-f2bb1d61/resource-sampler-audit.json) establishes that the selection matches the launched Node PID or Chromium directory in visible process command lines; it is not a complete descendant or ownership inventory. It includes matching reparented helpers, but can miss nonmatching, unreadable or namespace-invisible processes and can include another invocation using the same binary path. Cgroup totals can include other work and cache; summed RSS can count shared pages more than once. These observations do not identify an exclusive editor heap, a leak or the cause of the readiness failure.

### WebGL2-only matching failure and test correction

The [fresh WebGL2-only summary](evidence/2026-10-22-f2bb1d61/failed-webgl2-all-scenarios-summary.json) records a failed run of the original `f2bb1d61` harness against the same recovered runtime. It requested all 28 scenarios in a fresh Chromium process, but only one mode, so `scope.full` is **false**. It passed **82 checks and 16 scenarios**, then failed during the 17th scenario at `matching-browser.mjs:83`. All **29** shared native captures passed on their first attempt. There were no page errors and one resource-404 console message. All **31 source files and 16 runtime files** remained unchanged. The [selected raw evidence archive](evidence/2026-10-22-f2bb1d61/failed-webgl2-all-scenarios-report.zip) retains the report, failure document/PNG, original test, manifests, resource records and selected trace images; it omits the large original Playwright trace ZIP.

The [matching diagnosis](evidence/2026-10-22-f2bb1d61/failed-matching-diagnosis.json) distinguishes live properties from the presented inspector. Escape cleared selection without a separate publication wait. The next canvas click changed live selection to `[2]` and advanced counters from frame/scene **6/8 to 7/9**, satisfying the existing waits immediately. A trace image with swap time **1,264,418.613 ms** still showed empty selection and the page inspector. The single matching-opener click followed at **1,264,521.535 ms**; the first later image showing the selected source and Rectangle inspector has swap time **1,264,935.260 ms**, after that click. The modal remained hidden and no stroke-row click occurred. The configured 10-second readiness wait returned after **10.005 seconds**, with surrounding state evaluations taking **8–12 ms**. Near this failure cgroup memory was **4.18–4.27 GB** and its `max`, `oom` and `oom_kill` counters were stable; the whole-run memory peak is not evidence that pressure caused this event.

The supported inference is that the unseparated Escape transition can satisfy a later selection's frame/scene floors while its newly selected inspector has not yet been presented. The scene counter is a coalesced notification, not a per-control generation. The trace does not expose the actual native hit-test target, pointer-capture owner or button callback, and presentation timestamps alone do not prove the input scene used. The evidence therefore identifies a missing test boundary without establishing a particular application defect or complete event-consumption path.

Checkpoint `f8e0471e` changes only the matching test: when a selection exists, it snapshots state before Escape, requires exact empty selection and both newer frame/scene counters, and checks the page/layer/action visibility before the existing source click. When selection is already empty, it omits the redundant Escape. After the single opener click, it requires `ModalBackdrop` to become visible before the existing named-control readiness check and row click. Exact selection, modal closure, authored-document, history, redo and revision assertions remain intact. The 10-second deadlines remain unchanged; no retry, fixed delay, extra click or runtime modification was added.

The corrected [focused matching run](evidence/2026-10-22-f8e0471e/matching-focused-summary.json) **passed 25 Software2D / 22 WebGL2 checks**, including all three matching cases in each mode, in **180.13 seconds**. Its scope is both modes with matching plus common gates only (`full: false`). All **four** shared native captures passed on their first attempt. There were no page errors; Software2D retained one resource-404 console message and WebGL2 had no console errors. All **31 source files and 16 runtime files** remained unchanged during the run. The [separate raw evidence archive](evidence/2026-10-22-f8e0471e/matching-focused-report.zip) preserves this corrected-source result without replacing the original failure. It validates the corrected test path under these conditions, not the original native event target or every feature scenario.

The separate attempt to cover the **11 WebGL2 feature scenarios after matching** was **blocked before Chromium startup**: the local HTTP server could not listen on `127.0.0.1` (`EPERM`). The [launch summary](evidence/2026-10-22-f8e0471e/remaining-webgl2-launch-summary.json) records **zero checks, zero entered scenarios and no browser `report.json`**, with all **31 source files and 16 runtime files unchanged**. The [launch evidence archive](evidence/2026-10-22-f8e0471e/remaining-webgl2-launch-block.zip) preserves the error, runner, manifests and resource/terminal records. No escalation, alternative execution path or further attempt followed the denial. This is an environment block, not a failed application assertion. Geometry authoring, Flatten, local styles, angular paints, grid styles, region paints, network bucket, compact autosave, prototype input, prototype motion and bounded history remain outstanding for that coverage attempt. The focused 25/22 pass stands separately; different invocations and harness versions are not combined into a complete-suite pass.

### Local recovered-runtime identity and terminal browser results

| Input | Exact identity |
| --- | --- |
| Browser test harness | `f2bb1d618843bf777b76859d2e06bf361bef009b` |
| Corrected focused matching harness | `f8e0471eb4301a9b893e8185810550ebba5ddafc` |
| CI distribution build source | `3a0bf1f13c6ede9abe5360bc3691085a3bc981f0` |
| Recovered build artifact | GitHub artifact `12427798904` |
| Actual WASM payload | **51,205,801 bytes** |
| WASM SHA-256 | `7880a628e7117ab88f896f69baa9c9ccedcdc33792302a01f653b4465e1499b9` |
| Latest completed full recovered-runtime attempt | Both modes, all scenarios; **failed** with Software2D 141 checks / all 28 scenarios passed and WebGL2 27 checks before component-control readiness failed |
| Latest completed local single-mode attempt | WebGL2, all 28 scenarios requested; **failed** with 82 checks / 16 scenarios passed before the matching failure; `full: false` |
| Corrected focused matching attempt | `f8e0471e`, both modes, matching plus common gates; **passed 25/22 checks**, `full: false` |
| Additional remaining-scenario coverage | `f8e0471e`, WebGL2 only, 11 feature scenarios; **blocked before Chromium**, zero checks/scenarios |

This recovered CI runtime is a different binary from the earlier local runtime; their digests are not interchangeable. Its application source is compatible with the test-only `f2bb1d61` change, and the run records the build and harness identities separately. WebGL2 uses Chromium's ANGLE/Vulkan/SwiftShader path in these runs, so the results cannot establish physical-GPU throughput. No matched Figma document/reference screenshot was available for a numerical visual-parity comparison.

The independent exact-`f2bb1d61` CI job later completed successfully with its own freshly built runtime, as recorded in [the complete CI result above](#complete-browser-ci-at-f2bb1d61). That result does not replace the local failures or imply binary identity with this recovered distribution. The matching correction retains its separate focused result. The `e0d3cee3` workflow was still in progress at the historical **16:50:41 UTC** observation; the subsequently confirmed complete `76d3744c` checkpoint has its own record above.

### Source-delivery audit

The [read-only audit at `b83deee6`](evidence/2026-10-22-f2bb1d61/source-delivery-audit.json) inspected the tracked repository and build scripts. It found **31 Cargo manifests and 151 path-dependency edges** in a conservative closure across all targets, optional dependencies and development dependencies, all inside the repository and tracked. Required application XAML, embedded fonts, framework themes, browser bridge/loader inputs and lockfiles were present. There were no gitlinks, LFS pointers or archive-exclusion rules, and all 96 symlinks resolved. This source inventory is distinct from the active Vello/no-Skia graph checks.

The inspected build logic does not require FerroUI's `.git` metadata, but it does require the complete repository layout, including root `.cargo` configuration. Setup uses Git to fetch Emscripten and still downloads external toolchains and packages. The pinned public archive URL returned HTTP 200; no archive body, extraction or build from that ZIP was tested. This audit does not replace the separately recorded native and browser build evidence.

### Local history and restored documents

The production editor defaults to **100 committed undo entries**. Starting from an autosaved document constructs `Editor::new` from the restored document, with empty undo/redo history; startup does not create an undoable import from the demo. Undo history belongs to the editor session and is not part of portable JSON or durable document storage.

The [browser history regression](../tests/history-browser.mjs) crosses the production cap, makes one additional edit and checks exact one-step undo/redo with the expected history lengths. It does **not** traverse to or verify the oldest reachable undo state. The harness also starts each feature scenario from a durable restored document with fresh session history. These are local-session contracts, not remote collaborative or cloud version-history coverage. The second full recovered-runtime attempt passed the bounded-history case in Software2D and failed earlier in WebGL2. The separate exact-`f2bb1d61` CI suite subsequently passed that case in both modes, as did the complete predecessor `76d3744c` run. This establishes the stated newest-transaction checks at those sources, not traversal to the oldest reachable history state or validation of the newer native rich-text history implementation.

### Browser workload from the second recovered full attempt

The [second full-run terminal summary](evidence/2026-10-22-f2bb1d61/failed-second-full-summary.json) records a completed Software2D mode with all 141 checks and 28 feature scenarios. The overall two-mode run failed later in WebGL2; these measurements belong only to its completed Software2D workload. They are single observations on this shared host, not medians, FPS or physical-GPU measurements.

| Software2D workload | Observed result |
| --- | --- |
| Document and layer model | 10,000 nodes / 10,000 layer rows |
| Realized native layer controls | 25 before scrolling; 25 after scrolling |
| Import through first-frame/state verification | 1,217 ms |
| Keyboard nudge through preparation verification | 181 ms |
| Nudge / undo / redo preparation | Each prepared one node incrementally; zero full preparation and zero visual rebuilds |
| Layer projection on those three commands | Zero rebuilds; one reuse each |
| Zoomed root query | One candidate and 27 bounds tests among 10,000 roots, with one visible node |
| Compact autosave serialization | 42.6 ms; 13,192,068 bytes |
| Reload through exact restored-document verification | 2,135 ms |
| Portable JSON export | 20,712,219 bytes |

The browser timing boundaries include automation/protocol work and the waits in [the actual fixture](../tests/browser.mjs). The nudge measurement includes keyboard dispatch and the native preparation/frame verification; it is not the engine-only command cost measured below. The summary also records a 4 ms explicit persistence flush wait at that point in the workload; background serialization or saving may already have progressed before that wait begins, so it is not a complete end-to-end save measurement. The reloaded export digest matches the authored document exactly. This browser fixture differs from the native release benchmark's document and process.

### CI text-panning observations at `f2bb1d61`

After the complete editor suite, the same successful historical browser job ran its separate text-panning benchmark. That revision imported a seeded scene of **150 visible Inter text labels**, performed **12 warmup motions** and measured **36 pan observations per mode**. Each timed interval began before host pointer dispatch and ended after the expected camera position and a newer native `renderedFrames` value were observed. Two subsequent animation-frame callbacks were recorded separately as settling observations. The [current benchmark](../tests/text-render-benchmark.mjs) now separates more phases and uses a compact state bridge; the historical aggregate below is not a measurement of that newer method or glyph cache.

| Mode | Observations | Median, rounded log value | p95, rounded log value |
| --- | ---: | ---: | ---: |
| Software2D | 36 | **14.28 ms** | **20.62 ms** |
| WebGL2, SwiftShader requested | 36 | **20.30 ms** | **1,366.94 ms** |

These values and the verified measurement method are preserved in the [derived text-panning summary](evidence/2026-10-22-f2bb1d61/successful-ci-text-pan-summary.json) and decoded job log at lines 1567–1568. The source requests ANGLE/SwiftShader, but the benchmark's original device fingerprint was not retrieved. Only the two-decimal printed aggregates were retrieved; the official artifact's individual samples and richer benchmark JSON were not. They include automation, input dispatch, scheduling and frame observation; the additional post-observation settling callbacks are excluded from these reported values. They are not FPS, isolated drawing/GPU costs or a physical-GPU measurement. The high WebGL2 p95 remains part of the observation; the native model benchmark below must not be substituted for this browser tail latency.

### Paired release measurements at `f2bb1d61`

The historical native job ran `cargo run --locked --release -p ferrodesign-core --example performance -- 10000` at exact source `f2bb1d618843bf777b76859d2e06bf361bef009b`. Its [schema-5 JSON](evidence/2026-10-22-f2bb1d61/native-performance/performance-extracted-log.json), [timestamped step](evidence/2026-10-22-f2bb1d61/native-performance/performance-step.log), [provenance](evidence/2026-10-22-f2bb1d61/native-performance/provenance.json) and [complete measurement summary](evidence/2026-10-22-f2bb1d61/native-performance/SUMMARY.md) come from the successful job's decoded log. The 7,561-byte JSON payload has SHA-256 `758f544841855f50008f394f57e7b905618bb2b2387ec125be856b17fb2afe61`; it is a timestamp-stripped log extraction, not a byte-identity claim for a separately downloaded artifact. These observations do not measure the newer stroke, planarization, paragraph or scoped inline-text paths.

The release executable measured 10,000 flat filled rectangles with one selected node. Scoped commands and the explicit full-document transaction reference ran **in the same process**. Release mode, document/history equivalence, serialization round trips and component ID/history checks passed.

| Operation | Samples per path | Full-document reference p50 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: | ---: |
| Nudge | 48 | 17.284794 ms | 0.019659 ms | 0.023785 ms |
| Undo | 48 | 5.563399 ms | 0.012579 ms | 0.016715 ms |
| Redo | 48 | 5.515978 ms | 0.012298 ms | 0.015704 ms |
| Gesture begin | 12 | 4.617348 ms | 0.012539 ms | 0.013300 ms |
| Each gesture update | 288 | 5.920511 ms | 0.000751 ms | 0.001162 ms |
| Gesture commit | 12 | 14.630404 ms | 0.007411 ms | 0.009635 ms |
| Complete 24-update gesture | 12 | 161.505031 ms | 0.048162 ms | 0.054552 ms |

The update distribution contains all 288 individually timed updates across 12 gestures; phase medians need not sum to the complete-gesture median. These are native document-engine and transaction costs. They exclude browser events/bridge work, native control layout, scene drawing, GPU execution, display frame rate and durable-storage completion. They do not measure guide drawing, matching queries or bucket geometry. The small samples and single runner describe this observation; comparison with the older CI timings below is not evidence of an implementation speed change.

Five serialization samples measured pretty JSON at **29.115533 ms p50 / 20,152,206 bytes** and compact autosave at **13.962979 ms p50 / 12,802,055 bytes**, about **36.5% smaller** with exact round-trip equivalence. Parse/validate/index measured **44.227866 ms p50** across three samples. The separate dense-layout fixture with 9,999 mixed fixed/fill text children measured **7.344085 ms p50** across five samples.

Component authoring retains **whole-document Editor history snapshots**. With eight samples per operation in its separate 10,000-node fixture, actual variant switches measured **22.029776 ms p50 / 24.553801 ms p95**; text properties **22.909969 / 28.683793 ms**; visibility properties **27.464443 / 54.469802 ms**. The fixture includes **5,592,566 bytes of unrelated image URI data**. All component operations undid and redid exactly with stable materialized IDs. The faster no-history document-operation figures are not substituted for these production Editor costs.

### Paired release measurements at `f36dabf7`

The [schema-5 performance JSON](evidence/2026-10-22-f36dabf7/native-performance/performance-extracted-log.json) was extracted from the completed native job's decoded log in [run 39765407889 / job 119783951316](https://github.com/wieslawsoltes/FerroUI/actions/runs/39765407889/job/119783951316), source `f36dabf795f9e7b42ada46e34878b3ae5fd50697`. The workflow ran `cargo run --locked --release -p ferrodesign-core --example performance -- 10000` and printed its JSON in **Compare scoped editing, history and serialization costs**. The [raw supporting step](evidence/2026-10-22-f36dabf7/native-performance/performance-step.log) and [extraction provenance](evidence/2026-10-22-f36dabf7/native-performance/provenance.json) are preserved. This is job-log extraction, not a claim of byte identity with a separately downloaded benchmark artifact. The 7,493-byte JSON extract has SHA-256 `21c34a563a17f290ae3ab9eb2aa0f8e911c03d524dfcba411bed4f82fa87f578`.

The release executable compares scoped commands with an explicit generic full-document transaction reference **in the same process**, using 10,000 flat filled rectangles and one selected node. `releaseBuild`, document/history equivalence and serialization round-trip checks all passed. The reference is a workload in this executable, not a previous application version or a measurement from another runner.

| Operation, 48 samples each | Full-document reference p50 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: |
| Nudge | 16.479897 ms | 0.027372 ms | 0.044975 ms |
| Undo | 3.731835 ms | 0.016031 ms | 0.024235 ms |
| Redo | 3.527510 ms | 0.014918 ms | 0.029846 ms |

The gesture fixture records 12 gestures with 24 updates per gesture. Each phase is measured separately; the per-update row contains all 288 updates rather than dividing a total by the update count.

| Gesture phase | Samples | Full-document reference p50 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: | ---: |
| Begin | 12 | 4.186642 ms | 0.014748 ms | 0.018134 ms |
| Each update | 288 | 3.730643 ms | 0.001012 ms | 0.001543 ms |
| Commit | 12 | 12.151327 ms | 0.012353 ms | 0.013626 ms |
| Complete 24-update gesture | 12 | 106.014222 ms | 0.063640 ms | 0.068860 ms |

These are native model/transaction costs. They exclude browser event dispatch, native control layout, scene drawing, GPU execution, display frame rate and durable-storage completion. They do not benchmark the new guide drawing, matching query or bucket geometry paths, and the per-update measurement is not end-to-end pointer latency. Phase medians need not sum to the median of complete gestures.

Size-dependent work remains measurable. Across five samples, portable pretty serialization produced **20,152,206 bytes** at **47.802605 ms p50**; compact autosave produced **12,802,055 bytes** at **21.233539 ms p50**, about **36.5% smaller**, with exact round-trip equivalence. Parse/validate/index took **59.822304 ms p50** across three samples. A separate dense-layout fixture with 9,999 mixed fixed/fill text children took **10.432199 ms p50** across five samples. These native fixtures are distinct from the historical browser workload below.

Component authoring still uses **whole-document Editor history**. Its 10,000-node fixture includes **5,592,566 bytes** of unrelated embedded-image URI data and eight operations of each kind. Actual Editor variant switching measured **21.314896 ms p50 / 21.771297 ms p95**; text properties **20.904493 / 21.967517 ms**; visibility properties **28.715560 / 29.467406 ms**. Exact history restoration and stable materialized IDs passed. The isolated no-history figures in the raw JSON are not substituted for these production Editor costs.

## Continuation checkpoint: `2004673e` (2026-10-20)

[Source `2004673e`](https://github.com/wieslawsoltes/FerroUI/commit/2004673e2829e67a74c060fd4f6acf0ce78f3bc0) adds native angular paints and checked planar vector-network face discovery. Its full core source was tested before publication; native application and actual WASM rendering validation are separate gates and remain pending at this checkpoint.

| Gate | Completed observation |
| --- | --- |
| Document core | All **340** unit tests passed, including nine angular-paint and fifteen vector-face cases; no ignored tests |
| Persistence host | All **19** Node tests passed using `--test-isolation=none` |
| Actual Chromium IndexedDB | All **5** checks passed in Chrome for Testing **156.0.8078.4**, including atomic migration/abort handling, two-tab conflict protection and reload of **12,000,042** document characters |
| Framework browser bridge | Type checking, all **10** input/composition cases and bundling passed |
| JavaScript and source checks | Changed browser modules passed `node --check`; `git diff --check` passed |
| Resolved application dependencies | Vello present and no active Skia packages in the Emscripten, Linux x64, Windows x64, macOS ARM64 and macOS x64 normal/build graphs |
| Native application / current WASM pixels | Pending; one local native build was killed by the shared host memory limit. Subsequent attempts were stopped before the shared disk filled; no local native-test result is claimed |

The dependency audit confirms backend selection, not native execution on those five platforms. The storage tests run an actual browser, but do not execute the FerroUI WASM editor. These observations do not establish new editor pixel correctness or a Figma visual-parity score.

The resumed source `f1e29bfc` had a completed [framework CI run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39431182704) with all 14 jobs passing. Its separate [FerroDesign run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39431175291) passed all 316 core cases and 110 native application cases, then failed two text-Flatten pixel comparisons: 258 and 445 pixels differed by more than 4/255. The threshold remains fewer than 252 changed pixels in a 180×140 target. The WASM build and browser suite were skipped in that run.

[Checkpoint `bf57095f`](https://github.com/wieslawsoltes/FerroUI/commit/bf57095f7256a3444482beea64f77cd41c4bb685) preserves those assertions and failure status, adds opt-in raw pixel diagnostics, and permits the independent browser build after successful native compilation even if a subsequent native pixel assertion fails. Its [CI run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39488148455) is tracked separately. A completed older run is never substituted for validation of the current source.

## Earlier validation checkpoint: `f83ad955`

[Source `f83ad955`](https://github.com/wieslawsoltes/FerroUI/commit/f83ad9558c24117673e29077275f4176676a7acd) includes the complete local-style lifecycle, public geometry and shaped-text outlines, conservative flattening, current-color paint conversion and rejected-draft menu protection. The final core test artifact contains all three shadow-composition guards in this source.

| Gate | Completed observation |
| --- | --- |
| Document core | All 316 unit tests passed; the focused flatten filter separately passed all 13 selected cases |
| Style comparison allocations | Linked and unlinked cohorts have identical allocation counts and requested bytes for both tested edit paths; see scope below |
| Resolved application dependencies | Vello present and no active `ferroui-skia`, `skia-safe` or `skia-bindings` for Emscripten, Linux x64, Windows x64 and both macOS architectures |
| Persistence host | All 17 JavaScript cases passed |
| Actual Chromium IndexedDB | All 5 migration, rollback, conflict and large-document reload checks passed, including 12,000,042 document characters |
| Framework browser bridge | Type checking, all 10 input/composition tests and bundling passed |

The host, bridge, dependency and allocation checks preceded the final editor fixes; their exercised source inputs were unchanged by those fixes. The dependency checks establish backend selection, not native GUI execution. At this checkpoint, the application WebAssembly build, full browser interactions, native pixel suite and public geometry API pixel gates were still in progress. The added assertions and older successful screenshots were not substitutes for those results. Later results are recorded separately above.

The allocation fixture performs 16 identical edits with a 262,144-byte synthetic image URI in each cohort. `update_node` requests 678 allocations and 21,659,936 bytes in both cohorts; `update_nodes_with_layout` requests 806 allocations and 25,875,872 bytes in both. These are cumulative allocator requests, not peak memory. The comparison establishes no additional binding-comparison allocation overhead for this fixture; history, cache and metadata copies still exist. It does not measure editing latency, rendering or FPS.

## Preserved validated baseline: `945283ef`

Checkpoint `945283ef` added typed native effect authoring, deferred auto-layout drag previews and transactional drops, a retained spatial index for prepared root bounds, and consistent shadow order through rounded and general geometry. Its browser regressions additionally wait for a new native frame after selection and verify actual ComboBox popup focus before sending navigation keys.

The following local checks completed on Ubuntu 24.04 using the pinned Rust 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129 tools:

| Gate | Observed result |
| --- | --- |
| Document core | All 219 tests passed, including atomic typed effect commands and deferred layout-session ownership/history |
| Persistence host | All 17 JavaScript tests passed; Node 24.19.0 used `--test-isolation=none` to expose the individual cases |
| Native application | All 64 tests passed, including native capture loss/cancellation, effect-field parsing, spatial-index/reference-pixel comparisons and eight shadow pixel cases |
| Actual WebAssembly application | The final browser-profile build completed, all six required JavaScript modules resolved, and the active no-Skia dependency check passed |
| Resolved application dependencies | No `ferroui-skia`, `skia-safe` or `skia-bindings` in the normal/build graphs for the browser, Linux x64, Windows x64, macOS ARM64 or macOS x64 targets |

The cross-target dependency audit used locked, offline Cargo graph resolution. It verifies backend selection, not execution of the native desktop application on those operating systems. Production scene painting uses public FerroUI APIs; direct Vello references configure FerroUI's backend or provide backend pixels in native tests. The compiled XAML shell hosts native controls, including inspector dialogs constructed dynamically in Rust.

The corresponding exact-source CI run **completed successfully**, including both artifact uploads and cleanup. Its observed gates are:

| Exact-source CI gate | Observed result |
| --- | --- |
| Document core and persistence host | 219 Rust core tests and 17 JavaScript host tests passed |
| Native text preedit | All 6 focused tests passed |
| Public compositing | All 7 Base recording/replay tests and 7 Vello pixel tests passed |
| Native application | Compilation and all 64 tests passed |
| Actual Vello WebAssembly application | Build, all six required JavaScript imports, 10 web text-input bridge tests and active no-Skia dependency checks passed |
| Actual Chromium IndexedDB | All 5 transaction, migration, rollback, conflict and large-document reload checks passed |
| Chromium Software2D | All 62 interaction checks passed, including the 10,000-layer workload |
| Chromium WebGL2 | All 59 interaction checks passed; actual WebGL2 through ANGLE/Vulkan/SwiftShader |
| Runtime errors | No page errors or console errors in either context |
| Artifact publication | Complete build, screenshots, traces, reports and lockfiles uploaded; both ZIP sizes and GitHub-provided SHA256 digests matched, and both outer ZIPs and inner trace ZIPs passed CRC checks |

Both contexts used Chromium 156.0.8078.4. The first native application frame was observed after 1,498 ms in Software2D and 1,453 ms in WebGL2. All 34 shared static native captures passed on their first attempt, with no retries. Software2D capture attempts took 120–161 ms; WebGL2 attempts took 9,112–23,629 ms. These capture durations include screenshot acquisition and follow-up checks, and do not measure application frame rate or editing latency. The hardware used by this WebGL2 run was SwiftShader, so this is not a hardware-GPU throughput benchmark.

The [source-tagged evidence directory](evidence/2026-10-19-945283ef/README.md) preserves the unchanged report and paired initial/editor, effect-stack and held layout-drag PNGs. Visual inspection found the captured shell, effect controls and insertion marker legible and arranged in both modes. This is a review of this application's pixels; no matched Figma document/reference screenshot was available for a numerical visual-parity comparison.

### Actual 10,000-layer browser workload at `945283ef`

Software2D imported a 12,872,068-byte design containing 10,000 rendered nodes. The native layer list represented all 10,000 rows while realizing only 25 controls, both before and after scrolling.

| Operation | Full preparations | Incremental preparations | Prepared nodes | Rebuilt immutable visuals | Layer-projection builds | Layer-projection reuses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Native keyboard nudge | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard undo | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard redo | 0 | 1 | 1 | 0 | 0 | 1 |

The new zoom fixture then selected **1 root candidate from 10,000 roots with 27 bounds tests**, with one visible node at zoom 17.0833. `rootQueryReused` was false for that observed render, so these are actual query work counts rather than a zero-work cached query. Native tests separately cover unchanged-query reuse, dense full-view traversal, conservative effect bounds, refitting and exact pixels against the original ordered scan.

Import through the first native frame took 829 ms; the measured native keyboard nudge through the browser harness took 151 ms; reload and exact document restoration took 1,226 ms. The edited portable JSON was 20,652,219 bytes and its compact autosave was 13,132,068 bytes; autosave serialization took 33.3 ms. The report's 2 ms `commitMs` is only the remaining wait for the save queue to drain after earlier autosave work, not complete IndexedDB commit latency. The complete restored document SHA256 was `f011b40a7d2bf8d0e21497008f93b613ce7f154e326f2eda41052e83ca776332`.

These counters show avoided preparation, row allocation and root-bound work. Root signatures, drawing visible artwork, wide-parent bounds, structural layout and some history paths can still be linear. The observed browser times include the harness and are single-run functional measurements, not FPS. WebGL2 did not run the 10,000-layer fixture.

### Paired release measurements at `945283ef`

The unchanged [schema-5 benchmark JSON](benchmarks/2026-10-19-945283ef-editor-performance.json) is copied byte-for-byte from the successful CI validation artifact (7,424 bytes, SHA256 `fc7a2776c0f9438204e120096a8746c4b1672bb0081f9313325033a4fb31e497`). It compares scoped commands with an explicit generic full-document transaction reference in one release process on 10,000 flat filled rectangles with one selected node. Exact document/history and serialization equivalence checks passed. The reference is a workload in the same executable, not a previous release or a different CI runner.

| Operation, 48 samples | Full-document reference p50 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: |
| Nudge | 14.098 ms | 0.026 ms | 0.032 ms |
| Undo | 4.294 ms | 0.017 ms | 0.024 ms |
| Redo | 4.167 ms | 0.015 ms | 0.018 ms |

Across 12 gestures, a complete 24-update gesture measured 108.968 ms p50 on the reference path and 0.053 ms p50 / 0.070 ms p95 on the scoped path. This benchmark measures the existing scoped engine commands, not the new auto-layout drag index or root rendering hierarchy. It excludes browser events, native control layout, drawing and persistence.

Five serialization samples measured portable pretty output at 30.157 ms p50 / 20,152,206 bytes and compact autosave at 16.139 ms p50 / 12,802,055 bytes, a 36.5% smaller payload with exact round-trip equivalence. Parse/validate/index measured 47.701 ms p50 across three samples. A separate dense layout fixture with 9,999 mixed fixed/fill text children measured 8.798 ms p50 across five samples. These native fixtures are separate from the browser workload above.

Component synchronization retains full document history snapshots. Its separate 10,000-node workload includes 5,592,566 bytes of unrelated embedded-image data and measures actual Editor operations including history: variant switching 19.145 ms p50 / 19.369 ms p95; text properties 19.032 / 19.942 ms; visibility properties 25.298 / 26.163 ms, with eight samples each. Complete history restoration and stable materialized IDs passed. The isolated no-history document-operation figures in the raw JSON are not substituted for production Editor costs.

### Framework integration and earlier checkpoints

The separate [standard repository CI run 39306373798](https://github.com/wieslawsoltes/FerroUI/actions/runs/39306373798) completed successfully with all 14 jobs passing. Its checkout log and merge commit identify source `945283ef` tested as merge `70db461c11217d05d774a6736ec44bfa6c982f35` into base `0116724da88df86fa4e4575f7e9a0d78708f2204`. Source conventions, packaging, macOS, Windows x64/ARM64, Linux X11/Wayland/framebuffer, Android, iOS and all three browser configurations passed. These framework integration jobs are separate from the independent FerroDesign workspace's exact-source application tests; they do not establish native FerroDesign GUI execution on every platform.

The preceding combined implementation `12cbecd3`, [run 39304233073](https://github.com/wieslawsoltes/FerroUI/actions/runs/39304233073), passed the core/host, preedit and public compositing gates, then native checking rejected a new test's incorrect `PixelFormat` import. Checkpoint `945283ef` corrected that import; its complete local native suite and the CI native gate above passed. No browser result is attributed to the failed checkpoint.

The resumed baseline `bc2e9984`, [run 39254507451](https://github.com/wieslawsoltes/FerroUI/actions/runs/39254507451), passed its earlier gates and all 53 Software2D checks, then failed after 31 WebGL2 checks at the native blend selector. Its trace showed a changed selection before a new inspector frame; the subsequent arrow input nudged the selected layer. The revised harness retains native pointer/keyboard input, requires arranged controls and popup focus, and checks that blend selection does not move the layer. Both modes at `945283ef` passed that regression. The older complete result below is retained as historical evidence.

## Complete continuation result: `ecd5078d`

| Gate | Observed result |
| --- | --- |
| Document core and persistence host | 197 Rust core tests and 17 JavaScript host tests passed |
| Native text preedit | All 6 focused tests passed |
| Public compositing | All 7 Base recording/replay tests and 7 Vello pixel tests passed |
| Native application | Compilation and all 43 tests passed, including 8 layer-projection, 9 incremental-renderer and 7 backdrop cases |
| Actual Vello WebAssembly application | Build, required JavaScript imports and active dependency graph checks passed; no active Skia package |
| Actual Chromium IndexedDB | All 5 transaction, migration, rollback, conflict and large-document reload checks passed |
| Chromium Software2D | All 53 interaction checks passed, including the 10,000-layer workload |
| Chromium WebGL2 | All 51 interaction checks passed; actual WebGL2 through ANGLE/Vulkan/SwiftShader |
| Runtime errors | No page errors or console errors in either context |
| Artifact publication | Build, screenshots, traces, reports and lockfiles uploaded successfully; original artifact sizes and GitHub-provided SHA256 digests verified |

Both browser contexts used Chromium 156.0.8078.4. They exercised native vector networks, Unicode/IME and rich-text range editing, variables, component variants and typed overrides, image crop gestures and percentage fields, isolated compositing and transparent background-blur export, the page inspector, grid spans/reorder, compact authored-state autosaves, prototype animation/scroll/reversal, compact-window controls and durable reload. The first native application frame was observed after 1,713 ms in Software2D and 1,634 ms in WebGL2. These are functional checks in one browser engine on a CI runner, not native desktop GUI certification, operating-system IME certification, a hardware-GPU benchmark or a same-document Figma pixel comparison.

The 30 shared static native captures all completed. One WebGL2 variable-dialog capture needed the existing unchanged-state retry: the first attempt timed out after 30.058 seconds, and the second succeeded in 155 ms. Other WebGL2 captures took approximately 8.8–29.6 seconds and passed their first attempt. The report retains each attempt, unchanged document/selection/camera/control checks, context-health checks and actual PNG pixels. These capture-attempt durations include the screenshot call and follow-up checks and are separate from application first-frame and editing measurements; no exact cause for the slower captures is claimed.

### Actual 10,000-layer browser workload

This workload runs only in the Software2D context. It imported a 12,872,068-byte design with 10,000 rendered nodes. The native layer list represented all 10,000 rows while realizing only 25 controls, both before and after scrolling.

| Operation | Full preparations | Incremental preparations | Prepared nodes | Rebuilt immutable visuals | Layer-projection builds | Layer-projection reuses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Native keyboard nudge | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard undo | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard redo | 0 | 1 | 1 | 0 | 0 | 1 |

Import through the first native frame took 947 ms; the measured keyboard nudge through the harness took 159 ms; reload and exact document restoration took 1,557 ms. The edited portable JSON was 20,652,219 bytes and its compact autosave was 13,132,068 bytes; the recorded autosave serialization took 27 ms. The report's 3 ms `commitMs` is only the remaining wait for the save queue to drain after earlier autosave work, not complete IndexedDB commit latency. The complete restored document SHA256 was `f011b40a7d2bf8d0e21497008f93b613ce7f154e326f2eda41052e83ca776332`.

These preparation/projection counters verify avoided work; root traversal, wide-parent aggregation and some metadata comparisons can still be linear. The observed times are not FPS or a speedup calculated against a different CI runner. WebGL2 did not run this large-document fixture.

### Paired native measurements from the same successful run

The unchanged [schema-5 benchmark JSON](benchmarks/2026-10-19-ecd5078d-editor-performance.json) is copied byte-for-byte from the validation artifact (7,461 bytes, SHA256 `2211dd9f679345f2e0ddc598591874561117d54689fcd6f7bb6ead2835055bfb`). It compares scoped commands with an explicit generic full-document transaction reference in one release process, on 10,000 flat filled rectangles with one selected node. Exact document/history equivalence passed. The reference is a workload in the same executable, not a separately measured previous release.

| Operation, 48 samples | Full-document reference p50 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: |
| Nudge | 13.742 ms | 0.024 ms | 0.029 ms |
| Undo | 2.572 ms | 0.012 ms | 0.018 ms |
| Redo | 2.714 ms | 0.013 ms | 0.018 ms |

The complete 24-update gesture measured 82.627 ms p50 on the reference path and 0.059 ms on the scoped path, across 12 gestures. These engine measurements exclude browser input, native control layout, drawing and persistence. Five serialization samples measured portable pretty output at 36.188 ms p50 / 20,152,206 bytes and compact autosave at 20.210 ms p50 / 12,802,055 bytes, a 36.5% smaller payload with exact round-trip equivalence. This native benchmark fixture is separate from the browser workload above.

Component synchronization retains full history snapshots. Its separate 10,000-node workload includes a 5,592,566-byte unrelated embedded image and measures the actual Editor operation, including history: variant switching 17.678 ms p50 / 18.761 ms p95; text properties 17.930 / 18.387 ms; visibility properties 24.783 / 25.700 ms, with eight samples each. Its complete history round trip and materialized IDs were exact. The faster isolated no-history document-operation numbers are not substituted for those production Editor costs.

The earlier complete baseline remains `c2d2e71ac8b512bddc4a4e4d4c920d5e03f09457`, [run 39168689287](https://github.com/wieslawsoltes/FerroUI/actions/runs/39168689287). Its separate observations are retained below as history.

## Continuation checkpoints

[Run 39221682967](https://github.com/wieslawsoltes/FerroUI/actions/runs/39221682967), source `0efde37fa53801db1ec1829d7355090203fc52ec`, passed 131 core tests, 17 persistence-host tests, six native preedit tests, ten framework browser-input tests, the paired release benchmark, the actual Vello WASM build and five actual Chromium IndexedDB checks. Software2D completed all 28 browser checks, including Smart Animate midpoint pixels, clamped scroll and reversible hover/press interaction state. WebGL2 completed 15 checks and the native paint model edits, then a page screenshot timed out after fonts had already loaded. Trace frames continued and the diagnostic screenshot succeeded with the complete three-stop dialog. This run is a failure, not a complete validation pass.

[Run 39227652054](https://github.com/wieslawsoltes/FerroUI/actions/runs/39227652054), source `fc212577143968b8fa17886f5389970cd3ec7191`, passed all 183 core tests, 17 host tests and the schema-4 component/editor benchmark. The native preedit compilation gate found a missing `PushCompositingLayerPayload` import in the framework's shared test stream reader. Commit `b48a258c856769e87076e53bb691efb56bcd859b` fixes the import.

[Run 39228488797](https://github.com/wieslawsoltes/FerroUI/actions/runs/39228488797), source `b48a258c`, passed 183 core tests, 17 host tests, six native preedit tests and seven Base compositing tests. Vello's new earlier native pixel gate could not compile because Fontconfig development files were installed only after browser validation. The workflow now installs those files before native tests. This run did not reach a WASM build or browser validation.

[Run 39231069435](https://github.com/wieslawsoltes/FerroUI/actions/runs/39231069435), source `40131410`, passed 197 core tests, 17 host tests, six native preedit tests, seven Base compositing tests and seven Vello compositing tests. Native application compilation then reached the compiler's macro expansion limit in the enlarged diagnostics JSON object. Commit `4e3cfc11` splits that construction into smaller groups while retaining its flat bridge schema. Native application tests, WASM and browser checks were not reached in this run.

[Run 39233328958](https://github.com/wieslawsoltes/FerroUI/actions/runs/39233328958), source `40b77e14`, passed the same core, host, preedit and compositing gates. Native application checking cleared the macro issue and reported a type mismatch introduced by the blend selector: iterator positions were unsigned while FerroUI ComboBox selection indices are signed. The selector now converts its bounded index to `i32`, preserving the native `-1` empty-selection value. This run did not reach native application tests, WASM or browser execution. The next benchmark uses schema 5 labels `portablePretty` and `compactAutosave` to identify the serializers now used by their respective production paths; historical measurements keep their original schemas.

[Run 39234991185](https://github.com/wieslawsoltes/FerroUI/actions/runs/39234991185), source `52b89f0e`, passed 197 core tests, 17 host tests, six native preedit tests, all 14 public compositing tests and native application checking. The complete native application test binary compiled and ran: 32 of 35 tests passed, including all nine incremental-renderer tests. Three backdrop pixel assertions exposed destructive Source group composition clearing pixels beyond the replacement bitmap or its geometry clip. Both bitmap replacements now use public `RenderOptions.bitmap_blending_mode = Source` with the original clips retained. All seven backdrop tests and their pixel assertions remain in place for the next run. CI8 did not reach WASM or browser execution.

[Run 39238255672](https://github.com/wieslawsoltes/FerroUI/actions/runs/39238255672), source `6809faad`, passed the same core, host, preedit, compositing and native compilation gates, followed by all 35 native application tests. All seven backdrop tests and all nine incremental-renderer tests passed. The actual Vello WASM build and five Chromium IndexedDB transaction checks also passed. Software2D completed 20 interaction checks, including vector networks, native rich-text range styling and variable authoring, then could not open the component variant dialog. Its inspector button had mistakenly been placed in the icon table instead of the command-hook table; the native pointer click therefore had no command to invoke. WebGL2 completed 18 checks and the rich-text model assertions before that dialog's raw screenshot timed out after fonts had loaded. There were no recorded page or console errors in either mode. The failure screenshot showed the arranged rich-text dialog, while the earlier WebGL2 paint screenshot had recovered using the unchanged-state retry. This run is a failure, not complete browser validation.

The subsequent shell projection cache has eight native regression cases and adds cumulative projection counters to the existing 10,000-layer browser workload. All eight native cases and the browser nudge/undo/redo projection checks passed at `ecd5078d`, as recorded above. The passing 35-test gate at `6809faad` predates and does not validate them. Cache reuse avoids complete row/name materialization for a scoped edit, while expanded-set comparison and document-history metadata can still have linear costs.

The branch incorporates upstream `3d8b473` (FerroUI 0.3.0-preview.1), with the standalone sample lockfile aligned and all registry versions retained. Checkpoint `26170911` adds a bounded editor change feed and incremental render preparation; `468a02b7` adds native grid placement/spans/reorder, cumulative browser preparation counters and history-depth assertions. All nine native incremental-renderer pixel/count tests passed at `52b89f0e`, including affected leaves, ancestor bounds, camera reuse, undo, identity changes and conservative fallbacks. Their subsequent native and browser checks completed successfully at `ecd5078d`. The workflow runs all native application tests before the browser build. The browser workload requires nudge/undo/redo in a flat 10,000-layer document to each add one prepared leaf, zero full preparations and zero immutable visual rebuilds; those counts are not an FPS benchmark.

The subsequent fixes connect the component inspector button to its command and make the browser check require an open native dialog with the original selection. Static native checkpoints now share the existing verified screenshot path. The expanded harness retains real native pointer/keyboard operations and PNG captures. The paint checkpoint waits for newly arranged native controls and a render after the Add-stop input. A screenshot TimeoutError may receive one bounded retry only if document hash, selection, camera and native control bounds remain unchanged, the page responds, and the renderer has no lost context or panic. Per-attempt diagnostics remain in the report; a second failure, changed state, or invalid/blank capture fails the run. Continuous traces and font readiness remain enabled. Live Smart Animate captures keep their original timing-sensitive path. CI9 demonstrates one successful recovery of an unchanged paint dialog, not a general browser pass. The corrected component commands and all static checkpoints subsequently passed in the complete `ecd5078d` run.

## Framework integration CI

The repository's [standard PR CI run `39245212870`](https://github.com/wieslawsoltes/FerroUI/actions/runs/39245212870) checked head `ecd5078d` as GitHub's merge `86da361083cfd9146dda6bac99571590a61d244b` into base `53cd3e8648d8a27c04794d3fe48b7a88d64a71ff`. It completed with 13 successful jobs out of 14. macOS, Windows, Linux X11/Wayland/framebuffer, Android, iOS, packaging and all three browser build configurations passed, including their actual Chrome interaction suites. The sole failure was the source-convention gate, which identified one direct `thread_local!` declaration in the application's shell storage.

Checkpoint `5744ef7d85426246237aaa50984677ebcf6525da` corrects that declaration to `ferroui_base::ferro_thread_local!`, retaining its initializer while following FerroUI's platform-aware storage convention. The repository's existing thread-local converter check and generated-composition check pass against that correction. These merge-based framework results are separate from the FerroDesign workflow's exact branch-head native/WASM/browser validation; neither `ecd5078d` run validates the later macro correction. Subsequent candidate results are linked in PR #254.

## Earlier paired release measurements

These are **native editor-engine measurements on one GitHub runner**, not browser frame rate or hardware-GPU throughput. The same process compares the scoped command/gesture path with an explicitly defined generic full-document transaction reference, and asserts exact document/history equivalence. The flat fixture has 10,000 filled rectangles, one selected node, 48 nudge operations and 12 gestures of 24 updates. Raw results are preserved in [the source-tagged JSON](benchmarks/2026-10-19-0efde37f-editor-10000-nodes.json).

| Operation | Full-document reference p50 | Scoped path p50 |
| --- | ---: | ---: |
| Nudge | 9.253 ms | 0.020 ms |
| Undo | 1.509 ms | 0.008 ms |
| Redo | 1.491 ms | 0.008 ms |
| Gesture begin | 2.346 ms | 0.010 ms |
| Gesture update | 1.454 ms | 0.000631 ms |
| Gesture commit | 7.248 ms | 0.009 ms |
| Complete 24-update gesture | 46.008 ms | 0.044 ms |

The savings come from reducing snapshot, comparison and scene-update work for known mutation sets. They do not include native control layout, scene preparation, rasterization, browser input delivery or JSON serialization. Pretty JSON serialization remained 36.595 ms p50 for 20,152,206 bytes in this run; coalescing before serialization reduces its frequency. A compact-format candidate was measured separately in that run: 19.621 ms p50 for 12,802,055 bytes, preserving an exact compact-to-document-to-pretty round trip. A later implementation adopts this serialization path for autosaves; portable exports retain the pretty format. Its browser gates verify actual committed compact bytes, exact reload and authored-state isolation during active presentation. Those autosave checks subsequently passed in both browser contexts at `ecd5078d`.

The [schema-5 measurements at `52b89f0e`](benchmarks/2026-10-19-52b89f0e-editor-autosave.json) identify the production serializers explicitly. In that run, portable pretty output took 29.076 ms p50 for 20,152,206 bytes; compact autosave output took 15.751 ms p50 for 12,802,055 bytes, with exact document round-trip equivalence. Compact output was 36.5% smaller. These are five-sample native serialization measurements from the same process, independent of IndexedDB commit latency or browser rendering. Timing differences between CI runners are not treated as changes in implementation speed.

The [component workload at `fc212577`](benchmarks/2026-10-19-fc212577-editor-components.json) includes two matched 32-child variants, typed text/visibility changes, embedded images and vector paths, and a 5,592,566-byte unrelated image data URI among 10,000 nodes. The paired no-history document operation is compared with the same operation plus an explicit whole-document candidate clone; this is a reference workload, not a measured previous-version speedup. Both variants of the benchmark preserve exact document state and matching materialized IDs. Actual Editor operations are measured separately with their current full history snapshots:

| Component operation | Editor with history p50 | Editor with history p95 |
| --- | ---: | ---: |
| Variant switch | 16.656 ms | 17.138 ms |
| Text property | 16.986 ms | 17.677 ms |
| Visibility property | 20.875 ms | 21.322 ms |

The complete component undo/redo round-trip was exact. The remaining full-snapshot cost is visible rather than being hidden behind the faster isolated document-operation numbers.

## Earlier complete baseline: observed results

| Check | Result |
| --- | --- |
| Core unit tests | 79 passed: existing document/SVG/layout coverage plus vector-edit, Unicode text-range, prototype timing/key/overlay, isolated-root hit testing and interaction-authoring regressions |
| Native TextBox preedit regressions | All 6 tests in the focused `preedit` gate passed at `c2d2e71a` in run `39168689287`; it compiles the controls test binary and exercises pending-state ownership, selection deletion, empty updates, detachment and existing preedit regressions |
| Native full application check | Passed on Ubuntu 24.04 with Rust 1.99.0: `cargo check --locked -p ferrodesign-app --lib --tests --jobs 1`, checking the XAML shell, native text/paint editors, prototype runtime and test targets. This is a compilation check, not a native GUI runtime test. |
| FerroUI browser bridge TypeScript check/build | Passed |
| Browser text-input event regressions | 10 passed with Node, bundling the actual TypeScript bridge; includes Unicode insertion, composition deduplication, canceled-client recovery and listener cleanup |
| Browser host JavaScript and shell syntax | Passed |
| Browser persistence host tests | 13 passed with Node |
| Actual IndexedDB transactions in Chromium | All 5 checks passed, including atomic migration, rollback, stale-tab conflicts and reload of a 12,000,042-character document |
| Native atomic-save tests | 2 passed in the earlier standalone development host harness; this is separate from the current CI gates |
| Active browser dependency graph | No `ferroui-skia`, `skia-safe` or `skia-bindings` |
| Actual WebAssembly application build | Passed with the pinned Emscripten toolchain, including linking and optimization; all six required JavaScript modules verified in the distribution |
| Browser pixels and editing interactions | Software2D passed all 25 checks; WebGL2 passed all 24 checks. Both completed native vector/text/paint input, composition, pagehide/reload, prototype interactions, serialization and history. Software2D additionally completed the 10,000-layer workload. |
| Browser runtime errors | Zero page errors and zero console errors in both modes |
| Native GUI runtime | Not run on Linux, macOS or Windows in this environment |
| Exact Figma visual parity | Not measured |

The `c2d2e71a` CI browser distribution contains 16 files totaling 48,637,191 bytes. Its WebAssembly module is 47,941,251 bytes before HTTP compression, SHA256 `fbf2cb2818bbeff0b24f8d25ac4ae4b3f35cbb20691607436a504477a3b24f75`. The 438-line resolved dependency manifest contains Vello and no Skia package. Both build and validation archives passed CRC verification. These packaging checks establish build and import completeness; the Chromium interaction suite separately verifies rendered behavior.

The browser was Chromium 156.0.8078.4. The two runs are contexts in that browser engine, not tests of two different browser engines.

| Requested mode | Observed surface | First application frame |
| --- | --- | ---: |
| Software2D | Software2D / Vello CPU | 1,794 ms |
| WebGL2 | WebGL2 via ANGLE / Vulkan 1.3.0 / SwiftShader Device (Subzero) | 3,120 ms |

The WebGL2 context exercised the actual API path through a software GPU implementation. These first-frame observations include the local CI application startup; they do not establish hardware GPU throughput or production network latency.

## Browser regressions corrected

The [first CI browser run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39152600835) found that text inserted without a keydown updated only the hidden DOM input. FerroUI's browser bridge now forwards non-composing `insertText` payloads through the native text event, preserving native selection and resynchronizing the DOM after the native control finishes its edit. Composition input remains owned by the composition handlers, with a bounded duplicate-commit guard. This is a targeted text-insertion fix; browser autocorrect replacement ranges and input-only deletion are not newly implemented by this patch. The original multiline Unicode assertions passed in both rendering modes in the second run.

The second run's WebGL2 failure state contained the expected linear gradient and visible native paint controls with zero-size bounds. The harness sampled those controls about 7 ms after the document transaction, before the next native arrangement. A bounded readiness predicate now requires visible, finite, positive native bounds before deriving pointer coordinates. It does not bypass the field interaction or alter the paint assertions.

The same review found a native composition lifecycle bug: losing a text client before `compositionend` left the framework composing flag set, blocking subsequent input-only text. Native reset/client replacement now cancels preedit, and the TypeScript bridge synchronizes its composition state when the shared hidden input is cleared. Late events from the canceled composition cannot edit the replacement client. The browser suite now exercises normal composition, cancellation, resumed Unicode input before a delayed end, and late-event isolation against the actual WASM TextBox.

The [third run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39163539589), revision `fd800712`, passed the 13 existing editor checks in each mode and then exposed an incorrect assumption in the new preedit assertion. The real compiled-XAML TextBox retained its confirmed document value, while the test expected an intermediate selection deletion. That internal value depends on whether the native presenter exists when the TextBox attaches. The corrected test checks the native input client's explicit pending-preedit state and the complete confirmed document node, then requires exactly-once composition commit and cancellation behavior.

The editor also guards a separate durability boundary: when a native preedit update does remove the selection, its text-changing notification must not replace confirmed document text. `TextBox::is_composing_text()` exposes a stable native-client signal independent of caret/presenter synchronization. The shell keeps those intermediate notifications outside the document draft. A browser regression now starts with confirmed Unicode input, begins unfinished native composition, dispatches the real pagehide listener, checks durable text and rich runs, exercises one-step undo/redo, and reloads the page. These assertions passed in both modes at `c2d2e71a` in run `39168689287`; all six focused native signal regressions also passed in that run.

The [fourth run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39165941373), revision `f1184c3e`, exposed two harness timing assumptions. In Software2D, a second double-click at the identical position followed the first within the native 500 ms click-count window, so the next text editor did not open. The fixture now makes a real blank-canvas click at a different position before each opening double-click, establishing an independent native sequence without arbitrary sleeps. In WebGL2, all native gradient edits succeeded, but `locator.screenshot` timed out waiting for DOM canvas stability; the fallback page screenshot succeeded and showed the correctly arranged three-stop dialog. The paint artifact now uses a page screenshot at the same fixed viewport. All model, keyboard, history, composition and persistence assertions remain in place, and the complete corrected browser suite passed in the fifth run.

## Earlier large-document browser result

The Software2D part of run `39168689287` completed the full 10,000-layer workload in Chromium 156.0.8078.4. It imported a 12,592,068-byte document, rendered all 10,000 nodes, realized only 25 native layer rows before and after scrolling, exercised real pointer selection and keyboard nudge/undo/redo, and restored the exact 20,292,219-byte edited document from IndexedDB after a browser reload.

| Operation | Observed CI time |
| --- | ---: |
| Import through first native frame | 1,024 ms |
| Keyboard nudge through the browser harness | 322 ms |
| Reload and exact restored document | 1,756 ms |

These are end-to-end harness observations on one CI runner using CPU rendering, not browser frame-rate or hardware-GPU measurements. The report's `commitMs` field is the remaining wait for the save queue to drain after earlier autosave work; it is not complete save latency. The saved document SHA256 was `13c63f58a702cfd42eaf05cd113e0d8e63ba0603b1613a535e9fccadcf55782e`.

## Document-engine benchmark

The release benchmark creates 10,000 editable nodes, builds the spatial scene, issues point/viewport queries, performs 120 incremental drag updates and measures transaction history. Values below are actual measurements from the earlier development container; they are not browser FPS, GPU measurements or hardware-independent guarantees.

| Operation | Measured result |
| --- | ---: |
| Point hit query, average over 10,000 queries | 3.192 microseconds |
| Viewport query, average | 76.997 microseconds |
| Incremental drag refresh, median | 5.658 milliseconds |
| Incremental drag refresh, p95 | 7.623 milliseconds |
| Transaction commit | 11.944 milliseconds |
| Undo | 4.707 milliseconds |
| Redo | 5.251 milliseconds |

This run created 10,000 nodes in 12.354 ms and built the initial index in 21.735 ms. It ran concurrently with the WebAssembly compiler. These are container CPU/model timings from that development run; there is no same-run baseline for calculating a speedup and no browser-FPS claim. Correctness tests compare incremental state against full rebuilds, including clipped transformed subtrees, stacking changes and history.

Reproduce the workload from the sample directory:

```sh
cargo run --release -p ferrodesign-core --example performance -- 10000
```

## What the browser test verifies

`tests/browser.mjs` launches the actual application at a 1600 by 1000 viewport in two contexts, requesting Software2D and WebGL2. Each explicit mode must produce that exact canvas context; the test does not accept a fallback. Normal application startup tries WebGL2 and may fall back to Software2D. Chromium may use SwiftShader; that result establishes functional rendering, not hardware-GPU throughput.

The test waits for a completed native frame and dismissal of the loading surface, inspects native pixel diversity, draws a rectangle, edits vector anchors and pen tangents, edits multiline Unicode in the native on-canvas TextBox, changes paints through the native dialog, and dispatches prototype hover/press/drag/key/delay actions. It checks one-transaction history, canceled text edits, UTF-8 rich-run ranges, JSON import/export equality, durable reload, and bounded virtualized rows with a 10,000-layer document larger than 5 MiB. Deterministic browser composition events exercise explicit native preedit state, exactly-once commits, focus loss and confirmed-text durability through pagehide/reload; they do not certify operating-system IME behavior on every platform. Prototype rules are imported as fixtures for runtime validation; interaction-dialog authoring controls have not been browser-operated by this suite. The separate IndexedDB browser test uses actual transactions for migration, rollback, stale-tab conflicts and a document larger than 10 MiB. The application browser test also collects console exceptions, startup failures and FerroUI render-thread panics. Its screenshots, Playwright traces and report are written under `browser-report` and uploaded by GitHub Actions.

Lifecycle durability checks cover confirmed on-canvas text and native inspector drafts. Native fields commit on Enter or focus loss; numeric arrow keys in Effects also commit. Save, export, page hide and window close flush confirmed focused drafts before serialization. Browser fixtures type into the actual Width and Effect radius controls, dispatch page hide, and verify the committed document, one-step history and durable reload. Invalid drafts retain their validation and unload guard, and pending IME preedit remains outside the saved document. Deterministic browser events do not certify every operating system's close or IME behavior.

The focused native preedit gate can be reproduced from the sample directory:

```sh
CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_CODEGEN_UNITS=16 \
  cargo test --locked --manifest-path ../../Cargo.toml \
  -p ferroui-controls --lib preedit --jobs 1 -- --test-threads=1
```

The filter narrows execution, while Cargo still compiles and links the controls test binary. This gate uses the pure Rust controls/base test graph; it does not introduce a separate graphics backend.

An interaction test is evidence for those paths, not for every item in the feature inventory. There is no same-document image comparison to a Figma reference screenshot. Browser rendering timings and memory should be measured on the intended deployment devices before establishing a frame-rate or capacity target.

## Browser QA environment

The Playwright suites run Chromium against the actual static distribution using a loopback HTTP server. Set `FERRODESIGN_CHROMIUM=/absolute/path/to/chrome` to select an existing executable; otherwise they use Playwright's installed Chromium. The local validation environment uses Chrome for Testing 156.0.8078.4. Local reports and CI artifacts are distinct evidence sources, and each result above identifies its tested source. A successful native compile or a completed WebAssembly link does not establish browser interaction success.
