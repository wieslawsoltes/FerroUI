# Matching and inverse selection

The native **Select same…** inspector button and **Select same properties…** layer-menu/quick-action entries expose property-based selection. These commands operate on the current page and leave the document, revision, renderer change token, history and redo unchanged. Ordinary canvas picking and editing gestures have their own refresh behavior; this contract describes the matching command itself.

The core implementation is [selection/matching.rs](../crates/ferrodesign-core/src/selection/matching.rs); the native FerroUI dialog is [shell/selection.rs](../crates/ferrodesign-app/src/shell/selection.rs). Figma's [Select layers and objects](https://help.figma.com/hc/en-us/articles/360040449873-Select-layers-and-objects) documents the reference family of same-property and inverse commands. The comparison rules below specify this editor's actual behavior rather than asserting complete interchange with another product.

## Comparison rules

| Command | Required equality | Deliberately independent values |
| --- | --- | --- |
| Same appearance | Layer kind, ordered fill/stroke/effect stacks, layer opacity, blend mode, resolved four corner radii; for text, the text-property signature below | Position, dimensions, path geometry, names, style IDs, component source, layout and constraint metadata |
| Same fill | Complete ordered layer fill stack, including paint kind, paint parameters, stops, crop/source, opacity and visibility | Layer kind, geometry and style binding |
| Same stroke | Complete ordered stroke stack, including paint, width, alignment, cap, join, dashes, opacity and visibility | Layer kind, geometry and style binding |
| Same effect | Complete ordered supported effect stack, including kind, color/alpha, radius, offset, spread and visibility | Layer kind, geometry and style binding |
| Same text properties | Resolved character-style sequence; line height, letter spacing, horizontal/vertical alignment, auto-resize mode, direction and normalized OpenType feature tags | Actual text, UTF-8 byte offsets and lengths of equal styled spans |
| Same font | Set of actually used `(family, weight, italic)` faces in a text layer | Text content, face order, repetition, font size, color, decoration and paragraph layout |
| Same instance | Exact component source ID on an instance | Instance name, placement and local overrides; another variant has another source ID |

Paint stacks compare their stored supported values. An invisible paint or effect still participates in stack equality. Two empty stacks match. A style link does not make unequal effective values match, and independent layers with equal values can match. Fill matching compares layer fills, not a vector network's independent region overrides; appearance likewise does not include every possible document property.

Corner radii resolve the uniform radius to four values before comparison. Numeric hashing treats negative and positive zero equally, consistent with value equality. The portable JSON format cannot encode NaN; direct core callers must supply finite numeric data, because validation does not cover every numeric text or corner field. Image paints compare the complete source URI, scale mode and crop, without decoding the image or comparing raster pixels. Equivalent images encoded differently therefore need not match.

## Text resolution

Character signatures borrow font-family strings and retain only size, weight, italic, decoration and optional run color. Imported overlapping runs use the renderer's stable first-starting-run precedence. Invalid or empty ranges cannot contribute a style. Gaps resolve to the layer defaults. Adjacent equal resolved styles merge, so two equivalent style sequences match even when their content has different Unicode byte lengths or unnecessary run boundaries.

The font command extracts the set of faces from that resolved sequence. A layer whose entire text is overridden does not add an unused base font to the set; an empty text layer uses its base face. Text-properties matching sorts feature tags, preserving their values, and compares the ordered resolved style sequence. It does not compare the number of characters assigned to each style, independent paragraph directions, text shaping output or rendered glyph pixels.

## Page, hierarchy and selection behavior

Every source ID must still exist on the active page and be effectively visible and unlocked. Unsupported seeds, such as a rectangle for Same font, are ignored. Stale IDs, cross-page IDs, hidden layers and locked layers cannot broaden a query. When no eligible source signature remains, the selection command is a no-op.

Multiple selected values form a union: selecting a blue-filled and a red-filled layer and invoking Same fill selects eligible layers matching either complete fill stack. Duplicate source IDs and duplicate signatures are deduplicated. The result is a front-to-back depth-first walk in the same ordering used by the Layers panel. Hidden or locked ancestors prune their entire subtree, and page traversal checks depth and visited IDs defensively.

An active core transaction prevents the query from producing a replacement selection. The shell first validates pending inspector drafts and resolves active editing state. A rejected inspector value keeps its validation and selection instead of being silently discarded. Successful commands close the native palette and refresh only selection-related UI and adorners.

**Select inverse**, available as `Ctrl`/`Cmd+Shift+A` when the canvas has focus, selects the other eligible top-level objects. If an eligible visible, unlocked descendant is selected, its page-level ancestor is excluded too. Selected IDs that are stale, cross-page, hidden or locked do not exclude a root; a locked descendant selected through the inspection menu can therefore have its unlocked parent selected by inversion. Empty selection produces all eligible page roots. Inversion does not recursively select every descendant, and text fields retain their own keyboard behavior.

## Work and allocation boundaries

`Editor::matching_selection(SelectionMatch)` is a read-only query. `select_matching` installs a changed result; `invert_selection` implements the root-level complement. Matching hashes borrowed keys with a fresh randomized hasher, uses hash buckets only to narrow candidates, and always verifies full property equality before accepting a match. Hash collisions cannot create false matches.

The active page is traversed once, rather than scanned separately for every selected source. Paint, image and document payloads are not cloned. Signature preparation still hashes the bytes in image URIs and other compared data. Rich text allocates lightweight resolved-style descriptors, sorts runs/features and constructs a font set. Hash collisions and multiple distinct signatures sharing a hash can require additional full comparisons. These are work-saving mechanisms, not a constant-time claim or a measured frame-rate guarantee.

## Verification

[Core tests](../crates/ferrodesign-core/src/selection/matching/tests.rs) cover stack equality, mixed-source unions, hidden/locked ancestors, stale and cross-page seeds, exact component sources, Unicode and overlapping text runs, canonical typography, inverse selection, preserved history/redo/revision and a 10,000-layer multi-source fixture. The large fixture verifies results and invariants rather than enforcing a machine-dependent time threshold.

[Browser regressions](../tests/matching-browser.mjs) use actual native buttons and canvas input for matching fills/strokes/effects, visibility and locks, mixed selection, inverse selection and Unicode font/text properties. Fixtures are imported only to establish authored data; matching occurs through the running FerroUI application. [VALIDATION.md](VALIDATION.md) identifies completed source-specific runs and distinguishes targeted checks from complete browser validation.
