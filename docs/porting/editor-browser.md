# Editor browser integration

`samples/EditorDemo.Browser` is the browser host of the existing `EditorDemo`
runtime-XAML view. This is a FerroUI application linked for
`wasm32-unknown-emscripten`, not a JavaScript editor wrapped in a canvas. It uses
the same `TextEditor`, `TextArea`, editor session, document, folding, completion,
snippet and TextMate services as the desktop host.

## Framework and dependency mapping

| Requirement | Implementation |
| --- | --- |
| Application lifetime and native view | `BrowserApp` creates `EditorDemo` and installs its view with `ISingleViewApplicationLifetime::set_main_view`. |
| Rendering and text layout | Existing `BrowserAppBuilder` selects the browser windowing platform, native FerroUI rendering backend and HarfBuzz shaping. No editor-specific DOM text rendering is introduced. |
| Theme and font service | Native `FluentTheme`, bundled Inter for the UI and Noto Mono for code. FerroUI's font manager uses the ControlCatalog subsets of WenQuanYi Micro Hei and Noto Color Emoji for covered CJK and emoji characters. The generated site includes the font notices and licenses. |
| Runtime XAML | The unchanged `EditorDemo/MainView.xaml` is loaded through the same runtime XAML loader as the desktop application. |
| TextMate scanner | `initializeScanner` awaits `initialize_browser_oniguruma` before `runMain` constructs the grammar registry. The official scanner binary is embedded by `ferroui-textmate`. |
| Scanner module and notices | `ferroui-textmate` writes its flat JavaScript adapter and unchanged license/notice files to `OUT_DIR/browser-site`; `scripts/build-browser.sh` includes them in the final site. |
| Browser host bootstrap | `wwwroot/main.js` uses the standard FerroUI loader, attaches platform exports, awaits scanner readiness and starts the native application. A bootstrap failure is visible in the page and browser console. |
| Editing and popups | Real browser pointer, keyboard and text-input events flow through the platform's native input bridge into existing controls and popup layers. |
| Test observation | Read-only state reports native control bounds/hit testing, document and selection, token cache, fold state, viewport and renderer failures. Composition frame exports synchronize pointer events with the committed hit-test tree. |

The dependency on `editor-demo` disables its default desktop feature. The
browser host installs its own theme and lifetime without depending on a
headless or desktop host. Its Rust entry point does not require a change to the
shared view or a parallel application implementation.

## Startup ordering

The page obtains the module through the production loader and creates the
Emscripten runtime. It then attaches `FerroExports`, awaits the scanner
initializer and calls `runMain`. The last step creates the application, loads
XAML, constructs the registry, lays out controls and starts the normal browser
render loop. The platform closes the splash after its first rendered frame.

Calling `runMain` before scanner initialization is rejected. Calling it again
after startup preserves the existing application and document. Scanner
initialization is idempotent within the JavaScript realm; no grammar is compiled
against an uninitialized engine. The shared framework remains responsible for
the browser's event loop, renderer and clipboard security model.

Build with the repository-pinned browser toolchain:

```sh
scripts/browser/setup.sh
source .tools/env.sh
scripts/build-browser.sh editor-demo-browser
node scripts/browser/serve.mjs target/browser/editor-demo-browser
```

The standard builder supports `--debug`, `--out`, `--threads` and `--both` for
this binary package. Threaded deployment additionally requires the standard
thread-enabled toolchain and browser isolation. The browser test suite
targets the ordinary single-threaded application; threaded editor parity
is not inferred from loader compatibility.

## Browser test suite

`.github/workflows/editor.yml` builds the linked browser application separately
from the framework-independent `wasm32-unknown-unknown` tokenizer oracle. The
`browser` job links and packages the actual editor site, then uploads that exact
site as a one-day artifact. The `browser-conformance` job downloads it and runs:

```sh
node scripts/browser/tests/editor_demo.test.mjs \
  --screenshot target/browser/screenshots/editor-demo.png
```

The test harness opens the site in headless Chromium using the same DevTools
protocol harness as the existing browser platform tests. It runs once with
`Software2D` and once with `WebGL2`. Tests change state through real browser
input; the diagnostic exports cannot change editor content.

The startup checks verify a nonblank canvas, Noto Mono and fallback glyph
resolution for the Japanese text and emoji, cached native visual lines, and
Rust grammar tokenization. The input checks cover real browser IME composition and commit of
supplementary-character and combining sequences, unchanged document during
preedit and UTF-16 caret positions; ordinary keyboard text and newline input;
undo and redo; preservation of host shortcuts and viewport-only Ctrl+Page Down;
rectangular selection extension and contraction with page and document-boundary
keys;
plain-text and HTML
clipboard publication, cut and paste with browser permissions; native completion
accepted with Tab and a click on a suggestion; caret movement and wheel scrolling
over the editor while completion is open, and first-click activation of Find
when focus leaves the editor; linked snippet updates and field navigation; native search
popup input, replace-all and dismissal; folding and unfolding; wrapping; theme switching;
C# grammar switching and Reindent; JavaScript documentation comment expansion
in standalone code and inside HTML `<script>` blocks and indentation after
a JavaScript block opener followed by a line comment; Reindent across nested
HTML and JavaScript; outdent when typing an embedded JavaScript closing brace;
Undo and Redo for multi-character openers, unmatched closers beyond the current
line, nested generated pairs, paired Backspace, single-character IME commit and
an emoji selection surrounded and restored by Undo; viewport resizing; no renderer panic, editor error,
grammar error or browser console failure; and one load of the shared scanner
module. A multiline clipboard paste into JavaScript embedded in HTML verifies
indentation across a dispatcher batch boundary and restores the document with Undo.
The embedded JavaScript completion list also expands the bundled For Loop snippet,
verifies Undo of the prefix replacement, and edits its linked fields.
The JavaScript sample snippet also checks the choice list. The suite opens it
from the toolbar, chooses a value with the keyboard, verifies the linked edit
and next selection, and checks Undo, Redo, and free typing in a choice field.
It also checks that a transformed occurrence mirrors ordinary input while its
field is active, formats the value on Tab, and participates in Undo and Redo.

## IME context on a later line

The linked application selects text on the second logical line with native
keyboard events. The focused browser input mirrors that line and uses its
local UTF-16 selection offsets. The test starts a composition through CDP,
checks that the document stays unchanged during preedit, commits a CJK
character and a supplementary emoji, and undoes the single replacement.

A second check selects backward across the line break, compares the proxy
input's value, range and direction, and verifies the composition replacement
and undo. The single-line proxy uses one space for each CR or LF code unit, so
both ends of its selection continue to address the editor's original UTF-16
text.

The proxy kept the same UTF-16 length and both selection directions.

## Deletion from browser text input

The browser input proxy can receive a backward or forward deletion without a
usable key event. FerroUI dispatches the logical key through its normal input
route so editor handlers, read-only spans, caret stops and undo apply.
Cancelable `beforeinput` is handled before the proxy changes. For `input`-only
events, the editor publishes its resulting text and caret back to the proxy.
Composition and the exact-edit textarea retain their own event paths.

An injected cancelable `beforeinput` at the start of the second line removed
the preceding CRLF even though it was outside the proxy's current line. One
undo restored both code units.

The linked editor regression uses trusted browser input events without keydown
to delete a selection backward and a supplementary character with a combining
mark forward. It verifies the document, proxy value, caret, one undo step and
ordinary physical Delete.

## Browser corrections away from the caret

An `insertReplacementText` event can correct text outside the current
selection. For an `<input>` element, the browser supplies no target ranges.
FerroUI records the displayed text and selection in `beforeinput` and
reconstructs the range from the value after `input`. For an `input`-only
event, it compares with the latest displayed text. A model update or focus
change invalidates a recorded correction. [Input Events Level 2](https://www.w3.org/TR/input-events-2/#interface-InputEvent)

FerroEdit checks the UTF-16 range against its current surrounding text and
applies the replacement and resulting selection in one protected edit. Only
the surrounding lines are sent through the input bridge. Rejected corrections
restore the proxy, and undo restores the original document and selection.

The linked sample replays corrections on a later line with and without
`beforeinput`. It checks the document, CRLF and caret offsets, proxy contents
and undo. The replay covers the event bridge. OS spell-check menus and mobile
keyboards still need live acceptance.

## Touch gestures in the linked sample

The browser suite dispatches touch contacts through CDP. It checks caret
placement on a tap, scrolling with a stable caret and selection on a swipe,
and word selection on a double-tap. It also holds on the first word, extends
across lines, then returns to the original word and releases. The selection
survives and the document stays unchanged.

## The browser accessibility tree

FerroUI's on-demand mirror exposes the editor as a named, multiline textbox.
The mirror listens to the editor's text provider and refreshes the document
value on the next animation frame. The linked regression activates the
mirror, invokes the editor's focus action, and reads the Chromium
accessibility node through CDP. It checks the full value through an edit and
undo, including line breaks, an emoji and a combining character.

The browser input that receives keyboard and IME focus also carries the name
of the focused text peer and links to its mirror. The linked regression checks
the actual focus and name in Chromium and Windows Edge, follows focus to Find,
and checks that the shared input metadata clears when focus leaves the view.

## Apple keyboard layout

The browser registers the same Command and Option editing keymap as the
macOS desktop backend when the page reports an Apple platform. With that
keymap the editor uses Control for rectangular selection from the keyboard and
Option for a rectangle drawn with the pointer, because Control with a click is
the secondary click. Delete Line and Reindent stay on Control+D and Control+I.

The linked regression sets the platform before startup and routes Command
navigation, Find, Select All, Copy, Cut, Paste, and Undo through browser key
events. It also checks a Control+Shift rectangle and reads the published
selection marker through the browser clipboard.

## macOS pasteboard and AppKit text input

The [editor workflow](../../.github/workflows/editor.yml) pairs the linked
Software2D browser editor in Chrome with the native EditorDemo in AppKit.
The peers exchange rectangular and whole-line selections in both directions.
The test checks the browser marker and plain text, the exact document after
paste, and the original document after one undo. The AppKit copy peer stays
alive through paste so Chrome can read its promised custom formats.

Run the test in a dedicated macOS session because it writes to the general
pasteboard. Pass the executable path printed by Cargo as `TEST_EXE`.

```sh
cargo test --locked -p ferroui-native --test text_input_options
cargo test --locked -p editor-demo --test editor_clipboard_macos --no-run
node scripts/browser/tests/editor_clipboard_macos.test.mjs SITE TEST_EXE
```


## Remaining platform work

The browser test cases do not establish IME candidate UI correctness,
touch behavior on physical mobile devices, all bidirectional caret and font-fallback cases, broader
clipboard permissions and fallback on more browsers, every rich format, native OS input parity,
screen-reader text ranges, or arbitrary user-provided generators and embedded
controls. These remain explicit integration tests for the broader full-port
goal. The sample remains a small editor demonstration and does not claim IDE or
language-server functionality.

The browser platform forwards physical keys, IME composition events, and
standalone `input` events through distinct paths. The linked suite exercises
all three. A candidate-window interaction still requires a native IME and a
graphical session.
