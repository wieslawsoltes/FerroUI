# FerroUI

[![CI](https://github.com/wieslawsoltes/FerroUI/actions/workflows/ci.yml/badge.svg)](https://github.com/wieslawsoltes/FerroUI/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

FerroUI is a cross-platform UI framework for Rust: a retained-mode control library with a styled property system, data binding, XAML markup, control themes and a composition-based renderer. It is a port of the Avalonia UI framework to idiomatic, safe Rust that keeps the original architecture, contracts and behaviour, so that applications, themes and markup written against that model carry over with their structure intact.

> **Status: early development.** The framework builds and runs on macOS, paints themed controls and passes a large ported test suite, but the public API is not stable, several subsystems are incomplete and nothing has been published to crates.io. See [Project status](#project-status).

![Fluent theme rendered by FerroUI](docs/porting/images/themed_window_fluent.png)

## Highlights

- **Complete object model.** Styled and direct properties with value priorities, inheritance, coercion and change notification; logical and visual trees; routed events; layout; input, focus and gestures.
- **Styling and theming.** Selectors, styles, control themes, resources, theme variants and transitions. The Fluent and Simple themes are included.
- **Data binding.** Compiled and reflection-style binding paths, converters, multi-bindings, data validation and templates.
- **XAML.** A port of the XamlX compiler front end together with the framework's transformers and markup extensions. Documents are loaded at run time today; ahead-of-time compilation to Rust is designed and in progress.
- **Composition renderer.** A compositor with a render loop, dirty-region tracking and server-side visuals, drawing through a backend-neutral drawing context.
- **Swappable backends.** Rendering and windowing sit behind contracts. The current backends are Skia (Graphite on Metal, Ganesh, raster) for rendering, HarfBuzz for text shaping and a native macOS windowing backend.
- **Safe Rust.** The controls, markup, XAML and theme crates contain no `unsafe` code. It is confined to a small part of the object model core and to the rendering, text shaping, platform and interop backends.

## Project status

| Area | State |
|---|---|
| Property system, trees, layout, input, styling, resources | Ported |
| Data binding, animation, transitions | Ported |
| Media, text layout, fonts | Ported |
| Compositor and render loop | Working; composition animations and custom visuals outstanding |
| Skia and HarfBuzz backends | Working |
| macOS platform | Windows, input, clipboard, drag and drop, menus, tray icon; storage dialogs and accessibility outstanding |
| Controls | Most of the control set, including items controls, text input, menus, pickers, calendar, pages and automation peers |
| XAML | Run-time loader with the upstream test suites; ahead-of-time compiler outstanding |
| Themes | Fluent and Simple load for every ported control |
| Windows and Linux platforms | Not started |
| Browser (WebAssembly) | In progress |
| Vello render backend | Planned |

Roughly 70 percent of the upstream API surface that is in scope has a counterpart. The per-subsystem state is tracked in [docs/porting/CRITICAL-PATH.md](docs/porting/CRITICAL-PATH.md) and the per-member state in [docs/porting/TRACKING.md](docs/porting/TRACKING.md).

## Requirements

- Rust 1.89 or later
- macOS 11 or later with the Xcode command line tools (the only supported platform at present)
- Python 3 for the maintenance scripts (not needed to build)

The first build compiles or downloads Skia and can take a while.

## Getting started

Clone the repository and build the workspace:

```sh
git clone https://github.com/wieslawsoltes/FerroUI.git
cd FerroUI
cargo build --workspace
```

Run the examples:

```sh
# A window with a few controls built in code
cargo run -p ferroui-desktop --example hello_window

# Themed controls, Simple theme (add -- --fluent for the Fluent theme)
cargo run -p ferroui-themes-simple --example themed_window

# The ControlCatalog sample application
cargo run -p control-catalog-desktop
```

Run the tests:

```sh
cargo test --workspace
```

### A minimal application

An application is a class derived from `Application` that creates its main window when the framework has finished initialising. The window content can be built in code:

```rust
fn create_main_window() -> Ref<Window> {
    let text = TextBlock::new();
    text.set_text(Some("Hello from FerroUI"));
    text.set_font_size(24.0);

    let button = Button::new();
    button.set_content(Some(Rc::new("Click me".to_string())));
    button.click(|_, _| println!("Button clicked"));

    let panel = StackPanel::new();
    panel.set_spacing(12.0);
    panel.children().add(text);
    panel.children().add(button);

    let window = Window::new();
    window.set_title(Some("Hello".to_string()));
    window.set_content(Some(Control::boxed(panel)));
    window
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = AppBuilder::configure::<App>()
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}
```

The complete program, including the `App` class declaration, is [`hello_window.rs`](src/FerroUI.Desktop/examples/hello_window.rs). The API is still changing; that example is kept building and is the authoritative reference.

### Markup

Views can be written in XAML. FerroUI uses its own XML namespace:

```xml
<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="Hello">
  <StackPanel Margin="16" Spacing="8">
    <TextBlock Text="{Binding Greeting}" />
    <Button Content="Click" Command="{Binding ClickCommand}" />
  </StackPanel>
</Window>
```

## Repository layout

The layout follows the upstream project so that files can be compared side by side.

| Path | Crate | Contents |
|---|---|---|
| `src/FerroUI.Base` | `ferroui-base` | Object model, layout, input, styling, media, text, animation, binding core, compositor |
| `src/FerroUI.Controls` | `ferroui-controls` | Application model, top levels, the control set, automation peers |
| `src/Markup/FerroUI.Markup` | `ferroui-markup` | Binding path parsing and reflection-style bindings |
| `src/Markup/FerroUI.Markup.Xaml` | `ferroui-markup-xaml` | XAML run-time library and markup extensions |
| `src/Markup/FerroUI.Markup.Xaml.Loader` | `ferroui-markup-xaml-loader` | Compiler extensions and the run-time loader |
| `external/XamlX` | `xamlx` | XAML compiler front end |
| `src/Skia/FerroUI.Skia` | `ferroui-skia` | Skia render backend |
| `src/HarfBuzz/FerroUI.HarfBuzz` | `ferroui-harfbuzz` | HarfBuzz text shaping backend |
| `src/FerroUI.Native`, `native/FerroUI.Native` | `ferroui-native` | macOS platform backend and its native library |
| `src/FerroUI.MicroCom` | `ferroui-microcom` | COM-style interop runtime used by the native backend |
| `src/FerroUI.Desktop` | `ferroui-desktop` | Desktop entry point and platform detection |
| `src/FerroUI.Themes.Fluent`, `src/FerroUI.Themes.Simple` | `ferroui-themes-fluent`, `ferroui-themes-simple` | Themes |
| `samples` | `control-catalog`, `control-catalog-desktop`, `mini-mvvm` | Sample applications |
| `tests` | | Ported XAML test suites |
| `docs/porting` | | Porting guide, design notes and tracking documents |
| `scripts` | | Code generators, upstream synchronisation and status tooling |

## Documentation

- [Porting guide](docs/porting/PORTING-GUIDE.md): the class model, naming and mapping rules, and the conventions every ported file follows.
- [Critical path](docs/porting/CRITICAL-PATH.md): subsystem status and known gaps.
- [Tracking](docs/porting/TRACKING.md): generated per-file, per-type and per-member status.
- [XAML design](docs/porting/xaml.md): the markup pipeline, the run-time loader and the planned ahead-of-time compiler.
- [Browser platform design](docs/porting/browser-platform.md).

## Contributing

The project is in an early phase and its structure is still moving, so please open an issue to discuss a change before sending a pull request. Contributions are expected to follow the [porting guide](docs/porting/PORTING-GUIDE.md): port files one to one with their tests, keep behaviour identical to the reference implementation, and keep `cargo test --workspace` green.

## License

FerroUI is licensed under the [MIT License](LICENSE).

It is derived from other open source projects, and some files are carried under their original licenses. See [NOTICE.md](NOTICE.md) and the component-level notices it refers to for the full attributions and license texts.
