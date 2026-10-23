# Editor services port

This document records the framework-independent editing services in
`src/FerroUI.Edit.Core`. The reference is the pinned AvaloniaEdit checkout listed
in [avaloniaedit-textmate.md](avaloniaedit-textmate.md). Public positions count
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
| `Indentation/CSharp/IndentationReformatter.cs` | `editing/indentation/csharp/indentation_reformatter.rs` | Block and statement state machine, bracket alignment, labels, comments, strings and continuations |
| `Search/ISearchStrategy.cs`, `RegexSearchStrategy.cs` | `search/regex_search_strategy.rs` | Literal/regex/wildcard search, whole-word boundaries, UTF-16 results, ranged matching, grouped replace-all |
| `Search/SearchStrategyFactory.cs` | `search/search_strategy_factory.rs` | Search option compilation and pattern diagnostics |
| `Search/RegexSearchStrategy.cs` (`SearchResult`) | `search/search_result.rs` | Immutable match captures and .NET replacement expansion |
| `Folding/NewFolding.cs` | `folding/new_folding.rs` | Parsed region descriptions |
| `Folding/FoldingSection.cs` | `folding/folding_section.rs` | Anchored region extent, title and folded state |
| `Folding/FoldingManager.cs` | `folding/folding_manager.rs` | Validated region reconciliation, preserved user state, error recovery, definitions-only folding |
| `Folding/XmlFoldingStrategy.cs` | `folding/xml_folding_strategy.rs` | XML element/comment regions, attribute titles and UTF-16 offsets |
| `CodeCompletion/ICompletionData.cs`, `CompletionList.cs` | `completion/completion_list.rs` | Data contract, insertion text, priority, prefix/camel/substring ranking, navigation |
| `CodeCompletion/CompletionWindow.cs`, `CompletionWindowBase.cs` | `completion/completion_session.rs` | Anchored insertion range, live filter prefix, caret dismissal and completion commit |
| `Snippets/Snippet.cs`, `SnippetElement.cs`, `SnippetContainerElement.cs`, `SnippetTextElement.cs`, `SnippetReplaceableTextElement.cs`, `SnippetBoundElement.cs`, `SnippetCaretElement.cs`, `SnippetSelectionElement.cs` | `snippets/snippet.rs` | Structured expansion, selected-text insertion, linked fields, final caret; additional numeric snippet notation |
| `Snippets/InsertionContext.cs`, `IActiveElement.cs`, `SnippetInputHandler.cs` | `snippets/snippet_session.rs` | Active field traversal, document anchors, automatic bound-text synchronization and single-step undo |
| `Highlighting/HighlightingColor.cs`, `HighlightingRule.cs`, `HighlightingSpan.cs`, `HighlightingRuleSet.cs`, `IHighlightingDefinition.cs` | `highlighting/highlighting_definition.rs` | Full runtime font/color/decoration model, rules/spans and public loading APIs routed through typed syntax trees |
| `Highlighting/Xshd/XshdElement.cs`, `XshdSyntaxDefinition.cs`, `XshdReference.cs`, `XshdColor.cs`, `XshdProperty.cs`, `XshdRuleSet.cs`, `XshdKeywords.cs`, `XshdSpan.cs`, `XshdImport.cs`, `XshdRule.cs` | `highlighting/xshd/model.rs` | Ordered editable syntax nodes, unresolved local/external/inline references, full XML color metadata and source positions |
| `Highlighting/Xshd/IXshdVisitor.cs` | `highlighting/xshd/visitor.rs` | Typed fallible visitors, explicit child traversal, property visitation and an additional mutable visitor |
| `Highlighting/Xshd/V2Loader.cs`, `HighlightingLoader.cs` | `highlighting/xshd/loader.rs` | Namespace-aware v2 syntax loading, source diagnostics, structural/value checks and distinct parsing/compilation phases |
| `Highlighting/Xshd/XmlHighlightingDefinition.cs` | `highlighting/xshd/compiler.rs` | Forward reference registration, named/inline/external rules, ordered import expansion, case inheritance, cycle detection and bounded regex compilation |
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
selections are normalized, preventing duplicate edits to the same text.

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

`reindent_selection` applies the complete strategy plan to selected logical
lines, or the whole document when there is no selected text. A selection ending
at the start of a line excludes that untouched line, consistently with Tab and
Shift+Tab. The command preflights every planned edit before committing. The
FerroUI `Reindent` command and C# sample use this same service.

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
mutating the document. Text transforms operate on selected segments; line
transforms operate on the affected logical lines. With no selected text,
whitespace and case transforms use the whole document while DeleteLines uses
the caret lines. Disjoint multicaret selections leave intervening lines alone.
Whitespace commands exclude an untouched line at the end of a selection;
DeleteLines includes that endpoint line, following the pinned command. Deleting
the final line keeps its preceding delimiter, also following the source.

Tabs expand to a fixed indentation size, and complete groups of spaces become
tabs. Leading-only variants preserve whitespace within source code. Casing
uses locale-independent Rust Unicode mappings, including supplementary scalars
and expanding mappings; it does not use .NET's current-culture casing. Lone
surrogates and unchanged text stay outside the edit ranges.
`EditorSession::transform_text` applies all plans in one protected transaction.
Case/tab conversions use character-preserving offset mapping, and tab/space
groups remain separate edits so boundary anchors retain their logical place.
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
implementation through the same contract. CF_HTML generation returns a payload
with CRLF headers and UTF-8 byte offsets. It does not write to an operating-system
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
An unterminated non-multiline span ends at the logical line boundary. Extended
end expressions preserve trailing comments and explicit whitespace-mode changes
when that boundary alternative is added.

Executable patterns now apply the source's default explicit-capture policy,
including scoped `n` options and capture-reference remapping. Non-span rules
match inside actual span-bounded regions, retaining the source's per-region
match cache and zero-length-match errors. The dedicated
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
regex compilation. Its linear regex delegate handles simple expressions; the
backtracking engine supplies lookaround and backreferences. Compilation and
matching failures are returned. This is not a claim that the .NET regex engine
and the replacement engine have identical behavior for every possible pattern.

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

## FerroUI surfaces

These presentation files live in `src/FerroUI.Edit`. They use the framework's
controls, styled properties, popup placement, routed events, theme resources,
focus and explicit subscription disposal. No parallel widget or event system is
introduced.

| Upstream source | Rust source | Behavior |
|---|---|---|
| `Search/SearchPanel.cs`, `SearchPanel.xaml`, `SearchCommands.cs` | `search_panel.rs` | Overlay search/replace input, case/word/regex flags, wrapped next/previous navigation, capture replacement, atomic protected replace-all, diagnostics, visible result range queries |
| `CodeCompletion/CompletionWindow.cs`, `CompletionWindowBase.cs`, `CompletionList.cs` | `completion_window.rs` | Caret-positioned popup with framework `ListBox`, filtered prefix, selected description, tunneled navigation, Enter/Tab/double-click insertion, caret/document dismissal and focus restoration |
| `Snippets/SnippetInputHandler.cs`, `InsertionContext.cs` | `snippet_controller.rs` | Prevalidated selection replacement, active-field selection, Tab/Shift+Tab traversal, Escape termination, automatic mirror permission checks, document replacement cleanup |
| `Editing/EditingCommandHandler.cs`, `TextAreaDefaultInputHandlers.cs`, `Search/SearchCommands.cs` | `commands.rs` | Framework `ICommand` actions with weak editor ownership, read-only/selection/history enabled state, language reindent, protected edit checks and disposable enabled-state notifications |

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

The runnable [EditorDemo](../../samples/EditorDemo/README.md) loads its main view
from runtime XAML and exercises these APIs with bundled Rust/C# TextMate
grammars, Light+/Dark+ themes, explicit region folding, toolbar commands and a
status line. Its region provider demonstrates application-supplied fold ranges;
it is not a Rust or C# parser.

## Validation

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
  unchanged protected characters, Unicode expansion and character mapping;
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
At the typed-XSHD integration checkpoint, the unfiltered Core command passed
**240 tests**, including **22 typed-XSHD tests** and **three additional registry
regressions**. Later port stages can add tests without changing that historical
checkpoint count.

The later runtime-style checkpoint passed **264 Core tests** with no failures,
ignored tests or filters. The subsequent rich-text/clipboard integration gate
passed **271 Core tests, 91 control tests, and two demo tests**, also unfiltered
with no failures or ignores. The [rich-text/clipboard report](editor-rich-text-clipboard.md)
records the native integration coverage and command separately.

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

## Explicit remaining differences

- Protected multicaret replacement is atomic: an operation intersecting a
  protected interval is rejected. Upstream can delete only writable portions of
  a selection. Interval subtraction is exposed for hosts needing that policy.
- Logical rectangles retain virtual cells and support padded insertion and
  bounded multiline paste. Distinct virtual carets at the same physical line
  end are explicitly rejected for combined replacement. Native pixel-based
  rectangles, proportional fonts and bidirectional visual columns require text
  view validation separately from the logical service tests.
- Vertical movement in the service layer uses tab-expanded grapheme cells and
  retains its desired column across short lines. Wrapped visual-line movement
  and preferred pixel X positions belong to the text view.
- C# indentation is the pinned heuristic state machine. Dedicated raw-string
  and interpolation-expression parsing and syntax-tree formatting are absent
  from that source and are not added here. Locale-specific casing and every
  upstream input mode still require separate work. Title-case conversion throws
  in the pinned source and is not exposed as a working command here.
- Native completion and insight/overload controls, custom completion actions,
  rich descriptions and observable collections are implemented; see
  [editor-completion.md](editor-completion.md). Description placement remains
  inline below the list instead of a secondary side tooltip, and native
  composition replaces the source's literal XAML templates.
- The panel exposes snapshot search-result ranges to the renderer and handles
  literal and regex modes; wildcard matching is available through the service
  factory. Source search-option events and several lifecycle/command aliases
  remain separate presentation work. Localization and public strategy injection
  are potential extensions; the pinned panel has no localization class or
  public strategy setter.
- Structured snippet nodes support text, replacement fields, bound fields,
  selected text, containers and a final caret. Numeric snippet notation rejects
  variables, nested defaults, choices and transformations explicitly. Custom
  active-element implementations and every upstream snippet lifecycle event
  are not exposed.
- Typed XSHD loading, visitors, serialization and archived v1 conversion have
  regression coverage. The pinned upstream loader has XSD validation commented
  out; the Rust parser/compiler likewise validates supported structure and
  values directly, without checking unknown attributes against that schema.
  Default explicit captures, option scopes and bounded rule regions are
  adapted, but comprehensive .NET regex differential parity remains absent.
  Unicode/culture behavior and capture histories retain differences documented
  in [editor-xshd-runtime.md](editor-xshd-runtime.md). Source positions and XML
  formatting are not preserved by serialization.
- XML folding preserves previous regions while the input is malformed instead
  of publishing a partial recovery parse. Its generated document offsets are
  still UTF-16.
- The document highlighter uses stable live-line identities and a lazy dirty
  index instead of compressed state lists. Tracker registration initializes
  live-line metadata in O(total lines) if it has not already been built;
  highlighted results remain lazy. State-only updates use full line evaluation
  rather than a separate optimized scanner. These choices have regression
  coverage, but no benchmark-equivalence claim. Trait composition replaces
  protected virtual subclass hooks. Notifications include the actual last
  line when its incoming state changes, correcting a stricter condition in the
  pinned concrete source to follow the interface's inclusive contract.
- Rich-text writers are owned buffers rather than live-document insertion
  writers; the corresponding upstream model-writer implementation was commented
  out. Runtime XSHD retains font families, exact weights/slants and both
  decorations; native `Run`, brush and clipboard adapters are documented in
  [editor-rich-text-clipboard.md](editor-rich-text-clipboard.md). HTML
  customization uses concrete options instead of overridable style/class
  callbacks. Export uses
  flat spans, `<br/>`, pixel font sizes and ten-digit CF_HTML offsets. UTF-8
  projections replace lone surrogates; a surrogate pair split by formatting
  takes the leading code unit's style.

None of these differences is counted as behavioral parity merely because a
similarly named Rust type exists.
