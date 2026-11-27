# ferroui-osprey-native

An Osprey Native host on FerroUI. An application written in the
[Osprey](https://github.com/Nimblesite/osprey) language with Osprey Native (Elm-like: a `view` of
the state and `update(message, store)`) and compiled as a C library runs on FerroUI unchanged:
its regions are drawn with FerroUI controls, its scenes with FerroUI's drawing context, and its
events go back through the typed calls of the framework. It is the same design as Osprey's own
GPUI host. The design, the mapping of every node kind and the differences from the GPUI host are
in [docs/osprey-native-host.md](../../docs/osprey-native-host.md).

The crate is not published yet (`publish = false`).

## Build and run

The host speaks the boundary of Osprey 0.29: the exports `start` and `dispatch` and JSON
envelopes. Build the application as a library for the host first:

```sh
osprey <app folder> --compile --target=macos -o <build folder>/libOspreyApp.a
```

Then build the host with the application linked:

| Variable | Meaning |
|---|---|
| `OSPREY_NATIVE_APP_DIR` | The folder that holds `libOspreyApp.a` (required to link an application) |
| `OSPREY_RUNTIME_DIR` | The folder of `libosprey_runtime_macos_arc.a`: the folder of the `osprey` compiler (macOS) |
| `OSPREY_NATIVE_LINK_SEARCH`, `OSPREY_NATIVE_LINK_LIBS` | More link folders (`:`-separated) and cargo link specs (`,`-separated) an application needs |
| `OSPREY_NATIVE_APP_NAME` | The application's name: the folder of its store; `probe` turns on the probe test |
| `OSPREY_NATIVE_TITLE`, `OSPREY_NATIVE_WINDOW` | The window's title and size (`WIDTHxHEIGHT`, default `1200x800`) |

```sh
OSPREY_NATIVE_APP_DIR=<build folder> OSPREY_RUNTIME_DIR=<osprey folder> \
    cargo run -p ferroui-osprey-native --features desktop --profile fast-release
```

`--smoke` runs without a window: alone, it starts Osprey's probe application, taps Add and
expects "Count 1"; with a path, it replays an application's recorded `smoke.jsonl`.

Without `OSPREY_NATIVE_APP_DIR` nothing is linked: the library and its tests build, the tests run
against a stand-in application written in Rust, and the binary says how to link one.

## Tests

`cargo test -p ferroui-osprey-native --lib` runs the schema, envelope, scene and mapping tests and
drives the FerroUI tree with the stand-in. With the probe linked and `OSPREY_NATIVE_APP_NAME=probe`,
`the_linked_probe_counts` drives Osprey's probe through the real boundary and through the tree.
With `OSPREY_NATIVE_SOURCE` naming the `src` folder of the framework's `@osprey/native` package,
the schema is checked against the framework's own `Node` type.

## License

MIT. The protocol and the host design follow Osprey's specifications and its GPUI host (MIT,
Copyright (c) 2025-2026 NIMBLESITE PTY LTD); no Osprey source is copied. See [NOTICE.md](NOTICE.md).
