# macOS 26 theme

`ferroui-themes-macos` is independently selectable, derived from the working FerroUI
WinUI 3 theme at commit `98a56f09459c8ae02f07a2bcd235c0e87b4d44b3`. It owns its
compiled XAML, assembly, resources, templates and loader, with no dependency on the
WinUI theme crate. The existing WinUI theme remains available.

The target design is **macOS 26**, using Apple's WWDC25 AppKit and Liquid Glass
references. Native visual and motion fidelity is tracked separately from control
coverage and test results. FerroUI's acrylic material contract does not reproduce
all AppKit/WindowServer refraction and grouped-glass behavior.

## Use the theme

Within this workspace, add the theme dependency:

```toml
[dependencies]
ferroui-themes-macos = { workspace = true }
```

Applications using the facade can instead enable the `macos-theme` feature of
`ferroui`; the crate is re-exported as `ferroui::themes_macos` and `MacOSTheme` is
included in the prelude under that feature. Install it in application styles:

```rust
use ferroui_themes_macos::{ControlSize, MacOSTheme};

let theme = MacOSTheme::new();
theme.set_control_size(ControlSize::Regular);
application.styles().add(theme.as_style());
```

The theme and preferences are available in XAML after registering the assembly
through the normal compiled-XAML dependency path:

```xml
<Application xmlns="https://github.com/ferroui"
             xmlns:macos="using:FerroUI.Themes.MacOS">
  <Application.Styles>
    <macos:MacOSTheme ControlSize="Regular"
                      ReduceMotion="False"
                      ReduceTransparency="False"
                      IncreaseContrast="False" />
  </Application.Styles>
</Application>
```

Use FerroUI's requested `ThemeVariant` to select light or dark appearance. The
optional `MacOSTheme.HighContrast` variant is dark; `IncreaseContrast` strengthens
the current appearance without switching it.

### Live preferences

| Property | Default | Effect |
|---|---|---|
| `ControlSize` | `Regular` | Standard control height, font size, padding and corner radius |
| `DensityStyle` | `Normal` | The legacy `Compact` choice maps standard controls to `Small` |
| `AccentColor` | `None` | Follows the platform accent; an explicit color overrides this theme |
| `ReduceMotion` | `False` | Removes nonessential transition durations and stops repeating progress motion |
| `ReduceTransparency` | `False` | Uses opaque materials and requests opaque window backing |
| `IncreaseContrast` | `False` | Strengthens text, separators and focus outlines; uses opaque materials |
| `WindowActive` | `True` | Explicit inactive appearance preview or a scoped window override |

Settings invalidate hosted resources, so existing controls update. Clearing
`AccentColor` restores the current platform accent. `False` for an accessibility
property does not override a system preference that requires it: the effective
value combines the explicit theme preference with platform settings.

On macOS, `NSWorkspace` supplies Reduce Motion and Reduce Transparency. A separate
accessibility-change event updates controls even if the system's color values did
not change. The optional `IFrnAccessibilitySettings` native interface preserves the
original `IFrnPlatformSettings` ABI. Older native libraries fall back to the
previous behavior. Subscriptions are released when a theme is detached and
established again when attached to another owner.

### Control sizes and motion tokens

These are current **theme calibration values**, in device-independent points and
milliseconds. They are not published Apple specifications. Different AppKit
control types can have different intrinsic sizes for the same category; the native
capture fixture records those differences for comparison.

| Size | Standard height | Font size | Standard corner radius |
|---|---:|---:|---:|
| Mini | 18 | 9 | 4 |
| Small | 22 | 11 | 5 |
| Regular | 26 | 13 | 6 |
| Large | 34 | 15 | 17 |
| ExtraLarge | 42 | 17 | 21 |

Large and extra-large controls use capsule outlines. Motion resources are
`MacOSMotionFastDuration` (100 ms), `MacOSMotionStandardDuration` (180 ms), and
`MacOSMotionSlowDuration` (280 ms). All become zero with effective Reduce Motion.
Navigation replaces the whole typed `PageSlide` transition so an attached control
observes changes. Indeterminate progress removes its animated style and releases
the repeating clock, then restores it when motion is enabled; it does not run an
infinite zero-duration animation.

## Visual system and control coverage

The theme includes semantic light/dark text, fill, stroke, selection and status
colors; host-resolved system typography; accent-colored focus rings; size-aware
metrics; and regular, clear, sidebar, menu and toolbar materials. Each surface gets
its own mutable material instance, preventing one window's transparency
compensation from changing another window.

Adapted templates cover standard/default/toggle buttons, check and radio controls,
switches, sliders, text/search/numeric editors, combo and autocomplete controls,
date/time pickers, disclosure controls, tabs, menus, flyouts, tooltips, collections,
sidebars, toolbars, scrolling, progress and window chrome. Drawn chrome puts
close/minimize/zoom controls on the left and centers the title. Template parts,
bindings, input behavior and accessibility peers remain owned by FerroUI controls.

`scripts/macos-theme/audit.py` inventories every copied control file and implicit
default theme against the pinned WinUI input and the current WinUI theme, and checks
include reachability, assembly isolation and centralized durations; `--json` prints
the full inventory. `scripts/themes/resource_keys.py macos` checks that every key the
templates reference resolves for Light, Dark and HighContrast. Unchanged template
structures still use the macOS palette, font and metric resources. Inventory coverage
does not establish native behavior or visual equivalence.

SF Pro is resolved from the host with fallbacks. Apple fonts and SF Symbols are not
redistributed. Non-Apple font metrics can therefore differ from native references.
WinUI/Avalonia provenance is retained in package notices and source comments.

## Run the catalogue

The feature is optional, preserving the default catalogue dependency footprint:

```sh
FERROUI_CATALOG_THEME=macos cargo run -p control-catalog-desktop --features macos-theme
```

Settings exposes the theme selector, all five sizes, platform and explicit accent
choices, Reduce Motion, Reduce Transparency, Increase Contrast and an active-window
preview. Preferences persist when switching away from and back to macOS. A browser
catalogue built with `macos-theme` accepts `?Theme=macos`. The desktop catalogue
also takes `FERROUI_CATALOG_THEME_VARIANT=light|dark|highcontrast` and
`FERROUI_CATALOG_MACOS_INCREASE_CONTRAST=1`, which with `FERROUI_SMOKE_SCREENSHOTS`
give screenshot runs of a fixed appearance.

## Verification

```sh
python3 scripts/macos-theme/audit.py
python3 scripts/themes/resource_keys.py macos
cargo test --locked -p ferroui-themes-macos --lib
cargo test --locked -p ferroui-base --lib platform::default_platform_settings::tests
cargo check --locked -p control-catalog --features macos-theme --lib
```

The suite covers compiled/runtime loading, every default control theme, resource
parity across variants, style states, representative layout, settings on existing
controls, switch interruption/reversal, focus geometry, progress-clock release and
restoration, material isolation and subscription lifecycle. Both the build-time
compiler and runtime emitter must match the checked-in compiled XAML reference.
After intentionally changing markup, regenerate it:

```sh
cargo test -p ferroui-themes-macos --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
```

The [workflow](../../.github/workflows/macos-theme.yml) runs these contracts, a
native COM UUID/identity/lifetime smoke test, and the actual macOS backend's
accessibility tests. A separate macOS 26 job collects AppKit metrics and light/dark
active/inactive reference images. See [native capture instructions](../../scripts/macos-theme/README.md)
for WindowServer capture and the comparison matrix.

### Native fidelity acceptance

Full equivalence requires comparisons with matching OS version, display scale,
fonts, accent, accessibility settings and window activation. Acceptance areas are:

- AppKit geometry and semantic colors for each control and size.
- Focus, hover, press, disabled, mixed, selected and inactive rendering.
- Open menus, picker popups, sheets, scrolling and navigation behavior.
- Animation trajectories, interruption, reversal and reduced-motion behavior.
- WindowServer refraction, backdrop sampling and glass grouping.

FerroUI pickers and other controls retain their existing behavior; styling them
does not turn them into native AppKit widgets. The audit reports native visual,
material and motion fidelity as unverified until matching captures and comparisons
are available. A build, static coverage or unit test alone must not be reported as
100% native fidelity.

## Primary references

- [Build an AppKit app with the new design, WWDC25](https://developer.apple.com/videos/play/wwdc2025/310/)
- [Meet Liquid Glass, WWDC25](https://developer.apple.com/videos/play/wwdc2025/219/)
- [Human Interface Guidelines: materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Human Interface Guidelines: motion](https://developer.apple.com/design/human-interface-guidelines/motion)

Versioned WWDC25 sessions define the macOS 26 target; later releases and unversioned
guidance can describe different designs.
