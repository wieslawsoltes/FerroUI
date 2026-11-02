# Compiled XAML themes

XamlX resolves types, overloads, namespaces and includes at build time. The Rust backend used to emit a separate function for each deferred alias or brush, even when only its key, color and source position differed. The WinUI baseline at `df8b92d9` contains 14,421,918 bytes of generated Rust.

## Choosing the representation

| Consumer | Configuration | Representation |
| --- | --- | --- |
| Open-ended resource dictionary | Default | Typed Rust calls with static operand tables for repeated factories |
| Open-ended dictionary with binary resources | Enable `binary-resources` on the theme crate | Typed Rust calls with independently embedded FXB blocks for eligible runs |
| Application with known WinUI keys and controls | Set `FERROUI_WINUI_THEME_ROOTS` at build time | Compile the selected keys and their transitive XAML dependencies, using either backend |
| Library that owns its build script | Call `XamlGroup::resource_roots(&[...])` | Apply the same closed-world pruning to that group |

For example, the following builds a small WinUI consumer with an accent button and a text block. Cargo tracks changes to the environment variable. Unset the variable to build the full theme.

```sh
FERROUI_WINUI_THEME_ROOTS=Button,AccentButtonStyle,TextBlock cargo build --release
```

The build script retains the compact-density dictionary that the WinUI constructor requests from Rust. Supply keys your own Rust code looks up, controls created at runtime, and names computed by plugins or custom markup extensions. All clients of the same WinUI crate instance share the selected roots in a Cargo build.

The closed-world pass runs before merged dictionaries expand. It indexes simple keyed entries and explicit type keys, follows references across the group, and keeps every declaration of a live name across scopes and theme variants. A local type name shared by several namespaces conservatively retains them all. It preserves unknown key expressions, unkeyed entries and whole retained templates. It also preserves named objects observable in the outer namescope. Removed spans retain line breaks and UTF-16 columns for XamlX diagnostics. An unknown requested root fails the build.

The native emitter recognizes four deferred operations. They create a static resource alias, a literal-color brush, a brush bound to a dynamic color resource, or a brush initialized from a static color resource. Batching preserves dictionary, declaration scope, insertion order and source positions. A different property, constructor, sharedness, or unrecognized operation takes its original typed path. The binary backend replaces the operands of those same operations with versioned records. The wire format and reload API are described in the [FXB specification](porting/xaml-compiler/fxb.md).

Both backends name the required factory directly. The closed-world pass removes unused theme factories before rustc sees them. The framework also registers metadata for runtime XAML and value conversions. Calling the existing framework registration APIs can retain control and dialog implementations independently of whether their theme entry survives. Applications which use open-ended resource lookup should use the full dictionary.

## Measuring a consumer

`scripts/themes/bench/measure.py` builds a headless, release-mode consumer through the normal WinUI constructor. It first resolves the Button control theme, which forces an actual deferred lookup. Each startup sample runs in a new process with the executable already in the OS page cache. There is no window system, renderer or first-frame wait in this measurement.

```sh
git worktree add --detach ../FerroUI-baseline df8b92d9
python3 scripts/themes/bench/measure.py --source ../FerroUI-baseline --work-dir /tmp/theme-before
python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-native --compare-with /tmp/theme-before/stripped-probe
python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-fxb --binary --compare-with /tmp/theme-before/stripped-probe
FERROUI_WINUI_THEME_ROOTS=Button python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-button
FERROUI_WINUI_THEME_ROOTS=Button python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-button-fxb --binary
```

The harness reports generated Rust and referenced FXB bytes separately, strips a copy of the whole executable, and measures a clean theme rebuild with its dependencies warm. It also records fresh-process startup, theme construction, first Button lookup, toolchain, job count and revision in each run's JSON report.
