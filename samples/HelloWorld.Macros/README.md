# HelloWorld.Macros

The [HelloWorld](../HelloWorld) sample, one window with "Hello, World!" in the WinUI 3 theme and a
button that counts its clicks, written with the `rsx!` macro and `#[component]` (docs/declarative-ui.md, section 5), the count of the button a signal (section 6).

```sh
cargo run -p hello-world-macros
FERROUI_SMOKE_EXIT_MS=2000 cargo run -p hello-world-macros   # closes the window after two seconds
cargo build -p hello-world-macros --profile dist             # the build to ship: fat LTO, symbols stripped
```

Its size, next to the other styles and the XAML version, is in
[docs/binary-size.md](../../docs/binary-size.md), section 3.6.
