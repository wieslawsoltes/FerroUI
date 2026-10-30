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

`Timeline::sample` evaluates elapsed playback time including initial delay, delay between iterations, speed, repeat, fill and
normal/reverse/alternate/alternate-reverse direction. `scrub` evaluates the edit position in a
single iteration. `Player` implements play/pause/seek/stop with host-supplied time deltas. Compile
once after editing to skip validation while rendering; a compiled frame costs one binary search
per enabled track. `PlaybackBehavior` preserves the native Auto/Always/OnlyIfVisible policy.
`Player::advance_with_visibility` pauses the elapsed clock while hidden when required, so showing
a control resumes at its previous position instead of jumping by the hidden time. An explicit
`sample` or `scrub` remains independent of visibility.

Tracks support numbers, colors, points, vectors, sizes, rectangles, corner radii, relative
points/scalars, thicknesses and discrete values. RGB interpolation occurs
in linear light and alpha is linear, matching FerroUI's `ColorAnimator`. Cubic Bezier interpolation
solves the X polynomial rather than treating the progress as the curve parameter. Thirty named
easing variants match FerroUI's polynomial, sine, circular, exponential, back, bounce and elastic
families. A keyframe's easing belongs to the incoming segment. Effects, gradient brushes,
shadow collections, transform components and arbitrary object payloads remain available through
source editing or extension codecs; their preview uses the native framework animator when
running in the designer. The portable timeline does not invent interpolation semantics for
runtime-specific values.

## Native XAML

`to_xaml(target)` writes FerroUI `Animation`, `KeyFrame` and `Setter` markup, including timing,
iteration direction, fill, global easing and per-keyframe `KeySpline`. Place it in a
`Style.Animations` collection using a selector that names the target control. `to_style_xaml`
wraps that collection for an explicitly supplied selector. The host inserts the result through
the same document transaction used by other tools.

`import_xaml` accepts literal native animation markup. `ImportOptions` supplies property type
hints, host-resolved literals (`literal_values`) and underlying runtime values when keyframes omit
endpoints that their fill mode does not hold. The native host resolves named/CSS colors through
FerroUI’s color parser while the portable model retains numeric color channels. Strict import reports missing underlying values instead of changing their meaning. Unsupported attributes, custom animators,
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


## Independent programs and source identity

`AnimationPrograms::scan(&Document)` returns every `Animation` in source order, including
resource dictionaries, shared styles and nested styles. Each `AnimationProgram` has its own
selector, duration, delay, repeat policy, targets, diagnostics and optional decoded timeline.
There is no first-animation-per-control restriction. Import does not rewrite the document.

`id` is the document's stable syntax `NodeId`, used by edit commands and the active-program UI.
`source_key` is a persisted UUID in an ordinary XML comment. New programs receive one; a legacy
program is stamped when it is named, retargeted or edited. Keys survive reopening and formatting;
duplicating a program allocates a new key. Display names are separate encoded comments. None of
these comments requires a designer runtime assembly in the application.

The source planners return normal `Vec<Edit>` transactions:

| Planner | Behavior |
| --- | --- |
| `create` | Adds a named native animation to a new style under its owner. Empty programs are valid. |
| `rename` | Changes the display-name comment and keeps syntax and persisted identity. |
| `duplicate` | Copies the complete payload, changes the display name and allocates a fresh UUID. |
| `remove` | Removes only the selected animation node. |
| `set_selector` | Changes an isolated selector, or moves the program into a sibling style when setters/other animations share the original. |
| `replace_source` | Validates one Animation subtree, retains its source identity and applies it atomically. |

Hosts resolve selectors against their live tree, including parent scopes for nested `^` selectors.
The native designer uses FerroUI's actual selector parser and compiles a separate sampler for each
matching source control, retaining that target's own implicit endpoint values. Resource animations
and currently inactive selectors can receive explicit preview-only targets through
`DesignerSession::set_animation_program_preview_targets`. All-program preview samples independent
clocks; a later program in source order wins when sampled values target the same property.

`AnimationCodecRegistry` ships with the native literal codec and accepts custom `AnimationCodec`
implementations. A plugin identifies its payload, imports a typed timeline and receives the
original standalone source when encoding an edit. The native codec preserves XML namespaces,
resource `x:Key`, `x:Name`, `x:SetterTargetType` and authored comments. The registry preserves
program identity and display names across codec edits. Unsupported payloads retain their complete
source and a diagnostic; source editing and duplication stay available. A failed codec never
pretends to import a lossy timeline. A vendor element named `Animation` in a different namespace
is not silently treated as the native type.

The native designer also resolves accepted `Animation` objects from loaded styles, control
themes and resource dictionaries using their source URI, line and column. When a selected
program requires native interpolation, preview runs the whole selected batch through those
framework animations on controlled clocks. This supports native shadow collections, gradient
brushes, effects, integer interpolation, transform-component properties such as
`RotateTransform.Angle`, bindings/resources and registered custom animators without converting
them to discrete strings. Such programs remain selectable, playable, scrubbable and source
editable even when no portable codec can decode them. Native previews use elapsed runtime
time, including delays, easing, repeat and fill. Typed timeline scrubbing uses its iteration
edit position as described above.

`Animation::apply_for_preview` isolates playback metadata and retains native final fill and
automatically created transforms at disposable animation priority. Stop, program selection,
source reload and failed source acceptance dispose the runs and restore underlying values
and bindings. Completed fill retains its original priority frame, and overlapping component
programs share the lifetime of any temporary transform they create. Native clock errors are
surfaced as diagnostics. A program must have a loaded
source object and live preview targets; the designer never binds a program from another
compiled source merely because its selector or resource name matches. Whole CSS
`TransformOperations` keyframes are not a built-in native keyframe animator; use native
transform-component properties or a registered animator for those values. The native component
animator also cannot animate an existing CSS operation list; preview reports that limitation
explicitly and permits a registered replacement animator.

A standalone `ResourceDictionary` can use a resource preview factory that installs the loaded
dictionary on a real control. An animation without authored targets then previews on that
factory root after checking property-owner compatibility. The session reports this temporary
target without assigning the resource dictionary a control identity or changing its source.
Factories that synthesize a root in Rust retain authoritative URI/line/column identities on
their loaded animations; no root source annotation is required.

Native geometry values retain their actual kinds: `Vector`, `Size`, `Rect`, `CornerRadius`,
`RelativePoint` and `RelativeScalar` accompany the number, color, point and thickness types.
Corner-radius two-value shorthand means top/bottom. Relative values retain absolute or relative
units; unlike-unit transitions switch at 50%, matching the native animators. The native session
uses the live property's type metadata when selecting a value codec. Nonfinite components are
rejected before preview or source changes.

Global easing accepts native named curves, the default `SplineEasing`, spline property elements
and parameterized `SpringEasing`. The editor syntax `Spring(mass,stiffness,damping,velocity)`
exports an actual `Animation.Easing` / `SpringEasing` object. Spring interpolation matches the
runtime solver's damping behavior and preserves its physical endpoint rather than forcing it
to one. Native `KeySpline` remains linear or cubic Bezier. A zero-parameter SpringEasing is
rejected because the runtime's zero mass/stiffness produce nonfinite motion.

`animation_fragment` extracts any source subtree with its inherited namespace declarations.
`native_fragment` composes generated native markup into either a default or prefixed namespace.
Copying a larger control subtree should call `rekey_copied_programs` so copied programs receive
new persistent keys; ordinary named-reference renames must retain those keys.

## State groups and transition authoring

`VisualStateGroup` owns named `VisualState` entries, target/property/value `StateSetter` entries
and directional or wildcard `StateTransition` entries. Targets are control names in the owner's
scope; an empty target denotes the owner. `validate` rejects duplicate states/setters/edges,
unknown transition endpoints and invalid durations. `plan` returns a document transaction,
assigning a unique owner name when needed. `resolve_state_target` rejects ambiguous names and
does not cross template name-scope boundaries.

The generated source uses native `Style`, `Setter`, style classes and `Animation` objects. It
contains no parallel visual-state runtime. A metadata comment makes the group editable, while
its actual setters and keyframes remain authoritative after source edits. A state's generated
class activates its setters and animation layer. Transition animations use native `FillMode`
Forward; application behavior follows FerroUI's normal final-fill semantics. Define the values
needed by each destination state explicitly. Resource and binding expressions remain native
setter/keyframe expressions and are evaluated by the real runtime. Editing state endpoints
preserves authored intermediate keys, splines, custom animator payloads, program identities and
additional independent programs. Ownership comments distinguish state-managed tracks from
independent programs; duplicating an individual program makes the copy independent. Clearing a
state removes its managed tracks while retaining extension payloads. Changing transition duration rescales its absolute KeyTime
positions; unrelated edits retain separately authored timing.

`actions(owner_name, destination, from)` emits the existing Xaml.Behaviors class actions. An
explicit source selects that edge. With no source supplied, generated `ConditionalAction`
branches and style conditions select a directional edge from the state class that is active
when the event fires, with the wildcard/default state as fallback. Unrelated application classes
are retained. `remove` removes a generated group. `rewrite_design_metadata` updates encoded
target references after the language service renames or copies source; the owner is rebound from
the actual `.Styles` parent when reading.

The native session exposes `state_groups`, `upsert_state_group`, `remove_state_group`,
`state_group_actions`, `preview_state`, `preview_state_at` and `advance_state_preview`. Preview
uses the existing runtime animations and a controlled clock for binding/resource evaluation;
transition sampling and resource/binding endpoints require a rooted logical tree. Literal
endpoint samples also work on a disconnected artboard. Stop restores the authored
preview, including local values and bindings. Trigger previews execute real application callbacks;
changes to shared view models or external systems are outside that view reset.

## Behaviors, triggers and actions

`BehaviorModel` authoring reads and writes the existing `Interaction.Behaviors` collection.
Typed trigger variants cover `EventTriggerBehavior` and `DataTriggerBehavior`. Typed actions
cover property changes, commands, method invocation and adding/removing classes. `Custom`
variants support registered extension objects and arbitrary property maps. `Markup` retains
unknown property elements, text, comments and nested payloads, including explicit collection
wrappers. Namespace-qualified extension payloads survive editing an adjacent typed trigger.

`plan` inserts or replaces a behavior in one document transaction; `remove` validates its
collection membership before deleting it. A bound behavior collection is not replaced by an
unrelated local collection. The native session registers the existing interactivity/interactions
libraries, exposes authoring APIs and previews a trigger by executing its real `IAction`
collection with the existing name scopes and bindings. The preview host must be attached to a
Window or another rooted logical tree so action styling, binding and lifecycle initialization
have run; detached hosts receive a diagnostic before any action executes. Other behavior lifecycle/event logic is
available through the interactive preview and custom preview host registrations.
