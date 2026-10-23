# Editor demo

A FerroUI application whose main view is loaded from `MainView.xaml`
through the runtime XAML loader. It uses the Fluent theme and the public
`ferroui-edit` API. The Rust and C# examples use the bundled TextMate grammars
and Light+/Dark+ themes.

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
accepts Enter, Tab or double-click. Snippets select each editable field in turn;
Tab and Shift+Tab navigate, Escape ends synchronization, and linked name fields
update together in one undo operation.

The C# example installs the C# indentation strategy, so Enter derives the new
line's indentation from its language context. Reindent formats selected lines,
or the whole document when no text is selected, in one protected edit and undo
group. The Rust example uses the default previous-line indentation strategy;
it does not provide a Rust formatter.

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
backend. Library tests need neither Skia downloads nor a display. A headless
application-lifetime smoke run is available for build environments:

```sh
FERROUI_SMOKE_EXIT_MS=2000 cargo run -p editor-demo \
  --no-default-features --features headless
cargo run -p editor-demo --no-default-features --features headless -- --smoke-actions
```

The `--smoke-actions` mode opens the search and completion overlays, replaces a
search result, commits completion through a routed Tab key, edits and navigates
a linked snippet, folds regions, enables wrapping, changes the theme, and loads
the C# example. It allows a dispatcher frame between stages and shuts down with
exit code zero after its checks pass, or one if an action fails. The same mode
can be used with the desktop backend for a visible integration check.

The default headless renderer records layout and control behavior; it does not
produce a pixel-accurate editor screenshot. Use the desktop backend for visual
inspection and screenshot capture.

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
