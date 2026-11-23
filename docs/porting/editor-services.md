# Editor services port

This document records the framework-independent editing services in
`src/FerroUI.Edit.Core`. The reference is the pinned AvaloniaEdit checkout listed
in [editor-textmate.md](editor-textmate.md). Public positions count
UTF-16 code units. Logical document lines are one-based. Rust errors replace
exceptions on user input. See the crate `NOTICE.md` for attribution.

## Source mapping

Paths on the left are relative to upstream `src/AvaloniaEdit`; paths on the right
are relative to `src/FerroUI.Edit.Core`. A row identifies a behavioral port or
adaptation; the existence of the row does not imply that every upstream member
has been implemented. Presentation members belong to `FerroUI.Edit`.

| Upstream source | Rust source | Implemented behavior |
|---|---|---|
| `Editing/SelectionSegment.cs`, `Selection.cs`, `SimpleSelection.cs`, `EmptySelection.cs` | `editing/selection.rs` | Directed selections, coalescing, multicaret normalization, document change mapping |
| `Editing/CaretNavigationCommandHandler.cs`, `Document/TextUtilities.cs` | `editing/caret_navigation.rs` | Grapheme, word, line and document movement; surrogate and CRLF boundaries |
| `Editing/RectangleSelection.cs` | `editing/rectangle_selection.rs`, `editing/editor_session/virtual_space.rs`, `editing/editor_session/rectangular_paste.rs` | Rectangular logical-line ranges, tab-expanded cells, explicit virtual endpoints, padded insertion, bounded replacement and clipboard column paste |
| `Editing/IReadOnlySectionProvider.cs`, `NoReadOnlySections.cs`, `TextSegmentReadOnlySectionProvider.cs` | `editing/read_only_section_provider.rs` | Anchored protected ranges, boundary insertion policy, writable interval subtraction |
| `Editing/TextArea.cs`, `EditingCommandHandler.cs`, `TextEditorOptions.cs` | `editing/editor_session.rs` | Shared editing session, multicaret replacement, grouped undo, delete, selection, indentation, newline and tab commands |
| `Editing/EditingCommandHandler.cs` transformation helpers and commands | `editing/text_transformations.rs` | Leading/trailing whitespace cleanup, tab/space conversion, Unicode casing and selected/current-line deletion |
| `Indentation/IIndentationStrategy.cs` | `editing/indentation/indentation_strategy.rs` | Fallible strategy contract returning validated edit plans |
| `Indentation/DefaultIndentationStrategy.cs` | `editing/indentation/default_indentation_strategy.rs` | Previous-line indentation copying; intentional no-op for range reformatting |
| `Indentation/CSharp/CSharpIndentationStrategy.cs` | `editing/indentation/csharp/csharp_indentation_strategy.rs` | Configurable language strategy, single-line and range formatting |
| `Indentation/CSharp/DocumentAccessor.cs` | `editing/indentation/csharp/document_accessor.rs` | Snapshot line access, read-only parsing context and precise whitespace edit plans |
| `Indentation/CSharp/IndentationReformatter.cs` | `editing/indentation/csharp/indentation_reformatter.rs`, `editing/indentation/csharp/literal_scanner.rs` | Block and statement indentation, bracket alignment, labels, comments, continuations and C# string layout |
| `Search/ISearchStrategy.cs`, `RegexSearchStrategy.cs` | `search/regex_search_strategy.rs` | Literal/regex/wildcard search, whole-word boundaries, UTF-16 results, ranged matching, grouped replace-all |
| `Search/SearchStrategyFactory.cs` | `search/search_strategy_factory.rs` | Search option compilation and pattern diagnostics |
| `Search/RegexSearchStrategy.cs` (`SearchResult`) | `search/search_result.rs`, `regex_syntax.rs` | Immutable match captures with source numbering, regex replacement expansion and validation |
| `Folding/NewFolding.cs` | `folding/new_folding.rs` | Parsed region descriptions |
| `Folding/FoldingSection.cs` | `folding/folding_section.rs` | Tracked range, live UTF-16 text, title, folded state and typed metadata |
| `Folding/FoldingManager.cs` | `folding/folding_manager.rs` | Indexed folding ranges, validated reconciliation, preserved user state, error recovery and definitions-only folding |
| `Folding/XmlFoldingStrategy.cs` | `folding/xml_folding_strategy.rs` | Multiline XML element and comment folds, qualified attribute titles, UTF-16 offsets and recovery of closed folds before a parse error |
| `CodeCompletion/ICompletionData.cs`, `CompletionList.cs` | `completion/completion_list.rs` | Data contract, insertion text, priority, prefix/camel/substring ranking, navigation |
| `CodeCompletion/CompletionWindow.cs`, `CompletionWindowBase.cs` | `completion/completion_session.rs` | Anchored insertion range, live filter prefix, caret dismissal and completion commit |
| `Snippets/Snippet.cs`, `SnippetElement.cs`, `SnippetContainerElement.cs`, `SnippetTextElement.cs`, `SnippetReplaceableTextElement.cs`, `SnippetBoundElement.cs`, `SnippetCaretElement.cs`, `SnippetSelectionElement.cs` | `snippets/snippet.rs` | Structured expansion, selected-text insertion, linked fields, final caret; additional numeric snippet notation |
| `Snippets/InsertionContext.cs`, `IActiveElement.cs`, `SnippetInputHandler.cs` | `snippets/snippet_session.rs` | Active field traversal, document anchors, automatic bound-text synchronization and single-step undo |
| `Highlighting/HighlightingColor.cs`, `HighlightingRule.cs`, `HighlightingSpan.cs`, `HighlightingRuleSet.cs`, `IHighlightingDefinition.cs` | `highlighting/highlighting_definition.rs` | Full runtime font/color/decoration model, rules/spans and public loading APIs routed through typed syntax trees |
| `Highlighting/Xshd/XshdElement.cs`, `XshdSyntaxDefinition.cs`, `XshdReference.cs`, `XshdColor.cs`, `XshdProperty.cs`, `XshdRuleSet.cs`, `XshdKeywords.cs`, `XshdSpan.cs`, `XshdImport.cs`, `XshdRule.cs` | `highlighting/xshd/model.rs` | Ordered editable syntax nodes, unresolved local/external/inline references, full XML color metadata and source positions |
| `Highlighting/Xshd/IXshdVisitor.cs` | `highlighting/xshd/visitor.rs` | Typed fallible visitors, explicit child traversal, property visitation and an additional mutable visitor |
| `Highlighting/Xshd/V2Loader.cs`, `HighlightingLoader.cs` | `highlighting/xshd/loader.rs` | Namespace-aware v2 syntax loading, source diagnostics, structural/value checks and distinct parsing/compilation phases |
| `Highlighting/Xshd/XmlHighlightingDefinition.cs` | `highlighting/xshd/compiler.rs` | Forward reference registration, named/inline/external rules, ordered import expansion, case inheritance, cycle detection and bounded regex compilation |
| `Highlighting/IHighlightingDefinitionReferenceResolver.cs` | `highlighting/highlighting_definition.rs::HighlightingDefinitionResolver` | Host lookup for compiled definitions by name or alias, with source-positioned errors for unresolved members |
| `Highlighting/Xshd/SaveXshdVisitor.cs` | `highlighting/xshd/writer.rs` | Validated XML serialization, complete color/property output, regex-mode preservation and inline-import hoisting |
| Archived/commented `Highlighting/Xshd/V1Loader.cs` and legacy TeX resource | `highlighting/xshd/legacy.rs` | Compatibility conversion for legacy categories, references, extensions, escaping, marker rules and the historical `@` mini-language |
| `Highlighting/HighlightingEngine.cs` | `highlighting/highlighting_engine.rs` | Nested multiline spans, imports, ambient color merging, UTF-16 runs, span precedence, zero-width cycle protection |
| `Highlighting/HighlightedLine.cs`, `HighlightedSection.cs` | `highlighting/highlighted_line.rs` | Highlighted line results and styled ranges |
| `Highlighting/RichText.cs`, `RichTextModel.cs` | `highlighting/rich_text.rs` | Lossless UTF-16 text, formatting overlays, slices, concatenation and offset mapping |
| `Utils/RichTextWriter.cs`, `Highlighting/RichTextModelWriter.cs` | `highlighting/rich_text_writer.rs` | Nested formatting and indentation, plain-text and rich-text buffers |
| `Highlighting/HtmlOptions.cs`, `HtmlRichTextWriter.cs`, `HtmlClipboard.cs`, `HighlightingColor.ToCss()` | `highlighting/html_clipboard.rs` | HTML fragments and document ranges, safe CSS and hyperlinks, CF_HTML byte offsets |
| `Highlighting/HighlightedLine.cs` merging/export members | `highlighting/highlighted_line.rs` | Validated nested sections, property-aware merging, rich-text conversion and selected-range export |
| `Highlighting/DocumentHighlighter.cs` | `highlighting/document_highlighter.rs` | Lazy live-line cache, incremental state convergence, initial span state, change events and lifecycle checks |
| `Highlighting/IHighlighter.cs` and its state-change delegate | `highlighting/highlighter.rs` | Object-safe highlighter contract, active colors, balanced grouping, inclusive line notifications and disposal |
| `Highlighting/HighlightingManager.cs`, `Resources/Resources.cs` | `highlighting/highlighting_manager.rs` | Name/extension registry, lazy transitive dependency loading, cache invalidation |
| `Highlighting/Resources/*.xshd` | `highlighting/resources/*.xshd` | All 21 upstream XSHD resources, copied without modification with notices |

## Adaptation decisions

`EditorSession` is a shared `Rc` handle independent of the rendering framework.
It subscribes to the same document changes that the visual editor consumes.
Selections are copied out before mutation and updated outside document borrows.
Every multicaret operation preflights all replacement ranges before making the
first change and groups its changes in one document undo unit. Overlapping
selections are normalized, preventing duplicate edits to the same text. Adjacent
selections retain their separate ranges. The primary selection is tracked by
both endpoints, including virtual positions, so a caret shared by two ranges
keeps its original anchor. A merged range uses the primary selection's direction
when the primary included text. Replacing one range through the accessibility
provider preserves its direction. The regressions check the active range and the
exact caret returned by the provider. An independent AT-SPI client on a private
X11 display replaced the backwards selection at offsets 6–9 with offsets 3–5.
The old demo moved the caret to 5. The updated demo reported the caret at 3 and
the selection at 3–5.

Selections retain directed physical and virtual endpoints. Logical rectangles
preserve columns after short lines, and insertion materializes only the padding
it needs. Their detailed coordinate contract, tab behavior, multiline paste,
protection checks and undo rules are recorded in
[Virtual space and rectangular editing](editor-virtual-space.md). The original
string-based `RectangleSelection::to_selection` keeps its clamped behavior;
`to_document_selection` and `to_selection_with_virtual_space` preserve virtual
cells explicitly.

Ordinary navigation, delete, newline and tab commands query the current or
adjacent logical line. They do not flatten the whole document for each key.
Text search takes one immutable string snapshot, then builds a byte/UTF-16 index
once for all matches. The highlighting engine builds an index once per line,
avoiding repeated scans from the line start for each colored run.

### Language indentation

`IndentationStrategy` is a Rust trait whose operations return `DocumentEdit`
plans in original UTF-16 coordinates. `EditorSession` installs the default
copying strategy initially; hosts can install a shared language strategy or
disable automatic indentation. Enter builds a prospective document from a
persistent snapshot, applies all prospective newlines, and asks the strategy to
indent each new line. Only the resulting leading indentation is composed into
the real edit. No real-document observer sees the intermediate newline, and a
strategy error or protected-range rejection leaves the original text and
selection intact. Multiple carets share the prospective context and one undo
group. Existing whitespace after a split is replaced, avoiding doubled
indentation when Enter is pressed within a line's leading whitespace.

Enter at an ordinary physical caret or selection preserves read-only text.
It inserts the newline in the last writable span of a selection and removes
earlier writable spans. The indentation strategy runs only when the new
line is fully writable. When the selection admits no newline, Enter can
still indent the existing caret line if it is writable. A selection that
crosses read-only text uses the session's indentation strategy because
TextMate Enter rules receive an original contiguous range. The pinned editor
confirmed forward and backward selection behavior, protected trailing
whitespace, CRLF, supplementary characters, and undo restoration. Multicaret
Enter uses atomic protection.

Enter in a rectangular selection uses the same bounded row replacement as
multiline typing. It edits the writable spans in the first two rows of a taller
box and moves the caret to the bottom row at the resulting column. A single-row
box receives the line delimiter within its writable selection. The resulting
caret line is indented when it is fully writable. The row edits and indentation
are validated together and applied in one undo step.

`reindent_selection` applies the complete strategy plan to selected logical
lines, or the whole document when there is no selected text. A selection ending
at the start of a line includes that line. Tab and Shift+Tab keep their own
endpoint rule and leave that line untouched. Ctrl+I and the `Reindent` command
use the same strategy and preflight every edit before committing.

Tab uses the active caret's logical column for a single-line replacement and
inserts virtual padding separately. Multiline indentation skips protected line
starts. Shift+Tab removes the first writable fragment of each indentation
prefix. A multiline command uses one undo group and preserves selection
direction. Setting `EditorOptions::accepts_tab` to false lets FerroUI's keyboard
navigation move focus. Explicit Indent and Unindent commands remain available.

The C# strategy ports the pinned state machine for nested blocks, `switch`
labels, ordinary labels, bracket alignment, continuation lines, unbraced
statements, `else`, ordinary and verbatim strings, characters, comments and
directives. It scans preceding lines for context and stops at the requested
end line. Unlike the default strategy, this language-aware operation has a
cost proportional to the preceding parsing context. Whitespace changes are
separate edit ranges, preserving unchanged source code, its anchors and lone
UTF-16 surrogates. Indentation strings accept nonempty ASCII spaces/tabs; line
accessor replacements reject newline delimiters. These checks are stricter
than the source's string setter and return errors instead of exceptions.

### Text transformations

`plan_text_transformation` produces original-coordinate edit plans without
mutating the document. Text transforms use the selected segments, and line
transforms use the affected logical lines. An empty selection or a rectangle
without document text applies whitespace and case transformations to the
whole document. Delete Lines follows the caret lines. Disjoint multicaret
selections leave intervening lines alone. Whitespace commands exclude an
untouched endpoint line. Delete Lines includes that line. Ctrl+D invokes the
same native command as a host binding. It deletes writable portions in one
undo group and places the caret at the start of the edited range. A fully protected line is
selected around the original caret and creates no undo entry. Deleting the
final line keeps its preceding delimiter.

With a rectangle containing zero selected UTF-16 units, Delete Lines and
Shift+Tab target the active row. This includes rectangles spanning only
virtual cells. Tab indents the selected rows. Reindent, casing and whitespace
cleanup use the whole document. A fully protected active line becomes the selection.
Delete Lines on an empty final row collapses the rectangle to one caret and
adds no undo entry. Undo restores the original rectangle after a text edit.
The pinned editor confirmed the command scopes in twelve cases, including
both drag directions, protected rows and virtual columns. [Pinned editing commands](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Editing/EditingCommandHandler.cs)

Tabs expand to a fixed indentation size, and complete groups of spaces become
tabs. Leading-only variants preserve whitespace within source code. Native
casing commands use FerroUI's ambient text locale. The standalone planner
and session default to invariant casing. Hosts can supply a language tag
through `EditorSession::transform_text_in_language_checked`. Uppercase and
Lowercase apply simple Unicode scalar mappings, including Turkish and
Azerbaijani I rules. Invert Case uses each UTF-16 unit's uppercase-letter
category and preserves surrogate pairs. Simple casing retains the original
UTF-16 lengths, and unchanged text stays outside the edit ranges.
`EditorSession::transform_text` and the native commands plan
each writable fragment separately and commit the edits in one undo group.
Protected gaps break runs of spaces. Line cleanup and leading-only conversion
also preserve protected whitespace. Case and tab conversions use
character-preserving offset mapping. Tab/space groups remain separate edits
so boundary anchors retain their logical place.
Nonvisual callers can query the transformation's `offset_change_mapping` when
applying their own plan. Ordinary `TextDocument::apply_edits` uses normal
replacement mapping.

### Rich text and HTML

`RichText` owns lossless UTF-16 text and its format boundaries. `RichTextModel`
overlays specified properties, explicitly clears individual properties,
queries complete formatted and unformatted partitions, and maps boundaries
through document offset changes. Substrings and concatenation preserve lone
surrogates. Nested writers buffer plain or styled UTF-16 text and validate
balanced spans and indentation. The model coalesces only boundaries near an
edited interval, avoiding a scan of all earlier runs for every appended span.

Highlighted lines validate ranges and nesting before merging additional
properties or exporting a selection. Existing parser state is preserved.
HTML uses flattened effective spans so an inner `underline: false` can clear
an outer underline. Text and attributes are escaped; CSS values accept a
conservative color syntax, safely quoted font families, numeric weights, exact
slant, independent decorations and finite positive font sizes. Native ARGB
hexadecimal colors are converted to CSS RGBA, and hyperlinks reject
executable schemes. Document-range export validates the highlighter's document
identity and preserves CR, LF, CRLF, blank and trailing lines. The document
exporters consume `IHighlighter`, allowing hosts to supply another highlighting
implementation through the same contract. `HtmlExportOptions` enables custom
classes, inline styles, or unwrapped text for each effective style. CF_HTML
generation returns a payload with CRLF headers and UTF-8 byte offsets. It does not write to an operating-system
clipboard. Native run construction, resource resolution, renderer style
application, HTML publication and transactional copy/cut/paste are connected by
the separate [native rich-text and clipboard adapter](editor-rich-text-clipboard.md).

### Incremental document highlighting

`DocumentHighlighter` registers a `WeakLineTracker` and keys cached results by
stable live-line identity. It stores sections relative to the owning line,
projecting their current UTF-16 document offsets only when returning a result.
Edits earlier in the document therefore do not rewrite cached suffix offsets.
Changed lines become dirty; recomputation propagates through subsequent lines
until the resulting span state converges. Independent pending edits remain
dirty even when an earlier region converges, and uncached suffixes stay lazy.
Removal invalidates the previous surviving line as well as its successor. This
covers CRLF merges that change a survivor's text and delimiter lengths while
keeping its total length unchanged.

The object-safe `IHighlighter` trait exposes document access, highlighted lines,
active colors, state updates, named/default colors, change subscriptions,
balanced non-nestable groups and disposal. `Highlighter` is its idiomatic alias.
State-change ranges are inclusive and one-based. The concrete implementation
returns no default text color, following the pinned source. The existing
concrete `get_color_stack` method retains its opaque parser-state return value
for compatibility; new callers can use `get_span_stack` or `get_active_colors`
to make that choice explicit. The trait's `get_color_stack` returns actual span
colors in inside-out order.

Callbacks run without outstanding cache borrows. They can inspect states
already calculated by the active operation. Recursive highlighting returns a
structured error; a callback that edits the document or invalidates the cache
interrupts the operation, and later requests recover from the new document.
Explicit subscription disposal and weak tracker ownership prevent retained
listeners after the highlighter is disposed or dropped.

### Editable XSHD definitions

XSHD loading now has separate XML, editable-tree and executable-definition
stages. `XshdSyntaxDefinition::load` (or `load_xshd`) creates an ordered tree;
`compile` and `compile_with_dependencies` resolve it. The existing
`HighlightingDefinition::load` and `load_with_dependencies` signatures route
through these stages. The earlier direct XML compiler was removed, so the
public loading paths share the same validation and rule semantics.

`XshdElement` is an enum for colors, properties, rule sets, keywords, spans,
imports and rules. `XshdReference<T>` distinguishes absent, local, external and
inline references. A reference splits on its final slash, permitting definition
names that themselves contain slashes and an empty rule-set name for a main
rule set. Tree positions record one-based XML lines and UTF-16 columns; nodes
created by a host use zero coordinates. Source positions decorate structural,
reference and pattern errors without collapsing their error categories.

`XshdVisitor` has associated output and error types and is also exported as
`IXshdVisitor`. `XshdVisitorMut` supports tree transformations before compilation.
Traversal is explicit through node, reference and child-list helpers. Property
nodes participate in both visitor contracts; the pinned property's visitor
method returned no result. Owned Rust trees use structural equality rather
than managed object identity for inline references.

The parser accepts the standard namespace, prefixed uses of that namespace,
and namespace-free v2 documents. Foreign-namespace extension elements are
skipped. Unknown grammar elements, duplicate span members, conflicting inline
and named rule sets, invalid booleans and invalid font values produce errors.
It distinguishes attribute expressions, which retain whitespace, from element
expressions, which use extended regex whitespace/comment handling. Named color
references take precedence over inline style attributes, following the source.
XML parsing does not fetch external resources or enable DTD processing.

Compilation registers named colors and rule sets before translation, so forward
references work. Imports expand at their exact source position within both rule
and span sequences. Consequently, a preceding import wins an equal-position
match and a following import does not silently outrank an earlier local rule.
Cyclic imports are errors; a span can still recursively reference its own rule
set. Case settings inherit through inline definitions and local imports, while
explicit settings override them. A named rule set retains the case context of
its first translation, as in the pinned visitor. External definitions retain
their own root context. Simple keyword groups sort by descending UTF-16 length
inside an atomic alternation; punctuation-bearing groups add boundaries only
at letter/digit ends.

The engine also preserves the source's equal-position span rule: an inner span
start takes precedence over its enclosing span's end. This matters for escaped
delimiters, such as doubled quotes with a separately colored final delimiter.
An unterminated non-multiline span ends at the logical line boundary when it
is the innermost span. A nested span can keep it active across lines. Extended
end expressions preserve trailing comments and explicit whitespace-mode changes
when that boundary alternative is added.

Executable patterns now apply the source's default explicit-capture policy,
including scoped `n` options and capture-reference remapping. Non-span rules
match inside actual span-bounded regions, retaining the source's per-region
match cache and zero-length-match errors. Span-only state scans skip ordinary
rules; colored sections are produced lazily for requested lines. The dedicated
[XSHD runtime audit](editor-xshd-runtime.md) maps these changes and records the
remaining regex-engine differences.

`XshdSyntaxDefinition::to_xml` and `SaveXshdVisitor` write canonical v2 XML.
The writer preserves properties and all color metadata, including font family,
font size, underline, strikethrough and example text that parts of the pinned
writer omitted. It escapes XML characters and preserves attribute tabs/newlines
with character references. Forbidden XML characters and nonfinite font sizes
fail before replacing the writer's last complete output. Regex mode is preserved
when a default-mode expression must become element text. Programmatic inline
imports are hoisted to collision-free named rule sets, carrying their inherited
case setting; named inline colors are likewise emitted as named definitions.
Serialization preserves behavior and metadata, while canonicalizing tree layout
and discarding source coordinates. It does not reproduce the original XML
formatting, comments or prefixes.

`HighlightingManager` parses each registration once and keeps that typed tree.
It compiles only the requested definition and its transitive external
dependencies. `external_definitions` discovers references in colors, delimiter
colors, inline rule sets and imports. Hosts can also register an edited tree
directly with `register_syntax_definition`. Replacing a registration removes
its old extension associations and invalidates cached dependents. Expression
compilation remains lazy, so an unrelated malformed pattern does not prevent
another language from loading.

Hosts with compiled definitions can call `HighlightingDefinition::load_with_resolver`
or `XshdSyntaxDefinition::compile_with_resolver` with a
`HighlightingDefinitionResolver`. Closures and `HighlightingManager` provide
the same interface. A qualified reference uses the resolver's alias, even
when that alias equals the name of the input syntax. Unqualified references
stay in the input definition. `HighlightingManager::register_definition`
registers a compiled definition and its extension list alongside editable
XSHD, with the same replacement and cache invalidation behavior.

### Named syntax highlighting in markup

The `TextEditor.SyntaxHighlighting` styled property accepts a case-sensitive
XSHD name in markup. For example, `<TextEditor SyntaxHighlighting="C#"/>`
selects the bundled C# definition. The current `FerroLocator` scope can supply
a `dyn HighlightingDefinitionResolver` for custom names and aliases. The
bundled catalog supplies names absent from the host resolver. An unregistered
name resolves to no definition. The typed value formats as the
definition's canonical name, including when it was loaded by alias.

Style setters, local values and `{x:Null}` use the normal property precedence.
`set_syntax_highlighting` continues to take a compiled definition from Rust.
The chosen definition is applied to the current document and a replacement
document receives a new highlighter. The runtime loader and standard
build-time compiler read the same editor namespace and typed nullable value.
The pinned name converter confirmed the case-sensitive and absent-name
behavior. [Pinned name converter](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/HighlightingDefinitionTypeConverter.cs)

### Historical XSHD compatibility

The pinned `V1Loader.cs` and its dispatch in `HighlightingLoader` are commented
out. The legacy importer is therefore a documented compatibility extension
based on archived source, rather than an assertion that the active upstream
loader still supports v1. It converts the historical `RuleSets` representation
to the same editable v2 tree and keeps the bundled TeX resource usable.

Conversion preserves the historical category order: reference imports, keyword
groups, spans, preceding markers, following markers, then digit rules. Root
`extends` imports follow the main set's local rules. Legacy extension lists
split on both semicolons and vertical bars. It supports inherited/per-span
escape characters, backslash-style escapes, doubled closing delimiters,
`singleword` and `startofline` guards, marker inclusion/exclusion and the
`@C`, `@@`, `@!literal@` and `@-literal@` constructs. Unknown or unfinished
constructs return source-positioned errors. Legacy delimiter metadata remains
ignored as in the archived importer. Empty escape-character attributes return
errors instead of indexing an empty string; supplementary escape characters
use a complete Rust scalar instead of half a UTF-16 pair.

Search and XSHD use `fancy-regex` with an explicit backtracking limit and bounded
regex compilation. Its linear regex delegate handles simple expressions, and
the backtracking engine supplies lookaround and backreferences. Search and XSHD
share the reference word and decimal digit categories, including the separate
join-control rule for word boundaries. Compilation and matching failures are
returned. [Unicode fixtures and coverage](editor-xshd-runtime.md#unicode-word-and-digit-classes)

Regex search retains empty matches after consuming matches. The match loop
advances after an empty result and maintains `\G` continuity across searches.
Adjacent matches participate in the same atomic Replace All operation.
[End anchors and match iteration](editor-xshd-runtime.md#end-anchors-and-match-iteration)

### UTF-16 search offsets

Search uses byte offsets directly for ASCII text. For other text it measures
UTF-16 offsets between successive regex match boundaries. It keeps the
original document as the input for captures and replacement context. The
pinned AvaloniaEdit strategy and Rust regression produced the same
supplementary-character match and empty match in a restricted range after an
earlier match was excluded. A separate Rust test covers a lookbehind with
`\K` that reports a start before the preceding match's end.
[Pinned search range filter](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Search/RegexSearchStrategy.cs#L42-L53)

### Snippet edit ownership

Snippet mirrors run after document edits, through `text_changed`, and continue
the current undo group. They never mutate from a `changing` or `changed` handler.
Undo/redo replay suppresses new mirror writes because the history already
contains the bound-text edits. Subscriptions use weak references and explicit
disposal. Input handlers can also call `replace_active` directly.

Fields sharing an insertion boundary retain their order in the expanded snippet.
The active field owns typed text; each bound occurrence temporarily owns its
mirror replacement. Optional undo actions retain weak references to the session
and restore that ownership during replay, even after the user has selected a
different tab stop. The final caret also preserves its source position among
coincident empty fields.

`Snippet::prepare` validates and expands the entire snippet before the host
replaces a selection. The UI inserts through `EditorSession` to apply its
protected-range policy, then activates the prepared anchors within the same
undo group. Automatic mirrors use a host-supplied validator as well. If a host
later protects a bound field, its rejected mirror ends the session and records
an error; the user's already accepted edit remains undoable and protected text
is unchanged.

### Live folding sections

`FoldingSection::set_start_offset` moves the range and preserves its length.
`set_end_offset` and `set_length` resize it from the current start. These
operations update the manager's segment index and refresh the installed view.
The setters return `SegmentError` for invalid bounds, and `text_content` reads
the current text by UTF-16 offset and length.

The initial `NewFolding` tag is a snapshot of the parser's descriptor. A host
can replace it with a typed `Rc<T>` and retrieve it with `tag::<T>()`.
Reconciliation at the same start retains the section, its folded state and
its tag as it applies the latest title and extent.

Connected segment collections rebase during the document commit, before
public change callbacks see the updated text. Replacing a whole section with
nonempty text keeps the section over the inserted range. An affected section
that becomes empty is removed from the manager and unfolded. A detached handle
retains its last range and stops moving with document edits.

Rust regressions check that an earlier document subscriber sees the updated
range during the replacement.

## FerroUI surfaces

These presentation files live in `src/FerroUI.Edit`. They use the framework's
controls, styled properties, popup placement, routed events, theme resources,
focus and explicit subscription disposal. No parallel widget or event system is
introduced.

| Upstream source | Rust source | Behavior |
|---|---|---|
| `Search/SearchPanel.cs`, `SearchPanel.xaml`, `SearchCommands.cs` | `search_panel.rs` | Overlay search/replace input, case/word/regex flags, disposable options subscriptions, query reactivation, wrapped navigation, capture replacement, atomic protected replace-all, diagnostics, visible result ranges |
| `CodeCompletion/CompletionWindow.cs`, `CompletionWindowBase.cs`, `CompletionList.cs` | `completion_window.rs` | Caret-positioned popup with framework `ListBox`, filtered prefix, selected description, tunneled navigation, Enter/Tab/double-click insertion, caret/document dismissal and focus restoration |
| `Snippets/SnippetInputHandler.cs`, `InsertionContext.cs` | `snippet_controller.rs` | Prevalidated selection replacement, active-field selection, Tab/Shift+Tab traversal, Escape termination, automatic mirror permission checks, document replacement cleanup |
| `Editing/EditingCommandHandler.cs`, `TextAreaDefaultInputHandlers.cs`, `Search/SearchCommands.cs` | `commands.rs` | Framework `ICommand` actions with weak editor ownership, read-only/selection/history enabled state, directional deletion, language reindent, protected edit checks and disposable enabled-state notifications |

`SearchPanel::install` returns a reusable popup attached to an editor; its
`show(replace)` method focuses the query and `close_panel` returns focus to the
text area. Search settings are styled properties. Results retain capture data
from their immutable search snapshot. `results_in_range` exposes only the
ranges needed for viewport drawing.

`CompletionWindow::show_for` accepts a document insertion range and completion
items. The editor keeps keyboard focus while its tunnel handler navigates the
list. Selecting an item with the pointer updates the same core model. Completion
never holds its session `RefCell` borrow while mutating the document, since that
mutation synchronously raises editor events. Unchanged prefix refreshes retain
the user's chosen item, and replacing the editor document closes the session.
An attached framework property retains the active completion popup on its
editor. While it is active, snippet handlers defer Tab/Escape to completion;
field navigation resumes when completion closes.
Keyboard subscriptions attach to the text area itself, so this ordering also
works before the editor's control template has attached its visual children.

`SnippetController` is an `Rc` controller rather than a visual control. A host
retains it for the session lifetime. `TextEditor::insert_snippet` performs that
retention; dropping the controller detaches its input handlers. A linked
insertion requires one selection and reports a clear error for multiple carets.

`EditorCommand` adapts common actions to the existing FerroUI `ICommand`
contract. It watches routed text/selection changes and document update
completion, so undo availability is queried after the history transaction is
committed. Document policy and history property notifications also refresh the
enabled state when hosts directly protect a document or clear its undo stack.
Its property-specific observers avoid subscribing to the editor's expensive
full-text value. Hosts can call `refresh` after changing an external
protected-range provider. Clipboard execution uses the editor's asynchronous
platform clipboard methods; paste availability reflects edit permissions rather
than synchronously inspecting clipboard contents.

The same command cache exposes `EditorCommandKind::Reindent` and
`EditorCommandKind::Transform(TextTransformation::Uppercase)` (or another
transformation). Their `as_command()` values can be assigned directly to
framework button, menu-item and key-binding command properties.

`EditorCommandKind::SelectAll` remains enabled with an empty or read-only
document. The native command and the platform shortcut move a virtual caret in
an empty document back to the document start without creating an undo entry.
With search closed and no active input overlay, Escape keeps the selection and
caret in place. Modified Escape remains available to host shortcuts. The
pinned reference was exercised through routed key events for both behaviors.

Use `editor.command(EditorCommandKind::DeleteSelection).as_command()` for an
application's selection-only Delete action. The command stays available while
its editor exists, as in the pinned editor. It deletes writable fragments in
one undo step and preserves fully protected selections. A zero-width rectangle
keeps its column carets. A lone caret in virtual space returns to the physical
line end. `EditorCommandKind::Delete` retains the same forward deletion
behavior as the Delete key.

`EditorCommandKind::InsertLineBreak` exposes the same operation as Enter and
Shift+Enter. It is available while its editor exists and applies to a focused
text area. The editor uses the document's newline or the configured default,
honors protected spans, and applies the indentation strategy and TextMate
rules. An extension can cancel
the edit from `TextEntering`. Read-only Enter still raises `TextEntering` and
`TextEntered` and keeps the document, selection, and undo history intact.
`TextArea::perform_text_input` returns `Ok(false)` for a read-only line break.
The session checks the host binding after document update callbacks and before
committing a prepared line break.

`EditorCommandKind::Navigate` exposes caret moves by character, word, row,
page or boundary. Hosts can extend the current selection or request supported
rectangular movement. The native commands and key handler share the view's
navigation implementation. Movement keeps the desired horizontal position,
resolves folded elements through their visible boundaries, and remains
available in a read-only editor. The rectangular command follows the editor's
rectangular selection option. Extending an existing rectangle by a page or to
either end of the document updates its active corner while preserving the
anchor and rectangular row ranges.

The runnable [EditorDemo](../../samples/EditorDemo/README.md) loads its main view
from runtime XAML and exercises these APIs with bundled Rust/C# TextMate
grammars, Light+/Dark+ themes, explicit region folding, toolbar commands and a
status line. Its region provider demonstrates application-supplied fold ranges;
it is not a Rust or C# parser.

### Gestures by keymap

The Apple keymaps are those of macOS and of a browser on an Apple platform. Every other platform,
iOS included, uses the Control keymap. The last column names what checks the gesture on an Apple
keymap; "none" is a gesture only the code path shared with the Control keymap covers.

| Gesture | Control keymap | Apple keymap | Checked on an Apple keymap by |
|---|---|---|---|
| Copy, cut, paste, select all, undo | Control | Command | Apple check of `editor_demo.test.mjs`, macOS clipboard check |
| Find | Control+F | Command+F | Apple check of `editor_demo.test.mjs` |
| Replace | Control+H | Command+Option+F | `replace_gesture_follows_the_platform_in_the_editor_and_search_popup` |
| Delete Line | Control+D | Control+D | Apple check of `editor_demo.test.mjs`, `line_commands_stay_on_control_with_the_apple_keymap` |
| Reindent | Control+I | Control+I | none |
| Move or select by word | Control+arrow | Option+arrow | none |
| Delete a word | Control+Backspace or Delete | Control or Option with Backspace or Delete | none |
| Rectangle from the keyboard | Alt+Shift+arrow | Control+Shift+arrow | Apple check of `editor_demo.test.mjs`, macOS clipboard check |
| Rectangle with the pointer | Alt+drag | Option+drag | `apple_keymap_gives_box_word_and_copy_gestures_their_own_modifiers` |
| Select a word with a click | Control+click | Command+click | the same test |
| Open a link | Control+click | Command+click | `a_link_takes_the_command_modifier_of_the_apple_keymap` |
| Copy with a text drag | Control at the drop | Option at the drop | the modifier in the same test; no drop on an Apple keymap |
| Indent, outdent | Tab, Shift+Tab | Tab, Shift+Tab | none |

Inside a selection, a press with Option starts a text drag; outside it, a rectangle.

## Tests

Run `cargo test -p ferroui-edit-core --lib`. Service regressions cover:

- overlapping selections, duplicate carets, multicaret edits and grouped undo;
- grapheme clusters, supplementary characters, CRLF, short lines and final empty lines;
- tab-expanded rectangles, virtual navigation/padding, bounded multiline paste,
  no-op virtual deletion, protected-range intersections and stale-plan rejection;
- preflight rejection without partial multicaret mutation;
- C# blocks, labels, continuations, bracket alignment, comments and string states;
- automatic indentation after a split, multi-caret preview context, error
  rollback, whole-document reindent and protected formatting plans;
- whitespace and case transforms, disjoint selections, mixed line delimiters,
  writable fragment grouping, protected whitespace, language-specific casing, and
  selection and anchor mapping across undo and redo;
- upstream whole-word search cases, original-context range matching, captures,
  UTF-16 result offsets and zero-length replace-all;
- folding state preservation after edits, malformed-region atomicity and XML regions;
- completion ranking, wrapping selection and tracked insertion ranges;
- linked snippets through ordinary document edits, undo/redo and field traversal;
- coincident empty snippet fields, adjacent field boundaries, final-caret order,
  and replay after changing the active field;
- protected snippet mirror rejection and preserved completion keyboard selection;
- XSHD multiline stacks, lookahead, span boundaries and bounded zero-width recursion;
- compilation and execution of every one of the 21 bundled XSHD definitions;
- typed XSHD namespace handling, UTF-16 source coordinates, immutable/mutable
  visitors, forward/external references, import ordering/cycles, case inheritance,
  keyword prefixes, serializer escaping and transactional failure;
- compile/serialize/reparse highlighting comparisons for all 21 bundled
  definitions, including multiline state, and historical escaping/marker rules;
- lazy registry dependency discovery, legacy extension separators, missing
  dependencies and dependent-cache invalidation after replacing a definition;
- incremental convergence after a one-line edit in 1,000 cached lines, stable
  suffix identities after insertion/removal, independent queued edits and lazy
  frontier behavior;
- highlighter groups, initial states, inclusive state events, reentry errors,
  interruption recovery, unsubscription and disposal;
- exhaustive edits at every UTF-16 offset in a mixed CR/LF/span/surrogate
  document, plus 180 batches of three random edits and periodic undo/redo,
  compared with fresh highlighting;
- rich-text property overlays and clears, offset affinity, nested section
  merging, UTF-16 slicing and restoration of split surrogate pairs;
- semantic font family/weight/slant/strikethrough retention, legacy boolean
  overlay compatibility, independent setters and full XSHD runtime validation;
- HTML escaping, CSS/hyperlink rejection, blank/trailing lines and CF_HTML
  offsets in UTF-8 bytes, including native alpha-channel conversion and safe
  font-family fallback lists.

These are executable regression tests, not a complete upstream differential
suite. The central port inventory records missing counterparts separately.

The [rich-text/clipboard report](editor-rich-text-clipboard.md) records the
native integration coverage and command separately.

Run `cargo test -p ferroui-edit --lib` for the control-service integration tests
and `cargo test -p editor-demo --lib --no-default-features` for runtime XAML
construction and demo commands. These cover Unicode search selection,
replace-all undo, zero-width
navigation, completion commit, snippet replacement/mirrors/tab traversal, and
invalid/read-only snippet insertion without partial deletion. Desktop rendering
and platform input still require actual graphical runs.

The demo's default desktop feature uses FerroUI's standard desktop backend;
the optional headless feature supports application-lifetime smoke runs without
that native renderer. Headless results do not establish pixel-level visual
parity or operating-system clipboard/input-method behavior.

C# Reindent preserves raw-string closing margins, literal lines and trailing
spaces. The UTF-16 scanner tracks verbatim strings, nested interpolation,
format clauses and the quote and brace counts for raw strings. Enter preserves
existing whitespace after a caret inside a literal and applies the regular
indentation strategy at other carets in the same edit. The core tests cover
range reindentation, incomplete literals, multiline interpolation and
multicaret undo with CRLF and supplementary characters. [C# raw-string rules](https://learn.microsoft.com/en-us/dotnet/csharp/fundamentals/strings/raw-string-literals)

## Explicit remaining differences

- Ordinary typing, Backspace and Delete act on the writable portions of one
  selected range. Replacement goes in the last writable span. One undo restores
  the text and direction of the original selection. An unchanged pinned editor
  confirmed these outcomes for forward, backward and wholly protected
  selections. Directional deletion from an empty caret uses the same
  writable-span policy. Ctrl+Delete removes up to the next word start, Ctrl+Backspace
  removes back to a word start, and Backspace can remove one code point from a
  combining sequence. Native commands use formatted visual elements, so a
  collapsed fold can be removed as one target. Rectangular typing and deletion
  of a selected box apply the same rule per row and keep the box aligned to the
  first row's resulting column.
  Bounded multiline input uses corresponding rows and leaves the caret on the
  last row. Ordinary, rectangular, and whole-line Cut copy the full target and
  delete only writable spans. Whole-line Cut removes spans from right to left
  with separate undo entries. Clipboard Paste into an ordinary selection or
  box also preserves protected spans. A marked column paste from one caret skips
  protected rows. Independent multicaret edits and rectangle drops validate all
  destination ranges before applying changes.
- Logical rectangles retain virtual cells and support padded insertion and
  bounded multiline paste. Distinct virtual carets at the same physical line
  end are explicitly rejected for combined replacement. Native pixel-based
  rectangles, proportional fonts and bidirectional visual columns require text
  view validation separately from the logical service tests.
- Vertical movement in the service layer uses tab-expanded grapheme cells and
  retains its desired column across short lines. The text view resolves its
  preferred document-space X within a shaped row for line and page movement.
  Soft-wrap clamping uses the preceding valid caret stop. [Reference checks](editor-visual-lines.md#vertical-navigation-with-wrapping-and-bidirectional-text)
- C# indentation remains lexical. Reindent preserves existing formatting on
  lines inside multiline interpolations. Additional upstream input modes
  still require separate work. Title-case conversion throws in the pinned
  source.
- Native completion and insight/overload controls, custom completion actions,
  rich descriptions and observable collections are implemented; see
  [editor-completion.md](editor-completion.md). A secondary native popup follows
  the selected completion row. The controls use native composition and FerroUI
  content templates.
- The panel exposes snapshot search-result ranges to the renderer and handles
  literal and regex modes. The service factory also supports wildcards. Query
  and flag changes notify `SearchOptionsChangedEventArgs` subscribers after
  pattern validation and before results refresh. The unchanged pinned editor
  confirmed the sender and option payloads for accepted changes, including
  clearing the query. While the panel is open, valid query and flag changes
  select the next match and wrap to the first match when needed. A valid query
  with no matches clears the selection at the caret. Invalid patterns preserve
  the selection. The status reports the index of the match actually selected.
  The popup docks at the editor's upper right and uses the editor bounds as its
  anchor. Search navigation and query changes scroll the result's caret clear
  when it overlaps the popup and there is room below it. The editor compares the
  caret and measured popup rectangles in the same coordinate space. A laid-out
  editor test verified docking and showed that both Find Next and a query change
  reveal an overlapping result. A match on the same row outside the popup keeps
  the usual scroll position. [Pinned search
  alignment](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Search/SearchPanel.xaml#L68-L76)
  [Pinned search result
  reveal](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Search/SearchPanel.cs#L521-L533)
  The built-in panel uses the search factory in both implementations.
  `reactivate` focuses the query and selects its text. Find Next, Find Previous,
  Replace Next, Replace All, and Close Search Panel are native commands that
  hosts can also bind to menus or keys. F3, Shift+F3, Escape, Alt+R, and Alt+A
  route from the editor and the popup. Find Previous searches before the primary
  selection's lower offset and wraps to the final match when it reaches the
  start. This also applies to a backwards selection. Manual selection or caret
  changes clear the active match index. Find Next visits a zero-width match at
  the new caret and advances on subsequent searches from that match. [Pinned
  Find Previous
  command](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Search/SearchPanel.cs#L387-L399)
  Replacement commands require an open Replace panel and a writable document.
  Replace Next starts at the primary selection's lower offset for both selection
  directions and replaces the first match at or after that point in one action.
  It wraps to the first match when needed. After the edit it selects the
  following match or leaves the caret after the replacement. A zero-width regex
  replacement searches strictly after the replacement boundary before wrapping.
  With no matches, the command leaves the text and selection intact. A protected
  match or a document swap invalidates the planned edit. The pinned editor and
  the Rust fixture agree on the replaced match and resulting selection from
  three UTF-16 caret positions. [Pinned Replace Next
  command](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Search/SearchPanel.cs#L401-L412)
  Search installation is shared by the editor and the public install API.
  Uninstall removes the popup and its subscriptions so the editor can install a
  fresh panel. The editor and search popup use the platform command modifier
  with F for Find. Replace uses Control+H on Control-based platforms and
  Command+Option+F with the macOS keymap. Routed input tests cover both keymaps,
  including leaving Command+H available for the macOS Hide action.
  `SearchResultsBrush` styles matches through the native editor property. The
  panel updates its labels, tooltips, and match status when the platform
  language changes. It uses Simplified Chinese for `zh-Hans`, `zh-CN`, and
  `zh-SG` and falls back to English for the other tested tags. An unchanged
  pinned editor confirmed the fallback. Engine diagnostics retain their detail
  after the localized error prefix. [English
  resources](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/SR.resx)
  · [Simplified Chinese
  resources](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/SR.zh-Hans.resx)
- Case-insensitive literal Find uses the thread's FerroUI formatting locale.
  Turkish and Azerbaijani match `I` with `ı` and `i` with `İ`. The standalone
  search factory accepts a language for literal and wildcard modes and uses
  invariant casing by default. The factory compiles explicit character sets from
  the core's Unicode lowercase data, keeping UTF-16 match offsets and
  replacement text tied to the original document. Regex mode uses the backend's
  Unicode casing. [Pinned search
  factory](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Search/SearchStrategyFactory.cs)
- Regex search assigns unnamed groups before named groups and supports distinct
  explicit group numbers. Backreferences, conditionals, match lookups and
  replacements use the same numbers. `$+` expands the highest-numbered capture.
  Bare numeric escapes fall back to octal when the capture number is absent
  and the digits permit it. XSHD and search share class subtraction, class
  literal handling, control escapes and named Unicode blocks.
  [Regex syntax evidence](editor-xshd-runtime.md#numeric-escapes-and-character-classes)
  Extended mode recognizes form feeds outside character classes as spacing
  and keeps a repetition count with separated digits or delimiters literal.
  [Extended whitespace and repetition](editor-xshd-runtime.md#extended-whitespace-and-repetition)
  Find/Replace can resume an exhausted boundary scan at exact `\b` or `\B`
  candidate positions when the pattern requires that assertion at every match
  start. It keeps the original document context for captures and replacements.
  [Unicode boundary checks](editor-xshd-runtime.md#unicode-word-and-digit-classes)
  The standalone search strategy's Find Next stops at the first match
  that satisfies the requested range and whole-word filters. The pinned
  strategy and Rust regression both found the complete word after an
  embedded occurrence and reported its UTF-16 offset and named capture.
  Enumerating all matches reached the later backtracking branch and
  reported the engine's timeout or match-budget error.
  Replace All validates every expansion before editing. Balancing groups,
  duplicate capture declarations, capture histories, and exact UTF-16 or
  culture-specific regex behavior remain open.
- Structured snippet nodes expose extensible insertion contexts, active-element
  traits, transformed bound fields, lifecycle events and undo ownership; see
  [editor-snippets.md](editor-snippets.md). Native field background and outline
  rendering remains presentation work.
- Typed XSHD loading, visitors, serialization and archived v1 conversion have
  regression coverage. The pinned upstream loader has XSD validation commented
  out; the Rust parser/compiler likewise validates supported structure and
  values directly, without checking unknown attributes against that schema.
  Default explicit captures, option scopes and bounded rule regions are
  adapted, but comprehensive .NET regex differential parity remains absent.
  Unicode/culture behavior and capture histories retain differences documented
  in [editor-xshd-runtime.md](editor-xshd-runtime.md). Source positions and XML
  formatting are not preserved by serialization.
- XML folding publishes completed multiline regions before the first parse error
  and passes its UTF-16 offset to the folding manager. Existing regions after
  that boundary can remain folded while the user repairs the document. The
  parser catches invalid `xml:space` values, empty prefixed namespace
  declarations, attempts to bind the reserved `xmlns` prefix, and malformed
  processing-instruction targets. Misuse of a reserved namespace URI reports the
  start of the quoted value, including escaped URIs and incomplete tags.
  Reference cases include earlier closed folds, incomplete start tags, mixed
  line endings, and supplementary characters before the error. The namespace
  comparison matched 66 of 78 title-mode results, up from 40, and the Windows
  folding-manager regression confirms that an invalid namespace declaration
  keeps the uncertain suffix folded. The declaration pass reports invalid
  standalone values, invalid characters in quoted values, and unclosed quotes.
  It keeps declared encoding labels as metadata because the editor already holds
  decoded text. The expanded reference comparison matched 142 of 150 title-mode
  results, up from 98. Malformed qualified names now report the invalid colon or
  local-name start, including unfinished tags. CDATA outside the root reports
  its opening delimiter and keeps earlier completed folds. The expanded
  comparison matched 198 of 200 title-mode results, up from 148. The remaining
  input uses the reserved `xmlns` prefix on an element name. The parser enforces
  the namespace constraint for that prefix. [Namespaces in XML
  1.0](https://www.w3.org/TR/xml-names/)
- The document highlighter uses stable live-line identities and a lazy dirty
  index instead of compressed state lists. Tracker registration initializes
  live-line metadata in O(total lines) if it has not already been built;
  highlighted results remain lazy. State-only updates use a separate span scan,
  and cached state is distinguished from optional colored sections. These
  choices have regression coverage, but no benchmark-equivalence claim.
  Trait composition replaces
  protected virtual subclass hooks. Notifications include the actual last
  line when its incoming state changes, correcting a stricter condition in the
  pinned concrete source to follow the interface's inclusive contract.
- Rich-text writers are owned buffers rather than live-document insertion
  writers; the corresponding upstream model-writer implementation was commented
  out. Runtime XSHD retains font families, exact weights/slants and both
  decorations; native `Run`, brush and clipboard adapters are documented in
  [editor-rich-text-clipboard.md](editor-rich-text-clipboard.md). HTML
  customization uses a Rust policy trait. Export uses
  flat spans, `<br/>`, pixel font sizes and ten-digit CF_HTML offsets. UTF-8
  projections replace lone surrogates; a surrogate pair split by formatting
  takes the leading code unit's style.

- A caret move, a document swap or an options change during the background scan
  of an indented paste cancels that paste with a stale edit error and leaves the
  document unchanged. The paste is not applied at the new caret.
- A clipboard read of a Chromium web format on Wayland ends when its owner has
  been silent for two seconds. The format map has one deadline for the whole
  read and a limit of 64 KiB.

None of these differences is counted as behavioral parity merely because a
similarly named Rust type exists.
