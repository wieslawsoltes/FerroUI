# Native paragraph editing and text artwork

FerroDesign uses the public FerroUI `TextLayout`, `TextBox`, `TextPresenter`, shaped `GlyphRun`, `Geometry` and drawing-sink APIs. The application renderer does not draw through Skia or construct a second browser text editor. The editable document remains UTF-8; native caret, selection and style spans use UTF-16.

## Paragraph authoring

The character-style dialog opens a native paragraph panel for the selected hard paragraphs. Alignment, LTR/RTL/Auto direction, line-height multiplier, letter spacing, space before/after and first-line indent can be set or inherited independently. CRLF, Unicode line/paragraph separators and the final empty paragraph retain exact ranges. Shared `TextParagraphStyles` drive canvas text, native inline editing, caret navigation, selection geometry and the rich-text preview. Both compiled native themes bind the paragraph collection to their TextPresenter.

`Node::replace_text_range_preserving_runs` accepts the exact old UTF-8 replacement and new insertion end. It validates boundaries and unchanged text before mutation. `TextChangingEventArgs::text_change` reports the original text and actual UTF-16 replacement from native typing, paste, deletion and clear operations. Repeated identical words and paragraphs therefore retain the right style ownership. Whole-string programmatic assignments retain their checked diff fallback. Removing a whole paragraph keeps the surviving suffix paragraph style; removing only a separator joins the surviving prefix paragraph.

## Input methods and undo

IME preedit is a temporary projection. Its original selection and actual insertion location determine provisional style ranges; lookup advances through ordered spans instead of rescanning the entire collection for each paragraph. Empty preedit remains pending until explicitly cleared. On cancellation or before a following committed TextInput, the control restores the confirmed text and selection while composition ownership is still visible to observers. Generation checks prevent a synchronous application replacement from being overwritten by an obsolete composition snapshot.

The first nonempty clause is published only after the native control has removed the selected text and settled its caret. This ordering keeps ordinary TextPresenter text/caret synchronization from erasing that first clause. Updates received during the synchronous selection preparation are coalesced: the newest clause and cursor share one provisional deletion. A second platform update is not required to make the first clause visible, including styled multiline Unicode input.

A host text or selection change during preparation supersedes the old composition. Further clauses from that composition are ignored until its explicit clear. Clearing preserves a host text replacement; when only the selection changed, it restores the confirmed text while preserving the host's chosen indices. If a new clause arrives during confirmed-text restoration, it waits for restoration to finish and then starts with its own confirmed snapshot. Detaching or replacing the presenter invalidates older publication work.

Both provisional selection removal and cancellation use internal synchronization. Neither creates a native undo checkpoint nor clears an existing redo branch. Only committed TextInput enters edit history; canceled text cannot reappear after a later undo.

Native undo checkpoints retain character styles, paragraph styles and an optional application-owned immutable payload through `TextBox::text_undo_state`. FerroDesign stores only text and its two range collections in that payload. Undo/redo publishes the snapshot before TextChanging, so deleted rich styles are restored exactly. This uses the existing native history limit and edit grouping; it adds no second history cache. Completing inline editing still commits one document transaction. Each new inline session clears the old native history.

## Scoped typing work

Inline editing begins a node transaction. For documents without auto layout, each confirmed change refreshes only the text node and its ancestor dependencies. A cached layout count selects the existing full arrangement path for auto-layout documents, including imported documents whose stored geometry has not yet been arranged. This preserves their first-keystroke layout behavior and exact cancellation.

The canvas suppresses the text layer that the native TextBox is editing. Incremental preparation can skip exactly that valid, visible, non-mask Text leaf while updating its prepared ancestors; any other missing ID retains the conservative full fallback. A changed suppressed root needs no scene-node preparation or backdrop invalidation. Ending inline editing changes scene continuity and restores the canvas text through a full prepare. Ten-thousand-node regressions check scoped work and exact history; they are not a wall-clock performance measurement.

## FerroUI XAML artwork export

Text XAML export records the actual native layout through `ITextDrawingSink`. Each already-shaped glyph becomes its own public path using `GlyphRun::try_build_geometry` and `Geometry::try_get_path_data`. Advance/offset order, fallback face, paragraph wrapping, bidi, baseline placement and decoration pens come from that layout. Individual glyph draws are retained because combining overlapping translucent glyphs into one path changes alpha. Full affine transforms and the layer opacity apply once around the complete stack.

The initial supported subset uses solid text fills and rich solid colors. Visible gradient/image text paints, layer strokes, text masks, non-normal blends, effects or incomplete glyph outlines are rejected before any file is downloaded. A failed layer also rejects a mixed-selection export atomically. The editable text/ranges are unchanged. PNG uses the full native renderer; JSON retains editable document data.

Ordinary XAML ancestors retain their complete affine transform and composite opacity around their complete descendants. Single solid or gradient Frame backgrounds are supported. Unsupported ancestor masks, effects, non-normal blends, image backgrounds, derived Boolean containers, and generic container strokes/stacked backgrounds/rounded clipping are rejected before a partial result can escape. The specialized smoothed-corner and profiled-stroke paths retain their documented subsets.

Outline export is bounded per text layer to 50,000 glyphs, 100,000 drawing operations and 500,000 path segments. Those limits apply to export work, not interactive canvas rendering. Editable SVG retains hard-paragraph metadata but reports its wrapping/baseline/justification fidelity boundary.

## Verification boundary

Source includes exact-range/core history tests, native paragraph/IME/history tests, native glyph-vs-path and runtime-XAML pixel comparisons, and browser paragraph/Unicode history coverage. [VALIDATION.md](VALIDATION.md) records executed runs separately. Authored tests and successful parsing do not imply that the current source has passed the native or browser gate.
