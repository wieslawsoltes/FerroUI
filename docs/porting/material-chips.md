# Material chips

Material chips keep FerroUI's existing `Button` and `ToggleButton` command,
keyboard, focus and automation behavior. The themes add the native Material
anatomy through the `MaterialChip` attached properties and `MaterialChipLayout`.

The reference is AndroidX Material3 **1.5.0-beta02**, revision
[`e4d933c98df7baa7ea2b83eea6e086915961e0ff`](https://github.com/androidx/androidx/blob/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Chip.kt).
Effective defaults in `Chip.kt` take precedence over unused or superseded token
values. See [the Material theme guide](material-theme.md) for setup, color roles,
fonts and the shared motion resources.

## Theme keys

| Theme | Control | Appearance |
| --- | --- | --- |
| `MaterialAssistChip` | `Button` | Flat assist |
| `MaterialSuggestionChip` | `Button` | Flat suggestion |
| `MaterialFilterChip` | `ToggleButton` | Flat filter |
| `MaterialInputChip` | `ToggleButton` | Flat input |
| `MaterialElevatedAssistChip` | `Button` | Elevated assist |
| `MaterialElevatedSuggestionChip` | `Button` | Elevated suggestion |
| `MaterialElevatedFilterChip` | `ToggleButton` | Elevated filter |
| `MaterialExpressiveFilterChip` | `ToggleButton` | Expressive shapes and spacing |
| `MaterialExpressiveInputChip` | `ToggleButton` | Expressive input |
| `MaterialExpressiveElevatedFilterChip` | `ToggleButton` | Expressive elevated filter |

The original four keys use the complete anatomy, including the corrected native
padding. Existing `Button` and `ToggleButton` declarations continue to work. Use
`Content`/`ContentTemplate` for the label, `Command`/`CommandParameter` for the body
and `IsChecked` for selectable chips. Filter and input chips expose the checkbox
automation role and retain the native toggle provider. Assist and suggestion
chips retain the button role and invoke provider.

## Slots and geometry

Declare `xmlns:material="using:FerroUI.Themes.Material"` in the XAML document.

```xml
<ToggleButton Theme="{StaticResource MaterialExpressiveInputChip}"
              Content="Alex" IsChecked="True"
              material:MaterialChip.IsDismissible="True"
              material:MaterialChip.DismissLabel="Remove Alex">
  <material:MaterialChip.Avatar>
    <Border Width="24" Height="24"
            Background="{DynamicResource MaterialTertiaryContainerBrush}">
      <TextBlock Text="AL" FontSize="10" FontWeight="Medium"
                 HorizontalAlignment="Center" VerticalAlignment="Center"
                 Foreground="{DynamicResource MaterialOnTertiaryContainerBrush}" />
    </Border>
  </material:MaterialChip.Avatar>
</ToggleButton>
```

| Attached property | Default and purpose |
| --- | --- |
| `LeadingIcon`, `TrailingIcon` | Empty; independent decorative content slots, each 18 dp |
| `Avatar` | Empty; a circular 24 dp input-chip avatar |
| `ShowSelectedIcon` | `True`; supplies the selected check for filter/input chips |
| `IsDismissible` | `False`; supplies an independent trailing dismiss button |
| `DismissLabel` | `"Remove"`; localizable accessible name of that button |
| `DismissCommand`, `DismissCommandParameter` | Empty; independent action and parameter |
| `Kind` | `Assist`; assigned by the theme to `Assist`, `Suggestion`, `Filter` or `Input` |
| `IsExpressive` | `False`; expressive keys enable compact gaps and expressive slot motion |
| `FullCornerRadius` | Layout output; measured radius for the fully rounded selected shape |

Input avatars take precedence over the automatic check and `LeadingIcon`.
Without an avatar, the selected check takes precedence over `LeadingIcon` when
`ShowSelectedIcon` is enabled. Set it to `False` to control the leading content
entirely from the application. This automatic check is a convenience of the
FerroUI themes; AndroidX accepts caller-provided selected icon content. A dismiss
button takes precedence over decorative `TrailingIcon` content.

The painted surface has a **32 dp minimum height**, inside a **48 dp minimum
interactive target**. Content can make the surface taller. Ordinary label-only
assist, suggestion and filter chips have 16 dp at each label edge; input chips
have 12 dp. With ordinary spacing, leading icons start at 8 dp and their labels
start at 34 dp. An input avatar starts at 4 dp and its label at 36 dp. Slots use
8 dp gaps; expressive selectable chips use 4 dp beside a present icon/avatar.
Layout uses logical start/end coordinates and follows RTL automatically.
For a `PathIcon` authored in Android's 24 dp coordinate space, use
`Theme="{StaticResource MaterialSymbolIcon}"` to preserve its internal viewport
padding when the 18 dp slot scales it. Arbitrary application slot content keeps
its own theme.

Ordinary shapes use an 8 dp radius. Expressive shapes use 12 dp at rest, a fully
rounded selected shape and 8 dp while pressed. The selected radius follows the
actual surface size, including taller custom content. Disabled foregrounds and
avatars use 0.38 opacity; disabled selected/elevated fills use 0.12 independently
of their contents. Flat filter hover elevation is 1 dp in both selection states,
matching the effective `Chip.kt` default. Elevated chips use 1 dp at rest and
2 dp on hover.

## Motion

Assist and suggestion slot changes are immediate. Selectable slots retain outgoing
content until its exit finishes. Each slot has independent size and opacity
springs, preserving position and velocity when an animation is interrupted.

| Selectable theme | Entry width | Exit width | Entry opacity | Exit opacity |
| --- | --- | --- | --- | --- |
| Ordinary | `FastSpatial` | `DefaultEffects` | `SlowEffects` | `FastEffects` |
| Expressive | `FastSpatial` | `FastSpatial` | `DefaultEffects` | `DefaultEffects` |

These are motion roles, resolved through the theme's `MotionScheme`; they are
separate from choosing an expressive chip shape. Shape morphing uses
`FastSpatial`. `MaterialTheme.ReducedMotion` and the platform motion preference
settle the animations. Hidden or detached layouts release their clock observers.

## Independent dismissal

The dismiss button has its own 48 dp target, accessible name, focus, keyboard
activation, ripple and command availability. As with a larger interactive native
trailing composable, this slot makes the painted chip at least 48 dp tall.
`DismissCommand.CanExecute` disables the dismiss action while preserving the
body's independent enabled state.

On activation, the helper checks availability and raises the bubbling
`MaterialChip::dismiss_requested_event()` with the chip as its source. Subscribe
with `MaterialChip::dismiss_requested(&chip, handler)`. If the event is unhandled,
`DismissCommand` executes with `DismissCommandParameter`; handling the event
cancels command execution. `MaterialChip::request_dismiss(&chip)` follows the
same path and reports whether the request was raised.

Dismissal does not raise a body `Click`, toggle `IsChecked` or remove application
data. The application handles removal, confirmation or undo. The overview gallery
shows one implementation by hiding the recipient chip and offering a restore
action. The dismiss control explicitly pins its own template so the enclosing
Button-based theme cannot replace it with the chip-body template.

The regression suite is `src/FerroUI.Themes.Material/tests/chips.rs`; it exercises
native geometry, roles, actual keyboard/automation activation, command gating,
RTL, measured shapes, spring interruption and observer lifecycle.
