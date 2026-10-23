# Visual-line elements and native formatting

This report tracks the rendering extension contracts from AvaloniaEdit revision
`7277e36366d754219d7ede16b6613786b32caf4c`. The derived sources retain the
copyright and MIT license in `src/FerroUI.Edit/NOTICE.md`. The implementation
uses FerroUI's existing text source, text formatter, text layout, drawable runs,
brushes, routed events, cursor service and URI launcher.

## Source map

Upstream paths are relative to `src/AvaloniaEdit/`; Rust paths are relative to
`src/FerroUI.Edit/`.

| Upstream source | Rust source and contract | Adaptation |
| --- | --- | --- |
| `Rendering/VisualLineElement.cs` | `visual_line_element.rs`: `VisualLineElement`, `VisualLineElementData` | Object-safe element trait with independently owned lengths, UTF-16 offsets and mutable construction properties. Checked splitting, caret mapping, whitespace and pointer hooks remain element responsibilities. |
| `Rendering/VisualLineElementTextRunProperties.cs` | `visual_line_element_text_run_properties.rs`: `VisualLineElementTextRunProperties` | Copies native property values and collection handles into independently owned collections. Brush setters and run snapshots use native immutable brushes; native decoration objects retain their shared object semantics. |
| `Rendering/VisualLineText.cs` | `visual_line_text.rs`: `VisualLineText` | Splittable one-to-one UTF-16 text; native `TextCharacters`; document-aware word and grapheme movement across styling boundaries. |
| `Rendering/ITextRunConstructionContext.cs` | `i_text_run_construction_context.rs`: `ITextRunConstructionContext`, `VisualLineBuildOptions` | Stable document snapshot and optional weak native view. Text slices reuse a cached memory region. Mutable builder access is passed separately to transformers. |
| `Rendering/ITextViewConnect.cs` | `i_text_view_connect.rs`: `ITextViewConnect` | Explicit add/remove hooks for native view ownership. |
| `Rendering/VisualLineElementGenerator.cs` | `visual_line_element_generator.rs`: `VisualLineElementGenerator`, `VisualLineElementGeneratorRef` | Ordered generation with a scoped context argument. Reference wrappers compare `Rc` identity for native observable collections. |
| `Rendering/VisualLineConstructionStartEventArgs.cs` | `visual_line_construction_start_event_args.rs`: `VisualLineConstructionStartEventArgs` | First logical line in the viewport construction batch. The host raises the event before snapshotting extension collections. |
| `Rendering/VisualLine.cs` | `visual_line.rs`: `VisualLineBuilder`, `VisualLine` | Construction and immutable retained state are separate Rust types. The builder generates and transforms elements; the retained line provides document/visual mappings and native layout creation. Viewport geometry remains a host responsibility. |
| `Rendering/VisualLineTextSource.cs` | `visual_line.rs`: `VisualLineTextSource` | Native `ITextSource` adapter, observable extension errors, preceding-text validation and native EOL runs. Kept with `VisualLine` because it accesses the same immutable state. |
| `Rendering/IVisualLineTransformer.cs` | `visual_line_transformer.rs`: `IVisualLineTransformer`, `VisualLineTransformerRef` | Trait receives scoped mutable access to the builder before shaping. |
| `Rendering/ColorizingTransformer.cs` | `visual_line_transformer.rs`: `ColorizingTransformer` | Callback-based Rust base behavior, checked recursive use and connection hooks. |
| `Rendering/DocumentColorizingTransformer.cs` | `visual_line_transformer.rs`: `DocumentColorizingTransformer`, `DocumentLineColorizer` | Absolute document offsets restricted to the current logical line; visits visible continuations after generated multiline elements. |
| `Rendering/FormattedTextElement.cs` | `formatted_text_element.rs`: `FormattedTextElement`, `FormattedTextRun`, `FormattedTextContent` | Native `TextLayout`, `TextLine`, or custom `DrawableTextRun` content with real size, baseline and draw behavior. |
| `Rendering/SingleCharacterElementGenerator.cs` | `single_character_element_generator.rs`: `SingleCharacterElementGenerator`, `IBuiltinElementGenerator` | Native space glyphs and control-character labels; control label display columns affect following tab stops. |
| `Rendering/TabElementGenerator.cs` | `tab_element_generator.rs`: `TabElementGenerator`, `TabTextElement` | Required generator on long lines; tabs continue from the visual line's display columns. A wide configured marker still reserves one space width. |
| `Rendering/LinkElementGenerator.cs` | `link_element_generator.rs`: `LinkElementGenerator`, `MailLinkElementGenerator`, `LinkMatch` | Reuses the core bounded regex/search service. Custom matching strategies, URI resolution and element factories are supported. |
| `Rendering/VisualLineLinkText.cs` | `visual_line_link_text.rs`: `VisualLineLinkText`, `OpenUriRoutedEventArgs`, `LinkTextStyle` | Splitting preserves URI and click policy. Native routed URI event, modifier checks, hand cursor and top-level launcher. |
| `Folding/FoldingElementGenerator.cs` | `folding_element_generator.rs`: `FoldingElementGenerator`, `FoldingLineElement` | Merges overlapping folded ranges into an atomic native run, preserves label display columns, and unfolds on pointer activation. |
| Editor IME projection | `preedit_element_generator.rs`: `PreeditElementGenerator`, `PreeditTextElement` | A native element projects composition over a document range without editing it. Virtual-space padding remains separate from the underlined composition. |

`VisualLineError` replaces construction and extension exceptions with checked
results. Combining the small transformer types, text source, paired formatted
element/run types and paired web/mail generators is intentional. This table does
not claim that every member of upstream `TextView`, `VisualLine`, paragraph
properties, background layers or embedded-control hosting is supplied by these
files alone.

## Construction and ownership

All document offsets, document lengths and formatter columns use UTF-16 code
units. Pixel sizes stay in FerroUI device-independent units. An element may
consume zero document units while occupying one or more visual columns, or
represent a long document range using one atomic visual column.

The builder starts every generator in registration order and guarantees matching
finish calls on normal completion and error exits. Optional generators are
skipped above the source's 3,000-unit line cutoff; required generators such as
tabs, folds and composition continue to run. At each document position, all
zero-document-length elements are accepted in registration order, followed by
the first element that consumes document text. After a zero-length result or no
result, interest queries advance by one document unit to prevent repeated
construction at the same offset.

Elements may cross logical line boundaries. The builder extends its last line
to the containing logical line and rejects an element ending inside a CRLF
delimiter. It validates the generated document length against the complete
represented span. Transformations may replace elements only when their total
document length is preserved. Splits must return both the requested visual
boundary and the original document length. A partial atomic element is left
intact by a styling range.

Finishing consumes the mutable builder and returns `Rc<VisualLine>`. The line
retains its document snapshot and elements, and the native text source retains
the line. A view reference in the construction options is weak, preventing a
view/cache/source cycle. The public element data remains immutable after
finishing; its native reference-valued properties retain the framework's usual
reference semantics.

The native `ITextSource` method cannot return a Rust error. Its adapter records
the first invalid run and ends formatting safely; `VisualLine::create_text_layout`
then returns that recorded error. Callers using `text_source()` directly must
check `VisualLineTextSource::error()` after formatting. Zero or negative run
length, a run beyond its element, non-finite or negative drawable dimensions,
oversized preceding text and nonadvancing custom caret positions are reported
explicitly. No extension failure is silently presented as a successfully
formatted truncated line.

## Live TextArea integration

`TextArea` now owns this pipeline in `text_area_visual_lines.rs`. The previous
`DisplayLine` projection and overlay whitespace drawing have been removed.
A retained cache entry holds the immutable `VisualLine` and its native
`TextLayout`; all arranged geometry uses that pair. An unarranged view computes
only UTF-16 tab columns without building a second display-text model.

The public host exposes `element_generators`, `line_transformers`,
`visual_line_construction_starting`, `visual_lines`, `get_visual_line`,
`get_or_construct_visual_line` and `redraw`. Collections are shared native
`FerroList` values. Reordering invalidates construction while duplicated
identities connect only once; the final removal disconnects the entry.

The stable construction sequence is:

```rust,ignore
let mut builder = VisualLineBuilder::generate(
    document.create_snapshot(), first_line_number, global_properties.clone(),
    &generators, VisualLineBuildOptions {
        text_view: Some(area.downgrade()),
        wide_space_width: measured_space_width,
        end_of_line: marker_options,
    },
)?;
// Resolve syntax styles with NativeHighlightingStyle and apply them to their
// absolute UTF-16 ranges before the application's additional transformers.
builder.apply_text_run_properties(start, end, resolved_properties.as_ref())?;
builder.run_transformers(&transformers)?;
let visual_line = builder.finish()?;
let layout = visual_line.create_text_layout(
    paragraph_properties, available_width, f64::INFINITY, 0, None, None,
)?;
```

The cached entry retains both `Rc<VisualLine>` and `TextLayout`. Native paragraph
properties should use automatic height, or a deliberate minimum-height policy
that also handles every wrapped row. A fixed height below a drawable's natural
height causes native text metrics to overlap subsequent rows.

`get_visual_column`/`visual_offset` map a document offset relative to the first
logical line into formatter coordinates. `get_relative_offset` provides exact
element mapping; `document_offset` rounds a hit inside an atomic multicolumn
element to the nearest document boundary. `is_document_offset_visible` excludes
strict fold interiors. `source_lines` lists the logical lines with visible
element starts, while `first_document_line` and `last_document_line` delimit the
complete represented document span, including hidden rows.

The host advances after construction updates its height tree and reconciles hidden
rows in its height tree. It must account for every multiline element, including
composition replacements and custom generators, rather than deriving all
collapsed rows from the folding manager alone. Caret, selection, IME cursor,
popup, word wrapping and hit-test geometry use these retained mappings.

The host exposes shared native observable collections of
`VisualLineElementGeneratorRef` and `VisualLineTransformerRef`, connects and
disconnects entries through `ITextViewConnect`, and invalidates cached layouts
when either collection changes. It snapshots both collections once per viewport
construction batch after raising `VisualLineConstructionStarting`. Fully cached
paints do not raise another construction event or rebuild the built-in regexes.
No collection or cache borrow survives an application callback.

Removal while a batch is inside generator/transformer callbacks marks it stale
but defers disconnect hooks until all matching finish callbacks have returned.
The host checks document identity, version and its own layout revision before
publishing a line, then checks again after hidden cached elements are released.
That second check handles custom element destructors which edit the document.
Viewport eviction also releases values outside the cache borrow. Recursive
construction returns an explicit error; collection hooks can change membership
and the synchronization loop reconciles those changes before finishing.

Pointer processing finds the corresponding retained element and calls its
cursor/press/release hooks before starting editor selection. The native
`OpenUri` event is registered against `TextArea` and can be exposed by a host
delegate to `VisualLineLinkText::open_uri_event()`. Applications can handle it
to supply custom navigation; otherwise the native top-level URI launcher runs.

Preedit construction uses the current selection's absolute start and end plus
its starting virtual-space padding. `composition_range()` returns only the
underlined text, excluding padding. The existing IME cursor is a UTF-16 offset
within that range. A fold generator backed by a live manager verifies that its
document snapshot belongs to the same document and current version. Its view
connection retains a folding subscription with a weak view reference.

EOL markers are outside the element collection. CRLF, CR and LF have independent
glyphs; an end-of-file marker has an empty glyph. The text source reserves one
formatter position when enabled, while `visual_length_with_end_of_line_marker`
adds an exposed column only when a real line delimiter follows. They consume no
document content. When the host uses these native runs, it removes its previous
overlay EOL painting to avoid double rendering.

## Embedded controls and remaining differences

Custom native drawable content is supported directly. A native embedded
`Control` requires an additional host lifecycle and is not established merely
by accepting `DrawableTextRun`. FerroUI already provides
`documents::EmbeddedControlRun`, which reserves the control's measured desired
size and baseline. The editor must parent, measure and arrange those controls
outside render, retain identity across line rebuilds, dispose subscriptions and
parents on eviction, move focus before removal, and route child input without
also editing document text. `InlineUIContainer` cannot currently install an
external editor host because its collection-host setters are internal to the
Controls crate.

The concrete hosting review also requires arranging controls from final native
physical run order after bidi and wrapping, invalidating layout when desired
size/baseline changes, repositioning on scroll, clipping the content layer to
exclude the gutter, and avoiding template application while rendering. These
are tracked host integration requirements, not behavior claimed by the
standalone builder tests.

Other explicit adaptations and boundaries:

* Public generators and transformers are ordinary Rust traits/`Rc` objects.
  Their callbacks are not serializable XAML objects; native controls and routed
  events still use FerroUI's type and property system.
* `FormattedTextElement` accepts native `TextLayout`, `TextLine`, and custom
  drawable content. A direct `FormattedText` overload is not exposed because
  that native object's drawing method is internal to the Base crate.
* Control-character boxes currently use the existing native text drawing sink's
  rectangle operation. The upstream 2.5-unit corner radius needs a rounded
  rectangle operation in that sink to reproduce its exact appearance.
* `LinkElementGenerator` uses the existing bounded core regex dialect. Patterns
  outside that dialect and exhausted match budgets return explicit errors.
  This is unrelated to TextMate's separate native/browser Oniguruma backends.
* Native `Uri` is narrower than `System.Uri`; web links additionally validate
  nonempty authority and percent escapes. Exhaustive URI normalization and
  canonicalization equivalence is not claimed.
* The editor options expose spaces, tabs, EOL visibility, indentation, control
  boxes, web/mail links and the control-modifier click policy. Custom generator
  fields retain their glyph, brush and matching settings. Built-in composition
  has first priority, followed by application generators, folding, tabs, special
  characters and optional web/mail generators.
* Native paragraph formatting uses automatic row height so styled and drawable
  runs can expand a row. Geometry tests target actual measured rows. Exact
  upstream drawable line-gap and pixel appearance need reference comparisons.
* Arbitrary custom multiline spans are discovered when their owning line is
  constructed. Known folds and active multiline composition preseed collapse
  intervals even before painting; generators do not provide a global offscreen
  span index.

## Validation checkpoint

The live host checkpoint passes **126 native control tests**. This includes the
standalone element/property tests and thirteen new live-host cases, plus the
hidden-EOF height regression. Four obsolete `DisplayLine` model tests were
removed with that model; UTF-16 tabs, folds and composition now have native host
coverage. The selected-leading-indentation snippet undo regression also passes
through the native controller.

Live cases cover custom multiline generation, suffix hit testing and retained
owners, event-time extension registration, native font-height transformation,
no-construction cached paints, identity-based connection hooks, mutation and
recursive construction, deferred disconnection, reentrant extension destruction,
hidden-row element destruction, prepaint multiline IME cursor relocation,
atomic folded keyboard movement, overlapping offscreen fold unions, and actual
routed pointer activation of built-in hyperlinks/folding placeholders.

The cases exercise generator ordering and progress, finish-on-error, invalid
interest, long-line required generators, CRLF boundaries, styling splits and
document-length conservation, atomic styling, visible continuation coloring,
recursive transformer recovery, invalid run/metric/preceding-text/caret
diagnostics, custom drawable metrics/drawing/wrapping, fixed-width tab markers,
control-character display columns, grapheme/word movement, merged folds and
version checks, IME padding/composition isolation, EOL markers, Unicode/custom
regex links, link splitting, and native URI event routing.

The shared native gate is:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
cargo test --offline --no-default-features \
  -p ferroui-edit-core -p ferroui-edit -p editor-demo --lib
```

Native unit tests establish only the tested behavior. Browser layout, platform
URI launching, arbitrary custom formatter runs and embedded native-control
hosting require their respective integration evidence. The linked browser gate
is tracked independently in [editor-browser.md](editor-browser.md).
