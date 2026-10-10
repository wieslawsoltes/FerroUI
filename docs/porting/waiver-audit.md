# Waiver audit (2026-10-10)

The tracking of the port (`TRACKING.md`) excludes from its percentages what is declared intentionally not ported: the types and members of `docs/porting/data/member-waivers.toml` (`[[waive]]`), and the files that `docs/porting/data/path-overrides.toml` marks `replaced` or `not-applicable`. This document is the audit of every one of those declarations against the code as it is at the tracked commit of upstream (`17350180`) and the head of the port on 2026-10-10. What is left to port after it is in [REMAINING.md](REMAINING.md).

## The classes

| Class | Meaning | What the audit did |
|---|---|---|
| (a) still right | A concept of .NET without a counterpart (reflection, finalizers, `Dispose` where `Drop` does the work, hooks of source generators, obsolete members), or a member a named Rust construct replaces, and that construct exists | The waiver stays. Listed by group, with the reason of the group; the reason of each item is in the data file and in the page of its project |
| (b) stale | The type or member exists under a name that can be mapped | The waiver is removed and the item counts as present: found by the scanner, by the rename rule of the framework name (below), or by an `[[alias]]` that names the Rust item |
| (c) not justified | Behaviour of upstream that the port does not have | The waiver is removed and the item counts as missing. Nothing was ported in the audit |

## Method

1. **Every waived item, with the entry that waived it.** The scanner was run over the extraction of step 1 (the tracked commit, with the members of the Windows, X11, FreeDesktop and iOS projects) and every type, member and file it reported as waived or not applicable was written out with its reason: 3,099 members, 314 types and 135 files, under 853 distinct reasons (782 `[[waive]]` entries, 117 `replaced` or `not-applicable` overrides). Files that the scanner leaves out by itself because they declare no non-private type (assembly attributes, global usings) are no declaration of anyone and are not part of the audit.
2. **The Rust names of a reason were looked up.** A reason that says "ported as `X::y`" or "the variant `Z`" names items in backticks. Each such name was looked for among the definitions (`fn`, `struct`, `enum`, `trait`, `type`, `const`, `static`, `mod`, macros) of the crate of the project and then of the repository, with comments removed. A name that is defined nowhere, a reason that names nothing, and a reason that only has a weak match (the word occurs, but not as a definition) were read one by one (139 reasons).
3. **Reasons that admit something is missing were read.** Every reason with a phrase such as "not ported", "for now", "stage 2", "out of scope", "applicable when", "not kept", "dropped", "no caller", or with a claim that later work may have made false (no render thread, one thread, no GPU, no compiler), was read against the code (40 reasons). Every file-level override and every reason that covers eight items or more was read as well (155 reasons).
4. **Waivers that name the Rust item became aliases.** A waiver proves nothing: it holds whether or not the item it names exists. An `[[alias]]` is checked at every run, and the member counts as present only while the item exists on the type. The alias rule of the scanner covered methods; it now also covers constructors (`member = ".ctor"`), properties, fields, events, indexers and the functions of a type ported as a module, so that a waiver of the form "ported as `with_parent`" could become an alias. This is a mapping written by hand for one member, not a looser match: no default rule of the scanner was changed. A waiver was converted only when its reason states a one-to-one correspondence: a constructor named after what it takes (`with_*`, `for_*`, `construct_*` and a few named ones), or a member the reason gives one function for. Where the function named is where the behaviour went (a closure inside another function, a member of another type, one function for several members) the waiver stayed a waiver, in class (a).
5. **The rename rule of the framework name applies to members.** Rule 2 of the porting guide renames the framework in every identifier. The scanner applied it to types and files, not to members, and sixteen waivers existed only to say so (`AvaloniaPropertyType` is `ferro_property_type`). The scanner now looks for the renamed member as well.
6. **Entries that match nothing were removed**, among them five of the remote protocol whose members the scanner finds since the protocol was ported.

The audit reads reasons and checks names. It does not compare behaviour: an item of class (a) is one whose reason is sound and whose named replacement exists, not one whose replacement was tested against upstream.

## What changed in the data and the tool

- `member-waivers.toml`: 782 `[[waive]]` entries before, 603 after (173 removed because nothing is waived by them any more, 6 removed as not justified); 73 `[[alias]]` entries before, 239 after.
- `path-overrides.toml`: six `not-applicable` entries removed (four files that are behaviour the port lacks, the project of the VNC server, and a generated header that is no file of the tracked commit).
- `scripts/api-extract/projects.json`: the VNC project of the headless platform is out of scope instead of in scope with every file not applicable (the `expanded` list, read by the scanner only).
- `scripts/port-status/port_status.py`: aliases for constructors, single members and module functions; the rename rule for members.

## The headline before and after

Projects in scope whose members are extracted; the percentage is `present / (total - waived)`.

| | Files | Types (waived) | Members (waived) | Members missing | % |
|---|---:|---:|---:|---:|---:|
| On `main` before this work: the Windows and X11 projects in scope without their members | 2266 of 2266 | 2931 of 3137 (205) | 21123 of 23141 (2009) | 9 | 100.0 |
| With the members of the Windows and X11 projects (step 1), data files unchanged | 2347 of 2449 | 3161 of 3704 (314) | 25685 of 30643 (3099) | 1859 | 93.3 |
| After the audit | 2347 of 2453 | 3164 of 3717 (311) | 25928 of 30744 (2850) | 1966 | 93.0 |

The second number of `TRACKING.md`, the share of all members of every upstream source project of the extraction that the port has: 25928 of 37391, 69.3 %. Before the audit it was 25685 of 37391, 68.7 %.

How the status of every member moved between the second and the third row (the tool and the data of `main` against the tool and the data now, over the same extraction):

| From | To | Members | Why |
|---|---|---:|---|
| waived | present | 238 | class (b) |
| waived | missing | 11 | class (c): five members of `GrabResult` of the X11 backend, six of the EGL classes |
| not applicable | missing | 100 | class (c): the members of the four files below that are no longer marked not applicable |
| not applicable | out of scope | 11 | class (c): the VNC project |
| not applicable | present | 1 | `ImportedTexture.Dispose` of the file of the macOS OpenGL graphics: a type of that name exists elsewhere in the crate. A false positive of the name match, recorded here; it goes when the file is ported |
| missing | present | 4 | found by the two changes of the tool: the rename rule for members (`Methods.AvaloniaRemote`, `X11Atoms.AVALONIA_SAVE_TARGETS_PROPERTY_ATOM`) and aliases that already existed and now apply to a property and a constructor (`MagicProperty.Type`, a constructor of `BsonException`) |
| present | anything else | 0 | |
| waived | waived | 2850 | class (a) |

## Class (c), in one list

Behaviour of upstream that was declared not ported and is reported as missing now:

| Project | What | Size | Was declared |
|---|---|---|---|
| Avalonia.Native | `AvaloniaNativeGlPlatformGraphics.cs`: the OpenGL platform graphics of the macOS backend (display, context, surface, render target, external objects on OpenGL) | 1 file, 9 types (one of them found by name elsewhere in the crate), 70 members | not applicable "for now", until the desktop build has Ganesh on OpenGL |
| Avalonia.Skia | `Gpu/Vulkan/VulkanSkiaGpu.cs`, `VulkanSkiaRenderTarget.cs`, `VulkanSkiaExternalObjectsFeature.cs`: the backend on Vulkan | 3 files, 4 types, 31 members | not applicable while `Avalonia.Vulkan` is out of scope |
| Avalonia.OpenGL | `EglInterface()` and `EglInterface(string)`: loading the EGL library of the system by name; `EglDisplay()` and `EglPlatformGraphics.TryCreate()`, which create the default display through it | 4 members | left to the platform backend; no backend of the port does it (the Windows backend resolves ANGLE, the X11 backend has no EGL yet) |
| Avalonia.OpenGL | `EglDisplay.DisplayLockIsSharedWithContexts`, `EglDisplay.ContextSharedSyncRoot`: the monitor a display shares with its contexts | 2 members | applicable "when an EGL platform renders on the render thread": the render thread and a backend on EGL (ANGLE on Windows) exist now, so the condition has to be examined, not assumed |
| Avalonia.X11 | `XLib.GrabResult`: the result of `XGrabPointer` | 1 type, 5 members | used by the drag source, which is stage 2 of the X11 backend: not ported yet, and no bindings structure |
| Avalonia.Headless.Vnc | the VNC server of the headless platform | 3 files, 3 types, 11 members | every file not applicable, "out of scope": the project is now listed out of scope, with its size, instead of in scope with nothing to do |

Not class (c), though their reasons say "not ported": `AppBuilder.WithDataAnnotationsValidation` and the plugin it registers (attributes read by reflection); the two obsolete `SaveImage` overloads and the legacy `CreateDrawingContext(bool)` of the Skia backend (marked obsolete or throwing upstream); `HeadlessGlyphRunStub.GlyphTypeface` and `HarfBuzzTypeface.GlyphTypeface` (nothing reads them; DEVIATIONS.md); the source generators and the glue to NUnit and xUnit (replaced by the build crate and the harness of cargo).

## What the audit did not do

- It did not port anything, and it changed no source file of the port.
- It did not review the 2,850 members of class (a) for behaviour. For 1,085 of them (the X11 backend) the reason is one rule: the structure belongs to the bindings crate.
- It did not weigh the files the scanner leaves out because they declare no non-private type.
- The Windows backend has no waivers: what it lacks is reported as missing (1,290 members, most of them declarations of the interop file that later stages use).

## Totals

Items that were waived, or in a file marked not applicable, before the audit (the baseline: the tracked commit with the member detail of step 1, the data files as they were).

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) still right | 128 | 311 | 2850 |
| (b) stale: counts as present now | 0 | 2 | 238 |
| (c) not justified: counts as missing now | 7 | 1 | 11 |
| Total | 135 | 314 | 3099 |

| Project | (a) files | (a) types | (a) members | (b) types | (b) members | (c) files | (c) types | (c) members |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Avalonia.Base | 46 | 135 | 1035 | 1 | 117 | 0 | 0 | 0 |
| Avalonia.Browser | 2 | 2 | 88 | 0 | 7 | 0 | 0 | 0 |
| Avalonia.Build.Tasks | 3 | 1 | 28 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Controls | 0 | 7 | 148 | 0 | 41 | 0 | 0 | 0 |
| Avalonia.DesignerSupport | 0 | 0 | 1 | 0 | 2 | 0 | 0 | 0 |
| Avalonia.Generators | 31 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.HarfBuzz | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Headless | 0 | 2 | 13 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Headless.NUnit | 4 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Headless.Vnc | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |
| Avalonia.Headless.XUnit | 14 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Markup | 0 | 30 | 55 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Markup.Xaml | 1 | 2 | 31 | 0 | 9 | 0 | 0 | 0 |
| Avalonia.Markup.Xaml.Loader | 1 | 6 | 83 | 0 | 22 | 0 | 0 | 0 |
| Avalonia.MicroCom | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Native | 0 | 10 | 59 | 1 | 17 | 1 | 0 | 0 |
| Avalonia.OpenGL | 0 | 2 | 30 | 0 | 2 | 0 | 0 | 6 |
| Avalonia.Remote.Protocol | 0 | 1 | 3 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.Skia | 0 | 0 | 21 | 0 | 12 | 3 | 0 | 0 |
| Avalonia.Themes.Fluent | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 |
| Avalonia.X11 | 0 | 108 | 1085 | 0 | 0 | 0 | 1 | 5 |
| XamlX | 26 | 5 | 156 | 0 | 9 | 0 | 0 | 0 |

## Avalonia.Base

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 46 | 135 | 1035 |
| (b) | 0 | 1 | 117 |
| (c) | 0 | 0 | 0 |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `Animator.Apply` | alias free fn `apply_with_visual` |
| `AssemblyDescriptor.AvaloniaResources` | alias fn `ferro_resources` |
| `AvaloniaObject.this[]` (2) | alias fn `get_value_untyped` |
| `AvaloniaProperty..ctor` (2) | alias fn `for_added_owner` |
| `AvaloniaPropertyChangedEventArgs.SetSender` | alias fn `with_sender` |
| `AvaloniaSynchronizationContext..ctor` (2) | alias fn `with_dispatcher`, alias fn `with_priority` |
| `BinaryExpression.Type` | alias field `expression_type` |
| `BindingEntryBase.GetValue` | alias fn `try_get_value` |
| `BindingNotification..ctor` (2) | alias fn `with_error_and_fallback`, alias fn `with_error` |
| `Clock..ctor` | alias fn `with_parent` |
| `CombinedGeometry..ctor` (3) | alias fn `with_geometries`, alias fn `with_mode_and_transform`, alias fn `with_mode` |
| `CompiledBinding..ctor` | alias fn `empty` |
| `Compositor..ctor` (2) | alias fn `with_render_thread`, alias fn `with_scheduler` |
| `Compositor.Loop` | alias fn `render_loop` |
| `ContainerQuery..ctor` | alias fn `with_query_fn` |
| `ContainerQuery.ToString` | alias fn `to_display_string` |
| `ControlTheme..ctor` | alias fn `with_target_type` |
| `ControlTheme.ToString` | alias fn `to_display_string` |
| `CornerRadius..ctor` | alias fn `top_bottom` |
| `CroppedBitmap..ctor` | alias fn `with_source` |
| `Cursor.Default` | alias fn `default_cursor` |
| `DashStyle..ctor` | alias fn `with_dashes` |
| `DispatcherFrame..ctor` (2) | alias fn `with_dispatcher`, alias fn `with_exit_when_requested` |
| `DispatcherTimer..ctor` (5) | alias fn `with_callback_and_dispatcher`, alias fn `with_callback`, alias fn `with_interval`, alias fn `with_priority_and_dispatcher`, alias fn `with_priority` |
| `DrawingBrush..ctor` | alias fn `with_drawing` |
| `DrawingImage..ctor` | alias fn `with_drawing` |
| `EffectiveValue.CoerceDefaultValueAndRaise` (2) | alias fn `set_coerced_default_value_and_raise` |
| `EllipseGeometry..ctor` | alias fn `with_rect` |
| `Expression.Type` | alias fn `expression_type` |
| `ExpressionVariant.Type` | alias fn `variant_type` |
| `FuncFramebufferRenderTarget..ctor` | alias fn `with_scene_info` |
| `GenericTextParagraphProperties..ctor` | alias fn `with_options` |
| `Geometry..ctor` | alias fn `construct` |
| `GradientStop..ctor` | alias fn `with_color_and_offset` |
| `HitTestVisitor..ctor` (2) | alias fn `for_geometry`, alias fn `for_point` |
| `IAssemblyDescriptor.AvaloniaResources` | alias fn `ferro_resources` |
| `ImageBrush..ctor` | alias fn `with_source` |
| `ImmediateDrawingContext..ctor` | alias fn `borrowed` |
| `ImmutableRadialGradientBrush..ctor` | alias fn `with_radius` |
| `ImmutableSolidColorBrush..ctor` | alias fn `with_opacity` |
| `ImplicitAnimationCollection.Contains` | alias fn `contains_pair` |
| `KeySpline..ctor` | alias fn `with_points` |
| `LineGeometry..ctor` | alias fn `with_points` |
| `LockedFramebuffer..ctor` | alias fn `with_data` |
| `MatrixTransform..ctor` | alias fn `with_matrix` |
| `NthChildSelector..ctor` | alias fn `with_direction` |
| `Pen..ctor` | alias fn `with_brush` |
| `PinchEventArgs..ctor` | alias fn `with_angle` |
| `PixelFormat.FormatEnum` | alias field `format` |
| `PointerEventArgs..ctor` | alias fn `with_previous_points` |
| `PointerPointProperties..ctor` (3) | alias fn `based_on`, alias fn `with_pen_state_and_contact_rect`, alias fn `with_pen_state` |
| `PointerPressedEventArgs..ctor` | alias fn `with_platform_input_event_cookie` |
| `PolyBezierSegment..ctor` | alias fn `with_points` |
| `PolyLineSegment..ctor` | alias fn `with_points` |
| `PolylineGeometry..ctor` (2) | alias fn `with_points_and_fill_rule`, alias fn `with_points` |
| `PullGestureRecognizer..ctor` | alias fn `with_direction` |
| `RawPointerEventArgs..ctor` | alias fn `with_point` |
| `RawTouchEventArgs..ctor` | alias fn `with_point` |
| `RectangleGeometry..ctor` (2) | alias fn `with_rect_and_radii`, alias fn `with_rect` |
| `ReflectionBinding..ctor` | alias fn `empty` |
| `RenderTargetBitmap..ctor` | alias fn `with_dpi` |
| `ResourceDictionary..ctor` | alias fn `with_owner` |
| `ResourceProvider..ctor` | alias fn `construct_with_owner` |
| `RetainedFramebuffer..ctor` | alias fn `with_row_bytes` |
| `Rotate3DTransform..ctor` | alias fn `with_values` |
| `RotateTransform..ctor` (2) | alias fn `with_angle_and_center`, alias fn `with_angle` |
| `ScaleTransform..ctor` | alias fn `with_scale` |
| `SelectorMatch..ctor` | alias fn `sometimes` |
| `ServerCompositionGradientBrush.s_IdOfGradientStopsProperty` | alias fn `id_of_gradient_stops_property` |
| `ServerResourceHelperExtensions` | type |
| `ServerResourceHelperExtensions.GetServer` (4) | alias free fn `brush_get_server`, alias free fn `geometry_get_server`, alias free fn `pen_get_server`, alias free fn `transform_get_server` |
| `Setter.GetValue` | alias fn `get_value_boxed` |
| `SkewTransform..ctor` | alias fn `with_angles` |
| `SolidColorBrush..ctor` | alias fn `with_color` |
| `SourceUntypedBindingEntry.GetDefaultValue` | alias fn `cached_default_value` |
| `SpanRider..ctor` | alias fn `at_latest_position` |
| `SpanStringTokenizer..ctor` (2) | alias fn `with_message`, alias fn `with_separator` |
| `SplineEasing..ctor` | alias fn `with_points` |
| `Style..ctor` | alias fn `with_selector_fn` |
| `Style.ToString` | alias fn `to_display_string` |
| `StyledElement.TypedLogicalChildren` | alias fn `logical_children` |
| `Styles..ctor` | alias fn `with_owner` |
| `SwapchainBase.DisposeAsync` | alias fn `dispose` |
| `TemplateBinding..ctor` | alias fn `empty` |
| `TextCharacters..ctor` | alias fn `from_str` |
| `TextInputOptions.Default` | alias fn `default_options` |
| `Thickness..ctor` | alias fn `symmetric` |
| `TranslateTransform..ctor` | alias fn `with_offset` |
| `TypedBindingEntry.GetDefaultValue` | alias fn `cached_default_value` |
| `UnaryExpression.Type` | alias field `expression_type` |
| `ValueStore.OnChanged` | alias fn `on_expression_changed` |
| `ValueStore.OnCompleted` | alias fn `on_expression_completed` |
| `Visual.TypedVisualChildren` | alias fn `visual_children` |
| `VisualBrush..ctor` | alias fn `with_visual` |
| `WeakEvents.AvaloniaPropertyChanged` | alias fn `ferro_property_changed` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Present, declared where the scanner does not read | 0 | 38 | 207 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| `GetHashCode` | 0 | 0 | 23 | `Hash` is derived where a type can be hashed; value types made of floats are not `Eq`/`Hash`. |
| System.Numerics interop | 0 | 0 | 5 | Conversions to and from the vector and matrix types of the .NET runtime. |
| Finalizers and `Dispose` replaced by `Drop` | 0 | 2 | 18 | Ownership frees what a finalizer or `Dispose` frees: the reason names the owner or the `Drop` implementation. |
| Static constructors | 0 | 0 | 16 | Static state is initialised in place or on first use; class handlers are added where the property is registered. |
| Non-generic and explicit collection interfaces | 2 | 4 | 80 | The untyped `IList`/`ICollection`/`IEnumerable` faces of a collection have no counterpart; enumeration is a snapshot or a slice, named in the reason. |
| Reflection, attributes, trimming, polyfills, debugger aids | 19 | 25 | 69 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| MSBuild, Roslyn and test-framework tooling | 5 | 3 | 10 | The build tasks, the source generators and the glue to the .NET test frameworks: replaced by the build crate (`ferroui-build`), `ferro_property!` and the harness of cargo, as the reason states. |
| Async, awaiters and .NET threading contexts | 3 | 2 | 40 | The awaiter pattern, synchronization and execution contexts and the asynchronous twins of synchronous members: `Future` implementations and the synchronous member, named in the reason. |
| Observer and event plumbing | 0 | 3 | 82 | Observer classes, weak-event subscriber interfaces and listener interfaces are closures and handler lists; the reason names the function that takes the closure. |
| Class hierarchies ported as enums or traits | 0 | 30 | 148 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Setters, fields and internal state | 2 | 2 | 31 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Constructors and overloads under other names | 0 | 4 | 58 | Overloads that collapse into one function (a slice for two collection types, an `Option` argument), and constructors the port names after what they take. |
| Replaced by a named Rust construct | 15 | 22 | 248 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Browser

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 2 | 2 | 88 |
| (b) | 0 | 0 | 7 |
| (c) | 0 | 0 | 0 |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `AvaloniaView..ctor` | alias fn `with_host` |
| `BrowserAppBuilder.SetupBrowserAppAsync` | alias fn `setup_browser_app` |
| `BrowserAppBuilder.StartBrowserAppAsync` | alias fn `start_browser_app` |
| `BrowserPlatformOptions.AvaloniaServiceWorkerScope` | alias field `ferro_service_worker_scope` |
| `BrowserPlatformOptions.RegisterAvaloniaServiceWorker` | alias field `register_ferro_service_worker` |
| `DomHelper.CreateAvaloniaHost` | alias free fn `create_ferro_host` |
| `RenderWorker.WorkerThreadId` | alias fn `thread_id` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Present, declared where the scanner does not read | 0 | 1 | 50 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| Finalizers and `Dispose` replaced by `Drop` | 0 | 0 | 2 | Ownership frees what a finalizer or `Dispose` frees: the reason names the owner or the `Drop` implementation. |
| Static constructors | 0 | 0 | 1 | Static state is initialised in place or on first use; class handlers are added where the property is registered. |
| Reflection, attributes, trimming, polyfills, debugger aids | 1 | 0 | 0 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Async, awaiters and .NET threading contexts | 0 | 0 | 18 | The awaiter pattern, synchronization and execution contexts and the asynchronous twins of synchronous members: `Future` implementations and the synchronous member, named in the reason. |
| Class hierarchies ported as enums or traits | 0 | 0 | 4 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Setters, fields and internal state | 0 | 0 | 2 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Replaced by a named Rust construct | 1 | 1 | 11 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Build.Tasks

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 3 | 1 | 28 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| IL back end of the markup compiler | 1 | 1 | 3 | Exists only to emit or load IL. The Rust emitter and the run-time interpreter stand for the IL back end (xaml.md, section 9). |
| Reflection, attributes, trimming, polyfills, debugger aids | 1 | 0 | 19 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| MSBuild, Roslyn and test-framework tooling | 0 | 0 | 6 | The build tasks, the source generators and the glue to the .NET test frameworks: replaced by the build crate (`ferroui-build`), `ferro_property!` and the harness of cargo, as the reason states. |
| Replaced by a named Rust construct | 1 | 0 | 0 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Controls

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 7 | 148 |
| (b) | 0 | 0 | 41 |
| (c) | 0 | 0 | 0 |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `BindingEvaluator.Value` | alias fn `set_evaluated_value` |
| `ColumnDefinition..ctor` (2) | alias fn `with_value`, alias fn `with_width` |
| `ContextMenu..ctor` | alias fn `with_interaction_handler` |
| `DefaultMenuInteractionHandler..ctor` | alias fn `with_overrides` |
| `EmbeddableControlRoot..ctor` | alias fn `with_impl` |
| `IInlineHost.VisualChildren` | alias fn `with_visual_children` |
| `InlineUIContainer..ctor` | alias fn `with_child` |
| `InputPaneStateEventArgs..ctor` | alias fn `with_animation` |
| `MaskedTextBox..ctor` | alias fn `with_provider` |
| `Menu..ctor` | alias fn `with_interaction_handler` |
| `MenuBase..ctor` | alias fn `construct_with` |
| `MenuFlyoutPresenter..ctor` | alias fn `with_interaction_handler` |
| `NativeMenuItem..ctor` | alias fn `with_header` |
| `NavigatedFromEventArgs..ctor` | alias fn `with_parameter` |
| `NavigatedToEventArgs..ctor` | alias fn `with_parameter` |
| `NavigatingFromEventArgs..ctor` | alias fn `with_parameter` |
| `NavigationEventArgs..ctor` | alias fn `with_parameter` |
| `Notification..ctor` | alias fn `empty` |
| `NumericUpDown.Value` | alias fn `set_numeric_value` |
| `OffscreenTopLevel.Impl` | alias fn `offscreen_impl` |
| `RangeBase.Value` | alias fn `set_range_value` |
| `RefreshRequestedEventArgs..ctor` | alias fn `with_deferral` |
| `RowDefinition..ctor` (2) | alias fn `with_height`, alias fn `with_value` |
| `Run..ctor` | alias fn `with_text` |
| `ScrollChangedEventArgs..ctor` | alias fn `with_event` |
| `ScrollablePullGestureRecognizer..ctor` | alias fn `with_direction` |
| `SelectingItemsControlSelectionAdapter..ctor` | alias fn `with_selector` |
| `SelectionModel..ctor` | alias fn `with_source` |
| `SizeChangedEventArgs..ctor` | alias fn `with_event_and_source` |
| `SpinEventArgs..ctor` (3) | alias fn `with_event_and_mouse_wheel`, alias fn `with_event`, alias fn `with_mouse_wheel` |
| `TextChangedEventArgs..ctor` | alias fn `with_event_and_source` |
| `TextChangingEventArgs..ctor` | alias fn `with_event_and_source` |
| `TopLevel..ctor` | alias fn `construct` |
| `Track.Value` | alias fn `set_range_value` |
| `Window..ctor` | alias fn `with_impl` |
| `WindowBase..ctor` | alias fn `construct_with_resolver` |
| `WindowNotificationManager..ctor` | alias fn `with_host` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Present, declared where the scanner does not read | 0 | 0 | 3 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| `GetHashCode` | 0 | 0 | 1 | `Hash` is derived where a type can be hashed; value types made of floats are not `Eq`/`Hash`. |
| Static constructors | 0 | 0 | 4 | Static state is initialised in place or on first use; class handlers are added where the property is registered. |
| Non-generic and explicit collection interfaces | 0 | 1 | 33 | The untyped `IList`/`ICollection`/`IEnumerable` faces of a collection have no counterpart; enumeration is a snapshot or a slice, named in the reason. |
| Reflection, attributes, trimming, polyfills, debugger aids | 0 | 1 | 2 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Observer and event plumbing | 0 | 1 | 1 | Observer classes, weak-event subscriber interfaces and listener interfaces are closures and handler lists; the reason names the function that takes the closure. |
| Class hierarchies ported as enums or traits | 0 | 0 | 15 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Setters, fields and internal state | 0 | 0 | 11 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Constructors and overloads under other names | 0 | 0 | 5 | Overloads that collapse into one function (a slice for two collection types, an `Option` argument), and constructors the port names after what they take. |
| Replaced by a named Rust construct | 0 | 4 | 73 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.DesignerSupport

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 0 | 1 |
| (b) | 0 | 0 | 2 |
| (c) | 0 | 0 | 0 |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `SimpleWebSocketHttpRequest.RespondAsync` | alias fn `respond` |
| `SimpleWebSocketHttpServer.AcceptAsync` | alias fn `accept` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Constructors and overloads under other names | 0 | 0 | 1 | Overloads that collapse into one function (a slice for two collection types, an `Option` argument), and constructors the port names after what they take. |

## Avalonia.Generators

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 31 | 0 | 0 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Reflection, attributes, trimming, polyfills, debugger aids | 7 | 0 | 0 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| MSBuild, Roslyn and test-framework tooling | 19 | 0 | 0 | The build tasks, the source generators and the glue to the .NET test frameworks: replaced by the build crate (`ferroui-build`), `ferro_property!` and the harness of cargo, as the reason states. |
| Replaced by a named Rust construct | 5 | 0 | 0 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.HarfBuzz

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 0 | 3 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Constructors and overloads under other names | 0 | 0 | 1 | Overloads that collapse into one function (a slice for two collection types, an `Option` argument), and constructors the port names after what they take. |
| Replaced by a named Rust construct | 0 | 0 | 2 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Headless

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 2 | 13 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Reflection, attributes, trimming, polyfills, debugger aids | 0 | 0 | 1 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Setters, fields and internal state | 0 | 0 | 3 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Replaced by a named Rust construct | 0 | 2 | 9 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Headless.NUnit

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 4 | 0 | 0 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Reflection, attributes, trimming, polyfills, debugger aids | 4 | 0 | 0 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |

## Avalonia.Headless.Vnc

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 0 | 0 |
| (b) | 0 | 0 | 0 |
| (c) | 3 | 0 | 0 |

### (c) Not justified: the waiver is removed and the item counts as missing

| Item | Kind | Was waived because | Now |
|---|---|---|---|
| `AvaloniaVncLogger.cs`, `HeadlessVncFramebufferSource.cs`, `HeadlessVncPlatformExtensions.cs` | 3 file | not-applicable: out of scope: a VNC server that shows the frames of a headless application, built on a .NET VNC library; nothing in the port depends on it | out of scope |

## Avalonia.Headless.XUnit

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 14 | 0 | 0 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Reflection, attributes, trimming, polyfills, debugger aids | 14 | 0 | 0 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |

## Avalonia.Markup

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 30 | 55 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Class hierarchies ported as enums or traits | 0 | 30 | 55 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |

## Avalonia.Markup.Xaml

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 1 | 2 | 31 |
| (b) | 0 | 0 | 9 |
| (c) | 0 | 0 | 0 |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `CompiledBindingExtension..ctor` | alias fn `with_path` |
| `DynamicResourceExtension..ctor` | alias fn `with_resource_key` |
| `MergeResourceInclude..ctor` | alias fn `with_service_provider` |
| `PropertyInfoAccessorFactory.CreateAvaloniaPropertyAccessor` | alias fn `create_ferro_property_accessor` |
| `ReflectionBindingExtension..ctor` | alias fn `with_path` |
| `RelativeSourceExtension..ctor` | alias fn `with_mode` |
| `ResourceInclude..ctor` | alias fn `with_service_provider` |
| `StaticResourceExtension..ctor` | alias fn `with_resource_key` |
| `StyleInclude..ctor` | alias fn `with_service_provider` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Present, declared where the scanner does not read | 0 | 0 | 3 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| MSBuild, Roslyn and test-framework tooling | 1 | 0 | 0 | The build tasks, the source generators and the glue to the .NET test frameworks: replaced by the build crate (`ferroui-build`), `ferro_property!` and the harness of cargo, as the reason states. |
| Observer and event plumbing | 0 | 0 | 4 | Observer classes, weak-event subscriber interfaces and listener interfaces are closures and handler lists; the reason names the function that takes the closure. |
| Replaced by a named Rust construct | 0 | 2 | 24 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Markup.Xaml.Loader

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 1 | 6 | 83 |
| (b) | 0 | 0 | 22 |
| (c) | 0 | 0 | 0 |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `AvaloniaXamlDiagnosticCodes.AvaloniaIntrinsicsError` | alias const `FERRO_INTRINSICS_ERROR` |
| `AvaloniaXamlDiagnosticCodes.XamlXDiagnosticCodeToAvalonia` | alias fn `xaml_x_diagnostic_code_to_ferro` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaAttachedPropertyT` | alias field `ferro_attached_property_t` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaList` | alias field `ferro_list` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaListAttribute` | alias field `ferro_list_attribute` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaObject` | alias field `ferro_object` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaObjectBindMethod` | alias field `ferro_object_bind_method` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaObjectExtensions` | alias field `ferro_object_extensions` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaObjectSetStyledPropertyValue` | alias field `ferro_object_set_styled_property_value` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaObjectSetValueMethod` | alias field `ferro_object_set_value_method` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaProperty` | alias field `ferro_property` |
| `AvaloniaXamlIlWellKnownTypes.AvaloniaPropertyT` | alias field `ferro_property_t` |
| `AvaloniaXamlIlWellKnownTypesExtensions.GetAvaloniaTypes` | alias fn `get_ferro_types` |
| `IXamlIlAvaloniaProperty.AvaloniaProperty` | alias fn `ferro_property` |
| `IXamlIlAvaloniaPropertyNode.AvaloniaPropertyType` | alias fn `ferro_property_type` |
| `XamlIlAvaloniaClassProperty.AvaloniaPropertyType` | alias fn `ferro_property_type` |
| `XamlIlAvaloniaProperty.AvaloniaProperty` | alias fn `ferro_property` |
| `XamlIlAvaloniaPropertyFieldNode.AvaloniaPropertyType` | alias fn `ferro_property_type` |
| `XamlIlAvaloniaPropertyHelper.GetAvaloniaPropertyType` | alias fn `get_ferro_property_type` |
| `XamlIlAvaloniaPropertyNode..ctor` | alias fn `with_property_type` |
| `XamlIlAvaloniaPropertyNode.AvaloniaPropertyType` | alias fn `ferro_property_type` |
| `XamlIlPropertyInfoAccessorFactoryEmitter.EmitLoadAvaloniaPropertyAccessorFactory` | alias fn `emit_load_ferro_property_accessor_factory` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| IL back end of the markup compiler | 0 | 2 | 51 | Exists only to emit or load IL. The Rust emitter and the run-time interpreter stand for the IL back end (xaml.md, section 9). |
| Present, declared where the scanner does not read | 0 | 0 | 8 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| Reflection, attributes, trimming, polyfills, debugger aids | 1 | 0 | 0 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Class hierarchies ported as enums or traits | 0 | 3 | 10 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Setters, fields and internal state | 0 | 0 | 13 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Replaced by a named Rust construct | 0 | 1 | 1 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.MicroCom

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 0 | 6 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Class hierarchies ported as enums or traits | 0 | 0 | 6 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |

## Avalonia.Native

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 10 | 59 |
| (b) | 0 | 1 | 17 |
| (c) | 1 | 0 | 0 |

### (c) Not justified: the waiver is removed and the item counts as missing

| Item | Kind | Was waived because | Now |
|---|---|---|---|
| `AvaloniaNativeGlPlatformGraphics.cs` | 1 file | not-applicable: not ported for now (row 18 of CRITICAL-PATH.md): the desktop build of the Skia backend has Graphite on Metal and no Ganesh, so nothing could draw through the OpenGL platform graphics of the macOS backend. Applicable when the desktop build has Ganesh on OpenGL: the file is then ported to `avalonia_native_gl_platform_graphics.rs` under the mapped name, with the Skia side of the external objects on OpenGL (`Gpu/OpenGl/GlSkiaExternalObjectsFeature.cs`, reported as missing); the contracts it implements are in `ferroui-opengl` already (`i_gl_context_external_objects_feature.rs`) | missing |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `AvaloniaNativeMenuExporter..ctor` (3) | alias fn `for_application`, alias fn `for_tray_icon`, alias fn `for_window` |
| `AvaloniaNativePlatformExtensions.UseAvaloniaNative` | alias fn `use_ferro_native` |
| `AvaloniaNativePlatformOptions.AvaloniaNativeLibraryPath` | alias field `ferro_native_library_path` |
| `Helpers` | type |
| `Helpers.ToAvaloniaPixelPoint` | alias free fn `to_ferro_pixel_point` |
| `Helpers.ToAvaloniaPixelRect` | alias free fn `to_ferro_pixel_rect` |
| `Helpers.ToAvaloniaPoint` | alias free fn `to_ferro_point` |
| `Helpers.ToAvaloniaRect` | alias free fn `to_ferro_rect` |
| `Helpers.ToAvaloniaSize` | alias free fn `to_ferro_size` |
| `Helpers.ToAvnPoint` (2) | alias free fn `pixel_point_to_frn_point`, alias free fn `to_frn_point` |
| `Helpers.ToAvnRect` | alias free fn `to_frn_rect` |
| `Helpers.ToAvnSize` | alias free fn `to_frn_size` |
| `Helpers.ToAvnString` | alias free fn `to_frn_string` |
| `MacOSPlatformOptions.DisableAvaloniaAppDelegate` | alias field `disable_ferro_app_delegate` |
| `TopLevelImpl.MouseDevice` | alias fn `mouse` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Present, declared where the scanner does not read | 0 | 1 | 6 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| Finalizers and `Dispose` replaced by `Drop` | 0 | 1 | 11 | Ownership frees what a finalizer or `Dispose` frees: the reason names the owner or the `Drop` implementation. |
| Async, awaiters and .NET threading contexts | 0 | 0 | 1 | The awaiter pattern, synchronization and execution contexts and the asynchronous twins of synchronous members: `Future` implementations and the synchronous member, named in the reason. |
| Class hierarchies ported as enums or traits | 0 | 0 | 1 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Setters, fields and internal state | 0 | 0 | 10 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Replaced by a named Rust construct | 0 | 8 | 30 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.OpenGL

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 2 | 30 |
| (b) | 0 | 0 | 2 |
| (c) | 0 | 0 | 6 |

### (c) Not justified: the waiver is removed and the item counts as missing

| Item | Kind | Was waived because | Now |
|---|---|---|---|
| `EglDisplay..ctor` | 1 ctor | two of the three constructors are ported (`EglDisplay::new`, `EglDisplay::new_with_creation_options`); the one without arguments creates the default display of the EGL library of the system through `new EglInterface()`, which loads the library by name and is not ported: a platform resolves the entry points and names the interface in the options | missing |
| `EglDisplay.DisplayLockIsSharedWithContexts` | 1 property | says whether the contexts of the display share its monitor: the display and its contexts belong to one thread (`Rc&lt;EglDisplay&gt;`, `Rc&lt;EglContext&gt;`) and have no monitor (`lock` and `ensure_locked` return handles that do nothing). Applicable when an EGL platform renders on the render thread while another thread uses a context of the same display: the display then needs the monitor and this member | missing |
| `EglDisplay.ContextSharedSyncRoot` | 1 property | the monitor of the display for its contexts: the display and its contexts belong to one thread (`Rc&lt;EglDisplay&gt;`, `Rc&lt;EglContext&gt;`) and have no monitor. Applicable with `DisplayLockIsSharedWithContexts`, when an EGL platform renders on the render thread | missing |
| `EglInterface..ctor` | 2 ctor | the constructors that load the EGL library by name (`NativeLibrary.Load`) are left to the platform backend that has EGL: it resolves `eglGetProcAddress` of its library and calls `EglInterface::new` with the loader, as `GlInterface::new` is called | missing |
| `EglPlatformGraphics.TryCreate` | 1 method | the overload with a display factory is ported; the one without arguments creates the default display through `new EglInterface()`, which loads the EGL library by name and is not ported | missing |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `ExternalObjectsInterface..ctor` | alias fn `try_new` |
| `GlVersion..ctor` | alias fn `with_compatibility_profile` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Reflection, attributes, trimming, polyfills, debugger aids | 0 | 0 | 4 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Class hierarchies ported as enums or traits | 0 | 2 | 15 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Setters, fields and internal state | 0 | 0 | 1 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Constructors and overloads under other names | 0 | 0 | 3 | Overloads that collapse into one function (a slice for two collection types, an `Option` argument), and constructors the port names after what they take. |
| Replaced by a named Rust construct | 0 | 0 | 7 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Remote.Protocol

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 1 | 3 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Reflection, attributes, trimming, polyfills, debugger aids | 0 | 1 | 2 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Class hierarchies ported as enums or traits | 0 | 0 | 1 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |

## Avalonia.Skia

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 0 | 21 |
| (b) | 0 | 0 | 12 |
| (c) | 3 | 0 | 0 |

### (c) Not justified: the waiver is removed and the item counts as missing

| Item | Kind | Was waived because | Now |
|---|---|---|---|
| `Gpu/Vulkan/VulkanSkiaExternalObjectsFeature.cs` | 1 file | not-applicable: import of external images and semaphores into the Skia context on Vulkan (`IVulkanContextExternalObjectsFeature` of Avalonia.Vulkan): out of scope while Avalonia.Vulkan is (scripts/api-extract/projects.json). Applicable when a platform with Vulkan is ported, after the external objects of the backend on OpenGL and Metal | missing |
| `Gpu/Vulkan/VulkanSkiaGpu.cs` | 1 file | not-applicable: the GPU of the backend over a Vulkan platform graphics context (`IVulkanPlatformGraphicsContext` of Avalonia.Vulkan): out of scope while Avalonia.Vulkan is (scripts/api-extract/projects.json). Applicable when a platform with Vulkan is ported | missing |
| `Gpu/Vulkan/VulkanSkiaRenderTarget.cs` | 1 file | not-applicable: the render target and render session of the backend over a Vulkan render target (`IVulkanRenderTarget` of Avalonia.Vulkan): out of scope while Avalonia.Vulkan is (scripts/api-extract/projects.json). Applicable when a platform with Vulkan is ported | missing |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `DrawingContextHelper.WrapSkiaCanvas` | alias free fn `wrap_skia_surface` |
| `ISkiaSharpApiLease.SkCanvas` | alias fn `with_sk_canvas` |
| `ImmutableBitmap..ctor` | alias fn `resized` |
| `SkiaSharpExtensions.ToAvalonia` (3) | alias free fn `slant_to_font_style`, alias free fn `to_pixel_format_opt`, alias free fn `to_text_alignment` |
| `SkiaSharpExtensions.ToAvaloniaLtrbPixelRect` | alias free fn `to_ltrb_pixel_rect` |
| `SkiaSharpExtensions.ToAvaloniaMatrix` (2) | alias free fn `matrix44_to_matrix`, alias free fn `to_matrix` |
| `SkiaSharpExtensions.ToAvaloniaPixelRect` | alias free fn `to_pixel_rect` |
| `SkiaSharpExtensions.ToAvaloniaRect` | alias free fn `to_rect` |
| `SkiaSharpExtensions.ToSkia` | alias free fn `font_style_to_slant` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Present, declared where the scanner does not read | 0 | 0 | 5 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| Reflection, attributes, trimming, polyfills, debugger aids | 0 | 0 | 1 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Class hierarchies ported as enums or traits | 0 | 0 | 8 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Constructors and overloads under other names | 0 | 0 | 1 | Overloads that collapse into one function (a slice for two collection types, an `Option` argument), and constructors the port names after what they take. |
| Replaced by a named Rust construct | 0 | 0 | 6 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## Avalonia.Themes.Fluent

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 0 | 5 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 0 | 0 |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Non-generic and explicit collection interfaces | 0 | 0 | 5 | The untyped `IList`/`ICollection`/`IEnumerable` faces of a collection have no counterpart; enumeration is a snapshot or a slice, named in the reason. |

## Avalonia.X11

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 0 | 108 | 1085 |
| (b) | 0 | 0 | 0 |
| (c) | 0 | 1 | 5 |

### (c) Not justified: the waiver is removed and the item counts as missing

| Item | Kind | Was waived because | Now |
|---|---|---|---|
| `GrabResult.AlreadyGrabbed`, `GrabResult.GrabFrozen`, `GrabResult.GrabInvalidTime`, `GrabResult.GrabNotViewable`, `GrabResult.GrabSuccess`, `XLib.GrabResult` | 1 enum, 5 enum member | the result of `XGrabPointer`, which the drag source uses (stage 2 of docs/porting/x11-platform.md) | missing |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| Structures of a bindings crate | 0 | 107 | 1083 | The structure is declared by the bindings crate the backend uses (`x11-dl`, with the layout of the C headers); the port declares the enumerations and what the bindings lack. |
| Replaced by a named Rust construct | 0 | 1 | 2 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |

## XamlX

| Class | Files | Types | Members |
|---|---:|---:|---:|
| (a) | 26 | 5 | 156 |
| (b) | 0 | 0 | 9 |
| (c) | 0 | 0 | 0 |

### (b) Stale: the item exists and counts as present

| Item | Found as |
|---|---|
| `AnonymousParameterInfo..ctor` | alias fn `with_index` |
| `ContextDiagnosticExtensions.ToDiagnostic` | alias free fn `exception_to_diagnostic` |
| `TransformerConfiguration.GetCustomAttribute` (2) | alias fn `get_custom_attribute_for_property`, alias fn `get_custom_attributes_for_property` |
| `XamlAstClrProperty..ctor` (2) | alias fn `with_setter_methods`, alias fn `with_setters` |
| `XamlAstXamlPropertyValueNode..ctor` | alias fn `with_values` |
| `XamlNoReturnMethodCallNode..ctor` | alias fn `with_wrapped_method` |
| `XamlStaticOrTargetedReturnMethodCallNode..ctor` | alias fn `with_wrapped_method` |

### (a) Still right

| Group | Files | Types | Members | Reason of the group |
|---|---:|---:|---:|---|
| IL back end of the markup compiler | 20 | 3 | 37 | Exists only to emit or load IL. The Rust emitter and the run-time interpreter stand for the IL back end (xaml.md, section 9). |
| Present, declared where the scanner does not read | 0 | 0 | 2 | The item exists; a macro, an `extern` block or a naming rule of the port keeps the scanner from seeing it. The reason names the macro or the item, and the name was found in the tree. |
| `GetHashCode` | 0 | 0 | 2 | `Hash` is derived where a type can be hashed; value types made of floats are not `Eq`/`Hash`. |
| Reflection, attributes, trimming, polyfills, debugger aids | 6 | 0 | 4 | A .NET-only concept: attributes read by reflection (replaced by the markup metadata the reason names), annotations for the trimmer, polyfills of newer runtime API, obsolete members, debugger and diagnostic aids. |
| Class hierarchies ported as enums or traits | 0 | 2 | 3 | A class hierarchy is an enum with variants, or an abstract class is a trait plus an embedded state struct: the members live on the enum, the trait or the struct the reason names. |
| Setters, fields and internal state | 0 | 0 | 42 | The value is a public field or cell, is written by the one function that owns it, or is created with a struct literal: there is no separate setter or constructor of that name. |
| Constructors and overloads under other names | 0 | 0 | 29 | Overloads that collapse into one function (a slice for two collection types, an `Option` argument), and constructors the port names after what they take. |
| Replaced by a named Rust construct | 0 | 0 | 37 | A different design takes the place of the member; the reason in the data file names the Rust item, and the name was found in the tree. |
