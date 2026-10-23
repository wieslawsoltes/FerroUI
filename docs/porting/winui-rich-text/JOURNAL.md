# Rich text implementation journal

This journal records how the port is developed. [PROGRESS.md](PROGRESS.md) and
the reviewed mapping remain the authority for implementation status; a checkpoint
preserves work and does not establish complete parity.

## Current main integrated; selection options recorded — 2026-10-22

The branch incorporates FerroUI main through
[`cfa62aeb`](https://github.com/wieslawsoltes/FerroUI/commit/cfa62aeb80f5d28990f3d4e4b9e5d1ad9d7fb640)
in merge commit
[`1f793521`](https://github.com/wieslawsoltes/FerroUI/commit/1f7935218abd8ca9355a96c0c20c2a717f7856d5).
The one generated XAML-map conflict was resolved by regenerating the combined
corpus. Main's unrelated changes are preserved. The subsequent feature work
remains separate from this merge's validation.

The [Windows job at the merge](https://github.com/wieslawsoltes/FerroUI/actions/runs/39765696678/job/119780463926)
passed native CoreText compilation, 49 WinUI tests, two exact UTF-16 string
tests, four system-MSFTEDIT probes and 561 rich Controls tests. The preserved
[versioned native record](native-msftedit-navigation-options.json) contains
60 Home/End commands, 60 repeats, 300 horizontal commands, 290 vertical commands
and 140 selection-options/typing commands. It records the exact commit, probe
blob, job and Windows/MSFTEDIT versions. All earlier 220 horizontal and 282
vertical rows reproduce the preceding record exactly.

The added mixed movement sequence distinguishes the desired column saved by a
collapsed caret move from the actual column used while extending a selection.
The options matrix records each getter phase and the resulting story. This
engine keeps Replace set, reports StartActive on an insertion point, accepts
the writable Active bit without demonstrating OS focus, preserves Overtype,
and leaves an empty TypeText call unchanged. Nonempty overtype selections and
paragraph boundaries require distinct treatment. These are observations of the
recorded system engine; they do not establish full WinUI or platform parity.

The next probe expands TypeText to inserted supplementary scalars, combining
sequences, ZWJ sequences and paragraph breaks, with and without a preexisting
selection. Those 176 added commands have no recorded native outcome yet.
The workflow now uploads the raw probe log and exact source identities before
the longer Controls suite, so a later cancellation does not hide completed
native evidence. The extractor's seven integrity tests roundtrip every
preserved artifact, including the new options record.

## Native repair confirmed and horizontal probe prepared — 2026-10-21

The [Windows job at 17a84805](https://github.com/wieslawsoltes/FerroUI/actions/runs/39615974131/job/119276997549)
completed successfully. Native CoreText compilation and **49 WinUI tests**
passed, including all six new Windows-only layout ownership regressions.
Both exact-string tests, both native probes and **529 rich Controls tests**
also passed, with zero failures/ignored. This closes the two ABI-boolean compile
errors from the preceding checkpoint. It does not imply execution of a real OS
candidate/proofing session.

The [Linux job at the same commit](https://github.com/wieslawsoltes/FerroUI/actions/runs/39615974131/job/119276997010)
also completed successfully: 4,413 Base, 4,970 Controls, 308 Win32, 39 WinUI host,
210 Fluent, 213 Simple, 296 WinUI theme and 611 XAML tests passed. The existing
ignored counts are recorded in TESTS.md. All five reference generators passed
and their checked-in outputs matched exactly. Both jobs in this workflow are
therefore green; this result precedes the newly prepared focus/clipboard and
InputScope/extended-column changes.

The [confirmed navigation record](native-msftedit-navigation-confirmed.json)
uses MSFTEDIT 6.2.26100.33648. Every fixture and all 60 Home/End + 60 repeat + 282 vertical
rows exactly equal the preceding 6.2.26100.33438 record. The newly isolated
extension-column behavior is therefore reproduced across both recorded versions.

The next probe source adds 220 fresh-window horizontal commands over ASCII
directed selections, positive/negative/zero counts, UTF-16/combining/ZWJ carets,
word boundaries and mixed Hebrew/LTR text. Separate default and RTL-reading
window fixtures record the actual extended style without claiming that it
alone establishes paragraph direction. Native selection types and flags are
recorded before/after each command. SDK slots 60/61/62 and Character/Word values
were checked before adding the typed probe calls.

The parser adds an optional horizontal family, native integer type fields,
required u32 window-style provenance and strict Rust Debug string decoding.
Existing 264/282 records re-extract unchanged. Synthetic parsing checks verify
Unicode/escaped-backslash preservation and rejection of malformed escapes,
missing/overflowed styles, mismatched counts, invalid directed endpoints and
unrecognized fixtures. These are extraction checks. No native horizontal
destinations or production horizontal/Options semantics are inferred before
the new Windows probe executes.

## Focus handoff, independent clipboard and isolated native evidence — 2026-10-21

The documentation, native cancellation fix and fresh probe results were
published as [17a84805](https://github.com/wieslawsoltes/FerroUI/commit/17a84805be35267ef580624d24f8657819038f88).
Focus and clipboard source are separate subsequent checkpoints. The full port
is still incomplete; the draft PR is not a claim of native conformance.

### Completed integration CI and precise native failure

[Run 39612123175](https://github.com/wieslawsoltes/FerroUI/actions/runs/39612123175)
finished the full Linux job successfully at `92140e7c`: Base 4,413; Controls
4,970; Win32 308; WinUI host 39; Fluent 210 passed/two ignored; Simple 213/three;
WinUI theme 296/two; XAML 611/17, plus one ignored doctest. All five generators
and exact checked-in reference comparisons passed.

Its Windows adapter check and WinUI test compilation reported the same two
E0308 errors. `LayoutRequest.IsCanceled` returns the ABI byte type; the new
predicates had treated it as a Rust boolean. [e5f1cf8e](https://github.com/wieslawsoltes/FerroUI/commit/e5f1cf8e71d1a792eb67b068ec36d780323cd317)
changes only those checks to explicit nonzero/zero comparisons. It still needs
the subsequent Windows run. Independent later steps passed two exact HSTRING,
two native probe and 529 rich Controls tests. Those passing steps do not make
the failed job successful.

### Third versioned native record

[native-msftedit-navigation-isolated.json](native-msftedit-navigation-isolated.json)
retains all 60 Home/End commands, 60 repeats and 282 vertical commands, including
18 fresh-control/sequence observations. It records the exact source commit,
probe blob, run/job and Windows/DLL identity. This runner used MSFTEDIT
6.2.26100.33438, whereas the previous vertical record used 6.2.26100.33648.
All original 264 vertical and all Home/End rows are identical across the two.

The fresh rows resolve two different issues. Native extension really loses
its earlier horizontal column after an intervening short line: collapsed
Down2 from 5 reaches 15, while extension reaches 12. A four-command collapse
sequence visits 5→8→14→8→5; extension visits 5→8→10→8→1 with anchor 5.
By contrast, fresh final-empty-line Up1 and Down(-1) agree at 4; their earlier
difference depended on reused native state. Five new off-tree Rust fixtures
against the exact 235-case libraries produced three expected failures and two
passes, establishing the next navigation correction. It remains pending in
this documentation checkpoint.

### Accepted focus before synchronous callbacks

The [focus checkpoint](https://github.com/wieslawsoltes/FerroUI/commit/b16a5ce14a7bdebfed0752254f5b5d2eb9c3db42)
fixes an actual template remount during popup focus return. Previously the
keyboard had accepted the editor, but the input manager still saw the previous
popup child during LostFocus/GotFocus. A legitimate view requery retired the
owned composition.

The keyboard now prepares the accepted destination and root subscription
before those events, preserving ordinary post-event binding installation.
Accepted generations fence nested focus transitions, including a round trip
back to the same owner. Canceled nested requests do not invalidate the outer
accepted transition. Client/view callbacks also check focus generation.
Four public tests cover native/overlay remount, nested Lost/Got redirection and
return, cancellation, and ordinary client-installation timing. Their old-library
run produced two expected remount failures and two cancellation passes; all
four pass with the repaired Base implementation.

### Asynchronous independent-range clipboard

The [clipboard checkpoint](https://github.com/wieslawsoltes/FerroUI/commit/132c7853a17553e475e842d8c7c3327a60a37e6e)
maps ten exact pinned public identifiers: interface/runtime-class Copy, Cut,
Paste and CanPaste, plus document CanCopy/CanPaste. The implementation injects
FerroUI's clipboard and uses its local futures. BestAvailable, PlainText and
Rtf are explicit supported policies; arbitrary native numeric format values
are not invented. Availability checks read format metadata only.

Immutable Copy capture is independent of later edits. Pending Cut/Paste track
model mutation identity, shared range/format generation and an editor-change
subscription; held batches and away-and-back changes cannot redirect an older
request. Retrieved transfers are disposed once before final validation, also
on errors or dropped payload futures. Outgoing data belongs to the clipboard.
Successful edits preserve the directed editor selection and one undo unit.

Cut ends the original preview before copying committed content. A write failure
preserves the committed document/history but does not recreate that preview.
Paste parses and limits data first, then cancels the original preview and reads
live committed endpoints. Empty, invalid and fully rejected payloads preserve
the target. The existing composition and edit-group fences preserve independent
callback-owned edits.

### Completed local gate and remaining scope

The coordinated rebuild completed both ordinary Base variants and Controls in
222.97 seconds, then stopped before unrelated downstream linking. All recorded
source hashes stayed unchanged. The source-identical public harness passed
**235 tests, zero failures/ignored**, in 2.88 seconds: the previous 203, four
focus cases and 28 clipboard cases. The three private startup tests remain
excluded whole only in the local harness and present in full repository CI.

The reviewed map now contains 36 complete enum entries, 464 partial public
entries and 817 pending, with 638 source/reference files, 938 additional
framework dependencies and 827 include boundaries. No runtime source file is
complete. Native copy/per-range protection, OLE formats, bounded RTF exclusions,
terminal EOP and synchronous native ABI remain explicit under RT-I24.
InputScope object/transport work and the new extended-column correction are
the next source slices; no premature native or whole-port completion is recorded.

## Context menus, native-informed movement and geometry — 2026-10-21

The next published checkpoint is
[`92140e7c`](https://github.com/wieslawsoltes/FerroUI/commit/92140e7cad7096d30e4f6b7f7ebdb8cace679a0b),
with separate commits for observations, formatter geometry, cursor ownership,
context/markup integration, native CoreText layout, isolated native probes and
selection movement. The full port remains incomplete and PR #243 remains draft.

### Completed baseline and second native record

The entire [rich-text workflow at 41ea4f1b](https://github.com/wieslawsoltes/FerroUI/actions/runs/39604251320)
completed successfully. Linux passed 4,412 Base, 4,915 Controls, 308 Win32 and
31 WinUI tests; Fluent passed 210 with two ignored, Simple 213 with three ignored,
WinUI theme 296 with two ignored, and XAML 605 with 17 ignored. All five reference
generators and the exact checked-in-file comparison passed. Windows passed native
adapter compilation, 35 WinUI tests, two exact HSTRING tests, two native probes
and 474 rich-control tests. These are full checkpoint results, not coverage of
the subsequent source changes.

The second native record contains 60 Home/End commands, 60 repeats and 264
vertical commands on Windows 10.0.26100 / MSFTEDIT 6.2.26100.33648. Its Home/End
rows exactly match the first record from 6.2.26100.33438. The strict extractor
retains all exact text, native lengths, endpoints, flags, counts and HRESULTs in
[native-msftedit-navigation.json](native-msftedit-navigation.json), with source
blob and CI provenance.

Those observations corrected a second assumption: vertical movement collapses
to the directional normalized endpoint and applies the full requested distance.
Collapse alone counts as one when the edge cannot move further. Line extension
can reach a story edge as an additional unit. The Rust model continues to exclude
the native synthetic terminal paragraph mark. Some reused-control extended
column/EOL rows remain ambiguous, so the next probe adds 18 commands using fresh
controls and explicit sequential collapse/extension. The source now defines 282
vertical commands; those additional rows await their next Windows execution.

### Context and markup integration

ContextMenuOpening now shares the framework's routed event, ordinary context
request, popup positioner and owned-focus lifecycle. Pointer, Apps/Shift+F10 and
holding requests raise one cancellable event with root-DIP coordinates. A handler
may install/replace/remove the current menu; nested requests and detach/reattach
invalidate the older opening. Keyboard anchors use normalized selected edges or
the shaped caret, independent of selection direction.

Actual runtime verification exposed two production issues. Selecting a corner
after transforming its rectangle reversed the desired edge under RTL reflection;
the corrected code chooses the presenter-local point and then transforms it,
matching pinned TextBoxBase ordering. Ordinary popup dismissal also retired the
input connection because GotFocus announced an unchanged presenter before the
input manager's focus cache caught up. Removing that redundant view-identity
notification preserves the original session; actual view lifecycle and cursor
notifications remain. A separate redirect test confirms that focus accepted
outside the editor retires the session and cancels preview.

The first keyboard fixture also omitted Win32's Shift+F10 configuration. It now
uses the actual platform mapping instead of adding a control-specific shortcut.
Its RTL numeric expectations use fixed-font logical source columns and real
line origins; a full visual selection rectangle is not the first logical
selected grapheme in a reflected mixed-direction view.

Metadata probes independently reproduced three failures in existing composition,
selection-changing and text-changing arguments: concrete-value metadata rejected
the shared routed handle actually passed to markup handlers. Checked downcasts
through the normal routed base repair that mismatch. Runtime and compiled
base-argument handlers and exact typed Rust subscribers now see the same payload,
Handled and Cancel state. Two new class documents were generated by both real
compiler hosts. The full XAML gate passed 611 tests, zero failures and 17 ignored.

### Navigation and shared prerequisites

Line/Paragraph MoveUp/MoveDown now uses shaped line indices and hit testing,
retains horizontal position over short lines, and handles negative/extreme counts
without iteration or signed overflow. Cancellation and callback redirection
report accepted units. Independent navigation ownership protects nested commands
and view changes even when selection offsets match. Home/End includes the first
native endpoint and real-paragraph-mark corrections.

The initial 193-test integration run passed 185 and failed eight. Five new
fixtures attempted an invalid second document owner and were repaired through
actual presenter/overflow or explicit release/reassociation. The RTL failure
exposed a pre-existing formatter defect: a structural EOL/EOP marker entered a
directional glyph group, collapsing all Hebrew caret positions to one edge.
The correction separates the marker and uses the preceding logical run's trailing
caret. A direct formatter regression, including nonzero source origins and
one/two-unit markers, was demonstrated red before the fix and green afterward.

Two further red regressions showed that reassociating the same presenter during
layout/scroll cleared retained horizontal position (caret 10 instead of 14).
The editor now treats the same validated live view as unchanged. Actual
replacement, detachment and expired associations still invalidate it.

The shared input manager also needed a callback fence before native layout could
be trusted. A custom caret getter could change owner/client or publish newer
same-client geometry, then return an old rectangle that overwrote the new one.
Binding and cursor-query generations now protect publication, and the accepted
owner transform is captured after the getter. Three public integration tests
cover replacement, reset, focus round trips and nested geometry.

### Native layout and validation boundary

CoreText now distinguishes whole-control bounds from its current-caret text
fallback. Complete rectangles pass through native TransformBounds, validated
scale and HWND screen origin. Client/session/revision/root/owner/geometry checks
surround callback-bearing work and native setters. The last revision callback
precedes final pure geometry comparisons. Missing or stale native geometry is
left unavailable. Name/Pin hints map to PersonalFullName/PinNumeric. Arbitrary
requested-range geometry and actual candidate-window observations remain pending.

The coordinated ordinary-library build passed all 15 portable CoreText tests
(four option, seven geometry, four synchronization). A refreshed public harness
then passed 202 tests, including all 18 context tests, 27 movement tests, 22
Home/End tests, three cursor tests and the Base marker regression. Eighteen
relevant XAML checks passed against the rebuilt graph; generated Rust paths,
View builders and Simple/Fluent reference comparisons remained current.

A scratch-only follow-up reproduced one accepted movement-count error: a handler
redirecting an extended vertical move to the original line's story edge returned
zero instead of minus one. After the narrow two-file correction, Cargo completed
the matching ordinary Controls artifact while Base stayed byte-identical. The
runner then intentionally stopped before downstream compilation. The source-
identical public harness passed **203 tests, zero failures/ignored**. This is a
scoped library build plus test run, not a full Cargo no-run claim. Three private
startup tests remain excluded whole locally and retained in complete CI.

The [new workflow at 92140e7c](https://github.com/wieslawsoltes/FerroUI/actions/runs/39612123175)
has passed its full Base step but reported a native CoreText compilation failure;
diagnosis is pending the completed job log at this entry. Six new Windows-only
adapter tests therefore have no successful execution claim yet.

A separate public scratch probe also reproduces connection retirement when an
application replaces the template synchronously during popup focus return.
That legitimate view requery encounters the same stale manager focus cache; it
is being repaired independently. It does not invalidate the 203 completed test
results or make the broader focus contract complete. Clipboard TOM range APIs
and the full InputScope object/collection/value family are the next source slices.

The reviewed dispositions now contain 36 complete enum entries, 454 partial
public entries and 827 pending. No runtime source file is complete. Test counts,
source coverage and native characterization remain distinct forms of evidence.

## First native selection observations — 2026-10-21

The [Windows job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39600670835/job/119224339597)
at `eff6f53ade273677db8679d2c6089909fcf7a886` completed successfully:
native CoreText compilation, 35 WinUI contract tests, two exact counted-string
tests, one system MSFTEDIT characterization test and 474 rich-control tests.
The native probe performed 60 Home/End commands plus 60 repeated no-ops on
Windows 10.0.26100 / `Msftedit.dll` 6.2.26100.33438. The observed values are
preserved in [native-msftedit-home-end.json](native-msftedit-home-end.json),
with exact tested source/CI provenance and a strict reusable log extractor.

These observations exposed a real collapsed-Line mismatch in the preceding
Rust checkpoint: native Home/End first selects the normalized range edge, while
extension uses the directed active endpoint. Extended Line End also includes a
real paragraph mark and retains line-end affinity for the repeated operation.
Corrections and focused regressions are being developed from these values.
Native extension can additionally select the synthetic terminal paragraph mark;
the normalized Rust document still excludes that mark. The evidence records the
system engine and does not claim equivalence with external WinUI RichEdit.

The next native source checkpoint adds 264 MoveUp/MoveDown observations across
Line/Paragraph, positive/negative/zero counts, directed selections, Unicode,
empty stories and retained-column sequences. Its test definition is separate
from the completed Home/End run. Linux Base, Controls and all theme steps had
also passed at this point; their exact totals and the remaining generated-file
gate require the completed Linux job log.

## Selection boundaries and focused input policies — 2026-10-21

[`05050fa0`](https://github.com/wieslawsoltes/FerroUI/commit/05050fa0de090270ac2b8785874871743363c8ed)
adds selection Home/End for Line and Story. The Rust projection uses the directed
active endpoint, retains the anchor while extending, and reports accepted signed
UTF-16 displacement through cancellable selection events. Shaped line targets
retain real content ends independently of the terminator-inclusive ranges used
for ordinary movement. Complete linked overflow, wrapped/RTL text and headless
paragraph fallback have focused fixtures. Unsupported units fail before effects;
navigation preserves read-only access, preview and redo. Layout callbacks are
followed by a fresh selection capture, and model writes during selection or
publication retain external state instead of restoring obsolete history.

Native documentation differs in how it describes the selected line and collapse
count. The two exact public entries are therefore partial under RT-I21. The
Windows-only probe in
[`eff6f53a`](https://github.com/wieslawsoltes/FerroUI/commit/eff6f53ade273677db8679d2c6089909fcf7a886)
loads the system MSFTEDIT DLL from System32, verifies exact text and directed
selection setup, and records 60 initial commands plus 60 repeated no-ops.
Typed SDK-derived COM prefixes have compile-time offset checks and owned cleanup.
Logs include Windows/DLL versions, supplied/native story lengths, endpoints,
flags, HRESULT and signed deltas. Initial line destinations, count conventions
and final-EOP position are observations, not assumed parity assertions. This
probe remains distinct from the external WinUI engine and awaited its first
Windows execution when this checkpoint was written.

[`5fe5f5d3`](https://github.com/wieslawsoltes/FerroUI/commit/5fe5f5d34cf658595dfa6379c296beeaf0797abb)
repairs two real option-delivery gaps. A fresh WinUI adapter previously discarded
options sent before its lazy context existed, then created the context with a
Text scope. CoreText now caches the desired options and applies them before
focus entry. Existing focused bindings also observe effective changes to all
nine semantic-owner options, including inherited and cleared local values,
without resetting composition or replacing the client/connection. Weak owner
subscriptions follow binding lifetime. If an older backend setter completes
after a callback changes the same transport's owner or policy, the manager
reapplies the current values; CoreText serializes nested native scope setters and
retains failed desired values for retry. Existing scope mappings remain the
backend subset. Policy delivery does not establish OS prediction/proofing behavior
or implement the complete native InputScope family.

After rebuilding ordinary Base and Controls libraries with the exact XAML feature
union and checking source/fingerprint freshness, the public harness passed
**147 tests, zero failures**. That includes the previous 122 cases, three original
keyboard fixtures, all 16 new Home/End cases, four existing line-unit regressions
and both rich policy cases. The three private startup exclusions remain explicit;
their earlier actual Windows pass is recorded below. The separate portable
CoreText cache command passed **three tests, zero failures**; XAML ran no cases
under that filter. Seven new private Base lifecycle tests await the full CI gate.

Independent review found no concrete reentrant policy or native-probe ownership
defect. Local formatting/diff checks and focused runs are not a native execution
claim. The shared local filesystem later filled while no task build was running;
source/build work paused, and only proven obsolete task-owned artifacts were
removed after checking active references and current dependency reachability.
Required host/target libraries, original fixtures, commands, provenance and logs
were retained. No unrelated cache or process was changed.

The two Home/End mappings bring the reviewed public-entry dispositions to 36
complete enum entries, 446 partial and 835 pending. The source inventory remains
638 selected files, 1,317 exact public identifiers, 938 framework dependencies
and 827 include boundaries. All runtime source files remain incomplete until
their individual outstanding contracts are closed with evidence.

## Integration recovery and continued contracts — 2026-10-21

Work resumed from published head `0fba6ea72d031bdfc5b1d4662ef9bf84ebcc4590`
on PR #243. The shared-chat URL could not supply readable contents, so the actual
branch, PR, pinned sources, existing journal and CI logs supplied continuity.
The [baseline rich text workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39495835486)
provided concrete failures: Linux Base passed 4,405 tests; Controls passed 4,822
with one failure in committed selection notification; the Win32 and WinUI host
suites passed 308 and 28 tests. XAML passed 592 with ten failures and 17 ignored.
Theme/XAML reference drift, rich value initialization, missing `LetterCase`
registration, corpus accounting, source-pin expectations and a missing queued
caret-scroll dispatcher drain accounted for the integration failures. Native
Windows CoreText compilation, WinUI contracts and counted-string contracts
passed their steps; the control-selection regression also failed on Windows.
These results describe the old head, not subsequent changes.

[`c3f9a8d6`](https://github.com/wieslawsoltes/FerroUI/commit/c3f9a8d6126a46df0d1d121cfd35bf362a311a5b)
fixes the extra native SelectionChanged event by comparing committed selection
only, while eagerly sampling intrinsic input revisions to retain callback and
caret-round-trip invalidation. The original failing assertion is retained.
[`34aeed1c`](https://github.com/wieslawsoltes/FerroUI/commit/34aeed1cb6b3fff3dffeff4445d01cf4f3fc6463)
integrates main at `86b6b4ab41669bf830fba681504a4ee94752a4e6`, including the
current declarative view generator and framework work.

[`8ca12007`](https://github.com/wieslawsoltes/FerroUI/commit/8ca12007b9ffc9f42ad8d0c842fba908a51a6917)
adds a framework-specific TextRange construction transform before XamlX's
generic value initializer guard. Runtime and compiled markup use the same
temporary carrier and normal field assignments, then copy the value into the
collection. Attribute, property-element and `x:Arguments` forms are exercised;
unrelated value initializers remain rejected. Differential dumps now traverse
blocks and highlighter/range collections. Seven focused rich markup tests
passed. The first full run passed 603 tests with two failures and 17 ignored:
one differential fixture lacked a font-service test scope, and the deliberate
missing-cast refusal manifest omitted the new rich fixture. The checkpoint
repairs those harness issues; the later full rerun is recorded separately.

[`a6152242`](https://github.com/wieslawsoltes/FerroUI/commit/a6152242dddbd0baf0ba243f97313b95ce1c3173)
extends TOM Delete/Character and composition lifecycle contracts. Deletion resolves remaining units after removing an initial
selection, retains paragraph join rules and preserves independently directed
selection. Character replacement retains assigned/inherited formats and offers
a scalar-safe Rust extension to the native UTF-16 property. Review exposed a
transaction boundary where layout or deferred publication callbacks could change
the model and a failed group would restore an obsolete snapshot. Mutation fences
now preserve the external edit and live rebasing, reject stale completion and
discard obsolete undo, typing state and queued causes. Ordinary failed edits
still roll back; nested groups and held outer model batches have regressions.

Composition events now carry readonly StartIndex/Length and accepted text.
Explicit native begin captures the original document without a synthetic Delete,
raises an empty 0/0 start, and waits for the first real delta to establish a
composition extent. Empty completion/cancel closes once without history or lost
redo; legacy implicit startup remains supported. The pending automation range
is absent until an actual update. Reentrant callbacks and newer sessions retain
ownership. Native asynchronous nested event dispatch and OS automation before
the first update remain explicit conformance boundaries.

[`3a3c638b`](https://github.com/wieslawsoltes/FerroUI/commit/3a3c638b36948bb4a3e39aac6b3e6da4dd6ba2fa)
adds RTF import of supported paragraph/character stylesheet references and
allows hyperlink field results across paragraphs. The style implementation uses
the official RTF 1.9.1 grammar and existing FerroUI format decoding, with checked
inheritance and expansion budgets. Independent review found a URI amplification
risk when creating one link per paragraph; cumulative materialized URI/target
bytes are now bounded before cloning. A small-limit stream regression checks
rejection while an existing composition remains untouched. Named style and field
identity, section/table styles and advanced RTF features remain pending.

The source inventory adds `core/inc/TextCompositionEventArgs.h` as a reviewed
source rather than leaving it only in the include dependency closure: 633
selected files, 1,258 exact public API identifiers, 941 additional framework
dependencies and 826 include boundaries. No runtime source file is marked
complete. Test execution records below and in `mapping.json` qualify the
implementation claims; compiling or adding a fixture alone is not runtime proof.

The main-integration checkpoint's subsequent
[CI job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39590676737/job/119190896776)
passed all 4,405 Base, 4,827 Controls, 308 Win32 and 28 WinUI host tests before
the next push cancelled the remaining workflow steps. This confirms the
committed-selection regression repair. The newer Controls additions emitted no
remaining Rust errors after three fixture type-annotation fixes, but the local
monolithic lib-test compiler was killed with SIGKILL under the shared 8 GiB
memory limit. That command executed no tests. Dedicated CI and a temporary
focused external harness provide subsequent runtime evidence; memory-pressure
failure is not recorded as either a test pass or a behavioral assertion failure.

The final XAML rerun passed **605 tests, zero failures and 17 existing ignored**.
Its differential tree walker now enumerates inherited text decorations instead
of treating the collection as unreadable. All 127 model/emitter fixtures and
runtime/emitted rich-document equality remain strict.

A temporary external test harness reused those ordinary compiled libraries to
avoid the monolithic Controls test compiler. It retained original test bodies
and assertions, with three whole startup tests excluded because they require
private APIs. Its first diagnostic run executed 122 tests: **120 passed and two
failed**. All 40 RTF tests and 49 included input/composition/exact-edit tests
passed. The two range failures isolated an empty formatting carrier left by
whole-paragraph deletion and a fixture comparing a pre-installation hyperlink
snapshot with its canonical installed formatting. Full Controls CI, including
the excluded empty-start, empty-update and empty-completion tests, remains the
acceptance gate. No private production API was exposed for this harness.

The structural repair uses an empty document fragment for paragraph-spanning
deletion; same-paragraph run splicing remains unchanged. The inherited-font
assertion is retained. The hyperlink fixture now captures its installed model
before editing, retaining full snapshot equality and explicit formatting, URI
and embedded-control identity checks. After rebuilding the ordinary Controls
library, the focused suite passed **all 122 tests**. The repair is preserved in
[`73a5d338`](https://github.com/wieslawsoltes/FerroUI/commit/73a5d3384f5fcfa36ba30f604d38c521784189cd).

[`338ef632`](https://github.com/wieslawsoltes/FerroUI/commit/338ef632397acb5cc855c12e9c27aff94c93036a)
preserves the final differential collection comparison and theme fixture repairs.
The theme tests drain queued caret-scroll work after layout without relaxing any
caret/extent bounds. The WinUI source-provenance assertion recognizes the rich
document's actual source pin. All **12 Simple/Fluent rich-theme tests passed**
against their existing ordinary libraries with unchanged test bodies and public
helpers. WinUI execution and full generated-reference freshness remain separate
CI gates. The optional dedicated Loader pipeline test began rebuilding a large
host dependency variant and was interrupted; no runtime result is claimed for
that command.

The completed Windows job for `3a3c638b`
([run 39593957823](https://github.com/wieslawsoltes/FerroUI/actions/runs/39593957823/job/119202048452))
passed native adapter compilation, all 32 WinUI tests and both counted HSTRING
tests. Its rich Controls filter passed 453 tests and failed three: the two range
cases repaired above, and a Home/inline-break fixture with a hardcoded Unix
offset. The model and formatter both represent inline breaks as CRLF on Windows,
so the fixture selected the interior of that break instead of the following
line. The three private startup tests excluded from the local public harness
all passed on Windows. The overall failed job is retained as failure evidence;
these results do not establish a real OS IME session or a passing complete suite.

A dependency audit also found 59 missing public entries across candidate-window
alignment, context-menu arguments/delegate and the InputScope family. They are
now explicitly inventoried as pending. A single FerroUI content-type value does
not implement the native Names collection or all scope constants. Three existing
runtime-class range methods now share the already-reviewed partial mapping of
their interface counterparts. Stale control/event descriptions now name the
actual TemplatedControl owner, persistent presenter and composition range fields.

The regenerated dependency audit and source-integrity check passed with **638
selected source/reference files, 1,317 exact public identifiers, 938 additional
framework dependencies and 827 include boundaries**. The map records 36 complete
enum entries, 444 partial entries and 837 pending entries. No runtime source file
is marked complete; these counts describe inventory dispositions, not functional
parity percentages.

[`2b1cfe70`](https://github.com/wieslawsoltes/FerroUI/commit/2b1cfe70f7f9043a7058fc05d91c0332bfb5d81d)
corrects the inline-break navigation fixture to derive its offset from the
platform model. The three original Home/End keyboard fixture bodies passed on
Linux before and after that fixture correction. The Windows branch of the
assertion still requires the subsequent native runner; a host pass is not a
Windows execution claim.

The completed Linux job at `3a3c638b`
([run 39593957823](https://github.com/wieslawsoltes/FerroUI/actions/runs/39593957823/job/119202048744))
passed 4,405 Base, 308 Win32 and 28 WinUI host tests. Full Controls passed 4,895
tests and failed the two range cases repaired in `73a5d338`. Fluent passed 208
with two generated-reference failures; Simple passed 210 with those two failures
and the deferred-caret fixture; WinUI passed 293 with those two failures and the
source-pin fixture. XAML passed 604 with the collection-enumeration failure. The
fixture repairs are preserved in `338ef632`; the historical job remains failed.

All three theme regeneration commands and the differential emitter regeneration
completed successfully in that same job. Its artifact `12343074706` has archive
SHA-256 `97e609469c28eea8e5bc3a50b5013a92ba58b890aaa7a9aeaaf81cf2f651f510`.
Review verified the source run/head, exact eight-file archive, unchanged XAML
metadata and differential emitter output, and independently identical local
Simple/Fluent Rust generation. Each theme gains the RichEditBox document; the
generated document counts become 83 Simple, 87 Fluent and 151 WinUI.

The 17,277,327-byte WinUI Rust reference exceeds the connector's 16 MiB request
limit. A temporary branch-scoped workflow therefore copied only that file from
the verified artifact, checked its SHA-256 and exact Git blob
`1a292a76fdfd29016c5e6a08224a20552c088cc1`, and made an ordinary push after checking
that the branch had not advanced. The successful
[transfer run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39599133779)
produced [`4c867440`](https://github.com/wieslawsoltes/FerroUI/commit/4c867440ababd7acd1191af74604cd2b80d6ad12)
after the Simple/Fluent checkpoint
[`e8361dc2`](https://github.com/wieslawsoltes/FerroUI/commit/e8361dc2365507d68af0e26e7a9c6ee447da4ff1).
The workflow was then removed in
[`702f427c`](https://github.com/wieslawsoltes/FerroUI/commit/702f427ca9440727e76fd8343e9dc617e97bdf9f).
Its final tree exactly matches the locally reviewed three-file regeneration.
The regular conformance workflow now also regenerates and checks the markup
Rust-path reference. Complete freshness and theme-suite results remain attached
to the next actual CI execution, not inferred from the transfer.

## Reference and first published batches — 2026-10-17

The source reference is fixed at
[`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de).
The inventory retains all 230 files in `core/text`, 62 in `core/native/text`, and
23 in `components/text`. Repository inspection established that the public source
contains the RichEdit host and LineServices adapters, while the implementations
of `WinUIEdit.dll`, the external text object model, and the line engine behind
`Ptls-min.h` are external. Those boundaries are documented in [DEPENDENCIES.md](DEPENDENCIES.md).

The initial tracking batch was published as
[`656509591a045f50abb86807da7ecf11ad6375ce`](https://github.com/wieslawsoltes/FerroUI/commit/656509591a045f50abb86807da7ecf11ad6375ce).
It retained 609 source/reference files and 1,079 exact public XML identifiers.
Its reviewed map was empty; no runtime or parity success was asserted.

The core document, display/overflow, editor, RTF/clipboard, input-method and Rust
text object model batch was published as
[`432d10fb0335be3fb586290bf7467126a790646f`](https://github.com/wieslawsoltes/FerroUI/commit/432d10fb0335be3fb586290bf7467126a790646f).
The follow-up
[`fb15fa7bf618e21233edaf567a9ed0c0a5096e74`](https://github.com/wieslawsoltes/FerroUI/commit/fb15fa7bf618e21233edaf567a9ed0c0a5096e74)
corrected the hyperlink test to follow the source's Enter-on-key-release behavior.
Those published test files contain 21 document, 37 editor and 16 view test
definitions. Their presence is separate from the command/result records in
`mapping.json`.

## Workspace reconstruction and expanded audit — 2026-10-18

The execution environment disconnected before later work was checkpointed.
The restored environment did not contain that working tree. Work resumed by
cloning the published branch and the pinned source again. Later implementation,
mapping and documentation are reconstructed and reviewed against the actual
restored files; they are not represented as byte-for-byte recovery of unpublished
files. Previously observed local totals are not reused as current completion
claims.

The next inventory revision makes the block declaration, caret browsing,
hyperlink peer and text-options dependencies explicit. It also adds the native
UI Automation wrapper implementation/header, the generated native enum reference,
and the three automation model inputs. The public inventory now includes the
Text/Text2/TextRange/TextRange2/TextEdit/TextChild interfaces, their range units and
selection policy, text-attribute and annotation types, and text-specific pattern
and event dispatch members. Unsupported patterns and attributes stay pending;
expanding the denominator does not implement them.

The map is updated in implementation batches. Each reviewed entry names the
actual Rust symbol, its test evidence, and any observable translation. Runtime
test results are recorded only after their commands finish. Native Windows
execution, real input-method sessions, screen-reader interaction, exact rendering,
and external-engine contracts remain separate acceptance gates in [PLAN.md](PLAN.md).

## Continued source review and service contracts — 2026-10-19

The published branch was recovered at `2489efdcc60229a78eadbb52bcf4bc07bc5a3f86`
in an isolated checkout. Its GitHub CI failures were traced to a missing unsafe
block around an owned-BSTR COM test call; the repair was published as
`1b4704b2f96ac844dbfbe48b96fa5621e12e3ed3`.

Paragraph spacing and signed indentation were published as
`32b19d14facbf06a94c9715f1120d8e2a5b75a90`. This batch adds native-valued spacing
rules, point conversion, actual line/overflow behavior, signed indent storage,
RTF multiple spacing and eleven independent regressions. The enum values have
complete member mappings; the broader paragraph/native formatter contracts do
not inherit that completion status.

The document-service, edit-event and accessibility batch was published as
`871d566d25aab3e89021d7fc5db6edf32c07267e`. It adds shared default formats,
explicit get/set/find flags, native newline fixture assertions, UTF-32 and range
operations, cancellable/reentrant selection, synchronous TextChanging/deferred
TextChanged, framework content presenters, Text2 caret and TextChild hyperlink
providers. The integrated rich-text filter passed all 181 tests. Initial fixture
failures were corrected by using the existing per-thread test scope, installing
the framework keyboard device for focus tests and performing layout before
inspecting the automation visual tree. Production behavior was not weakened to
make those assertions pass.

The Win32 text-related filter subsequently passed 34 tests, including ten text
pattern/range bridge cases. Its first compilation caught a pattern-array arity
mismatch after TextChild was added; `f1a051f8eaddd2d9b651a731206eebd9fa3a252a`
repairs the complete twelve-interface registration. These runs occurred on a
Linux host using the portable Win32 test harness; they are not Windows
screen-reader or real native accessibility-client execution.

The mapping audit also found three historical test symbols absent from the
published sources. The ellipsis entry now points to an existing executed test,
and two absent scroll-test references were removed pending real regression
coverage. The generator checks every retained target and test symbol. No
unpublished test definition is treated as recovered or executed evidence.

Further source review established that real templates require RichEditBox to use
FerroUI TemplatedControl with a persistent rich presenter, not inherit the display
control's finite-height pagination behavior. It also distinguished display
batching from model notification batching, and native tab stops from the current
formatter's fixed-width tabs. These are implementation tasks, not stored-property
substitutes. The next batch preserves one document/editor while completing the
template/presenter, scrolling and stream-service boundaries.

## Persistent editor view, streams, units and language — 2026-10-19

The next implementation checkpoint makes RichEditBox a FerroUI TemplatedControl
with one persistent RichEditBoxTextPresenter. The model, directed selection,
composition and undo history survive template replacement. Header, placeholder
and description use ordinary framework presenters; document extent, scroll
offset, caret, IME, hit testing and automation share the presenter's coordinates.
Automation retains the semantic editor owner and exposes no duplicate document
through its raw presenter peer. Both physical and logical scroll alignment use
the last selected line and the established viewport.

Document and range streaming use bounded, explicit UTF-8/UTF-16 Read/Write
operations and the existing RTF parser, with complete validation before editing.
TOM Character, Word, Sentence, Paragraph, Line, Story and HardParagraph units now
have separate native-valued service contracts; Unicode navigation/search remains
distinct. An unmeasured editor falls back from Line to Paragraph, while a
measured presenter supplies current display-line boundaries. Cluster and other
unimplemented native units are not aliases for Unicode graphemes.

Inherited TextElement.Language now reaches FerroUI shaping and fallback through
CultureInfo, with the pinned source's XML-language validation and application
language default. Character-format snapshots, editor history and supported RTF
language controls preserve the tag. Windows locale resolution, the complete
LCID namespace and dual proofing languages remain separate native contracts.

The integrated command
`cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast`
completed with **246 passed, 0 failed, 4434 filtered out**. New coverage consists
of 15 stream, 19 TOM unit, 8 live-pointer, 6 language, 10 presenter/control and 7
automation tests. A real double-rebase bug in batched range replacement was fixed
by updating internal weak live positions at each model replacement while keeping
public document notifications batched. Tests cover nested edits, rollback,
undo/redo, reentrant callbacks and weak lifetime. Geometry fixtures now execute
the actual layout manager, and the unmeasured Line fixture uses wrapping rather
than a line break that was also a TOM paragraph boundary.

This checkpoint does not assert complete native parity. Theme templates,
compiled markup, TextEdit composition notifications and the remaining external
engine contracts continue in subsequent implementation and validation batches.

## Accepted composition, TextEdit and owned flyout focus — 2026-10-19

Published as [`ed836e831fc8af5114e2ce8ae7632867c7a2332f`](https://github.com/wieslawsoltes/FerroUI/commit/ed836e831fc8af5114e2ce8ae7632867c7a2332f).
The editor publishes accepted preview deltas and one finalization, after its
normalization, filtering and UTF-16 limits. Cursor-only movement emits no text
cause. Explicit cancellation finalizes with empty text; successful commit creates
one history unit without an intermediate cancel. Nested groups queue causes and
discard them on rollback; an internal mutation identity rejects stale composition
even before a surrounding model batch publishes its revision.

Portable TextEdit inherits Text independently of Text2, exposes the full live
composition range and disposes/rewires observations with the owner/document.
A conversion target remains absent until a backend provides that distinct range.
The Windows bridge adds the SDK interface and dispatcher-based BSTR SAFEARRAY
payloads. Array creation now cleans up initialized native slots on conversion
failure and releases locks during unwinding. Length-aware BSTR allocation retains
embedded NUL text. Public arbitrary-peer raising, EventsSource and lazy native
activation are a subsequent contract, not inferred from editor-provider events.

RichEditBox tracks the actual focused descendant and current target of its owned
context flyout. Active selection/composition survive that transition; focus
return, closure, replacement or genuine blur clears weak subscriptions. Generic
content hosts reveal the caret through the presenter's own logical scrolling.
The control's retained visual/model state does not alone retain a native IME
transport across a different popup root; the framework owner protocol is next.

The full controls command passed **4706 tests, zero failures**. The initial run
had six remote display failures because sandboxed loopback listening was denied;
all eight remote tests passed with loopback access, followed by the successful
full-suite rerun. One new caret-boundary assertion exposed only roundoff:
60.00000000000001 versus a viewport height of 60. The test now uses FerroUI's
existing geometry comparison tolerance and retains diagnostic values; production
scroll geometry was not padded to satisfy that assertion.

The Win32 `--lib text_` command passed **41 tests, zero failures** on Linux,
including four TextEdit and three array cleanup cases. CI on the previous
92551b30 checkpoint identified the missing published themes, stale generated Rust
path fixture and a scanner-unresolved `Self` reference in a highlighter type
registration. The equivalent explicit `TextHighlightRange` registration passed
the source scanner regression. Theme and generated-path verification follow in
the next checkpoint. Native OS/client and pixel-conformance evidence remains
separate from these results.

The reviewed map now includes the AutomationPeer partial and UIAWindow source
pairs: **630 selected sources, 919 additional framework dependencies, 806 include
boundary edges and 1258 exact public identifiers**. New mappings cover the
published presenter, streams, native-valued units, immediate pointers, inherited
language and TextEdit behavior. The previous Cluster/grapheme equivalence claim
has been removed. Twenty complete entries are enum fields; no source file or
entire rich-text subsystem is declared complete by that count.

## Inherited presentation versus document edits — 2026-10-19

The first complete WinUI theme run reached 293 passing tests but failed the new
opened rich-editor command-flyout test: undo was disabled after the theme was
applied. A diagnostic assertion localized the loss before the flyout opened.
The source trace was `TextElement` inherited font/foreground invalidation through
`BlockInlineHost`, followed by a document revision and invalidation of editor
history. The same path could invalidate an accepted composition when focus or
theme presentation changed.

The fix adds inheritance provenance to borrowed and owned property-change
arguments, independently of resulting binding priority. Ancestor propagation and
inheritance-parent changes set this flag; clearing a local document value does
not, even when the new effective value is inherited. A separate inline-host
layout route now reaches the view without notifying document observers. Local
formatting remains a semantic mutation. Inherited feature and decoration
collections use the same distinction. Existing snapshot capture already used
base values, so it retains inheritance across undo rather than freezing the
previous theme into restored runs.

`cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast`
passed **276 tests, zero failures**, including four new regressions covering
preview/history retention, actual layout refresh, inherited mutable formatting,
and local formatting inside a model batch. The Base `--lib inheritance` command
passed **34 tests, zero failures**, including assertions distinguishing parent
propagation from clearing a local value and checking owned arguments. Only
formatting of the new test file followed these runs. The real-theme rerun and
the remaining theme/markup/workspace gates are recorded in their own subsequent
checkpoint; these focused results do not establish those gates.

## 2026-10-20 — Published-state recovery, TOM casing and map reconciliation

Recovered PR #243 at `86ee377d`. Earlier unpublished local files were not
available in the new workspace, so new work is built from the published source.
Reconciled the source/member map with the already published shaped Cluster,
explicit/default paragraph tab, and public AutomationPeer TextEdit/EventsSource
implementations. Source files remain partial; the map retains the full pinned
inventory and does not turn lexical counts into parity percentages.

Added a dedicated structured `ChangeCase` implementation and native-valued
`LetterCase`. Complete selected-string Unicode casing is partitioned back into
existing inline structure, preserving cross-run final-sigma context, expanding
characters, paragraph/link/control content, and directed UTF-16 selection.
No-op edits preserve history/revision; successful edits form one undo unit.
Over-limit expansions fail before truncation; nonexpanding changes preserve an
already-over-limit document. Changed programmatic operations recapture committed
content after preedit cancellation. RT-I15 records the exact portable casing and
native locale/position boundaries.

Validation is pending at the time of this journal entry. The task-owned baseline
build was stopped to release memory while unrelated compiler jobs used the shared
environment; the stopped run is neither a pass nor a failure.
Focused and final integration results will be appended after they complete.

The same continuation adds exact UTF-16 range/text/resulting-selection input
requests, independently retained composition spans and generation/revision-aware
CoreText acknowledgement recovery. Empty replacement deletion, selection-only
updates, cancelled/reentrant acknowledgement and last-successful native lengths
have dedicated fixtures. Unsupported native clause formatting explicitly fails;
the legacy TextBox route and native OS execution remain tracked separately.

Source-map reproduction completed successfully after fetching the missing pinned
Git blobs in one batch: 630 source/reference files, 1,258 public identifiers, 919
framework dependencies and 806 include boundaries. A subsequent `--check` was
clean. These are source integrity/reconciliation gates, not runtime test results.

The continuation also supplies compiled RichEditBox themes for WinUI, Fluent and
Simple; owner-bound formatting commands preserve independent mixed state,
unrelated underline decorations and one history transaction. New actual-theme,
runtime/compiled XAML and catalog fixtures cover the framework integration.
`TextCommandBarFlyout.cpp` and its header are newly selected pinned source inputs,
so their command behavior and transitive include boundaries are now part of this
rich-text map. The native query/set branches were read directly before recording
the mapping. RT-I17 records the MenuFlyout/palette translation and remaining
native touch, lazy-part, localization and appearance contracts. Runtime gates
for these additions are pending until their completed results are appended.

The RTF reader now shares an incremental, non-replacing byte decoder across body
text, font names and field instructions. Seventeen portable code pages have
literal multilingual fixtures; font charset/code-page precedence, default-font
resets, Unicode fallback, malformed boundary handling and decoded UTF-16 bounds
are separately covered. `encoding_rs` is pinned to 0.8.42 with default features
disabled. The public Text XML remains a reference-only specification because the
native RTF engine is external; the reader/writer/stream API rows now link the
actual codec implementation and fixtures. RT-I18 preserves Symbol/OEM/Johab,
unlisted pages, vendor/native mappings, original-byte roundtrip and unsupported
full-content boundaries. Runtime gate results follow only when completed.

The native boundary follow-up retains exact counted UTF-16 query slices and
embedded NULs in owned HSTRING values while strictly rejecting malformed edit
payloads. Callback generation fences cover client access, subscription setup,
disposal and state queries. Host fixtures and Windows-only actual-adapter
regressions are linked separately from native execution evidence.

After adding the native text-command source/header and all continuation anchors,
the regenerated record and `--check` passed with **632 selected files, 1,258
public API identifiers, 942 additional framework dependencies and 826 include
boundary edges**. The map records 36 complete API entries, 428 partial and
794 pending entries; no runtime source file is marked complete. These numbers
measure the mapped inventory, not a percentage of functional parity. The source
integrity command and its actual result are recorded in `mapping.json`.

## 2026-10-22 — Published checkpoint and native horizontal evidence

Resumed PR #243 at `d148193deef581a1e99fc837c45c29ab9c6957c2`.
The previous InputScope and extended-column implementation was not published;
this continuation reconstructs it against the committed source and recorded
native behavior.

The [dedicated workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39618880985)
has now completed successfully. Linux passed 4,413 Base, 5,002 Controls, 308
Win32 and 39 WinUI-host tests. Fluent passed 210 with two ignored, Simple 213
with three ignored, WinUI theme 296 with two ignored, and XAML 611 with 17
ignored plus one ignored doctest. All five reference generators and the exact
checked-in reference comparison passed. Windows passed 49 WinUI tests, two
exact HSTRING tests, three native probes and 561 rich Controls tests.

`native-msftedit-navigation-horizontal.json` preserves all observations from
the Windows job, including 220 fresh-control horizontal commands. Its commit,
probe blob, workflow/job URLs, OS and DLL version remain attached to the data.
The 282 vertical, 60 Home/End and 60 repeated Home/End rows equal the previous
records exactly. These are system MSFTEDIT observations, not a claim of
conformance with the externally supplied WinUI editing engine.

The broader workflow had one failing catalog assertion on macOS: the expected
page total still included only the TreeDataGrid addition, omitting this branch's
Rich Text page. The corrected assertion includes both additions and verifies
that Rich Text is available in the Text section. Runtime verification of this
correction and all new implementation follows in subsequent checkpoints.

## 2026-10-22 — Live input scopes, navigation and measured typing evidence

The working branch merged current main at `1f793521`; its sole generated XAML
conflict was resolved by regeneration. Subsequent commits publish the live
InputScope object model and XAML/View integration (`e02522cb`), platform input
hints (`01d06a36`), and horizontal plus mixed vertical selection movement
(`f10fb838`). These changes use FerroObject, shared FerroList collections,
inherited TextInputOptions snapshots, existing platform adapters, and the
document/editor's directed selection and geometry APIs.

Final focused gates passed 11 Base InputScope tests, two View builder tests,
three runtime XAML InputScope tests, all 129 differential corpus inputs, XAML
regeneration/freshness, and 70 selection tests. The selection gate executes 294
literal native horizontal observations, six explicitly different terminal-EOP
cases and eight mixed vertical steps. The earlier full XAML feature run exposed
three issues; the focused repair results and original failures are both retained.
Later full CI found a further shared-list metadata contract: InputScope Names
must expose Capacity alongside Count and Add. That failure remains visible until
its correction receives a separate gate. The narrow fix is published in
`182a7e04`, using the same checked Capacity metadata as other FerroUI lists.

The expanded system MSFTEDIT probe at workflow head `394c11ab` completed all
four native tests. Its Windows job also passed 49 WinUI, two exact UTF-16 and 561
rich Controls tests. The new immutable `native-msftedit-typing.json` records 316
options/typing commands. Each typed Unicode scalar consumes one target grapheme
in overtype mode; CR and LF insert paragraphs independently without consuming
target text. This includes the observed two-paragraph result for TypeText CRLF,
which must remain distinct from ordinary text-loading normalization.

The raw artifact now records the actual synthetic merge checkout and probe blob
before the full Controls gate. Its measured checkout is `dfc982df`, separately
from workflow head `394c11ab`; the downloaded archive digest matches GitHub's
published SHA-256. Earlier immutable records used `provenance.commit` for the
workflow head and did not measure the checkout. Their observations and verified
probe blobs remain unchanged. TESTS.md and mapping.json preserve this provenance
distinction and do not infer a full Linux pass from the successful Windows job.

The source inventory now includes the pinned InputScope XamlOM declaration file
alongside its implementation, generated IDL and documentation. It also includes
the seven typography enum declarations and 45 additional public identifiers,
retained as pending until implementation and verification are mapped. The
regenerated inventory and check pass with 640 files, 1,362 public API entries,
938 framework dependencies and 827 include boundaries. Counts describe
mapped source and member coverage; they do not establish functional parity.

## 2026-10-22 — Typography, backend delivery and restoration integration

The continuation publishes all 43 Typography property families and ordered
OpenType projection (`a73a7246`), IBus ContentType delivery (`7796984b`), the exact
Browser DOM edit bridge (`58939d8d`, `0259ab9d`, `b563a5d0`), optional AppKit
semantic hints (`ac92b201`), and complete IMM32 scope arrays (`9d08fc66`). These
consume existing FerroUI properties, formatting, input connections, platform
adapters and native interface generation. `TYPOGRAPHY.md` records every property,
enum feature arm and eleven pre-existing FontWeight values reused by this port.

The independent typography audit compares pinned defaults, declarations and
feature-add order. The FontWeights audit compares every named value, typed
metadata field and parser entry. Runtime evidence remains separate: the fresh
Typography gate passed 27 cases, including two actual Inter-font shaping tests;
full Base passed 4,425 and full XAML passed 617 with 17 ignored. The original
XAML failures and the corrected 130-input corpus count remain in the record.

Main was merged again at `74fa8217`. The complete generated WinUI reference was
regenerated, then stored in five deterministic parts to fit the publication
transport. Its byte concatenation matches the original output. Production
emission remains one file. Six reference/storage tests passed with one ignored.
Commit `e15ce7e8` publishes the regenerated XAML references and all 43 generated
Typography View builder methods after CI identified their stale output.

Full Controls testing revealed that semantic constructor formatting was being
promoted into local snapshot assignments during restore. The first constructor
correction `c43a907c` did not solve that defect. The subsequent fix `17298a56`
restores assigned fields exactly and reinstates missing Bold/Italic/Underline/
Hyperlink defaults through current values. It preserves style precedence and
explicit reset values. The case-conversion fixture now asserts exact input
snapshot identity and effective formatting independently. The final full
Controls gate passed 5,068 tests with no failures or ignored tests.

The Windows checkpoint at `c43a907c` passed native adapter compilation, 52 WinUI,
two exact UTF-16, 23 IMM32 input-scope and six MSFTEDIT tests. The immutable
`native-msftedit-links-gravity.json`, published in `20dddd04`, adds 137 link and
1,577 gravity commands with measured checkout/probe identity and a verified
artifact archive digest. Earlier movement and typing rows remain equal despite
the observed DLL build changing from .33648 to .33438. Link and Gravity require
separate semantic implementations; those observations are not counted as ports.

The same CI checkpoint exposed an IBus generated-signal type inference error
which blocked multiple backend builds, plus stale XAML/View reference output.
These are integration failures to repair, not missing-runtime excuses. The
IBus repair adds explicit generated signal types without changing callback
lifecycle policy. Its actual private-bus and backend gates follow independently.
The complete port remains a draft: candidate/proofing services, unimplemented
API members, external engine contracts and native rendering/OS sessions remain
visible in the source inventory and deviations.

## 2026-10-22 — Merge shared TextLineBounds support

Main `0ce4e90c20d89a03590ccdde555d5daa87e88ee6` brings PR #294's enum,
cap-height metrics, formatter policy, TextBlock property and WinUI template
changes. The merge combines those APIs with this branch's tab-stop properties,
line-spacing copy behavior and typography. Registry accounting includes both the
new enum and the rich value types. The generated Rust-path list also now includes
the previously omitted Typography class.

The authoritative WinUI reference was regenerated into the existing deterministic
parts. Its regeneration test passed, followed by the full fresh theme suite:
301 passed, two ignored. View-builder consistency passed. The pending rich-control
layout implementation will consume this shared API; it does not introduce a
second enum or duplicate the formatter. Native requested-family metrics, line-gap
policy and fallback behavior remain independently reviewed contracts.

## 2026-10-22 — Characterize the actual selection's Gravity

The independent-range observations do not establish the behavior of the live
selection object. A separate system-MSFTEDIT probe now starts from
`ITextDocument::GetSelection`, queries both extended interfaces, and checks their
canonical IUnknown identity before using Gravity. Its matrix declares 250
commands across 18 fresh controls: styled and paragraph boundaries, all five
modes, duplicates, directed movement and replacement, invalid values, null/empty
typing and the untouched empty control. Inserted spans are read independently
so the resulting caret format cannot be mistaken for the inserted format.

The strict parser retains raw HRESULTs, nullable counted strings, before/after
states, direction, interface identity and command counts. All 34 extraction
tests pass, including unchanged historical-record round trips. The new probe
is selected by the existing Windows workflow; native execution and production
selection Gravity remain pending until a new immutable observation is captured.

The merged checkpoint `c1e1a0b4` has now completed its rich-text CI. The Windows
job passed, as did Linux Base (4,433), Controls (5,073), Win32 (329), WinUI (42)
and all three themes (Fluent 210, Simple 213, WinUI 301). XAML passed 616 tests
and failed the generated Rust-path freshness check because its reference lacked
Typography. The authoritative generator supplies that missing row; it is included
with this checkpoint. These CI results precede the new layout, geometry,
namescope and independent-range Gravity implementations.

## 2026-10-22 — Record native selection Gravity and a passing shared checkpoint

Both rich-text CI jobs at `8508220d` passed. Linux passed Base 4,433,
Controls 5,073, Win32 329, WinUI host 42, Fluent 210 (two ignored), Simple
213 (three ignored), WinUI theme 301 (two ignored), and XAML 617 (17 ignored).
All five explicit generated-reference comparisons passed. Windows passed
52 WinUI tests, two exact UTF-16 tests, 23 IMM32 scope tests, seven native
MSFTEDIT probes and 609 selected rich-control tests. These results belong to
that published checkpoint and precede the next implementation changes.

The immutable `native-msftedit-selection-gravity.json` captures the actual
selection through `GetSelection`: 250 commands, 18 fixtures and 18 successful
canonical interface-identity checks. The measured checkout is `038c29ce`,
workflow head `8508220d`, probe blob `7d80afb5`, artifact 12448130293. Its
archive SHA-256 matches GitHub's digest. This run used MSFTEDIT
6.2.26100.33648 on Windows 10.0.26100. All preceding fixture declarations,
counts and six observation families match the earlier link/gravity record
exactly, although that record used DLL build .33438. All 34 extractor tests
pass with the new record included in historical round-trip validation.

This is native behavioral evidence for the next selection implementation;
recording it does not make the pending selection Gravity or external WinUI
engine contracts complete. Older records remain immutable.

## 2026-10-22 — Accept empty browser composition results

The full Browser suite exposed an actual empty-result defect: the DOM adapter
called the legacy `commit("")` path, whose established meaning is cancellation.
Exact clients now update their retained preview through the existing exact-edit
composition API. An accepted empty result replaces that tracked span and calls
`complete`, with the final directed selection already included in the single
undo transaction. Native Escape cancellation retains its original-value test.
Final value and UTF-16 selection validation now also covers LF-only source.

Five regressions cover absent/repeated previews, empty deletion, directed
undo/redo, raw CR/CRLF preservation, invalid values and surrogate endpoints,
interception, focus handoff and read-only changes. Preview range publication
checks the original session and generation after callbacks. The retained tail
format regression now queries the whole nonempty tail rather than a backward
insertion boundary. The initial Browser gate passed 271 and failed two; the
corrected fresh full executable passes 278 with no failures or ignored tests.
This is the portable adapter gate, not a physical browser/IME session.

## 2026-10-22 — Port rich line stacking and consume shared TextLineBounds

Rich block layout now separates allocated line advance from the natural glyph
draw origin, following the pinned `ParagraphNode::ApplyLineStackingStrategy`.
The reproducible comparison compiles the actual C++ and Rust function bodies:
all 960 cases pass with maximum absolute difference 0.0000027791. Nine control
regressions pass for wrapped fragments, first/last precedence, overflow fitting,
empty lines, range geometry and explicit rich-edit spacing.

RichTextBlock owns the existing shared TextLineBounds property. Its captured
mode reaches paragraph, empty-document and linked-overflow formatting, source
invalidation and clipping. Ten control regressions pass, and the existing
rich-document runtime/emitter fixture now covers all four values. The generated
View builder and XAML references include the new owner. Base paragraph-copy
coverage also confirms tabs, spacing and bounds survive copying independently;
the fresh full Base executable passes 4,437 tests.

`LAYOUT.md` records formulas, source provenance and the exact policy boundary of
this checkpoint. Natural lines still use the shared formatter's half-gap and
actual fallback-face metrics; native requested-family metrics and per-run object
bounds are a subsequent implementation. Source formula agreement is not native
font selection, LineServices or pixel conformance.

## 2026-10-22 — Connect exact requested-range input geometry

The shared input client now has an optional requested-range geometry capability.
RichEditBox supplies exact UTF-16 ranges from current shaped fragments, including
insertion and line-break positions, composition previews and complete linked-page
coverage. Stale source, layout, owner or connection state rejects the query.
CoreText transforms every fragment corner to screen coordinates before union;
only an unsupported client uses the existing caret fallback.

The portable gates pass three Base geometry tests, 13 rich-control tests and
eight CoreText host tests; full WinUI host passes 50 and Win32 host 329. The
overflow fixture uses its already attached parent rather than reparenting a
control whose old child collection still owns it. Five Windows-specific native
adapter regressions compile/run only in the separate Windows gate after push.
`INPUT-GEOMETRY.md` documents the capability, ownership checks and limits; this
framework translation does not claim native WinUI engine geometry or candidate
window verification.

## 2026-10-22 — Implement independent range formatting Gravity

Independent RichEditTextRange objects now expose all five native-valued
RangeGravity constants through the ordinary typed registry. Setters canonicalize
Outward to UIBehavior and reject invalid raw values. Shared Rust clones retain
one range object; independent Duplicate-style clones reset the mode. One resolver
supplies collapsed queries, plain insertion, clipboard text and Unlink, with
the range's explicit typing patch overlaid afterward.

The 23 focused Gravity tests pass, including 45 literal native affinity fixtures,
190 commands from ten retention fixtures and 24 invalid values. Reentrant
brush callbacks, exact restoration of temporary editor typing, late clipboard
changes and endpoint publication keep the newer accepted state. Range metadata
stays separate from document undo and grouped rollback.

`RANGE-GRAVITY.md` explains the visible UTF-16/EOP boundary and states what this
commit does not complete: physical TextPointer rebasing and the live selection's
Gravity. The newly measured actual-selection record is independent evidence for
that next consumer. No completion is inferred from enum registration alone.

## 2026-10-22 — Resolve text names through live template scopes

TextElement.FindName delegates to the existing nearest logical NameScope and
returns the registered live object. Ordinary reparenting, explicit scope changes
and shared control/text registrations require no cached lookup state. A durable
metadata-generator override exposes the method. The inherited Name property
continues through the registered direct-property system, with no duplicate
supplementary markup declaration.

Template application attaches the actual TemplateResult scope before callbacks,
preserving an explicit nearer root scope. Deferred XAML uses weak scope-chain
context: a nested resource can outlive its temporary construction scope and
still find a live declaring scope, without retaining the named control tree.
A live incomplete scope keeps its normal local and asynchronous behavior.

The initial actual-XAML gate passed 618 and failed two outer-lookup assertions.
The scope-chain correction preserves both assertions, and all three rich-name
XAML cases now pass, including replacement and declaring-tree release. The final
full XAML suite passes 620 with 17 existing ignores; runtime helpers pass all
202 tests. The final full Controls executable passes 5,136, with loopback access
for its six existing remote tests. Prior Name metadata fixture errors and the
loopback-restricted run are recorded as failed gates rather than passing counts.

## 2026-10-22 — Shared Link fields and measured live selection Gravity

The source tranche adds a common canonical field table to DocumentSnapshot and
consumes it from typed projected Link ranges, paragraph-owned Hyperlink nodes,
history, transformations, RTF and transfer. The existing visible UTF-16 APIs keep
their coordinate contract. The bounded Link engine uses all 137 native
transitions from 59 fixtures; live replay reseeds their observed before ranges
and does not claim native paired-range tracking.

Actual-selection Gravity now has a separate shared-editor consumer, based on the
250-command/18-fixture record with canonical interface identity checks. Its 29
regression definitions cover transient formatting, real typing, composition,
metadata and callback/history ownership. Two null-BSTR calls remain native
transport observations. Synthetic EOP, cached duplicate formatting, automatic
AT_END_OF_LINE and native trailing-paragraph replacement alignment stay partial.

Read-only review found callback-bearing equality/content-pool reads between
snapshot validation and structural mutation. The follow-up checks the model
mutation version at both preflight boundaries, including no-op restoration, and
marks externally interrupted editor groups so stale rollback cannot overwrite a
callback's accepted content. Four focused installation definitions cover the
identified gap. Arbitrary direct-child writes during reused-object structural
setters remain a separate stronger boundary.

The actual standalone Link core passed 23 tests; all newly added framework test
results remain pending the combined central gate at this documentation checkpoint.
No earlier green framework result is attributed to this new source. Details,
source anchors and remaining contracts are in LINK-FIELDS.md, RANGE-GRAVITY.md,
RT-I34/RT-I35 and the appended TESTS.md evidence ledger.

## 2026-10-22 — Verify the published layout and input checkpoint

The dedicated workflow at `86f2cd639836991ca336612dc0073612104e5436`
completed both jobs successfully (run 39852362560). Linux repeats the full
4,437 Base, 5,136 Controls and 620 XAML passes, all three theme suites, both
portable Windows host suites and all five generated-reference checks. Windows
passes 65 WinUI tests, including the five requested-range geometry cases, two
UTF-16 contracts, 23 IMM32 scope tests, seven native MSFTEDIT probes and 664
selected rich-control tests. This closes the Windows execution gap recorded for
that checkpoint without attributing these results to later source changes.

## 2026-10-22 — Extend paragraph TypeText characterization and retain architecture provenance

Prepared and integrated a native-only paragraph TypeText extension: 16 fresh
actual-selection fixtures add 240 commands while retaining the original
250-command matrix unchanged. The complete family now declares 490 commands /
34 fixtures. Existing snapshots inspect actual post-edit UTF-16 spans, primary
and duplicate state, exact strings/HRESULTs, Bold/Italic and paragraph alignment.
No production paragraph replacement policy or historical evidence was changed.

The isolated extractor gate passed 39 tests with zero failures. It now accepts a
bounded 512-command family while retaining exact per-fixture, identity, width and
continuity checks; tests preserve all original 250 rows and 18 interface records.
The main-checkout rerun also passed the same 39-test suite. Windows
execution of the added native fixtures is pending; the earlier published green
checkpoint is not attributed to this later probe source.

The ARM64 collection workflow records verified runner/toolchain architecture,
measured checkout, probe blob, exact commands and exit codes. Its serial log is
canonical extractor input. The separate four-thread process is diagnostic only;
interleaved output and a tolerated diagnostic failure are not a native conformance
pass. New observations will receive a distinct immutable record with verified
archive/artifact provenance. The existing RT-I35 paragraph-policy limitation
remains until those measurements and a subsequent implementation gate resolve it.

## 2026-10-22 — Prepare bounded passive endpoint measurements

Added a separate system-MSFTEDIT `range_tracking` source family: 15 fresh controls,
six calls each, 90 commands. The matrix configures primary ranges and duplicates,
then records external replacements and both mode readbacks for previously
unmeasured endpoint relationships. Identical-text and retained-prefix/suffix
inputs test whether tracking depends on the original operation span rather than
the minimal visible-text difference. No result is predicted or used to change
production behavior before measurement.

The entire earlier oracle is an unchanged prefix, including the 490-selection,
1,577-independent-Gravity and 137-Link source matrices and ARM64 diagnostics.
The additive schema preserves raw failed HRESULTs, exact state continuity,
six-step fixture declarations and a 90-command family budget. The isolated and
real-checkout extractor gates each passed 50 tests, including 11 new protocol
regressions; independent review found no concrete source/schema blocker.

Existing measured JSON remains immutable. New native execution and artifact
capture are pending. Snapshot GetFont/GetPara calls make these records unsuitable
for Stage B untouched/duplicate formatting-cache conclusions. The prepared probe
blob is `64262e70bb48d4241fbe4626d27df85c388284ba`; the source patch SHA-256 is
`2fc8c2e0d1cd99dbf12b6227086fe76d74516656cea42c3ec9233828b3602b51`.

## 2026-10-22 — Validate requested-family metrics, common fields and live-selection Gravity

Completed the coordinated fourteen-target build and final host execution after
integrating family metrics/optional fallback-scale transport, common Link fields,
typed projection, exact RTF/clipboard/filter preservation, live selection Gravity
and model-install preflight. Full Base passed 4,470; full Controls 5,251; full XAML
620 with 17 existing ignored. Runtime helpers passed 202, Browser 278, WinUI/Win32
host 50/329, View 73, HarfBuzz eight, Native/Wayland six/118, Android/iOS host
165/102 and private-session IBus 35. All executed tests passed.

The first complete gates exposed two missing-brush drawing fixtures, four real
history/checkpoint synchronization errors and three fixture setup/expectation
errors. Earlier XAML execution exposed an equal Text binding echo during undo.
Corrections retain the original native/geometry/inheritance assertions and add
exact current-history and binding undo/redo checks. Accepted callback changes
remain authoritative; no guard is weakened to accept an obsolete outer edit.
The final record preserves unsuccessful compile/runtime attempts separately.

The durable framework-family-link-validation.json records all 81 changed source
hashes, the prior published base, exact build/runtime commands and log hashes.
The family arithmetic script freshly compiled the pinned C++ and actual Rust
helper and passed 3,240 cases. The native extractor passed 50 cases. New Windows
execution of the 490-selection and 90-endpoint matrices remains pending; existing
measured JSON is immutable. No full native source-file completion or external
WinUI/LineServices/pixel equivalence is claimed.

## 2026-10-23 — Restore the X11 IBus portal smoke contract

The rich-text conformance workflow at published head `e5a0778a` completed on
Linux, Windows x64 and Windows ARM64. The broader CI run `39865036853` failed
only the X11 input-method smoke step: its independent portal double rejected
`org.freedesktop.DBus.Properties.Set(ContentType)` with `UnknownProperty` and
retired the input context before key handling. The production proxy and the
FreeDesktop unit-test service already use the correct write-only `(uu)` property.

The X11 example's portal service now accepts the same property. Its existing
real-window smoke exercises ordered input-scope precedence, a live scope change,
and return to the legacy content hint after clearing the scope, before checking
consumed and forwarded keys. Validation of this correction is recorded separately
from the successful earlier dedicated workflow and the failed broader run.

## 2026-10-23 — Preserve verified x64 and ARM64 native measurements

Recovered the original official artifacts of conformance run `39865036758` for
workflow head `e5a0778a`, measured merge checkout
`87aa7390668cf0450e18c4f3f7839df5db22ccd1`, probe blob
`64262e70bb48d4241fbe4626d27df85c388284ba`. Both downloaded ZIP byte lengths and
SHA-256 digests match GitHub metadata; every member passes CRC verification.
The new immutable `native-msftedit-e5a0778a-x64.json` and
`native-msftedit-e5a0778a-arm64.json` retain archive and member hashes, extractor
identity, exact commands, source manifests and platform details. Historical
records remain unchanged.

Each strict extraction includes 490 actual-selection Gravity commands across
34 fixtures, 90 passive-range commands across 15 fixtures, 1,577 independent
Gravity commands and 137 Link operations, alongside the earlier navigation
families. Every extracted behavioral observation agrees between x64 MSFTEDIT
6.2.26100.33648 and ARM64 MSFTEDIT 6.2.26100.9278. ARM64 canonical serial and
separate four-thread diagnostic runs both exited zero; only serial observations
are used as evidence. The x64 artifact extraction also equals the independently
retrieved decoded workflow log. These remain system-engine observations; they do
not establish packaged WinUI runtime parity or unobserved formatting caches.

That dedicated workflow passed Linux Base 4,470, Controls 5,251, XAML 620 with
17 existing ignored, WinUI/Win32 host 50/329 and all three theme suites plus five
generated-reference checks. Windows passed 65 WinUI, two exact-HSTRING, 23 IMM32,
eight system probes and 779 selected rich Controls tests. Those results belong
to the measured checkpoint, before this continuation's implementation changes.

## 2026-10-23 — Port measured TypeText paragraph replacement

The verified extension supplies sixteen new native fixtures and 240 commands.
Typed replacement now retains a selected final structural paragraph mark when
the range ends at the next paragraph's start. A replacement crossing an earlier
mark takes the trailing paragraph's alignment, including inherited None, for
every inserted paragraph. Soft inline breaks remain distinct. Other paragraph
properties retain their existing, separately documented policy.

Planning occurs before length limits and overtype consumption; original directed
selection remains in the undo snapshot. The literal replay checks all represented
command fields, 64 paragraph-alignment queries and eleven inserted-character
format observations. Twelve framework regressions cover UTF-16 boundaries,
accepted-input limits, history/group rollback, retained inheritance, publication,
real RichEditBox integration and the selected-mark/overtype combination. The last
combination is a framework composition check, not a directly measured native
fixture. Independent source review compared all sixteen fixtures and 240 literal
rows against the official x64 record without a mismatch.

The 50 extractor tests pass and pinned inventory regeneration passes. Rust test
execution is still pending here: initial Controls and Base test code generation
were killed by SIGKILL under concurrent workspace memory pressure. The failed
builds are not reported as conformance passes; metadata checking and later
execution are recorded at their actual checkpoint.

## 2026-10-23 — Port paragraph/page hosted-object measurement

RichTextBlock now measures embedded controls per paragraph fragment using that
page's inner width, remaining height and source-derived collapsed margins. The
editor presenter keeps its established additive paragraph-spacing policy. Object
runs freeze page-local size/baseline values; retained continuations rebind only
objects and preserve shaped text/bidi ownership. Last successful host metrics
live on InlineUIContainer across source regeneration and reset on child changes.
Geometry-only reads never measure or move a child.

Review exposed stale-layout arrangement and detach-callback ownership transfers.
Cached layout identity, generation and actual visual-parent checks now guard
measure/synchronization/arrange and inline child transitions, preserving newer
application changes. Twenty-one Controls and four Base regression definitions
cover constraints, continuations, cache lifetime, immutable geometry, original
editor spacing and callbacks. These are test definitions until executed.

The first metadata-only check found two test-fixture API mismatches (FerroList
count and LineBreak length), both corrected. A separate review tightened native
alignment assertions so Center/Justify can no longer masquerade as Left. Final
metadata/runtime validation follows main integration. Native paragraph detach/
backtracking scheduling, TextDpi and physical rendering remain explicit limits.
