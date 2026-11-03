# FerroBoard native sample

The complete passive UI is shared by MainView.xaml. MainWindow is the desktop wrapper;
SampleSession retains its existing public desktop fields. MainViewSession hosts the same
settings/commands/keyboard through a single-view root. Keep the session alive while its view
is in use: unloading saves data and permits reattachment; final session release/Drop saves,
removes subscriptions and retires the owned engine. Native hosts must supply SampleStorage
through AppBuilder::with before setup_with_lifetime. App::save_session and release_session
support native lifecycle adapters. This does not yet provide an iOS executable or package.
See ../../docs/NATIVE-SINGLE-VIEW.md for integration and actual validation limits.

The sample uses the real keyboard engine, bundled resources, native input adapters and public
SettingsPanel. Passive views are compiled XAML; presentation uses actual ReactiveUI. Input, focus,
gestures, rendering and engine synchronization remain in the library.

From the repository root:

```sh
source scripts/env.sh
cargo run --locked -p ferroboard-sample
```

An SSD-local macOS development bundle is available after building:

```sh
source scripts/env.sh
cargo build --locked -p ferroboard-sample
python3 scripts/bundle_sample.py
open /Volumes/SSD/temp/ferrokeyboard/FerroBoard.app
```

The bundler/checker now use native schema2 with the selected SDK patch and MIT license.
Regenerate historical manifests with the current helper. Relocated integration users should
follow INTEGRATION.md for portable bundling with explicit binary/output paths.

The development bundle hard-links the tested executable; rebundle after a rebuild. It is not a signed/notarized
portable distribution. Temporary work, data and build/dependency caches stay on the SSD.
FERROBOARD_SMOKE_EXIT_MS=4000 closes only this sample's window for startup/teardown checks.

The hosted window has the original editable field categories, keyboard modes and settings pane.
Source storage loads first, command-line options override it, then SettingsPane.Attach constructs
its UI. There is one public panel; application bindings share its existing target context, theme
snapshot and language rows. Its43 scalar editors use the original helper/state behavior, including
preserving raw out-of-range settings while sliders display their native limits. Languages filter
all bundled presets plus ABC; themes include the original Sunset resource. Reset does not change
status text. Native subscription/owner cleanup occurs explicitly at session end.

For example:

```sh
source scripts/env.sh
cargo run --locked -p ferroboard-sample -- --theme=dark --darktheme=sample_sunset \
 --langs=en-US/alphabetical --numberrow --keyheight=61.25 --longpress=450 \
 --focus=EmailBox --settings=false
```

Other settings keys are lighttheme, onehanded, split, floating and glide. Keys ignore ordinal case;
values preserve spelling. Encode spaces as %20; plus stays literal. Later duplicate options replace
earlier values. Invalid values fail at the source-defined point. The settings option closes the
pane only for the exact value false. Below900 logical units the pane overlays; at900 it is inline.
The first narrow layout starts closed. Host visibility, floating placement and keyboard focus stay
in the library.

Source settings, emoji history, pinned clipboard entries and learned words persist synchronously in
/Volumes/SSD/temp/ferrokeyboard/sample-data; FERROBOARD_DATA_DIR selects another SSD folder.
Windows ApplicationData settings deliberately migrate to a keyboard.settings JSON file. Source
history/dictionary filenames and save triggers are retained. Text-field contents/passwords are not
persisted. Loading restores only the English learned dictionary; saving writes every live one.

The library's ProtectedPasswordTextBox keeps plaintext out of its automation peer. Full secure-text
roles and external PasswordBox API remain unverified. The host uses native minimal scrolling to
keep the focused field visible when the docked keyboard opens or resizes. Uno's centered/animated
scroll hints remain available to optional custom handlers. The application uses an explicit
App-owned resource registry.

The public panel's additional Rust release_native_ownership boundary releases native captures and
source subscriptions. It is not source Dispose or automatic CLR collection. Ordinary removal and
rebuild preserve source callbacks. Automatic collection of full callback cycles remains a documented
runtime gap; see docs/reproducers/SETTINGS-PANEL-CALLBACK-CYCLES.md. Domain VMs own no native controls.

The native startup/teardown and saved raw values have been revalidated with the integrated panel.
This is an intermediate milestone. Full Page/Window lifecycle, Padding/Header geometry, error and
metadata contracts, physical input/accessibility/performance/platform tests and the complete port
remain incomplete. See docs/PORTING.md, docs/VALIDATION.md and docs/CONTINUATION.md for exact evidence.

## Optional emoji font

Set `FERROBOARD_EMOJI_FONT` to a local TTF/OTF font file before starting the sample. The file is
read once using Rust I/O and registered with FerroUI's font manager. No font is downloaded or
installed system-wide. Emoji absent from that font continue through the normal fallback chain.
A missing/unreadable file fails startup;an unsupported font is reported and keeps system fallback.

For reusable-library integration,use `ferroboard_controls::EmojiFont`,call `configure` on your
`FontManagerOptions`,and call `install` from `AppBuilder::configure_fonts`. See
[the native font guide](../../docs/NATIVE-FONTS.md). The old Uno Task/Stream compatibility layer is retired; use Rust readers and native FerroUI font options.

Opt-in native window probe (desktop only):

```sh
source scripts/env.sh
cargo run -j 1 --locked --offline -p ferroboard-sample --example window_focus_probe
```

This creates two instances of the existing compiled sample, with independent volatile
settings/document stores and learning/clipboard/history recording disabled. No user settings
or clipboard data are loaded. In Quick actions, “Activate window A/B” calls the peer's native
Window.activate(); it does not focus a field manually. Titles show the current target and
window count. Closing one window removes its activation action and reports whether its
keyboard is still alive after Closed dispatch. Closing both ends the native desktop lifetime.
The private ProbeApp subclasses the sample App through native base construction; the normal
sample constructor, resources and startup/session behavior are preserved. This is a diagnostic
entry point, not complete multiple-window/platform verification. See
[the native observations and initial activation limit](../../docs/NATIVE-MULTI-WINDOW.md).

The window probe additionally reports BOTH roots' native active flag, remembered focus
scope and keyboard target in each title. Activation callbacks are observed after native
scope restoration; they never focus a field or invoke the engine. An inactive peer may
have no keyboard target. Read the current AX root's diagnostics, rather than only the
snapshot summary header: those can represent different instants during activation.
[The standalone comparison and text-preservation limit](../../docs/NATIVE-ACTIVATION-COMPARISON.md)
record the current evidence. The agent completed six peer actions with two full A/B return cycles after setup; the private app is closed.
The earlier B-to-Ban observation did not reproduce and is retained without an assigned cause.

## Native selection probe

Source scripts/env.sh, then run cargo run --locked -p ferroboard-sample --example
selection_write_probe. Focus Message and use Quick actions: Check complete range,
Check reentrant replacement, Check unchanged caret collapse and Check inactive
replacement. The last check focuses Full name, then changes Message through a
retained native target while its input client is inactive. The title must
report PASS with focus/target true. The replacement result is a!ef. Settings and
documents are volatile, learning/history/clipboard suggestions are disabled and
incognito is enabled. Closing before all four checks or after a failed check
returns failure. This diagnostic reuses the compiled application views; keyboard
editing behavior remains in the library. See docs/NATIVE-SELECTION-SIGNAL.md.
