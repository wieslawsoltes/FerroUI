# WinUI rich text: RichEditBox, RichTextBlock and the document model

This page is the plan and the register of the port of WinUI's rich text: the controls `RichEditBox`, `RichTextBlock` and `RichTextBlockOverflow`, the block document model (`Block`, `Paragraph`, `TextPointer`, the highlighters, typography), the text object model of RichEdit (TOM: `RichEditTextDocument`, `RichEditTextRange`, `ITextSelection`), RTF and the rich clipboard, the UI Automation text pattern, and the input method (IME) connection the editor uses on every platform. It records the source and its pin, the owner's decisions, the inventory, the rules that keep upstream-ported code unchanged, the landing plan and the register of translations.

The work comes from pull request #243 (branch `feat/winui-rich-text-rust-port`, head `111ce1ba`), which remains a draft reference and is not merged as it is. It lands as a series of small pull requests from `main`, R0 to R13 (section 9); #243 is closed after the last of them, with a comment that maps each area to its pull request.

## 1. Source and pin

- **Reference:** [microsoft/microsoft-ui-xaml](https://github.com/microsoft/microsoft-ui-xaml), branch `winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (MIT), the pin of the WinUI theme (`winui-theme.md`), of the WinUI controls (`winui-controls.md`) and of every other WinUI port of the repository.
- **The pin of #243:** #243 names `fe244ca6ac4d482712b234a9de3cf771cc6481de` everywhere (507 commits not in `7e4b0c2`, and not a descendant of it; merge base `8a98b88b`). The sources the port cites are byte-identical at both commits: `git diff 7e4b0c2 fe244ca6` is empty over
  - `dxaml/xcp/core/text` (230 files), `dxaml/xcp/core/native/text` (62 files) and `dxaml/xcp/components/text` (23 files, with their isolated tests);
  - `dxaml/xcp/dxaml/lib/RichEditBox_Partial.cpp`, `RichTextBlock_Partial.cpp`, `TextBoxBase_Partial.cpp` and `dxaml/xcp/core/core/elements/`;
  - `controls/dev/CommonStyles/RichEditBox_themeresources.xaml`, `TextBox_themeresources.xaml` and `controls/dev/TextCommandBarFlyout`;
  - every other upstream path named in the Rust sources of #243 (39 paths; checked on 2026-10-30).

  No file whose name relates to text, typography, input scope, hyperlinks, paragraphs or highlighting differs between the commits, apart from the ink highlighter button of `controls/dev/InkToolbar` (not part of this port). So the change of pin changes no code; every landing pull request names `7e4b0c2` in its notices, headers and links.
- **What the reference does not contain:** the RichEdit engine (`WinUIEdit.dll`, the `Microsoft.UI.Text.winmd` producer) and the LineServices implementation behind `Ptls-min.h`. The repository has the host of the editable RichEdit and the adapters around LineServices, not the engines. The editor, TOM and RTF of this port are therefore a Rust implementation of the documented contract (the TOM and RTF 1.9.1 documentation, and the XML documentation of `Microsoft.UI.Text`), checked against the host code (`TextBoxBase.cpp`, `RichEditBox.cpp`) and against observations of the system Msftedit (section 8).
- **Attribution:** the section "WinUI" of `src/FerroUI.Controls/NOTICE.md` (and of `src/FerroUI.Base/NOTICE.md` and the platform crates where code is derived) names the files of each area as it lands, at the pin `7e4b0c2`; the converted RichEditBox template is covered by `src/FerroUI.Themes.WinUI/NOTICE.md`.

## 2. Decisions (owner, 2026-10-30)

The owner accepted the twelve recommendations of the review of #243.

| # | Decision |
|---|---|
| D1 | #243 stays a draft reference and is closed after the last landing pull request (R13), with a comment mapping each area to its pull request. It is not rebased or merged. |
| D2 | The controls, the document model, TOM and RTF go into the core crate `ferroui-controls`. The code pages of RTF (`encoding_rs`) are behind an optional cargo feature of `ferroui-controls`, `rtf-codepages`; without it RTF reads and writes the ANSI code page 1252 and Unicode escapes only. The public surface stays small (section 7). |
| D3 | `RichEditBox.Document` and `RichEditBox.TextDocument` both return the TOM, `RichEditTextDocument`, as in WinUI. The shared document model under the controls is a FerroUI-named property (`RichDocument`). `RichTextBlock.RichDocument` and the `Paragraph` content of `RichEditBox` are recorded as FerroUI additions. |
| D4 | The Fluent and Simple themes get no RichEditBox template; `RichEditBox` is in `WINUI_ONLY_CONTROLS` (`src/FerroUI.Controls/testing/winui_only.rs`). Its template comes from the WinUI theme only. |
| D5 | The platform IME changes apply only to clients that opt in to the new connection (`TextInputMethodClient::input_method_connection` returns `Some`), which RichEditBox does. The legacy path, and so every TextBox, keeps upstream behaviour on every backend. TextBox moves over to the connection later, in its own pull request with recorded deviations. |
| D6 | The rework of the `Popup` and `PopupFlyoutBase` lifecycle lands in R4 with recorded deviations and direct Popup tests. The upstream order of `Popup.IsOpen` and the flyout's `IsOpen` is kept unless a test shows the other order is needed. |
| D7 | The name-scope lookup of text elements moves into `TextElement::find_name`; the template root of a `TemplatedControl` gets no name scope of its own (#243's C10 is not taken). |
| D8 | Input scopes reach the backends through the attached-property accessor `TextInputOptions::get_input_scope_names(element)` in 0.4. The struct field `TextInputOptions::input_scope_names` is added only in 0.5. |
| D9 | Undo units are deltas (the range, the removed and inserted fragments, the selections before and after), as RichEdit keeps them. R7 lands at least snapshots shared between neighbouring units; deltas follow in F1 if they are not in R7. |
| D10 | A `LineBreak` is one unit on every platform, U+000B (vertical tab) in the model, as TOM keeps a soft break. It is converted to the platform line ending only on plain-text export. |
| D11 | No evidence JSON is committed. The observation extractor, the case tables it writes (`*_native_cases.rs`, with the hash of their source) and an on-demand oracle job are kept. |
| D12 | The pin is `7e4b0c2` (section 1). |

## 3. Inventory

| WinUI type or service | FerroUI type | Existing counterpart | Treatment | PR |
|---|---|---|---|---|
| `RichEditBox` | `RichEditBox` (`TemplatedControl`), `RichEditBoxTextPresenter`, the IME client, the format commands, the context menu | `TextBox` + `TextPresenter` (plain text) | New type. It shares TextBox's `CaretBlinkInterval` and the context-menu path; its presenter derives from `RichTextBlock`, not `TextPresenter`. | R8 |
| `RichTextBlock` | `RichTextBlock`, `rich_text_layout` | `TextBlock`, `SelectableTextBlock` | New type, not a TextBlock subclass. The layout reuses `TextFormatter`, `TextLineImpl` and `TextLineBreak` and stacks paragraphs, line bounds, trimming, hit testing and pages itself. | R6 |
| `RichTextBlockOverflow` | `RichTextBlockOverflow`, the overflow links | none | New: one document and selection across a weakly linked chain of containers. | R6 |
| `Block`, `Paragraph`, `BlockCollection` | `documents::{Block, Paragraph, BlockCollection}` | none (main has the inlines only) | New. `Paragraph` reuses `InlineCollection`; there is no second inline model. | R5 |
| `Hyperlink` (inline) | `documents::Hyperlink` and its automation peer | `HyperlinkButton` (a control) | New, a different concept; navigation reuses the launcher. | R5, R6 |
| `TextPointer`, `LogicalDirection` | `documents::{TextPointer, TextPosition, TextRange, TextGravity}` | `media::text_formatting::LogicalDirection` | New. No second public `LogicalDirection`: the direction of a pointer (`TextPointer::logical_direction`) is `TextGravity`. | R5 |
| `TextHighlighter`, `TextRange` (XAML value) | `documents::{TextHighlighter, TextHighlightRange, TextHighlighterCollection}` | none | New. The XAML name `TextRange` maps to `TextHighlightRange`, because `documents::TextRange` is the live range of the model. | R5 |
| `Typography` attached properties, `TextElement.Language` | `documents::Typography`, `TextElement::language`, `media::font_typography` enums | `FontFeatures` on TextBlock and TextPresenter | Opt-in extension of the existing text elements; nothing changes until a property is set. | R1, R5 |
| (none; the shared document) | `documents::RichTextDocument`, its snapshots, character and paragraph formats, link fields | none | New, the model under both controls. | R5, R7 |
| TOM: `RichEditTextDocument`, `RichEditTextRange`, `ITextSelection`, `ITextCharacterFormat`, `ITextParagraphFormat`, the units, find, case and stream options | `rich_text::{RichEditTextDocument, RichEditTextRange, RichEditTextSelection, ...}` | none | New, a Rust implementation of the documented contract over the editor. | R7 |
| RTF, the rich clipboard | `rich_text` RTF reader and writer, the transfer formats | the plain-text `IClipboard` path of TextBox | New; code pages behind `rtf-codepages` (D2). | R7 |
| Link fields | `rich_text` link fields and field stories | none | New. | R7 |
| UIA Text, Text2, TextRange, TextChild, TextEdit | `automation::provider::i_text_*`, the rich automation peers; on Windows the text pattern nodes | none (TextBox exposes Value only) | New, used by the rich peers only; TextBox's peer is unchanged. | R6, R9 |
| `InputScope`, `InputScopeName`, `InputScopeNameValue` | `ferroui_base::input::{InputScope, InputScopeName, InputScopeNameValue}`, the attached property `TextInputOptions.InputScopeNames` | `TextInputOptions::content_type` | Additions (D8). | R3 |
| The exact-edit IME connection (text services of `TextServicesHost`) | `ferroui_base::input::text_input::{TextInputMethodConnection, TextInputMethodEdit, TextInputMethodRangeGeometry}` | `InputMethodManager`, `TextInputMethodClient` | Additions with defaulted trait methods; legacy clients keep upstream behaviour (D5). | R3 |
| Tab stops, the line-metrics mode, the font-fallback scale | `media::text_formatting::{TextTabStop, TextLineMetricsMode, FontFamilyLineMetrics}`, `media::FontFallbackMapping` | `TextFormatter` | Opt-in extensions; the defaults are identical to upstream. | R1 |
| `ContextMenuOpening`, `ContextMenuEventArgs` | `ContextMenuEventArgs` and a typed routed event | `ContextRequestedEventArgs` | Added beside the existing arguments; generic code reaches the rich views through an extension point, not a downcast. | R4, R6 |
| RichEditBox template | `src/FerroUI.Themes.WinUI/Controls/RichEditBox.xaml`, converted by `scripts/winui-theme/convert.py` | none | WinUI theme only (D4). | R8 |

WinUI has no `FlowDocument`; the shared `RichTextDocument` plays that role.

## 4. Architecture

- **One model, two views.** `RichTextDocument` holds the blocks, the formats, the live ranges and the link fields. `RichTextBlock` shows it read-only (with selection), `RichEditBox` edits it through `RichTextEditor`, and the TOM (`RichEditTextDocument`) is a facade over the same editor. A `RichTextBlockOverflow` chain shows one document across containers.
- **Offsets are UTF-16 code units**, the public index unit of FerroUI. Internal UTF-8 storage is reached only through conversions that reject positions inside a surrogate pair; grapheme, scalar and structural positions are separate quantities. A paragraph mark is one unit, and so is a `LineBreak` (D10).
- **Layout** produces `TextLine`s with the existing formatter and draws with `DrawingContext` on the backend the platform chose; core code has no DirectWrite, Direct2D, COM, Win32 or native RichEdit dependency.
- **Edits are transactions.** Each mutation reports its real replaced range and inserted length; live ranges, `TextPointer`s, the view's selection and the link fields are rebased from that report, not from a diff of the old and new text (#243's S2). Typing, including Backspace, merges into one undo unit until a break (a caret move, a format change, a paste); a failed group rolls back fully.
- **Reentrancy:** no `RefCell` borrow is held across a call that can run application code; a reentrant edit returns an error instead of panicking.
- **Input methods:** the editor is a client of the exact-edit connection (D5). Composition never adds undo units; one commit is one unit; a programmatic edit, a pointer press or a focus loss ends a composition explicitly, raising `TextCompositionEnded` and resetting the IME (#243's S4 and S5).

## 5. Changes to upstream-ported code

The rules of `winui-controls.md` section 6 (O10) apply to every change this port makes to an upstream-ported file:

1. A change on a default path (one every application uses) is avoided. Where the rich controls need something new, it is an extension point or an opt-in member whose default reproduces upstream exactly.
2. When a default-path change is needed (an upstream bug the rich controls hit), it is recorded as a deviation in `DEVIATIONS.md` with a test of the default path, in the pull request that makes it.
3. Formatting-only changes to upstream-ported files are not made (no reflow with another line width).
4. Generated files (`builders.rs`, `rust_paths*.rs`, the emitter outputs, every `compiled_xaml.rs`) are regenerated with their scripts, never merged by hand. The WinUI theme's `compiled_xaml.rs` stays one file.

The changes #243 made on default paths and what the landing pull requests do with them:

| Area (#243 hunk) | #243 | Landing |
|---|---|---|
| IME on every backend (P1, P6, P8, P20, P24, P28, P33, P34) | applied to every TextBox | only for clients with the connection (D5); legacy unchanged; R9 to R12 |
| `InputMethodManager` (B5) | rewritten; no reset of the new root's IME | legacy clients keep the upstream reset (the expectation of `input_tests.rs` restored); R3 |
| Focus (B3) | a nested focus change aborts the outer sequence | recorded with a direct nested-focus test, or left out; R3 |
| `TextInputOptions` (B7) | a new public field | attached-property accessor (D8); R3 |
| Popup and PopupFlyoutBase (C8, C11) | lifecycle rewrite, reversed IsOpen order | recorded deviations, upstream order kept (D6); R4 |
| Generic ContextMenu and PopupFlyoutBase (C13) | downcasts to `RichTextBlock` | an extension point; R4 |
| TemplatedControl name scope (C10) | every template root gets a name scope | moved into `TextElement::find_name` (D7); R5 |
| Inline culture (C17) | every run gets the resolved language | `None` unless `Language` is set; R5 |
| Formatter (B17, B24, B25, B29) | features split shaping; line-break drop; bidi caret; justification order | each recorded as an upstream bug fix with a default-path test; R2 |
| Deferred-content name scope (M1 of the review) | weak chain of declaring scopes | re-applied on the hot-reload `DeferredContent` with its row updated, in the pull request that needs it |
| Browser input element (P28) | `<input>` becomes `<textarea>`, paste unblocked | only for clients with the connection; `browser-platform.md` section 14; R10 |

## 6. Templates and themes

- `RichEditBox` gets its template from `src/FerroUI.Themes.WinUI/Controls/RichEditBox.xaml`, converted from `controls/dev/CommonStyles/RichEditBox_themeresources.xaml` at the pin by `scripts/winui-theme/convert.py`, with its resources in the theme's `Resources/`. The differences of the conversion are recorded in `docs/porting/winui-theme/DEVIATIONS.md`: a `MenuFlyout` stands for `TextCommandBarFlyout` (as for TextBox), there is no selection flyout, and `x:Load` is not supported.
- Fluent and Simple have no RichEditBox template (D4). In an application on those themes `RichEditBox` shows nothing; use `WinUITheme`.
- `RichTextBlock` and `RichTextBlockOverflow` are not templated and need no theme.
- R8 updates the tables of the WinUI theme that say RichEditBox has "no FerroUI control" (`scripts/winui-theme/waivers.json`, `docs/porting/winui-theme/TRACKING.md`, G-23 of the theme's DEVIATIONS) and reruns `scripts/winui-theme/status.py`.
- The ControlCatalog gets one page per control, in its section (Text): RichTextBlock with its overflow in R6, RichEditBox in R8.

## 7. API stability

- Everything here is an addition to `ferroui-base` and `ferroui-controls`; within 0.4 the public API only grows. The API gate of `ci.yml` (`cargo-semver-checks` against 0.4.0, and the snapshot of property defaults) guards it.
- **The public surface is the WinUI-facing one:** the controls, the document types, `RichEditTextDocument`, `RichEditTextRange`, `RichEditTextSelection` and their option enums. The editor (`RichTextEditor`, its edit kinds and selections), the snapshots (`DocumentSnapshot`, `InlineSnapshot`), the RTF reader, writer and limits, and the field stories are `pub(crate)`, or `#[doc(hidden)]` where a test crate needs them.
- The error enums (`TextInputMethodError`, `TextTabError`, the edit, document and RTF errors) and the option enums that may grow are `#[non_exhaustive]` from the start.
- Existing public functions keep their signatures. Where a backend needs another form (the browser's `interop::input_helper`), a new function is added and the old one stays as a wrapper.

## 8. Evidence and the Msftedit oracle

- #243 committed 1.57 million lines of evidence JSON (per-run observations of the system Msftedit, inventories and maps). None of it lands (D11).
- What lands: the observation extractor (`scripts/winui-rich-text/`), the case tables it writes (`src/FerroUI.Controls/rich_text/*_native_cases.rs`, included from test modules only, each with the hash of its source), and the oracle that characterises the system Msftedit, moved from `ferroui-win32`'s tests into a tool and run by an on-demand job (R9, R13).
- Native observations are test oracles for the editor; a case table is regenerated with the extractor, never edited by hand.

## 9. Landing plan

Each pull request carries its tests, its `DEVIATIONS.md` rows and a line in `CHANGELOG.md` under "Unreleased"; generated files are regenerated with their scripts; the checks of "Source conventions" (both generators, the thread-local check, the upstream-name check) pass. The defects the review of #243 found are fixed in the pull request that carries the code.

| PR | Content | Depends on | Fixes | Windows VM |
|---|---|---|---|---|
| R0 | This page | none | | no |
| R1 | Base text extensions, all opt-in: tab stops and their layout, the line-metrics mode, the font-fallback mapper and scale, the typography enums | none | | no |
| R2 | Base text fixes, each an upstream bug fix with a default-path test: shaping split by font features, the line break's release of unconsumed runs, the bidi caret and bounds at line ends, the collapsed-line assertion, justification in logical order | R1 | | no |
| R3 | Input: the defaulted client methods, the input-scope types and attached property, the exact-edit connection with `#[non_exhaustive]` errors, the commit routing of text input; `InputMethodManager` with legacy clients on upstream behaviour; focus changes with a direct test | none | B5, B7 | no |
| R4 | Popups: the Popup and PopupFlyoutBase lifecycle with direct tests, the zero-size anchor, `show_at_rect_core`, the context-request placement, the context-menu extension point | none | C13 | no |
| R5 | Documents: the inline invalidation and culture, `Block`, `Paragraph`, `BlockCollection`, `TextPointer`, the highlighters, typography on TextBlock and TextPresenter (with a layout benchmark), `TextElement::find_name`, the markup transformer of `TextHighlightRange` | R1 | C10, C17, S11 | no |
| R6 | RichTextBlock, RichTextBlockOverflow, the rich layout, the context menu through the R4 hook, the automation providers and peers; catalog page | R4, R5 | S10 | no |
| R7 | Editor services: `RichTextEditor`, TOM, units, find, case, stream, clipboard, RTF (`rtf-codepages`) | R5 | M6, S1, S2, S3, S6, S8 (at least shared snapshots), N3 | no |
| R8 | RichEditBox, its presenter, IME client and format commands; the converted WinUI template; `WINUI_ONLY_CONTROLS`; the theme tracking; catalog page | R3, R6, R7 | S4, S5, N1, N2, N4, S13 (D3) | yes: RichEditBox in the WinUI theme |
| R9 | Windows: IMM32 for connection clients, the UIA text pattern, the WinUI CoreText host; the oracle as a tool | R8 | P1-P22 as opt-in | yes: Japanese, Chinese and Korean IMEs in TextBox and RichEditBox; Narrator and Inspect |
| R10 | Browser: the connection protocol with the old public functions kept; `browser-platform.md` section 14 | R8 | P28-P32 as opt-in | no |
| R11 | FreeDesktop, Wayland, X11: the connection for IBus and Fcitx, the IBus content-type property non-fatal | R8 | P23-P27 | Ubuntu VM (IBus, Fcitx) |
| R12 | macOS, iOS, Android: content hints, input options; `patches/0003` regenerated | R8 | P33-P37 | iOS simulator and Android emulator (never with a VM) |
| R13 | CI: the regeneration check repository-wide and the on-demand oracle job, actions pinned by SHA | R9 | | no |
| F1 | Performance: a layout cache per paragraph, the selection drawn as an overlay, an incremental IME preview, the fast path near fields, delta undo units if not in R7 | R8 | S7, S8, S9 | no |

R1, R3 and R4 are independent; R6 to R8 are sequential; R9 to R12 may land in any order after R8.

## 10. Register of translations

The architectural translations of #243's own register (RT-A, RT-P and RT-I), condensed. They are how WinUI concepts are expressed in FerroUI, not differences from upstream-ported code; those are in `DEVIATIONS.md`.

| ID | WinUI | FerroUI |
|---|---|---|
| RT-A01 | C++ dependency objects, COM wrappers, WinRT collections | `Ref<T>`, styled and direct properties, shared collection handles, Rust errors and routed events |
| RT-A02 | LineServices with DirectWrite analysis and Direct2D rendering | The FerroUI formatter, shaping, Unicode and font services and `DrawingContext`; pixels differ by backend |
| RT-A03 | `WinUIEdit.dll`, `Microsoft.UI.Text.winmd` | A Rust editor and TOM implementing the documented contract (section 1) |
| RT-A04 | The windowless text host, TSF and UI-manager callbacks | The FerroUI IME client, dispatcher, focus, routed input and clipboard |
| RT-A05 | Separate headers, implementations and generated WinRT files | One Rust module per header and implementation pair; markup registration replaces the ABI plumbing |
| RT-A06 | Windows Unicode classification tables | The Unicode services of `ferroui-base` |
| RT-I01 | `TextPointer`, `PlainTextPosition`, document and range positions | `documents::text_position` and the range methods of the model and views |
| RT-I02 | Linked text containers, block layout, `RichTextBlockOverflow` | One model, one primary view and weak overflow links; the content source is a layout-resolved weak cache (RT-I38) |
| RT-I03 | Text and paragraph formatting properties | `TextElement`, `Inline`, `Block`, `CharacterFormat`, `ParagraphFormat` |
| RT-I04, RT-I18, RT-I19 | RTF and the clipboard of RichEdit; code pages and font charsets; style sheets and field results (RTF 1.9.1) | The RTF reader and writer, the transfer formats, `encoding_rs` tables behind `rtf-codepages` |
| RT-I05 | Hyperlink activation and navigation | The inline `Hyperlink`, host hit testing, focus tracking and the launcher |
| RT-I06 | `RichEditBox`, `TextBoxBase`, `TextServicesHost` | `RichEditBox : TemplatedControl` with one `RichEditBoxTextPresenter : RichTextBlock` and the editor |
| RT-I07, RT-I13 | Editing, clipboard and composition notifications; TextEdit composition notifications | Model notifications, routed events of the controls, cause-aware provider observations |
| RT-I08, RT-I12, RT-I15, RT-I21, RT-I23, RT-I26, RT-I32, RT-I35, RT-I36 | `RichEditTextDocument`, `ITextRange`, `ITextSelection`: streams, case, Home/End, Up/Down/Left/Right, typing options, gravity, `SetText` endpoints | Facades over the editor and the model, sharing its transactions |
| RT-I09 | TextHighlighter, highlight regions, TextHighlightMerge | `TextHighlighter`, `TextHighlightRange`, shared collections, a half-open region merge |
| RT-I10 | The UIA text provider wrappers and text attributes | Portable Text, TextRange, Text2, TextChild and TextEdit providers; on Windows the automation nodes |
| RT-I11 | `TextElement.Language`, `LanguageTag` | Inherited culture values and character-format patches |
| RT-I14 | Inherited presentation and format dirtiness | Property provenance and `IInlineHost::invalidate_layout` |
| RT-I16, RT-I27 | Text-service callbacks; browser `beforeinput` and composition | The exact-edit connection (D5) |
| RT-I17 | `RichEditBox_themeresources.xaml`, `TextCommandBarFlyout` | The converted WinUI control theme and a `MenuFlyout` with the owner's format commands |
| RT-I20 | Input dependencies of RichEditBox: candidate-window alignment, `ContextMenuEventArgs`, `InputScope` | The input-scope types and the typed context-menu event |
| RT-I22, RT-I37 | `ContextMenuOpening`, `TextControlFlyoutHelper` | A typed routed event and the existing context-menu and popup lifecycle; overflow flyouts resolve through the primary view |
| RT-I24 | Copy, Cut, Paste, CanPaste of ranges and the document | Injected asynchronous clipboard services |
| RT-I25 | `Typography` and its 43 attached-property families | Inherited attached properties, seven enums, and patches over the font-feature services |
| RT-I28, RT-I29, RT-I30 | IBus, AppKit and IMM32 input hints | The IBus `ContentType` property, an optional native options interface, `SetInputScopes` on the window; for connection clients only (D5) |
| RT-I31 | Paragraph stacking and `TextLineBounds` | The shared formatter with an opt-in line-metrics mode |
| RT-I33 | `TextElement.FindName` | `TextElement::find_name` over the nearest logical name scope (D7) |
| RT-I34 | `ITextRange.Link` | Projected field ranges over the model |

What remains outside the port (RT-P of #243): the exact LineServices metrics, the parts of TOM and RTF not yet implemented (OLE objects, tables, math), proofing, prediction, handwriting and touch grippers, and pixel parity with native text. Each member that is not implemented stays absent; none returns an inert success.

## 11. Deviations

- Differences from WinUI of the controls and the TOM are listed below as each area lands.
- Changes to upstream-ported code are in `DEVIATIONS.md`: "WinUI additions to upstream controls", the text and input sections, and the platform sections (Windows, browser, X11 and Wayland, Android, iOS, the native backend).
- Differences of the converted RichEditBox template are in `docs/porting/winui-theme/DEVIATIONS.md`.

### 11.1 RichTextBlock and RichTextBlockOverflow (R6)

| WinUI | FerroUI | Why |
|---|---|---|
| `RichTextBlock` has `Blocks` only; its document is internal. | The shared document is also the property `RichDocument` (`rich_document_property`); `Blocks` is the content property. | Decision D3: one model under the view and the editor; a FerroUI name, so it never reads as `RichEditBox.Document`. |
| Paragraphs are formatted by LineServices and drawn with DirectWrite. | Paragraphs are formatted by the FerroUI text formatter (with the opt-in line-metrics mode of R1) and drawn through `DrawingContext`. | RT-A02: pixels differ by backend; line breaking and metrics follow the shared formatter. |
| `RichTextBlockAutomationPeer` exposes the Text pattern. | Until R8, the peer is a Text element named by the document text, without the Text pattern. | The text pattern is built on the text object model (R7, R8). |
| `RichTextBlockOverflow` falls back to the master's context flyout (`GetContextFlyout`). | The same, through the `Control::effective_context_flyout` virtual (R4) instead of a type test in the flyout code. | Layering (S10). |
| The selection context menu of WinUI (`TextCommandBarFlyout` with Copy) opens by default. | Only a configured `ContextMenu` or `ContextFlyout` opens; `ContextMenuOpening` is raised before it. | The automatic selection menu of the legacy selection manager is not projected (RT-I22, RT-I37). |

### 11.2 The editor, the text object model, RTF and the clipboard (R7)

| WinUI | FerroUI | Why |
|---|---|---|
| `WinUIEdit.dll` (RichEdit) edits the story; TOM is its COM interface. | `RichTextEditor` edits the shared `RichTextDocument`; `RichEditTextDocument`, `RichEditTextRange` and `RichEditTextSelection` are facades over it. The editor, the RTF reader and writer and the field stories are `#[doc(hidden)]`. | RT-A03: the engine is not in the reference; the documented contract is implemented (S12: only the WinUI-facing types are API). |
| Undo keeps one unit per typing group, including Backspace (`EM_STOPGROUPTYPING`). | The same: typing, exact IME insertions at the caret and Backspace at a collapsed caret merge into one unit until a caret move, a format change, a paste or Delete; a unit shares its snapshots with its neighbours. Units are document snapshots, not deltas (F1). | #243 review M6, S1, S8; decision D9. |
| A soft line break (`LineBreak`, VT) is one character in TOM. | One unit, U+000B, in the model on every platform; plain text on the clipboard gets the platform's line terminator. | Decision D10 (#243 used `\r\n` on Windows: two units). |
| Live ranges follow the edits RichEdit makes. | Ranges, `TextPointer`s and link fields follow the replacement the editor made; a text diff is used only for writes that bypass the editor. | #243 review S2. |
| Removing a paragraph mark keeps the format of the trailing paragraph. | The same for typing over the mark, Backspace, Delete and Cut. | #243 review S6. |
| RTF reads every Windows code page. | With the cargo feature `rtf-codepages` (encoding_rs); without it, the code page 1252 and Unicode escapes only. | Decision D2. |
| `UndoLimit` limits the undo stack. | It limits the redo stack too. | #243 review N3. |

### 11.3 RichEditBox (R8)

| WinUI | FerroUI | Why |
|---|---|---|
| `Document` and `TextDocument` return the text object model. | The same (`document()`, `text_document()`); the shared document is the property `RichDocument`, and `Paragraph` content in markup fills it (FerroUI additions). | Decision D3 (#243 had `Document` as the shared model, S13). |
| Programmatic edits, undo and paste during a composition end it with `TextCompositionEnded` and reset the input method. | The same: a composition the editor ends outside the control's own composition calls raises `TextCompositionEnded` and resets the IME. | #243 review S4. |
| A click or focus loss terminates the composition and keeps its text (`TextBoxBase.cpp`). | The same: the preview is committed, then the IME is reset (#243 discarded it). | #243 review S5. |
| `TextCompositionStarted` precedes the first `TextChanging` of a composition. | The same, at the caret with no composed text yet. | #243 review N1. |
| `SelectionStart` and `SelectionLength` are current in `TextChanging`. | The selection is read from the editor, so it is the edit's. | #243 review N4. |
| Shift+Enter inserts a soft line break. | It inserts a paragraph, as #243 did, until the Windows check confirms RichEdit's behaviour. | #243 review N2 (open). |
| The context menu is `TextCommandBarFlyout`. | A `MenuFlyout` with the same commands (`docs/porting/winui-theme/DEVIATIONS.md`, G-30). | FerroUI has no command bar flyout. |
| The template is in every theme. | Only the WinUI theme has it (`WINUI_ONLY_CONTROLS`); Fluent and Simple show nothing. | Decision D4. |

