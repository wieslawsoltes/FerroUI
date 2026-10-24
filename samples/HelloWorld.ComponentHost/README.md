# HelloWorld.ComponentHost

The [HelloWorld](../HelloWorld) sample, one window with "Hello, World!" in the WinUI 3 theme and a
button that counts its clicks, written with XAML (`App.xaml`, `MainWindow.xaml`, compiled by the build script), whose window shows the component `Hello` (written with `rsx!`) through a `ComponentHost` (docs/declarative-ui.md, section 9.1).

```sh
cargo run -p hello-world-component-host
FERROUI_SMOKE_EXIT_MS=2000 cargo run -p hello-world-component-host   # closes the window after two seconds
cargo build -p hello-world-component-host --profile dist             # the build to ship: fat LTO, symbols stripped
```

Its size, next to the other styles and the XAML version, is in
[docs/binary-size.md](../../docs/binary-size.md), section 3.6.
