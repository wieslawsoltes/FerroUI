# FerroUI designer animation engine

`ferroui-designer-animation` is an independently reusable Rust timeline model. It does not depend
on a UI dispatcher, renderer, window system or timer. The designer workspace supplies a clock and
applies samples at animation priority; recording authors keyframes and leaves base property values
intact.

## Author, scrub and play

```rust
use ferroui_designer_animation::{AnimationValue, Easing, Player, Timeline};

let mut timeline = Timeline::new(2.0)?;
timeline.record(7, "Opacity", 0.0, AnimationValue::Number(0.0))?;
timeline.record(7, "Opacity", 2.0, AnimationValue::Number(1.0))?;
timeline.track_mut(7, "Opacity").unwrap().set_easing(2.0,
    Easing::CubicBezier { x1: 0.25, y1: 0.1, x2: 0.25, y2: 1.0 })?;
let values = timeline.scrub(0.5)?;
let native_xaml = timeline.to_xaml(7)?;
let compiled = timeline.compile()?;
let next_frame = compiled.sample(0.75)?;
# Ok::<(), ferroui_designer_animation::AnimationError>(())
```

`Timeline::sample` evaluates elapsed playback time including delay, speed, repeat, fill and
normal/reverse/alternate/alternate-reverse direction. `scrub` evaluates the edit position in a
single iteration. `Player` implements play/pause/seek/stop with host-supplied time deltas. Compile
once after editing to skip validation while rendering; a compiled frame costs one binary search
per enabled track.

Tracks support numbers, colors, points, thicknesses and discrete values. RGB interpolation occurs
in linear light and alpha is linear, matching FerroUI's `ColorAnimator`. Cubic Bezier interpolation
solves the X polynomial rather than treating the progress as the curve parameter. Thirty named
easing variants match FerroUI's polynomial, sine, circular, exponential, back, bounce and elastic
families. A keyframe's easing belongs to the incoming segment. Custom effects, brushes, geometry
or arbitrary object values remain in the source editor; the portable timeline does not invent
interpolation semantics for a runtime-specific animator.

## Native XAML

`to_xaml(target)` writes FerroUI `Animation`, `KeyFrame` and `Setter` markup, including timing,
iteration direction, fill, global easing and per-keyframe `KeySpline`. Place it in a
`Style.Animations` collection using a selector that names the target control. `to_style_xaml`
wraps that collection for an explicitly supplied selector. The host inserts the result through
the same document transaction used by other tools.

`import_xaml` accepts literal native animation markup. `ImportOptions` supplies property type
hints and underlying runtime values when keyframes omit endpoints that their fill mode does not
hold. Strict import reports missing underlying values instead of changing their meaning. Unsupported attributes, custom animators,
resource expressions and nested binding setters report an error and stay in the source editor.
The engine does not discard unrecognized source or claim a successful lossy conversion.

Native duration parsing follows FerroUI's invariant `TimeSpan` grammar: a bare integer denotes
days, while seconds use `hh:mm:ss.fffffff`. Export rejects repeat counts beyond the native signed
64-bit range and timing that cannot fit the finite playback clock. Imported implicit endpoints
respect `FillMode`; the implicit final segment retains the last keyframe's spline. Initial-delay
fill uses the first actual keyframe in playback direction, without applying global easing.
Tracks with a captured underlying value retain their implicit endpoints on export, so that
reverse initial-delay fill remains unchanged. Keep the target's underlying property in the
surrounding XAML and provide its value again when re-importing that animation.

All public mutable models can be serialized with Serde. Deserialized or directly modified models
are validated before sampling/export/compilation. Invalid values, NaN/infinity, inconsistent
property types and duplicate keyframe times are rejected. Authored models are independent of the
runtime object lifetime; a host can preserve them when rebuilding the preview.
