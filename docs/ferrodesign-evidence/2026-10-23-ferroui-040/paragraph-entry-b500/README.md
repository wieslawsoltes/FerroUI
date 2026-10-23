# Paragraph entry diagnosis from the failed b500 shard

The original **Software2D text-effects job failed** on the second inline
paragraph session: the unchanged caret assertion expected UTF-16 index `3` and
observed `1`. This folder retains that failure and a reviewed setup correction.
**The correction has not yet executed in a browser.** It does not establish a
full-suite, font-rendering, performance, or visual-parity pass.

The exact source is
[`b5008093b3c201b79131810f044e101058b1b165`](https://github.com/wieslawsoltes/FerroUI/commit/b5008093b3c201b79131810f044e101058b1b165),
tree `a20788573f2ffcff7da17722770fdd5e864d001c`, from
[run 39977448194, job 120501196550](https://github.com/wieslawsoltes/FerroUI/actions/runs/39977448194/job/120501196550).
The process exited `1`; the CI proof and original report remain failed. Seven
other selected scenarios completed successfully in the same isolated shard.

## Observed input and native state

The original
[split-input JSON](Software2D-native-paragraph-split-input.json) contains both
sessions, complete before/after native snapshots, typography, and passive
capture/bubble key events. The second session follows **Escape cancellation and
the same canvas-click/Enter entry helper**; it is not a Tab-reentry test.

| Observation | First entry | Second entry |
|---|---|---|
| Native inline layout before navigation | Original Unicode paragraph text; caret `0` | LF plus the empty-line placeholder; caret `1` |
| Paragraph style count before navigation | `3` | `2` |
| Requested navigation | Ctrl+Home, Right, Right | Ctrl+Home, Right, Right |
| Observed final caret | `3` | `1` |
| Actual text after navigation | `A🦀 first\nשלום second\nLast` | Exactly `\n` |
| Document revision before/after navigation | `14` / `14` | `17` / `17` |

Both sets of keys were trusted, reached the FerroUI input element, and were
handled for Home/Right. No observer events were dropped. The second layout is
already reduced to the newline before those navigation keys; its revision does
not change while the keys run.

The recorded native press that precedes the second session has `clickCount: 2`
at **9,375 ms**. The preceding press was `clickCount: 1` at **8,916 ms**, a
**459 ms** interval at the identical delivered screen point
`(115.34091186523438, 316.9090881347656)`. These are native monotonic timestamps;
they are not compared directly with the passive DOM event clock.

## Source-backed interpretation

The frozen canvas implementation emits `EditText` for a text-layer press whose
native click count is two. The shell then focuses the inline TextBox and selects
all text. The former test helper checked only the ancestor's
`CanvasHost.focusWithin`, then unconditionally sent Enter. That ancestor flag is
also true while the child TextBox is focused. In that state plain Enter is
multiline text input, replacing the selected text with LF before the caret
check. The native press, layout, and source jointly support this setup failure;
the original entry did not capture the click's DOM timing or a state between
the click and Enter.

[analysis.json](analysis.json) records the exact source hashes and reviewed line
ranges, the observed states and key events, and this explicitly source-inferred
mechanism.

## Reviewed correction and unchanged assertions

[correction.patch](correction.patch) changes only the entry helper against the
frozen source. After the same real canvas click it reads the actual native
editing state. It sends Enter when editing remains closed; an already-open
session must instead have the expected layer, focused TextBox, and native
double-click count. Before navigation it now asserts the complete original
text, exact document and styles, and unchanged history. It records bounded
passive entry evidence and preserves the original action error if capture fails.

Every existing caret, paragraph split, cancellation, commit, rich-style,
serialization, and exact undo/redo expectation remains. No production code,
input-recognition threshold, wait deadline, or assertion tolerance is changed.
The reviewed [helper snapshot](corrected-helper.mjs.txt) is separately hashed so
later font coverage in the same module cannot be mistaken for this checkpoint.

## Retained evidence and integrity

- [job-metadata.json](job-metadata.json) and
  [artifact-metadata.json](artifact-metadata.json) retain the parsed GitHub API
  records; [job.log.gz](job.log.gz) losslessly preserves the complete decoded job
  log, including its original BOM.
- [report.json.gz](report.json.gz) is the lossless original complete report;
  [proof.json](proof.json) retains the original process failure and exact
  source/tree/harness/distribution identity.
- The original [failure document](Software2D-paragraphs-failure.ferrodesign) and
  [failure screenshot](Software2D-paragraphs-failure.png) are unchanged.
- [archive.json](archive.json) records the full 37,260,313-byte ZIP's verified
  GitHub SHA-256 and all 65 central-directory members. Only five selected
  members were decompressed and CRC-checked. Large trace archives are not copied
  into git. Original and stored hashes for every retained payload are in
  [analysis.json](analysis.json).

The separate b500 benchmark's forced shutdown remains a failure in its own
evidence; this paragraph diagnosis makes no claim about that outcome.
