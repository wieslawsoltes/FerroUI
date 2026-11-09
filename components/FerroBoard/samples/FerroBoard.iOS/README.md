# FerroBoard iOS sample

This separate native launcher uses the pinned FerroUI iOS backend, Skia/HarfBuzz,
and the complete shared FerroBoard.Sample MainView. The root library/desktop and
independent consumer locks remain separate. It is an application, not an iOS
keyboard extension or a replacement for system IME services.

As the pinned upstream mobile documentation explains, focusing ordinary text
fields can also display the OS keyboard. This launcher keeps the shared native
text fields and does not suppress UIKit's keyboard. Custom `ITextInputTarget`
fields can avoid system text responders. Native coexistence and placement still
need device/simulator verification.

Foundation NSFileManager resolves the user-domain Application Support directory in
the app sandbox. The existing file storage adapter writes into its FerroBoard
subdirectory. There is no desktop HOME or SSD path fallback on the device.
Startup reports an unavailable native locale/directory explicitly. Background
notifications save the live session; returning to foreground retains it. UIKit
may kill a suspended process without a final callback, so termination is not
claimed to trigger a save. Settings/history/dictionary formats stay unchanged.

Build from the FerroBoard root, keeping all caches and generated output on SSD:

```sh
source scripts/env.sh
cargo build --locked --offline --manifest-path samples/FerroBoard.iOS/Cargo.toml \
  --target aarch64-apple-ios-sim \
  --target-dir /tmp/ferrokeyboard/native-ios-host-stage/cross-target
python3 scripts/bundle_ios.py \
  --binary /tmp/ferrokeyboard/native-ios-host-stage/cross-target/aarch64-apple-ios-sim/debug/ferroboard-ios \
  --out /tmp/ferrokeyboard/ios-bundles/FerroBoard.app
```

The bundler verifies the executable's actual iOS platform/minimum version,
writes native scene/launch metadata and ad-hoc signs simulator bundles. It never
creates/boots a simulator, installs an app or changes another port's device.
A device bundle is unsigned and needs a real signing identity/provisioning profile.
The host macOS test build does not open an application window.

Consult docs/NATIVE-IOS-HOST.md for actual results and limits. Compilation and
packaging do not establish UIKit input/focus, native touch/multitouch, physical
devices, sandbox persistence through relaunch or accessibility parity. The default
system emoji fallback is used; this launcher does not download optional fonts.
