# HelloWorld.Hooks

The [HelloWorld](../HelloWorld) sample, one window with "Hello, World!" in the WinUI 3 theme and a
button that counts its clicks, written with a component that runs again (`#[component(rerender)]`, docs/declarative-ui.md, section 7), its count in a hook (`use_signal`).

```sh
cargo run -p hello-world-hooks
FERROUI_SMOKE_EXIT_MS=2000 cargo run -p hello-world-hooks   # closes the window after two seconds
cargo build -p hello-world-hooks --profile dist             # the build to ship: fat LTO, symbols stripped
```

The theme is constructed from the units the sample uses, 16 of the 166 of the WinUI theme: the
build script names the theme (`build.rs`, `ferroui_build::Build::theme`), the build finds the
controls the sources make, and `main.rs` constructs the theme with the table it wrote
([docs/xaml-themes.md](../../docs/xaml-themes.md), section 3, "An application without markup").

Its size, next to the other styles and the XAML version, is in
[docs/binary-size.md](../../docs/binary-size.md), sections 3.6 and 4.24.
