# Editor demo

A FerroUI application with a main view compiled from `MainView.xaml`. Its
standalone hosts use the Fluent theme. The Control Catalog also hosts the view
under Text → FerroEdit. The Rust, C#, JavaScript, and HTML examples use the
public `ferroui-edit` API, bundled TextMate grammars, and Light+/Dark+ themes.

```sh
cargo run -p editor-demo
cargo run -p editor-demo -- --dark
cargo test -p editor-demo --lib --no-default-features
```

The toolbar changes the sample language, opens search or replace, displays
completion at the current prefix, inserts a linked snippet, reindents source,
folds explicit regions, and toggles wrapping and the theme. Switching language reloads that
example. Search supports case matching, whole words, regular expressions,
capture replacement and replace-all. Completion preserves text-area focus and
accepts Enter, Tab or double-click. JavaScript completion also includes bundled
snippets inside HTML `<script>` blocks. Type `for` and open completion to expand
the For Loop snippet. Snippets select each editable field in turn;
Tab and Shift+Tab navigate, Escape ends synchronization, and linked name fields
update together in one undo operation.

The C# example installs the C# indentation strategy, so Enter derives the new
line's indentation from its language context. Reindent formats selected lines,
or the whole document when no text is selected, in one protected edit and undo
group. Rust, JavaScript, and HTML use their bundled indentation rules for Enter,
typing, and Reindent.
In the JavaScript example, Enter inside `/** */` expands the documentation
comment and places the caret after the new ` * ` prefix. The bundled language
configuration also continues that prefix on subsequent lines. The same rules
apply inside `<script>` blocks in the HTML example, including indentation after a
block opener followed by a line comment.
Ordinary clipboard paste in the language examples uses the bundled indentation
rules when enabled by the editor strategy. The demo enables this option and
preserves string content. Paste a nested JavaScript block inside `<script>` to
see the indentation adjust in a single undo step.
The examples use the bundled pairing rules for ordinary typing. Type `(` to
insert a pair, or select text and type `[` to surround the selection.

`Ctrl/Cmd+F` opens search, `Ctrl/Cmd+H` opens replacement, `Ctrl+Space` opens the
sample completion list. The status line reports one-based logical line and
column, document line count and UTF-16 length. Region folding recognizes
`// region title` / `// endregion` and `#region title` / `#endregion`; it is a
small example provider rather than a Rust or C# parser. Applications can supply
their own `NewFolding` ranges.

The desktop entry point supports the same smoke options as the existing
samples. A graphical display and the normal desktop dependencies are required:

```sh
FERROUI_SMOKE_EXIT_MS=4000 cargo run -p editor-demo
FERROUI_SMOKE_SCREENSHOT=/tmp/editor-demo.png \
FERROUI_SMOKE_SCREENSHOT_MS=2000 FERROUI_SMOKE_EXIT_MS=4000 \
  cargo run -p editor-demo -- --dark
```

The library tests load the actual XAML view and exercise snippet and language
switching behavior. Rendering, clipboard and input-method integration also need
desktop checks; unit tests alone do not establish platform parity.

The desktop feature is enabled by default and uses FerroUI's standard desktop
backend. On Windows the sample can also use the system Direct2D backend:

```powershell
cargo run --locked -p editor-demo --no-default-features --features direct2d
```

On Linux, the Wayland feature opens the editor directly on a Wayland compositor.

```sh
cargo run --locked -p editor-demo --no-default-features --features wayland
```

Library tests need neither Skia downloads nor a display. A headless
application-lifetime smoke run is available for build environments:

```sh
FERROUI_SMOKE_EXIT_MS=2000 cargo run -p editor-demo \
  --no-default-features --features headless
cargo run -p editor-demo --no-default-features --features headless -- --smoke-actions
```

The `--smoke-actions` mode opens the search and completion overlays, replaces a
search result, commits completion through a routed Tab key, edits and navigates
a linked snippet, folds regions, enables wrapping, changes the theme, and loads
the C# example. It then checks an encoded file round trip, the modified flag,
and the layout and removal of an inline button. The last step checks that
routed keyboard navigation through a two-line CRLF document publishes the
updated active caret and selection ranges. It allows a dispatcher frame
between stages and shuts down with
exit code zero after its checks pass, or one if an action fails. The same mode
can be used with the desktop backend for a visible integration check.

The default headless renderer records layout and control behavior; it does not
produce a pixel-accurate editor screenshot. Use the desktop backend for visual
inspection and screenshot capture.

## Windows assistive technology

The editor workflow runs the Direct2D sample on a private desktop and uses the
Windows UI Automation client to check selection, glyph bounds, the active
TextPattern2 caret, and the matching system caret. Run the same check locally
from the repository root.

```powershell
$env:FERROUI_TEST_WINDOWS_DESKTOP = 'FerroEditUIA-' + [guid]::NewGuid().ToString('N')
cargo test --locked -p editor-demo --no-default-features --features direct2d --test editor_clipboard_windows -- --fixture text-automation
```

The same test executable accepts `--fixture inline-control` to check
native pointer and keyboard input in a hosted TextBox, its exposed text
range, and focus handoff when scrolling evicts it.

Run the Direct2D sample in a Windows test session. Use this text in the editor
and in Notepad so both apps exercise the same caret moves.

```text
alpha beta
omega gamma
```

With Narrator active and scan mode off, focus the text in each app. Press
`Ctrl+Home`, `Down`, `Right`, then `Shift+Right`. Check what Narrator
announces for each move against Notepad. Narrator key + Alt + X opens the
speech recap for reviewing the announcements.
[Microsoft Narrator keyboard commands](https://support.microsoft.com/en-us/accessibility/windows/narrator/appendix-b-narrator-keyboard-commands-and-touch-gestures)

For Magnifier, enable text cursor following and move the caret between
separated lines using the keyboard. Check that the magnified view follows
the caret while the pointer stays still. The smoke actions verify the
editor's text ranges and selection notifications. These live checks verify
the behavior of the assistive technology itself.
[Microsoft Magnifier tracking settings](https://support.microsoft.com/en-us/accessibility/windows/magnifier/chapter-2-choosing-view)

## Browser application

`samples/EditorDemo.Browser` hosts this exact view through FerroUI's browser
single-view application lifetime. It uses native FerroUI controls, text layout,
input, popup, clipboard and rendering services. TextMate's bundled Oniguruma
WebAssembly scanner initializes before the view creates its grammars.

```sh
scripts/browser/setup.sh
source .tools/env.sh
scripts/build-browser.sh editor-demo-browser
node scripts/browser/serve.mjs target/browser/editor-demo-browser
```

Open the printed address. `?RenderingMode=Software2D` selects the raster canvas;
`?RenderingMode=WebGL2` selects the WebGL canvas; `?ThemeVariant=dark` starts with
the dark editor theme. Every required script, grammar, theme, font and scanner
is in the built site or embedded application module; there is no CDN dependency.

The linked browser acceptance checks drive real pointer, keyboard and text-input
events, inspect native control geometry and read back canvas pixels:

```sh
node scripts/browser/tests/editor_demo.test.mjs \
  --screenshot target/browser/screenshots/editor-demo.png
```

They cover both canvas modes, scanner-backed language highlighting, UTF-16 text
input, undo/redo, plain-text/HTML clipboard transfer, completion, linked snippets,
search/replace, folding, wrapping, theme
and language switching, resize and console errors. See
[browser integration](../../docs/porting/editor-browser.md) for the startup and
validation contracts and the remaining platform checks.
