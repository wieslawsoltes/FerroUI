# Material dynamic color

`ferroui-themes-material` supports all 48 AndroidX `ColorScheme` roles through
`MaterialColorScheme`. There are two sources of dynamic color: locally generated
HCT seed schemes and lossless imports of Android platform colors. Both feed the
same live Material resources, state-layer brushes, and compatibility resources.

## Generate a seed scheme

```rust
use ferroui_base::media::Color;
use ferroui_base::styling::ThemeVariant;
use ferroui_themes_material::{MaterialColorScheme, MaterialTheme};

let theme = MaterialTheme::new();
let seed = Color::from_uint32(0xff00639b);
theme.set_color_scheme(ThemeVariant::light(), &MaterialColorScheme::from_seed(seed, false));
theme.set_color_scheme(ThemeVariant::dark(), &MaterialColorScheme::from_seed(seed, true));

// Material Color Utilities contrast: -1 minimum, 0 standard, 0.5 medium, 1 high.
let high_contrast = MaterialColorScheme::from_seed_color(seed, true, 1.0)?;
theme.set_color_scheme(MaterialTheme::high_contrast(), &high_contrast);
# Ok::<(), ferroui_themes_material::MaterialDynamicColorError>(())
```

Seed generation ports the phone palette definitions, tone-delta constraints,
and dynamic contrast curves from Google's Material Color Utilities at commit
[`bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3`](https://github.com/material-foundation/material-color-utilities/tree/bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3/typescript),
verified on 2026-10-25. The exactly pinned Rust
[`material-colors` 0.4.2](https://docs.rs/material-colors/0.4.2/material_colors/)
supplies CAM16/HCT gamut solving and temperature analysis. Versioned role rules
live in `dynamic_color_spec.rs`, so an older dependency's role definitions cannot
silently replace the selected specification. Source alpha is ignored and
generated roles are opaque. Invalid or nonfinite contrast inputs return an error.

The palette style is independent of the theme's expressive motion scheme:

```rust
use ferroui_themes_material::{MaterialDynamicColorOptions, MaterialPaletteStyle};

let scheme = MaterialColorScheme::from_seed_with_options(seed,
    MaterialDynamicColorOptions {
        dark: false,
        contrast_level: 0.5,
        style: MaterialPaletteStyle::Expressive,
    })?;
```

Supported styles are `Monochrome`, `Neutral`, `TonalSpot` (default), `Vibrant`,
`Expressive`, `Fidelity`, `Content`, `Rainbow`, and `FruitSalad`. These correspond
to the Material Color Utilities variants. The full algorithm and contrast
meaning are described in the upstream
[scheme guide](https://github.com/material-foundation/material-color-utilities/blob/main/dev_guide/creating_color_scheme.md).

### Select the source specification

`from_seed`, `from_seed_color`, and `from_seed_with_options` use
`MaterialColorSpec::Spec2021`, matching the pinned upstream
`DynamicScheme.DEFAULT_SPEC_VERSION`. **For the newer expressive phone color
system, select `Spec2025` explicitly.** The visual theme's expressive motion
setting and the palette style remain separate choices.

```rust
use ferroui_themes_material::{
    MaterialColorScheme, MaterialColorSpec, MaterialDynamicColorOptions,
    MaterialPaletteStyle,
};

let scheme = MaterialColorScheme::from_seed_with_spec(seed,
    MaterialDynamicColorOptions {
        dark: false,
        contrast_level: 0.0,
        style: MaterialPaletteStyle::Expressive,
    },
    MaterialColorSpec::Spec2025,
)?;
```

The 2025 specification changes accent hues and chromas, uses hue-sensitive and
chroma-scaled surfaces, resolves accent tone constraints independently, and
selects container foregrounds using its revised contrast curves. Some palette
chromas differ between light and dark mode. `MaterialDynamicPalette::from_seed_with_spec`
therefore also accepts an explicit `dark` argument when generating all five
101-tone palettes.

The API applies the same fallback as the upstream phone implementation:

| Requested specification | Neutral, Tonal Spot, Vibrant, Expressive | Monochrome, Fidelity, Content, Rainbow, Fruit Salad |
| --- | --- | --- |
| `Spec2021` (default) | 2021 rules | 2021 rules |
| `Spec2025` | 2025 rules | 2021 rules |
| `Spec2026` | 2025 rules | 2021 rules |

`MaterialColorSpec::effective_for(style)` exposes that selection. Upstream's
2026 rules introduce a separate CMF variant with multiple source colors; this
single-seed API exposes the nine styles above and does not claim to implement
CMF or the separate watch-platform color specification.

`MATERIAL_COLOR_UTILITIES_VERSION` records the exact Google source revision;
`MATERIAL_HCT_SOLVER_VERSION` records the independently pinned Rust solver. The
port includes current 2021 role corrections as well as 2025 support: container
foreground contrast curves and tones, the Monochrome secondary container,
Vibrant neutral-variant chroma, inclusive hue-rotation boundaries, and Google's
yellow tone-99 correction. Black HCT tones are clamped at the mathematical zero
boundary before contrast evaluation, preventing a negative floating-point
roundoff value in the Rust solver from incorrectly choosing a white foreground.

These seed-generation rules are independent of AndroidX's baseline colors and
Android's device resources. For example, current 2021 seed rules use tone 30 for
a standard light Tonal Spot `onPrimaryContainer`, while the AndroidX baseline and
API 31 mapping use tone 10. Selecting a source specification never changes those
baseline or import contracts. To reproduce an actual device palette, use the
platform imports below.

## Native Android system-color bridge

For an Android application in this workspace, enable the optional feature:

```toml
[dependencies]
ferroui-themes-material = { workspace = true, features = ["android-dynamic-color"] }
```

Create the theme after the FerroUI Android application is initialized, then import
the device palette:

```rust
use ferroui_themes_material::{MaterialAndroidColorSource, MaterialTheme};

let theme = MaterialTheme::new();
let source = theme.use_android_dynamic_colors()?;
match source {
    MaterialAndroidColorSource::Android34ColorRoles => {
        // Current system role colors, including the device's contrast adjustments.
    }
    MaterialAndroidColorSource::Android31TonalPalette => {
        // Android 12/13 wallpaper palettes and AndroidX-derived neutral tones.
    }
}
// Add theme.as_style() to the application's styles as usual.
# Ok::<(), ferroui_themes_material::MaterialAndroidColorError>(())
```

The bridge obtains `FerroAndroidApplication::try_context()` and reads public
Android color resources through the backend's safe Java interoperability API.
Applications supply no JNI code, Java helper, AndroidX dependency, or extra
resource package. The optional `ferroui-android` dependency is restricted to
Android targets; enabling the feature on desktop does not link an Android
renderer. The same method remains callable on desktop and returns
`MaterialAndroidColorError::UnsupportedPlatform`.

`use_android_dynamic_colors` validates both complete schemes before replacing the
light and dark theme dictionaries; it also updates the default dictionary to the
imported dark scheme, consistent with the theme's default fallback. Missing
resources, unsupported Android releases, or an application that has not yet been
initialized return a descriptive error and retain the existing colors. The
method reports which resource contract it used. On API 34+ an incomplete direct
role scheme is an error; it does not silently downgrade to the older tonal
algorithm and discard system contrast decisions.

### Refresh and subscription lifetime

The method subscribes to the existing
`IPlatformSettings::color_values_changed` notification using a weak theme
reference. A second call replaces that subscription. Dropping the theme or
calling `stop_android_dynamic_colors()` disposes it; stopping retains the last
successfully imported colors.

```rust
// Re-read after an application-specific Android resource-overlay change.
let source = theme.refresh_android_dynamic_colors()?;

// Keep the current palette, but stop following subsequent platform notifications.
theme.stop_android_dynamic_colors();
# Ok::<(), ferroui_themes_material::MaterialAndroidColorError>(())
```

Automatic refresh follows the notifications delivered by FerroUI's Android
platform settings. That backend currently observes Android configuration changes
and publishes when its reported theme, contrast preference, or accent colors
change. The theme does not install an additional wallpaper receiver or an
Android `UiModeManager` contrast listener. A custom resource overlay or a contrast
change that leaves the backend's reported values unchanged therefore needs an
explicit `refresh_android_dynamic_colors()` call from the host's notification.
If an automatic refresh finds an incomplete palette during a configuration
transition, it retains the last complete palette. Explicit refresh returns the
error for the application to inspect.

### Native resource fidelity

On API 34+, the bridge follows AndroidX's
[`values-v34/styles.xml`](https://android.googlesource.com/platform/frameworks/support/+/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/androidMain/res/values-v34/styles.xml)
mapping to public `system_*_light` and `system_*_dark` colors. Inverse roles read
the opposite variant, fixed roles use the unqualified `system_*_fixed` resources,
and surface tint reads the corresponding primary. All imported ARGB values are
preserved, including the system's current contrast adjustments.

On API 31–33 it reads all 65 actual system palette colors according to
[`values-v31/styles.xml`](https://android.googlesource.com/platform/frameworks/support/+/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/androidMain/res/values-v31/styles.xml).
The eleven additional tones in each neutral family are calculated from that
family's tone-40 color by the pinned AndroidX float CAM16 and HCT path in
`android_palette_math.rs`. Its float32 rounding boundaries and HCT lookup table
are preserved separately from the locally generated seed algorithm. Actual
device tones pass through unchanged; missing system resources are errors.

## Android 12–13: custom tonal-palette import

`MaterialDynamicPalette` holds primary, secondary, tertiary, neutral, and
neutral-variant palettes. `from_android31_colors` requests the exact tones used
by AndroidX's `dynamicTonalPalette` function, and `scheme_android31` maps those
tones to roles using the AndroidX API 31–33 implementation.

```rust
use ferroui_themes_material::{MaterialDynamicPalette, MaterialPaletteFamily};

// For a custom host or a captured reference palette. This closure is illustrative;
// read_androidx_tone is application/platform code, not a FerroUI API.
let palette = MaterialDynamicPalette::from_android31_colors(|family, tone| {
    read_androidx_tone(family, tone)
})?;
let light = palette.scheme_android31(false)?;
let dark = palette.scheme_android31(true)?;
theme.set_color_scheme(ThemeVariant::light(), &light);
theme.set_color_scheme(ThemeVariant::dark(), &dark);
```

The three accent palettes require tones 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95,
99, and 100. Both neutral palettes additionally include 4, 6, 12, 17, 22, 24, 87,
92, 94, 96, and 98. AndroidX calculates those additional neutral tones using
`Color.setLuminance` from the corresponding tone-40 system color. This lower-level
adapter expects the supplied palette to include those calculated values; the
native bridge above provides them automatically. The data-only import never
guesses missing tones or substitutes the seed-generation algorithm for the
platform's calculation. `ANDROID_SYSTEM_TONES` and `ANDROID_NEUTRAL_TONES` expose
the requested sets.

The role mapping follows
[`DynamicTonalPalette.android.kt`](https://android.googlesource.com/platform/frameworks/support/+/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/androidMain/kotlin/androidx/compose/material3/DynamicTonalPalette.android.kt)
at the theme's pinned AndroidX revision. The neutral-variant choice is
intentional: AndroidX uses it for API 31–33 to avoid the older platform's
lower-chroma neutral palette.

| Role group | Light tone | Dark tone | Palette |
| --- | ---: | ---: | --- |
| Primary, secondary, tertiary | 40 | 80 | Corresponding accent |
| On accent | 100 | 20 | Corresponding accent |
| Accent container | 90 | 30 | Corresponding accent |
| On accent container | 10 | 90 | Corresponding accent |
| Fixed accent / dim | 90 / 80 | 90 / 80 | Corresponding accent |
| On fixed / variant | 10 / 30 | 10 / 30 | Corresponding accent |
| Background, surface | 98 | 6 | Neutral variant |
| On background, on surface | 10 | 90 | Neutral variant |
| Surface variant / on variant | 90 / 30 | 30 / 80 | Neutral variant |
| Inverse surface / on inverse | 20 / 95 | 90 / 20 | Neutral variant |
| Outline / outline variant | 50 / 80 | 60 / 30 | Neutral variant |
| Surface bright / dim | 98 / 87 | 24 / 6 | Neutral variant |
| Surface container lowest | 100 | 4 | Neutral variant |
| Surface container low | 96 | 10 | Neutral variant |
| Surface container | 94 | 12 | Neutral variant |
| Surface container high | 92 | 17 | Neutral variant |
| Surface container highest | 90 | 22 | Neutral variant |
| Surface tint | 40 | 80 | Primary |
| Inverse primary | 80 | 40 | Primary |
| Scrim | 0 | 0 | Neutral variant |

As in the AndroidX function, the four error roles retain the baseline light/dark
scheme values. `MaterialTonalPalette::from_tones` also permits sparse imports for
application-specific adapters; using a sparse palette in a scheme fails with
`MissingTone` when a required tone is absent. Imported values, including alpha,
are preserved exactly.

`MaterialTonalPalette::from_hct` and `MaterialDynamicPalette::from_seed` generate
complete local palettes for applications that need direct tone access. They are
explicit alternatives to importing device colors.

## Android 14 and newer: custom direct-role import

API 34 and newer expose complete scheme roles. AndroidX reads its
`mc3_light_scheme` and `mc3_dark_scheme` style attributes directly. Reconstructing
these roles from the thirteen system tones would discard platform contrast and
palette decisions. Use `MaterialColorScheme::from_android_system_roles` instead:

```rust
let scheme = MaterialColorScheme::from_android_system_roles(true, |role| {
    // Resolve the matching mc3_dark_scheme attribute in the platform bridge.
    read_android_scheme_role(role)
})?;
theme.set_color_scheme(ThemeVariant::dark(), &scheme);
```

All 43 roles supplied by AndroidX are required. The four error roles and scrim
retain their baseline defaults when absent, just as in AndroidX; explicit values
for these five roles are accepted and preserved. This path can import the
platform's high-contrast scheme without applying a second contrast adjustment.

These data adapters remain available without the native bridge feature. They
support custom Android hosts, captured reference colors, and applications on
other platforms. A complete Material Theme Builder scheme can also be supplied
through `MaterialColorScheme::new`, or generated from a seed with explicit options.

## Validation

The checked-in `tests/fixtures/dynamic-color-phone.csv` contains **165,888
independently calculated ARGB values**: 3,456 schemes covering 16 saturated and
achromatic seeds, all nine styles, light and dark modes, six contrast levels
(`-1`, `-0.5`, `0`, `0.25`, `0.5`, `1`), and the 2021/2025 source specifications.
Every role is compared byte for byte. Fixture generation also checks that the
2026 selection matches the documented fallback for all 1,728 corresponding
phone cases. These are finite reference cases, not a proof of every possible
seed and floating-point contrast value.

The reference generator executes Google's original TypeScript, independently of
the Rust implementation. It records the source commit, SHA-256 of every source
file, TypeScript compiler version, and fixture SHA-256 in
`tests/fixtures/dynamic-color-provenance.json`. To regenerate after checking out
the pinned upstream commit and obtaining TypeScript 5.9.3:

```sh
node scripts/material-theme/generate-dynamic-color-reference.mjs \
  /path/to/material-color-utilities/typescript \
  /path/to/typescript/lib/typescript.js \
  src/FerroUI.Themes.Material/tests/fixtures
```

The script emits JavaScript to a temporary directory, resolves upstream's few
extensionless module imports, and leaves the algorithm source unchanged. It
does not call the Rust implementation to generate expected values.

Additional tests cover source-default selection, variant fallback, the black
HCT boundary, primary foreground/background contrast at contrast +1, invalid
inputs, exact imported colors, fixed-role mode invariance, neutral-variant
surface mapping, missing-tone errors, and API 34 role completeness.
Native bridge tests exercise public system-resource names, inverse and fixed-role
selection, lossless imports, complete-before-install behavior, API 31 system-tone
requests, and added neutral-tone generation. The CAM16 port has reference primary
coordinates, endpoint, luminance, and gamut checks. These deterministic tests do
not replace device testing of Android resource and configuration notifications.
The theme's resource tests separately verify that replacing a scheme updates the
Material resources consumed by controls.
