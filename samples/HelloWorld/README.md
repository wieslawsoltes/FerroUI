# HelloWorld

The smallest FerroUI application: one window, "Hello, World!" in its middle, in the WinUI 3 theme
(`ferroui-themes-winui`). It is written in XAML: `App.xaml` declares the theme
(`<WinUITheme />`), `MainWindow.xaml` the window, and the build script compiles both to Rust
(`build.rs`, docs/porting/xaml.md, section 9).

The crate depends only on what the application uses (the framework, the compiled markup and its
runtime library, the WinUI theme and the desktop entry point), so its executable is the size
floor of an application in that theme ([docs/binary-size.md](../../docs/binary-size.md)).

```sh
cargo run -p hello-world
FERROUI_SMOKE_EXIT_MS=2000 cargo run -p hello-world   # closes the window after two seconds
cargo build -p hello-world --profile dist             # the build to ship: fat LTO, symbols stripped
```

The executable of the `dist` profile is `target/dist/hello-world` (`hello-world.exe` on Windows).

## The same application in each style of the declarative Rust API

The same window, written with `ferroui-view` (docs/declarative-ui.md), with a button that counts
its clicks:

| Crate | Style |
|---|---|
| [`hello-world-builders`](../HelloWorld.Builders) | The builders, the count a signal |
| [`hello-world-macros`](../HelloWorld.Macros) | `rsx!` and `#[component]`, the count a signal |
| [`hello-world-hooks`](../HelloWorld.Hooks) | A component that runs again, `#[component(rerender)]` with `use_signal` |
| [`hello-world-component-host`](../HelloWorld.ComponentHost) | This XAML application, with the window's content a component (`ComponentHost`) |

Their sizes, next to the ControlCatalog and the gallery of the WinUI theme, are in
[docs/binary-size.md](../../docs/binary-size.md), section 3.6.
