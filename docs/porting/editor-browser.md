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
| Theme and font service | Native `FluentTheme`, bundled Inter for the UI and the unchanged Noto Mono face for code. Both register through FerroUI's font manager; the browser does not assume system fonts exist. The Noto face's hash, origin and OFL license accompany the sample and generated site. |
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
thread-enabled toolchain and browser isolation. The acceptance gate introduced
here targets the ordinary single-threaded application; threaded editor parity
is not inferred from loader compatibility.

## Browser acceptance gate

`.github/workflows/editor.yml` has a separate linked-application browser job in
addition to the framework-independent `wasm32-unknown-unknown` tokenizer oracle.
It builds the actual editor site and runs:

```sh
node scripts/browser/tests/editor_demo.test.mjs \
  --screenshot target/browser/screenshots/editor-demo.png
```

The test harness opens the site in headless Chromium using the same DevTools
protocol harness as the existing browser platform tests. It runs once with
`Software2D` and once with `WebGL2`. Tests change state through real browser
input; the diagnostic exports cannot change editor content.

The checks require a rendered, nonblank canvas; native Noto Mono family resolution; cached native visual lines;
successful Rust grammar tokenization; real browser IME composition and commit of
supplementary-character and combining sequences, unchanged document during
preedit and UTF-16 caret positions; ordinary keyboard text and newline input;
undo and redo; plain-text and HTML
clipboard publication, cut and paste with browser permissions; native completion
accepted with Tab; linked snippet updates and field navigation; native search
popup input, replace-all and dismissal; folding and unfolding; wrapping; theme switching;
C# grammar switching; viewport resizing; no renderer panic, editor error,
grammar error or browser console failure; and one load of the shared scanner
module.

Local and hosted validation results must be recorded separately: adding this
gate does not by itself establish that it has passed. The source/asset inventory
and official tokenizer oracle remain independent gates. A browser build check
without linking, page startup and real input is not counted as application
validation.

## Remaining platform verification

The browser acceptance cases do not establish IME candidate UI correctness,
mobile touch selection, all bidirectional caret and font-fallback cases, denied
clipboard permission/fallback paths and every rich clipboard format, native OS input parity,
screen-reader text ranges, or arbitrary user-provided generators and embedded
controls. These remain explicit integration tests for the broader full-port
goal. The sample remains a small editor demonstration and does not claim IDE or
language-server functionality.

The browser platform currently forwards ordinary text through key events and
IME text through composition events. Its `beforeinput` callback only adjusts
selection; an isolated `insertText` event without either a key or an active
composition is not a covered input path. The acceptance test starts a real CDP
IME composition before committing it, and uses real key events for ordinary
typing. Emoji-picker and other composition-free text insertion need separate
browser-platform coverage.
