# WinUI Gallery on FerroUI

A FerroUI port of [Microsoft WinUI Gallery](https://github.com/microsoft/WinUI-Gallery) at commit `14061eeac21c38ae99f302edf3d9136d4f652c4c` (W10 of [docs/porting/winui-controls.md](../../docs/porting/winui-controls.md)). It shows the native WinUI controls of `ferroui-controls` (NumberBox, RadioButtons, RatingControl, AutoSuggestBox, InfoBar, InfoBadge, ProgressRing, the icon elements, NavigationView, Frame and FramePage) and the upstream-ported controls in the pages of the gallery, with the WinUI theme.

The whole pinned catalog is browsable: 126 pages and 350 sample definitions, each with the original XAML and code of its source file, the path of that file and its SHA-256. A subset of the pages have native examples: reviewed adaptations of the original markup in `Pages/`, with their handlers in Rust. `port-status.json` records the state of every page; every page with native examples is `partial`, since it does not reproduce every example, event, binding or pixel of the original.

## Run

```sh
cargo run -p winui-gallery --features desktop
cargo run -p winui-gallery --features desktop -- --page=Button
```

The feature `desktop` uses FerroUI's desktop host and the WinUI theme. Without it, the library has the catalog, the gallery view and the examples, and can be hosted by an application:

```rust
let gallery = winui_gallery::GalleryView::new()?;
window.set_content(Some(std::rc::Rc::new(gallery.root())));
gallery.navigate("Slider");
// Keep `gallery` alive with the view: its handlers hold weak references.
```

The feature `capture` renders pages to PNG files without a window, with Skia and HarfBuzz:

```sh
cargo run -p winui-gallery --features capture --bin winui-gallery-capture -- --pages=NumberBox,RatingControl --output-dir=captures
cargo run -p winui-gallery --features capture --bin winui-gallery-capture -- --all --dark --output-dir=captures/dark
```

The capture host has only the fonts it embeds: glyphs of the symbol font (`SymbolThemeFontFamily`, Segoe Fluent Icons on Windows) show as missing glyphs where the system has no such font.

## Structure

- `Gallery.xaml` and `gallery.rs`: the shell (search, categories, history, theme, the source browser). Only the selected page is created; replacing a page releases its controls and handlers.
- `catalog_data.rs`: the pinned catalog, written by `generate_catalog.py` from a checkout of WinUI Gallery at the pin. `assets-manifest.json` and `generate_assets.py` record the images copied from the gallery.
- `Pages/*.xaml` and `examples.rs`: the native examples, with their adaptations recorded per page. The original WinUI XAML of a page can also be loaded through the WinUI XAML importer (`ferroui-markup-xaml-winui`) from the page ("Load XAML preview"), which reports what it cannot translate.
- `*_samples.rs`: the Rust behaviour of the examples (input, feedback, media, navigation, platform).

## Tests

```sh
cargo test -p winui-gallery --features theme --lib
```

The tests create every native page, drive the examples through their controls (key events, as a user does) and check the catalog: every page and sample with its source path and the SHA-256 of its file. Tests of controls whose template only the WinUI theme gives run with the theme (the feature `theme`).

The workflow `.github/workflows/winui-gallery.yml` runs them, and renders the pages nightly and on demand.

## Attribution

See [NOTICE.md](NOTICE.md): the catalog, the sample sources and the images come from WinUI Gallery (MIT, Copyright (c) Microsoft Corporation).
