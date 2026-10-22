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
| `Editing/RectangleSelection.cs` | `editing/rectangle_selection.rs` | Rectangular logical-line ranges and tab-expanded visual columns |
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
| `Highlighting/HighlightingColor.cs`, `HighlightingRule.cs`, `HighlightingSpan.cs`, `HighlightingRuleSet.cs`, `IHighlightingDefinition.cs`, `Xshd/V2Loader.cs`, `Xshd/XmlHighlightingDefinition.cs`, `Xshd/HighlightingLoader.cs` | `highlighting/highlighting_definition.rs` | Named/inline colors, XSHD v2 parsing, local/inline/external rulesets, bounded regex compilation |
| `Highlighting/Xshd/V1Loader.cs` and legacy TeX resource | `highlighting/highlighting_definition.rs` | Legacy literal spans, keywords, digit rules and preceding/following markers needed by the bundled TeX definition |
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
conservative color syntax and finite positive font sizes, and hyperlinks reject
executable schemes. Document-range export validates the highlighter's document
identity and preserves CR, LF, CRLF, blank and trailing lines. The document
exporters consume `IHighlighter`, allowing hosts to supply another highlighting
implementation through the same contract. CF_HTML generation returns a payload
with CRLF headers and UTF-8 byte offsets. It does not write to an operating-system
clipboard.

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
- tab-expanded rectangles and protected-range intersections;
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
- HTML escaping, CSS/hyperlink rejection, blank/trailing lines and CF_HTML
  offsets in UTF-8 bytes.

These are executable regression tests, not a complete upstream differential
suite. The central port inventory records missing counterparts separately.

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
- Rectangular selections clamp to existing text. Virtual-space insertion,
  pixel-based rectangles and bidirectional visual columns still need the text
  view integration.
- Vertical movement in the service layer uses logical UTF-16 columns. Wrapped
  visual-line movement and sticky preferred X positions belong to the text view.
- C# indentation is the pinned heuristic state machine. Dedicated raw-string
  and interpolation-expression parsing and syntax-tree formatting are absent
  from that source and are not added here. Locale-specific casing and every
  upstream input mode still require separate work. Title-case conversion throws
  in the pinned source and is not exposed as a working command here.
- Completion insight/overload-provider windows and rich completion descriptions
  require separate presentation work. The popup currently displays plain text
  descriptions and concrete `CompletionItem` values.
- Search localization and custom search strategy injection remain separate
  extension surfaces. The panel exposes snapshot result ranges to the renderer.
  The built-in panel handles literal and regex modes; wildcard matching is
  available through the service factory.
- Structured snippet nodes support text, replacement fields, bound fields,
  selected text, containers and a final caret. Numeric snippet notation rejects
  variables, nested defaults, choices and transformations explicitly. Custom
  active-element implementations and every upstream snippet lifecycle event
  are not exposed.
- XSHD v2 runtime semantics are covered by representative tests. XML schema
  validation, XSHD serialization/visitors and comprehensive .NET regex
  differential parity remain. The legacy importer supports bundled TeX; the
  full historical v1 escape-character and `@` mini-language is not implemented.
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
  out. Native UI `Run` construction, colorizer integration, font families,
  strikethrough, arbitrary font weights and framework brush/resource resolution
  are not represented by the current core color model. HTML customization uses
  concrete options instead of overridable style/class callbacks. Export uses
  flat spans, `<br/>`, pixel font sizes and ten-digit CF_HTML offsets. UTF-8
  projections replace lone surrogates; a surrogate pair split by formatting
  takes the leading code unit's style.

None of these differences is counted as behavioral parity merely because a
similarly named Rust type exists.
