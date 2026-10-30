# Repeater port journal

## Published starting points

| Commit | Change |
|---|---|
| `f67b92f3` | Stable-key mapping added to the existing native item-source views |
| `0dd3a661` | Hierarchical selection port with lazy source traversal |
| `fa1d1ae1` | Repeater control, services, ownership, layout algorithms and source-derived tests |
| `5acfd247` | Native animation preferences connected to Repeater transitions |
| `50265bb5` | Continuation branch incorporates the current FerroUI main branch |
| `c3d46d1c` | Source/API/dependency/scenario inventories and the native acceptance checkpoint published to PR #269 |

These are recoverable commits on `feat/winui-repeater-port`, not inferred local
state. The source notice pins WinUI
`7e4b0c2096be402f51cc8f7d203ebf67cff183d0`.

## 2026-10-20 — source-based continuation

The remote branch was recovered at `5acfd247`. Earlier conversational reports
mentioned source/API maps, XAML integration work and validation failures, but the
remote branch did not contain a Repeater documentation directory. The maps in
this directory were therefore reconstructed from the pinned official source
and the actual Rust tree; unverifiable earlier claims are not reused as current
validation evidence.

The extraction records all 302 upstream files, all defined IDL declarations,
direct include boundaries, and 265 test declarations. Three test declarations
are commented out in the upstream source; ignored source tests are retained as
such. API/test identifiers remain navigation and mapping evidence only.

The GitHub Contents API comparison on 2026-10-20 found Repeater tree object
`1de63a85615582e33d24e19795bd73d374195ac1` at both pinned revision and upstream
main. This establishes source-subtree currency for that checkpoint while
keeping the existing derivation pin stable.

Review found that `flow_layout_upstream_tests.rs` existed without a parent-module
declaration, so those source-derived tests were not part of the recovered test
binary. The declaration is restored for the integration build; execution is
tracked separately in the validation record.

Lined-flow review separated true source behavior from native framework
adaptation: FerroUI's outer `DesiredSize` requires margin-aware constraint
handling; partial metadata/realization and timer lifetime need explicit
regressions; the pinned aspect-ratio cache midpoint expression uses a relative
count where an absolute source index is required. That correction is documented
as intentional rather than silently attributed to upstream.

The native `ItemsSourceView` path now preserves an explicitly supplied view's
identity, notifications and stable-key mapping. New source-identity and release
scenarios are present for integration validation. The lined-flow release test
exercises its existing `Drop` timer shutdown; the continuation also adds a
defensive stop when a queued timer callback cannot upgrade its weak owner.

The acceptance module reproduces two nested-template/cache scenarios using
native data-context bindings, templates, notifying groups and scrolling hosts.
It adds exact stable-reset read-count/identity checks and the ignored upstream
grid-insertion placement scenario. Three interaction scenarios retain their
layout/offset/count assertions with native scroll/collection actions in place
of Windows UI Automation. These tests are recorded as present and pending the
integration run, not as passing because their names match.

Runtime metadata tests cover native class names and constructor requirements in
place of COM activation. Compiled XAML tests exercise native layout/template
construction, keyed factory dictionaries, item binding and control-item
identity. The native catalog example supplies all four layouts and a large
notifying source; source TestUI assets remain reference entries.

The final two partial scenario relationships now point to dedicated source-derived
tests. Reentrancy coverage exercises all four rejected API changes, restoration
of the rejected layout property, original template contents and layout lifetime.
The element-factory scenario checks the empty/add/remove data and factory traces
with the same element and parent identities. It retains FerroUI's live source
count contract instead of reproducing WinRT's cached `GetSizeCore` call counts;
that intentional boundary is recorded in WR-22. All 265 upstream declarations
now have a named scenario relationship or an explicit known difference. This
closes the mapping ledger, not the still-separate runtime validation gate.

Future entries should state the concrete behavior changed, source comparison,
regression test, executed command/result and published commit. Do not record an
intended implementation or a source-name match as a completed parity gate.

## 2026-10-21 — recover the published head and close native lifecycle gaps

The continuation recovered PR #269 at `3facaa20`, then inspected the actual
completed CI logs. The focused workflow had executed 627 passing XAML tests
(17 intentionally ignored) and 4,827 passing Controls tests, with one failing
animated aligned-bring target-identity assertion. These results supersede the
older conversational handoff's three outstanding Controls failures.

Commit `688d23f8` merges main `86b6b4ab` into the existing PR branch. The native
View API requires generated builders for the new Repeater classes. Commit
`91675d5b` regenerates those builders, corrects the catalog's two native page
additions, and extends the metadata scanner's bounded bitflags evaluator. Direct
`empty()` expressions now produce the same value as runtime metadata, while
unknown/foreign calls remain unresolved and `from_bits_truncate` masks against
declared flags. All 44 scanner library tests passed locally; generated builders
also passed the generator's consistency check.

Commit `46718059` makes scheduled work cancellable after it enters the current
render-frame snapshot. The phasing regression cancels an obsolete callback
while a prior callback recycles and replaces its element; replacement work
starts on the next frame. Template/selector replacement also releases the old
application implementation outside `RefCell` borrows, allowing reentrant
destructors to observe or replace the newly published value safely.

Commit `aaab8857` validates structural selection revisions across synchronous
source reads, child-resolution callbacks and child-source initialization.
Same-source edits, ancestor disposal and nested child realization now retain
correct node identities or return `SelectionChanged`. Removing a source
refreshes real `SelectedItem`/`SelectedItems.Count` bindings through one
notification sequence. All five new selection regression functions passed;
the combined native Repeater checkpoint reported 404 passed and the one
previously known scrolling identity failure. No failing test was excluded.

The current upstream main Repeater subtree was rechecked through the GitHub
Contents API and still equals pinned tree
`1de63a85615582e33d24e19795bd73d374195ac1`. The source pin remains unchanged.
Native safety corrections are recorded in WR-23 through WR-31 rather than
attributed to the pinned C++ behavior.


## 2026-10-21 scrolling ownership and rendered catalog closeout

Commit `e2a16bb3` keeps an animated bring target and each containing Repeater
realization pinned through the operation, separately from the temporary layout
anchor. Pin generations prevent old progress/completion from scrolling or
unpinning a recycled control. Replacement, external scrolling, viewer/content
changes and host/target detachment cancel against the original owners. Native
clock regressions cover intermediate and final frames, caller-owned pins,
nested containers and reuse of the exact control while an old request exists.
The original aligned-bring identity assertion now passes with its exact final
geometry, offsets and realization ranges retained.

The new removed-target regression exposed a second issue inherited from the
pinned source: a suppressing layout could release a pinned row, leaving the row
outside layout tracking but still excluded by the view manager's automatic
recycling flag. The correction retires affected pinned-pool rows on Remove,
Replace and unkeyed Reset. Keyed Reset moves them into the existing key pool,
keeping control identity, pins and generation for surviving keys and clearing
leftovers at arrange. This is a recorded source correction, WR-29.

Commit `8fd6b892` follows a trace dependency audit that found diagnostic item
selectors had no runtime consumer even though the mapping advertised logging
replacements. Selected lifecycle and arranged-bound snapshots now use the
existing native Logger.
Filtering and delivery are deferred through the dispatcher with weak references
so application sinks can inspect layout state or change the source after the
host operation. Windows ETW/debugger trace details and runtime-profiler class
registration are explicitly separated from this portable diagnostic behavior.

Commit `9039d2a5` adds the catalog/browser regression. Its first execution used
an earlier CI artifact in both WebGL2 and
Software2D and captured layouts, actual wheel scrolling, recycling and all four
collection edits. Inspecting those pictures exposed clipped LinedFlow columns.
The pinned algorithm explicitly permits regular-path overflow when minimum
widths prevent shrinking, so the native sample now supplies complete
ItemsInfoRequested aspect-ratio/minimum-width metadata. The compiled catalog
regression checks resized viewports and mutations; the browser gate additionally
rejects clipped or missing columns. Both browser CI variants retain PNGs and a
JSON report. Exact executed revisions and results are recorded in VALIDATION.md.

The focused CI run for `9039d2a5` rebuilt the generated compiler output, ran all
4,847 Controls tests (including 415 Repeater tests) and 627 runtime/compiled
XAML tests successfully, with no failures. The XAML suite retained 17 ignored
tests, and both snapshot generators were explicitly executed successfully.
The checked-in generated output matched. Separate local runs passed 4,380 Base,
103 XamlX, 49 native View and 44 metadata-scanner tests. The Controls suite also
passed locally in full after enabling the existing remote-transport tests to
bind loopback sockets; no known failing test was filtered out. Rendered browser
and compiled catalog results are recorded separately from these unit counts.

## 2026-10-21 — completed catalog and platform execution

Documentation checkpoint `812de219` closes the source/dependency mapping and
records the lifecycle corrections and initial native execution evidence. The
runtime checkpoint remains `9039d2a5`; later documentation changes do not alter
that tested code.

The macOS workspace completed with no failed tests. ControlCatalog passed 971
tests, retaining 20 ignored tests; all five Repeater page scenarios and all nine
catalog view-model scenarios passed. The page scenarios exercise all four
layouts over the large source, scrolling/recycling, collection edits,
LinedFlow viewport constraints after resizing and edits, and weak page lifetime.
The workspace also passed 4,847 Controls tests, 4,380 Base tests, 627 XAML tests
(17 ignored), 103 XamlX, 44 scanner and 51 native View tests. Android, framebuffer,
Wayland, X11, iOS and the required Windows suites supplied separate platform
integration evidence, with exact job links and scope in VALIDATION.md.

Reading the Windows subprocess logs exposed a masked informational automation
failure despite the green job badge: 49 passed, 136 failed and 39 ignored.
An independent comparison with main revision `5e6d8392` found identical outcomes
for all 224 named tests and identical missing-element errors following the same
native child-window creation panic. This is retained as an existing automation
baseline limitation, not reported as a passing suite or a new Repeater failure.

The rebuilt nonthreaded and threaded browser gates passed both WebGL2 and
Software2D, with 12 recorded stages per host/backend and no panics. Each
10,000-item initial source realized
24 Stack, 162 UniformGrid, 162 Flow and 161 LinedFlow controls. Current screenshots
show seven complete LinedFlow columns without the earlier clipped-edge labels,
and show the inserted item after Item 09999 following move-to-end. The reports and
artifact digests are recorded separately from the earlier diagnostic artifact.

Screenshot review also exposed an imprecise browser assertion: exported `hit`
flags include buffered rows clipped above the scroller, so the first reported
label after scrolling could overlap the toolbar. The browser test now restricts
visible labels to the sample's scroller viewport and requires each glyph probe's
full text rectangle to fit inside it. The host does not export ScrollViewer
bounds: an initial test-only attempt to read that missing export timed out.
The corrected test derives the viewport from the measured RealizationStatus
row and the sample's declared row spacing, border and padding, with the page
filling the fixed-height browser view. The report records that derivation,
viewport, probe rectangle and colour count. Horizontal clipping and
consecutive-label assertions remain enabled. This changes the test harness
only; runtime and catalog sources remain at `9039d2a5`.

The corrected harness is published as `db118fb5`. It passed against the exact
nonthreaded and threaded `9039d2a5` compiled catalog artifacts, in WebGL2 and
Software2D: 12 stages each, 48 PNGs total, zero panics. Every glyph probe fit
inside the derived viewport. All four scrolled reports contain Item 00100
through Item 00111, with 37 realized controls. The threaded reports confirm
render work on the render thread. VALIDATION.md preserves artifact identities,
archive/report hashes, commands, browser version and the separation between
the original CI runs and these corrected local replays.
