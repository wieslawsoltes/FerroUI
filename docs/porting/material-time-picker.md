# Material time picker

`MaterialTimePicker` supplies the Material analog clock and numeric time editor.
The inherited FerroUI `TimePicker` theme remains available for existing applications.
The new control is the picker content: the application supplies its dialog title,
keyboard/clock mode button, cancel/confirm actions, and any surrounding surface.

The reference is AndroidX Material3 1.5.0-beta02,
[`TimePicker.kt`](https://github.com/androidx/androidx/blob/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/TimePicker.kt)
and [`TimePickerTokens.kt`](https://github.com/androidx/androidx/blob/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/TimePickerTokens.kt),
at commit `e4d933c98df7baa7ea2b83eea6e086915961e0ff`. Where the generated token table
differs from the current component, the implementation follows the component.

## Use

```xml
<material:MaterialTimePicker Hour="14" Minute="30" Is24Hour="True"
                             IsVibrant="True" />
```

```rust
let picker = MaterialTimePicker::new();
picker.set_time(14, 30);
picker.set_is_24_hour(true);
picker.set_display_mode(MaterialTimeDisplayMode::Input);
picker.add_handler(MaterialTimePicker::time_changed_event(), |_, event| {
    let valid_time = event.new_time();
    // Persist the valid time or update a preview.
});
```

`Hour` is always 0–23 and `Minute` is always 0–59. `Is24Hour` changes presentation
and input interpretation, and does not change the selected time. `SelectedTime`
is a bindable minute-precision `TimeSpan` within one day. `set_time` updates both
units atomically, emitting at most one `TimeChanged` event. Setting a `TimeSpan`
discards seconds; values outside one day are rejected.

`Selection` identifies the active hour or minute unit. `DisplayMode` switches
between `Dial` and `Input`. `Orientation="Horizontal"` places the display beside
the clock; numeric input keeps the native horizontal field arrangement.
`IsVibrant` selects the expressive component overload's colors, sizes and shapes.
`MotionScheme` and `ReducedMotion` inherit the theme defaults through the template.

Numeric drafts remain separate from the valid time. For example, entering `99`
in the minute field keeps `Minute` at its last valid value and makes
`IsMinuteInputValid` and `IsInputValid` false. The error text is announced politely.
Non-digit or over-length replacements are rejected without replacing the previous
text or selection. Clearing a field selects zero in that unit, preserving AM/PM
for a 12-hour hour field. External `Hour`, `Minute`, or `SelectedTime` changes
replace drafts with formatted valid values.

`HourLabel`, `MinuteLabel`, `AmLabel`, `PmLabel`, `HourErrorText` and
`MinuteErrorText` are application-localizable strings. The numeric representation
currently uses ASCII decimal digits. Platform locale-specific numerals and
native haptic error feedback are not supplied by this control.

## Source measurements

| Element | Classic | Vibrant |
| --- | --- | --- |
| Analog time field | 96 × 80 dp | 100 × 120 dp; portrait 24-hour width 132 dp |
| Numeric input field | 96 × 72 dp | 100 × 120 dp |
| Analog field type | Display Large, 57 sp | Display Large, 57 sp |
| Numeric input type | Display Medium, 45 sp | Display Large, 57 sp |
| Field corner | 8 dp | 16 dp |
| Separator width | 24 dp | 16 dp |
| Display-to-clock vertical gap | 36 dp | 12 dp |
| Vertical clock bottom space | 24 dp | 0 dp |
| Vertical period group | 52 × 80 dp; input height 72 dp | 56 × 120 dp |
| Horizontal period group | 216 × 38 dp | 216 × 40 dp |

The 114 dp classic 24-hour field width in the token table is not used by the
pinned `ClockDisplayNumbers`; the actual width is 96 dp. The enabled
`isUpdatedTimepickerToggleEnabled` flag uses separate AM/PM buttons with a full
unselected corner, a 12 dp checked/pressed corner, and bold checked text.
Their selected colors are primary container/on-primary-container. Their
unselected surface is surface-container-lowest, with on-surface-variant text.

The dial is 256 dp. Classic landscape constraints select 238 dp at a maximum
height of 330–383 dp and 200 dp below 330 dp; the vibrant dial remains 256 dp.
Label-center radii are 101/256 and 69/256 of the dial diameter. The 24-hour ring
boundary is strictly less than 74/256 of the diameter for the inner ring.
The selector handle is 48 dp scaled with the dial; its center marker remains
8 dp and the hand stroke remains 2 dp. Clock labels use Body Large, 16 sp.

Classic clock surfaces use surface-container-highest; vibrant uses
surface-container-lowest. The hand is primary and label color inside the moving
handle is on-primary. The renderer clips the selected foreground to the actual
animated handle rather than recoloring an entire number before the handle arrives.

## Interaction and accessibility

Pointer taps select hours or the nearest five minutes. A drag selects every
minute and the hand follows the continuous pointer angle. In 24-hour mode,
distance selects the inner 12–23 ring or outer 0–11 ring. Pointer capture is
released on completion, cancellation, hiding, retemplating and detachment.

The hand uses the theme's default spatial spring and follows the shortest angular
path, retaining velocity when retargeted. A 180-degree tie follows the native
negative rotation. Hour/minute labels crossfade using default effects motion.
Reduced motion settles active spring state immediately. Pending hour-to-minute
advancement is cancelled when the dial leaves the visual tree or input mode changes.

The clock contains real `RadioButton` nodes with selection-item and invoke automation.
Arrow keys move focus around the current ring and between the two 24-hour rings;
they do not select a value. Enter or Space selects the focused value. Tab escapes
to the next display control and Shift+Tab returns to the active display control.
Clock-number activation snaps the hand, as the native `ClockText` uses `SnapSpec`.
An accessibility invoke can advance to minutes, including when the invoked number
was already selected. Keyboard activation and selection-provider actions never
advance. Each successful activation completes an edit even if the time is unchanged.
The composite exposes a value provider accepting `HH:mm`; the selected numeric
unit retains `TextBox` editing, clipboard, IME composition, and edit automation.

`AutoAdvance` controls automatic movement from hours to minutes. Hosts with touch
exploration enabled must set it to false; FerroUI does not currently provide a
cross-platform accessibility-service-state signal for this component to query.
Accessibility invocation honors the host's `AutoAdvance` setting. Numeric
typing or paste advances after a valid two-digit hour, or a new digit 2–9 in
12-hour mode, when `AutoAdvance` is enabled.

The gallery's **Material time pickers** page exercises classic and vibrant clocks,
format and mode switching, numeric input, and landscape layout. Tests cover the
source geometry and thresholds, actual arranged templates, active versus inactive
error colors, invalid drafts,
clipboard and IME commits, keyboard traversal, automation value/selection,
pointer capture, interrupted motion, real visual-tree rendering through label
crossfades, and detached weak ownership. Native Android
rendering traces and device accessibility testing remain necessary for a claim
of complete platform equivalence.
