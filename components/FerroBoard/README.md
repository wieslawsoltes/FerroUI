# FerroBoard

Rust-native on-screen keyboard for FerroUI, ported from ScreenKeyboardControl
`3694995fabbebc45a801a455880e3d31888367f1`. Public controls remain
`OnScreenKeyboard` and `ScreenKeyboardHost`. The complete typing, layout, gesture,
glide, language, emoji and clipboard engines are included, with native text/password
adapters, themes, icons and the full editable sample. Core has no controls dependency.

## Try it

From the FerroUI repository, run `cargo run --locked -p control-catalog-desktop`
and open **Text → FerroBoard**. The compiled XAML page contains message, email,
number and protected-password inputs. Its keyboard is private and in memory;
learning, clipboard history and emoji history are disabled. The catalog retains
its default WinUI theme.

The full configuration sample remains independently runnable:

```sh
cargo run --locked --manifest-path components/FerroBoard/Cargo.toml -p ferroboard-sample
```

For the private full sample without file persistence, append `--example memory_sample`.
On macOS the standard sample may request access to its chosen data directory.

## Use the library

Applications use an ordinary path dependency on `src/FerroBoard.Controls` inside
this directory. The Core and Core.Ferro crates remain separately available.
ControlCatalog demonstrates these direct dependencies without adding keyboard
packages to the global SDK dependency or release-version tables. Call `ferroboard_controls::register_types()` at the application
composition root before loading library XAML. See the independent
[compiled/runtime XAML consumer](tests/FerroBoard.Native.Consumer) for input overrides,
settings, custom themes/layouts, targets, providers and actions.

This is a source-integrated component; publishing these crates to crates.io is
separate. It is an independent Cargo workspace with its own Apache-2.0 license and
version. It does not inherit the framework's MIT package license.

## Isolation

All keyboard code, resources, fixtures, reference inputs, platform launchers and
the sample live here. The parent SDK only adds dependency declarations, the catalog
page/registration, CI coverage and documentation. No keyboard state machine or
editor logic is placed in framework controls or catalog view models. There is no
vendored/private SDK and no new CLR/BCL emulation.

To separate the component later, move this directory and repoint its six shipping
Cargo manifests to the selected SDK, then remove the catalog wiring and parent
workspace exclude entry. [Integration details](docs/PORTING.md).

## Validation and limits

[Validation](docs/VALIDATION.md) records the actual branch checks. Headless and
compiled-consumer tests do not certify touch hardware, every desktop/mobile/browser
platform, accessibility clients, IME, fonts or signed distribution. A-001 remains
user-deferred. Clipboard monitoring uses a500ms poll; default host reveal uses native
minimal reveal. Nine historical sub-minute local-time offsets have documented native
interoperability differences.

The unchanged C# source is a reference only. Ordinary Rust builds/tests require no
.NET installation. Migration documents and source/member maps retain their original
audit scopes and do not imply full API/runtime emulation or complete platform parity.

## License

Apache-2.0: [LICENSE](LICENSE), [NOTICE](NOTICE), [third-party notices](THIRD-PARTY-NOTICES.md).
FlorisBoard, Unicode, icons, native font fixtures and dependency license records are
retained alongside their data. [Provenance](docs/provenance.json).
