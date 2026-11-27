# FerroRibbon

A ribbon control library for FerroUI, ported from
[RibbonSpace](https://github.com/wieslawsoltes/RibbonSpace). It provides adaptive
layout, command controls, linked Quick Access items, menus, backstage, search,
customization and persistence, usable from Rust and from XAML.

This folder is a Cargo workspace of its own, consumed from this repository by path.
Nothing in the FerroUI workspace depends on it, so a FerroUI application links none of
it unless it adds `ferroribbon` itself.

| Path | Contents |
| --- | --- |
| `src/FerroRibbon.Core` | Models, commands, adaptive layout, KeyTips, search, state; no controls |
| `src/FerroRibbon` | The controls, their themes and markup metadata |
| `src/FerroRibbon.Platform` | Collation and number formatting from the platform (ICU, Windows) |
| `samples/FerroRibbon.Sample` | The gallery: Word, Excel, PowerPoint, Tools, CAD and Settings pages |
| `samples/FerroRibbon.NativeProbe` | A small sample of the layout algorithm on native controls |
| `samples/FerroRibbon.Browser` | The gallery in the browser |
| `tests` | Independent consumers of the libraries and the saved reference data |

## Features

`ferroribbon` has one optional feature. `http-images` loads icons named by `http(s)`
URIs on native targets and links an HTTP client, an async runtime and native TLS. Without
it such an icon reports a load failure. The gallery enables it. The browser target uses
the browser's own `fetch` and does not need the feature.

## Build and test

From this folder:

```sh
cargo test                                   # the libraries and their consumers
cargo test -p ferroribbon-sample             # the gallery's tests (builds a rendering backend)
cargo run -p ferroribbon-sample              # the gallery
cargo run -p ferroribbon-sample -- --word    # one page: --excel, --powerpoint, --tools, --cad, --settings
```

On Linux `ferroribbon-platform` needs the ICU development package and `pkg-config`.

Capture tests are ignored by default. To run them, set `FERRORIBBON_CAPTURE_DIRECTORY`
to an empty absolute directory and pass `--ignored`.

The browser build is described in `samples/FerroRibbon.Browser/README.md`.

## License

MIT, with Apache-2.0 portions in `ferroribbon`. See `LICENSE`, `NOTICE.md` and the
notices next to the code they cover.
