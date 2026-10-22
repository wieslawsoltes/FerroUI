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
| `Editing/TextArea.cs`, `EditingCommandHandler.cs`, `TextEditorOptions.cs`, `Indentation/DefaultIndentationStrategy.cs` | `editing/editor_session.rs` | Shared editing session, multicaret replacement, grouped undo, delete, selection, indentation, newline and tab commands |
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
| `Highlighting/DocumentHighlighter.cs` | `highlighting/document_highlighter.rs` | Lazy line cache invalidation following document edits |
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
| `Editing/EditingCommandHandler.cs`, `TextAreaDefaultInputHandlers.cs`, `Search/SearchCommands.cs` | `commands.rs` | Framework `ICommand` actions with weak editor ownership, read-only/selection/history enabled state, protected edit checks and disposable enabled-state notifications |

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
committed. Its property-specific observers avoid subscribing to the editor's
expensive full-text value. Hosts can call `refresh` after changing an external
protected-range provider. Clipboard execution uses the editor's asynchronous
platform clipboard methods; paste availability reflects edit permissions rather
than synchronously inspecting clipboard contents.

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
- upstream whole-word search cases, original-context range matching, captures,
  UTF-16 result offsets and zero-length replace-all;
- folding state preservation after edits, malformed-region atomicity and XML regions;
- completion ranking, wrapping selection and tracked insertion ranges;
- linked snippets through ordinary document edits, undo/redo and field traversal;
- protected snippet mirror rejection and preserved completion keyboard selection;
- XSHD multiline stacks, lookahead, span boundaries and bounded zero-width recursion;
- compilation and execution of every one of the 21 bundled XSHD definitions.

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
- Full C# indentation/reformatting, line transforms and every upstream editing
  command are not implemented by the current default-indentation service.
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
- The document highlighter preserves the cache before an edit and rebuilds
  subsequent requested lines. It does not yet stop invalidation at an unchanged
  state/content suffix.
- Rich-text model/writers, HTML clipboard serialization and highlighting line
  merge APIs are not implemented here.

None of these differences is counted as behavioral parity merely because a
similarly named Rust type exists.
