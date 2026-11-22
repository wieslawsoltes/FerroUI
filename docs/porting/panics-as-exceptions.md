# Panics as exceptions

Status: step 1 of #567, 2026-11-06: `ferroui-base` first, then every other crate of the
workspace. The exception-like flows that production code relies on are converted where that is
clean (section 5); the rest is deferred with its reason (section 6). 2026-11-19: the flows of
the development tools are converted too (hot reload, the previewer, the Designer's stages and
animation preview, section 10); what is left as (E) is listed with its reason in section 3.
0.6 (breaking changes allowed): the boundaries where the framework calls the application's own
code get fallible forms, and errors of application code reach one defined place, the
unhandled-exception events of the dispatcher, without a panic (section 11). What still catches
a panic is listed in 11.4: boundaries that only matter in a build that unwinds.

Issue #567 replaces panics that carry an error between framework functions with `Result`, and
keeps `catch_unwind` only at true boundaries. The owner decided (issue #567, 2026-11-06) that the
shipped build aborts on panic, fully supported: `dist` uses `panic = "abort"` (since part 5 of
#567), with `dist-unwind` for applications that host plugins. On 2026-11-07 the owner extended
this to `release` (`release-unwind` keeps unwinding); `fast-release`, the profile for running
applications and tools locally, unwinds for the development tools. So no production code path may rely on catching a panic
to work correctly. With abort, `catch_unwind` never returns `Err`: a panic runs the panic hook
and ends the process. Cargo ignores `panic` for tests, benches, build scripts and proc macros, so
the test suites always unwind.

This page classifies every use of `catch_unwind`, `resume_unwind`, `panic_any` and
`std::panic::set_hook` / `take_hook` in `src` (found with
`grep -rn "catch_unwind\|resume_unwind\|panic_any\|set_hook(\|take_hook(" src`, `use` lines and
comments left out, test files included in the counts) and records what happens to each, and what
happens with abort.

## 1. Classes

- **(B)** Boundary, kept: an FFI callback, the top of the dispatcher loop or of a thread, or
  application code whose panic must not take the loop down, where the reference isolates the
  exception too (an exception in one subscriber does not stop the others, or is rethrown later on
  the dispatcher). The try/finally of the reference around such code is part of the boundary.
  Each is marked `// Boundary (#567): ...` in the code.
- **(E)** Exception-like flow, to become `Result`: a panic used to carry an error between
  framework functions (a conversion or validation failure, a binding error, a parse error, a
  lookup that finds nothing) that framework code catches and turns into an error value or a
  fallback. Listed too: a typed panic payload that reports an expected failure although nothing
  catches it.
- **(T)** Test or diagnostics support: a test file, a `cfg(test)` module, a testing feature, a
  test harness or a test page. Untouched.
- **(R)** Rethrow: a `resume_unwind` that raises a caught panic again on purpose. Kept when it
  belongs to a (B) site.

The column **With abort** of the tables says what happens with `panic = "abort"`: "no change"
when the catch only matters when there is a bug (a panic of application code, which is a crash
anyway; an FFI guard), so correct operation is the same; "breaks" when the feature relies on
catching, because an expected error travels as a panic.

## 2. Counts

Per crate, after the conversions of section 5. The (T) column counts the sites of test files.

| Crate | B | R | E | T | All |
|---|---:|---:|---:|---:|---:|
| `ferroui-base` | 15 | 14 | 0 | 76 | 105 |
| `ferroui-system` | 3 | 4 | 0 | 6 | 13 |
| `ferroui-markup`, `ferroui-markup-xaml`, `-loader`, `-winui` | 0 | 0 | 0 | 9 | 9 |
| platform crates (section 8) | 27 | 13 | 4 | 36 | 80 |
| libraries: Headless, Dock, Behaviors, Controls, Rx, ReactiveUI, Splat (section 9) | 37 | 21 | 5 | 214 | 277 |
| tools: View, Remote, Inspect, MCP, DevTools, HotReload, HotPatch, Designer (section 10) | 19 | 1 | 1 | 7 | 28 |
| **All** | **101** | **53** | **10** | **348** | **512** |

The (E) sites before this step were 23: the throw and the error scope of the markup runtime (2,
with the 2 (R) of the scope, `xaml_error_scope.rs`), the failure of a composite page transition
(1, `composite_page_transition.rs:58`), the deferred resources the Designer reads (1,
`animation_runtime.rs:190`, now a (B)) and the UI Automation calls of Win32 (1,
`automation_node.rs:578`, now a (B)) are converted (section 5).

On 2026-11-19 the four (E) catches of the development tools became (B) with their flows
converted (section 10): the previewer's load and show (`remote_designer_entry_point.rs:380`),
the Designer's animation preview (`animation_runtime.rs:328`) and candidate stages
(`preview_stages.rs:1665`, `:1760`). The 10 (E) sites left, each with its reason:

| Site | Why it stays |
|---|---|
| `ferroui-win32` `wgl_platform_open_gl_interface.rs:90`, `ferroui-skia` `gl_skia_external_objects_feature.rs:479`, `ferroui-reactiveui` `binding_errors.rs:47` | converted already (5.4, 5.5, section 9); listed as (E) for the record |
| `ferroui-vello` `combined_geometry_impl.rs:133`, `gpu/window_surface.rs:166` | panics of third-party code (`linesweeper`, the `Drop` of a `wgpu::Surface`) with no `Result` on our side; mitigated (5.5) |
| `ferroui-headless` `headless_unit_test_session.rs:696` | the `Canceled` of a test task; tests always unwind |
| `ferroui-reactiveui` `view_locator.rs:17`, `ferroui-splat` `log_host.rs:17`, `default_log_manager.rs:28` | a missing registration (a misconfiguration, a bug of the application); nothing catches them |
| `ferroui-hot-reload` `lib.rs:651` | `raise_reload_error` outside every step; nothing reaches it in correct operation |

## 3. What works and what breaks with `panic = "abort"`

Works the same in correct operation (the catch matters only for a bug, which aborts instead of
being logged, isolated or raised later): every (B) and (R) site, among them the dispatcher, the
render loops, the task executor, the FFI guards of Win32, X11/GLib, Wayland, the native (macOS)
backend, Android and the browser, the native control hosts, the Dock and Behaviors helpers, the
ReactiveUI and Splat observers, and the headless test session. The panic hooks of Android and
the browser run before the abort, so the crash reports stay.

Works with abort since this step (section 5):

- a fault of a ReactiveUI binding source with an inner error (`binding_errors.rs`): it goes, as a
  `TargetInvocationError` and without a panic, to the `ThrownExceptions` of the reactive object
  whose notification runs, or else to `RxState::default_exception_handler()` (owner decision of
  2026-11-07, #567);
- the error scope of the markup runtime (`with_xaml_errors`): `load_winui`, `parse_winui` and
  `parse_winui_as` return the error of a failing deferred resource, include or template of a
  fresh root as `Err`;
- the failure of a page transition (`CrossFade`, `PageSlide`, `Rotate3DTransition`,
  `CompositePageTransition`): `JoinError::Panicked` of the handle of `IPageTransition::start`;
- the deferred animation resources of the Designer that fail to build: left out;
- a missing required OpenGL or EGL entry point (5.5): the creation of the context, display or
  platform graphics fails with an `OpenGlError`, and the WGL start falls back to the next
  rendering mode;
- a GL context that cannot be made current while Skia releases a snapshot (5.5):
  `IGlContext::try_ensure_current` fails and the image is released without it;
- the page transitions of the control catalog (5.5): `JoinError::Panicked` of their handles;
- the Vello combined geometry of a path whose coordinates are not finite (empty, without
  reaching the sweep) and the swap chain of a lost device (left, without releasing it) (5.5).
- a UI Automation call the state of a control refuses (`set_value` on a read-only progress bar,
  scrolling a direction that cannot scroll, expanding a menu item without a submenu, ...): the
  provider returns an `AutomationError` and the Win32 node answers its result code (5.4).

Breaks with abort (an expected error travels as a panic), production paths:

| Site | What breaks |
|---|---|
| `ferroui-vello` `combined_geometry_impl.rs:133` | a `CombinedGeometry` of finite paths that the third-party sweep (`linesweeper`) still gives up on aborts instead of drawing nothing (paths that are not finite are refused before the sweep, 5.5) |
| `ferroui-vello` `gpu/window_surface.rs:166` | dropping a `wgpu` surface whose context cannot be made current while the device is not known to be lost aborts (a lost device's surface is left, 5.5) |

Works with abort since 2026-11-19, development tools (section 10):

- the errors the XAML runtime reports in a step of a hot reload (`catch_reload`): a deferred
  resource, include or template of the candidate that fails is `ReloadError::Failed`, as the
  `ReloadError`s of the resource lookups of a candidate are (#876);
- the errors of the edited document in the previewer of `DesignerSupport` (the load and the show)
  and in the candidate stages of the Designer (their layout, map and states): the error of the
  load or of the stage;
- the animation programs the framework rejects in the Designer's preview
  (`Animation::try_apply_for_preview`): the error of the seek.

The (B) sites of these tools isolate the code of the previewed application (its constructors,
handlers, layout callbacks and animators) and still need a build that unwinds (`dev`,
`fast-release`, `release-unwind` or `dist-unwind`); hot reload is `cfg(debug_assertions)` and
never in `release` or `dist`.

No change although listed as (E): the `Canceled` of the headless test session (tests always
unwind), `ViewLocator::current` of ReactiveUI and the two `LoggingError` panics of Splat (nothing
catches them; a misconfiguration is a crash either way).

## 4. `ferroui-base`

### 4.1 Counts

| Class | Sites | Change |
|---|---:|---|
| (B) | 15 | kept, each marked `// Boundary (#567): ...` |
| (R) | 14 | kept: 13 belong to (B) sites, 1 is the cleanup-and-rethrow of `Animation::apply_for_preview` |
| (E) | 0 | the one of step 1, the failure of a composite page transition, is converted (section 5.2) |
| (T) | 75 | untouched |
| **All** | **104** | |

Outside tests the crate has 29 sites, all in the dispatcher (24), the render loop and the
custom visual handler (2), the binding reload (1) and the animations (2). None of them is a
panic that framework code of this crate catches and turns into an error value or a fallback:
every `catch_unwind` outside tests either isolates application code as the reference does, or
runs a finally block and raises the same panic again. The exception-like flows whose catch is in
another crate are the page transitions (converted, 5.2), the deferred content of a resource
dictionary (converted with the markup scope, 5.1) and the preview of an animation (deferred,
6.1).

Line numbers are those after the boundary comments were added.

### 4.2 Dispatcher (`threading/`)

| File:line | Function | Catches or raises | Class | Change |
|---|---|---|---|---|
| `dispatcher_main_loop.rs:82` | `Dispatcher::push_frame` | catches a panic of a job of the frame | B | kept: the try/finally of the reference; the frame stack and the shutdown are settled first |
| `dispatcher_main_loop.rs:103` | `Dispatcher::push_frame` | raises that panic again | R | kept (belongs to :82) |
| `dispatcher_main_loop.rs:122` | `Dispatcher::main_loop` | catches a panic of the loop | B | kept: the top of the dispatcher loop; the token registration is disposed of first |
| `dispatcher_main_loop.rs:126` | `Dispatcher::main_loop` | raises it again | R | kept (belongs to :122) |
| `dispatcher_frame.rs:134` | `DispatcherFrame::run` | catches a panic of the platform run loop | B | kept: try/finally of the reference; the frame stops running first |
| `dispatcher_frame.rs:150` | `DispatcherFrame::run` | raises it again | R | kept (belongs to :134) |
| `dispatcher_invoke.rs:277` | `Dispatcher::send_inline` | catches a panic of the application callback | B | kept: goes through the unhandled-exception filter and events of the dispatcher |
| `dispatcher_invoke.rs:284` | `Dispatcher::send_inline` | raises it again when not handled | R | kept (belongs to :277) |
| `dispatcher_operation.rs:335` | `OperationCore::execute` | catches a panic of the job | B | kept: the completed handlers run whatever the job did, the depth is restored |
| `dispatcher_operation.rs:343` | `OperationCore::execute` | catches a panic of the completed handlers | B | kept: application handlers, the finally of the reference |
| `dispatcher_operation.rs:357` | `OperationCore::execute` | raises the panic of the job again | R | kept (belongs to :335) |
| `dispatcher_operation.rs:360` | `OperationCore::execute` | raises the panic of the handlers again | R | kept (belongs to :343) |
| `dispatcher_operation.rs:379` | `OperationCore::invoke_core` | catches a panic of a local callback | B | kept: the exception of the operation, filtered on the dispatcher or kept for its observer |
| `dispatcher_operation.rs:385` | `OperationCore::invoke_core` | catches a panic of a `Send` callback | B | kept: as :379 |
| `dispatcher_operation.rs:479` | `OperationCore::wait_with_limit` | catches a panic of the jobs the wait runs | B | kept: try/finally of the reference; the handlers are removed first |
| `dispatcher_operation.rs:507` | `OperationCore::wait_with_limit` | raises it again | R | kept (belongs to :479) |
| `dispatcher_operation.rs:597` | `OperationCore::push_operation_frame` | catches a panic of the nested frame | B | kept: try/finally of the reference; the timer and handlers go first |
| `dispatcher_operation.rs:604` | `OperationCore::push_operation_frame` | raises it again | R | kept (belongs to :597) |
| `dispatcher_operation.rs:624` | `OperationCore::take_outcome` | raises the kept panic of the callback to the reader of the result | R | kept (belongs to :379/:385): the reference rethrows the exception of an operation |
| `dispatcher_operation.rs:777` | `DispatcherOperation::wait` | raises the kept panic of the callback to the waiter | R | kept (belongs to :379/:385) |
| `dispatcher_task.rs:134` | `TaskShared::take_outcome` | raises the panic of a faulted task to its observer | R | kept (belongs to :338/:370): the reference rethrows the exception of a faulted task |
| `dispatcher_task.rs:276` | `DispatcherTask::wait` | raises the panic of a faulted task to the waiter | R | kept (belongs to :338/:370) |
| `dispatcher_task.rs:338` | `TaskStart::run` | catches a panic of the callback that makes the future | B | kept: application code; the panic faults the task |
| `dispatcher_task.rs:370` | `TaskBody::poll` | catches a panic of the future | B | kept: application code; the panic faults the task |

### 4.3 Rendering, data and animations

| File:line | Function | Catches or raises | Class | Change |
|---|---|---|---|---|
| `rendering/render_loop.rs:107` | `DefaultRenderLoop::timer_tick` | catches a panic of a render loop task | B | kept: the top of the render loop; logged as "Exception in render loop", as the reference does |
| `rendering/composition/server/server_custom_composition_visual.rs:30` | `guarded` | catches a panic of a custom visual handler | B | kept: application code on the render thread; logged as the reference logs the exception of a handler |
| `data/local_binding_reload.rs:252` | `<LocalBindingReload<T> as Drop>::drop` | catches a panic of the rollback | B | kept: application observers run from a destructor, where a second panic during unwinding would abort |
| `animation/animation.rs:415` | `Animation::apply_for_preview` | catches a panic of the preview setup | R | kept: cleanup and the same panic raised again, not an error value (see 6.1) |
| `animation/animation.rs:451` | `Animation::apply_for_preview` | raises it again after disposing of the setup steps | R | kept (belongs to :415) |

Until this step the table had a seventh row, `animation/composite_page_transition.rs:58`
(`CompositePageTransition::start`), an (E): the `resume_unwind` of the `JoinError::Panicked` of a
child transition, so that the task of the composite failed with it. Converted (section 5.2), with
the `panic!` of `i_page_transition.rs` (`when_all`) and of a canceled child, which the grep does
not count.

### 4.4 Tests and test support (T)

75 sites, untouched. They are in test files (`*_tests.rs`, `tests/`, `animation/tests/`) and in
these `cfg(test)` modules of other files:

| File | Sites | What |
|---|---:|---|
| `ferro_property_metadata_construction_oracles.rs` | 2 | `cfg(test)` module (`ferro_property_metadata.rs:293`) |
| `utilities/decimal.rs` | 5 | `mod tests` |
| `media/glyph_drawing_options.rs` | 3 | `mod tests` |
| `utilities/object_pool.rs` | 1 | `mod tests` |
| `utilities/ferro_property_dictionary.rs` | 1 | `mod tests` |
| `utilities/thread_bound.rs` | 1 | `mod tests` |
| `rendering/platform_render_interface_context_manager.rs` | 1 | `mod tests` |
| `rendering/native_render_scope.rs` | 1 | `cfg(all(test, target_os = "macos"))` module |

## 5. Conversions

Behaviour under the default unwinding profile is the same, except where noted; the deviations
are recorded in `DEVIATIONS.md` (Markup metadata and markup events, W2).

### 5.1 The markup throw and the error scope (`ferroui-markup-xaml`)

Before: a member that a load reaches without an error channel (the contracts the framework
calls: `IDeferredContent::build` of a deferred resource, the resource contracts of
`ResourceInclude` and `StyleInclude`, the `build` of template content) threw with
`crate::throw`. Outside a scope that is a panic with the message of the `XamlLoadError`; inside
`with_xaml_errors` the throw recorded the error and unwound with a private token
(`resume_unwind`), which the scope caught (`catch_unwind`) and turned into `Err`. With abort the
scope could not work: the WinUI importer's fresh-root loads aborted on the error of a deferred
resource.

The members that markup calls through metadata already had an error channel: the `try fn`
methods and `try_get` properties of `ferro_markup_type!` (`DeferredContent.Build`,
`XamlIlRuntimeHelpers.DeferredTransformationFactoryV1` to `V3`, `TemplateContent.Load`,
`OnFormFactorExtension.ShouldProvideOption`, `PropertyInfoAccessorFactory.CreateFerroPropertyAccessor`,
`ResourceInclude.Loaded` and `Owner`, `StyleInclude.Loaded`, `XBindExtension.ProvideValue`)
return `Result<_, MarkupInvokeError>` or a `XamlLoadError`, which the run-time loader turns into
a load error. What remained were the trait contracts, which return a value. After:

- **The scope does not unwind.** `with_xaml_errors` runs the load and returns the first error
  reported in it; it no longer catches panics (a panic leaves the scope and closes it, with a
  drop guard). Inside a scope a failing member reports its error
  (`xaml_error_scope::report(error) -> Result<(), XamlLoadError>`, the error back when no scope
  is open) and returns what it has for nothing. The first error is kept; later ones, often
  consequences of the first, are dropped.
- **`crate::throw_or(result, fallback)`** (new, crate-private) is the throw of those members:
  `Ok(value)` is the value; `Err(error)` goes to the scope and gives `fallback()`; outside a scope
  it panics with the message, as before.
- **`IDeferredContent::try_build`** (new provided method, `ferroui-base`):

  ```rust
  fn try_build(
      &self,
      service_provider: Option<&Rc<dyn IServiceProvider>>,
  ) -> Result<Option<BoxedValue>, DeferredContentFailed> // default: Ok(self.build(service_provider))
  ```

  `DeferredContent` (`ferroui-markup-xaml`) overrides it: an error of `try_build_with` goes to the
  scope and the result is `Err(DeferredContentFailed)`; outside a scope it panics as `build`
  does. `ResourceDictionary::try_get_value` builds with it: a failed build is not found and stays
  deferred (it was not stored before either: the panic unwound past the store).
- **The members that a load reaches** report instead of throwing:
  `DeferredContent::build_with` (an empty `DeferredResult`), `TemplateContent::load_as` and
  `load` (`None`: the template builds nothing), `ItemsPanelTemplate::build` (`None`), and the
  resource contracts of `ResourceInclude` and `StyleInclude` (`has_resources`,
  `try_get_resource`, `owner`, `owner_changed`, `add_owner`, `remove_owner`: as if the include
  were empty; through a private `loaded_or_reported`). Their signatures do not change.
- **The run-time loader stops early.** `ferroui-markup-xaml-loader` checks
  `xaml_error_scope::has_error()` after each node it evaluates (`Interpreter`'s
  `evaluate_node`) and returns a load error, which the scope replaces with the error reported.
- **`throw` stayed the fatal fallback** until 0.6, which removes it (11.2)
  (`xaml_error_scope::throw(error) -> !`, a plain `panic!`) for members that a scoped load does not reach and that have no value to go on with:
  `XamlIlRuntimeHelpers::deferred_transformation_factory_v1` to `_v3` and `_for` (the loader and
  metadata call `try_deferred_transformation_factory_for`), `RelativeSourceExtension::provide_value`,
  `OnFormFactorExtension::should_provide_option`,
  `PropertyInfoAccessorFactory::create_ferro_property_accessor` (metadata calls the `try_` forms),
  `WindowDrawnDecorationsTemplate::build` (window chrome, after the load) and
  `XBinding::create_instance` (its only error is a missing target property; the loader and the
  property system always pass one). Inside a scope these now panic instead of giving `Err`; none
  is reached by the loads of `load_winui`.

Behaviour under unwinding: the result of a scoped load is the same `Err`, with the same typed
error and inner exception (`fresh_root_deferred_resource_error_is_returned_and_default_loader_still_throws`
of `ferroui-markup-xaml-winui`). What differs is what runs between the failure and the end of
the load: the member returns its fallback and the code that called it goes on (a resource lookup
continues in the parent nodes and fails with "Static resource not found", which the loader
returns and the scope replaces), up to the next node of the loader, instead of unwinding at
once. A `catch_unwind` inside the scope no longer sees these errors. Outside a scope nothing
changes. Tests: `xaml_error_scope::tests`.

Run-time loading of application XAML outside the WinUI importer (`FerroRuntimeXamlLoader::load`,
`parse`) opened no scope before 0.6: a failing deferred resource of such a load panicked. Since
0.6 the loader opens one, and the error is the `Err` of the load (11.2).

### 5.2 Failures of page transitions (`ferroui-base`, `ferroui-system`)

Before: `IPageTransition::start` returns a `JoinHandle<()>`, whose contract reports a failure to
run an animation as `JoinError::Panicked`. The transitions made that value with a panic, which
the executor of `ferroui-system` caught: `when_all` of `i_page_transition.rs` panicked with the
error of a failed `AnimationTask` (`CrossFade`, `PageSlide`, `Rotate3DTransition`), and
`CompositePageTransition` raised the payload of a failed child again (`resume_unwind`) or
panicked with `Canceled` when a child was canceled. With abort, a failed animation of a
transition aborted.

After: a new executor primitive of `ferroui-system`,

```rust
pub fn spawn_local_on_result<F, T>(future: F, schedule: Arc<dyn Schedule>, first_poll: FirstPoll) -> JoinHandle<T>
where
    F: Future<Output = Result<T, JoinError>> + 'static,
    T: 'static;
```

runs a future as `spawn_local_on` does and writes its output to the handle: `Err(error)` ends the
task with `error` without a panic. The body of a transition is now a
`Future<Output = Result<(), JoinError>>` (`start_async`, crate-private, on the schedule of the
current dispatcher at the continuation priority, as `start_local_with_priority` did), `when_all`
returns `Result<(), JoinError>`, and the composite returns the `JoinError` of a child. The
handles end with the same values as before: `JoinError::Panicked` with a `String` payload, the
message of the animation error or of `Canceled`, or the payload of the child. The only difference
under unwinding: the panic hook no longer prints these failures. The signature of
`IPageTransition::start` does not change. Test: `executor_tests.rs`,
`a_task_on_a_schedule_takes_its_outcome_from_the_result_of_its_future_without_a_panic`.

The six page transitions of the control catalog (`samples/ControlCatalog`) that panicked with
the error of `AnimationTask::when_all` are converted too (5.5).

### 5.3 The deferred animation resources of the Designer

`animation_runtime.rs:190` of `ferroui-designer` read the animation resources of the edited
document with `ResourceDictionary::get` inside a `catch_unwind`, so that a deferred resource that
fails to build is left out. It now reads them inside `with_xaml_errors`: the failure goes to the
scope (5.1) and the key is left out without unwinding. The catch stays as the boundary of the
constructors of the previewed application that the builder runs (a development tool).

### 5.4 UI Automation calls the state of a control refuses (`ferroui-controls`, the platform bridges)

Before: the automation peers rejected a call of a client that the state of the control does not
allow with `panic!`, the exceptions of the reference: `set_value` on a read-only progress bar
(`InvalidOperationException`), `scroll` and `set_scroll_percent` of a scroll viewer in a
direction that cannot scroll (`InvalidOperationException`) or with a percentage out of range
(`ArgumentOutOfRangeException`), `add_to_selection`, `remove_from_selection` and `select` of a
radio button (`InvalidOperationException`), `expand` and `collapse` of a menu item without a
submenu (`InvalidOperationException`), `set_value` of a combo box that is not editable
(`InvalidOperationException`) and of a calendar (`NotSupportedException`); in
`ferroui-controls-treedatagrid` also `set_value` of a row and `toggle` of a read-only check box
cell (`NotSupportedException`). `AutomationNode::invoke_sync` of `ferroui-win32` caught the panic
and answered `E_FAIL`; with abort a UI Automation client (Narrator, Inspect, UI tests) ended the
application.

After: one error type, `ferroui_controls::automation::AutomationError`:

```rust
#[non_exhaustive]
pub enum AutomationError {
    ElementNotEnabled(ElementNotEnabledError),
    InvalidOperation(String),
    ArgumentOutOfRange(String), // the name of the parameter
    NotSupported(String),
}
```

with `From<ElementNotEnabledError>`, so that `ensure_enabled()?` keeps working. The members that
can be refused return it: `IRangeValueProvider::set_value`, `IScrollProvider::scroll` and
`set_scroll_percent`, `ISelectionItemProvider::add_to_selection`, `remove_from_selection` and
`select`, `IExpandCollapseProvider::expand` and `collapse`, and `IToggleProvider::toggle`
(`Result<(), AutomationError>`; they returned `Result<(), ElementNotEnabledError>`).
`IInvokeProvider::invoke` keeps `ElementNotEnabledError` (nothing refuses it otherwise), and
`IValueProvider::set_value` keeps `ProviderError`, which now carries an `AutomationError` for the
combo box, the calendar and the rows. The 14 throw sites return the error.

The bridges: the Win32 nodes (`AutomationNode::enabled`, `automation_node_value.rs`,
`text_pattern_marshalling.rs`) and the WinUI proxies map it to the result code the runtime of
the reference returns for the exception:

| Error | Result code |
|---|---|
| `ElementNotEnabled` | `UIA_E_ELEMENTNOTENABLED` (0x80040200), as before |
| `InvalidOperation` | `COR_E_INVALIDOPERATION` (0x80131509, the value of `UIA_E_INVALIDOPERATION`) |
| `ArgumentOutOfRange` | `COR_E_ARGUMENTOUTOFRANGE` (0x80131502) |
| `NotSupported` | `COR_E_NOTSUPPORTED` (0x80131515) |

The other bridges keep their outcome for a refusal: macOS hands the error to the dispatcher
(`raise_on_error`), which raises it from its run loop as the reference does, Android answers "not performed" (`From<AutomationError> for
NodeActionError`), AT-SPI fails the D-Bus call, iOS answers false and the browser ignores it.
The catch of `invoke_sync` stays, marked `// Boundary (#567)`.

Under unwinding the refused calls of a client answered `E_FAIL` (the panic); they now answer
the code of the table, the one the reference answers. The ink toolbar peers, which refused
selection operations with an `ElementNotEnabledError` for want of another error type, now
refuse them with `InvalidOperation` (`COR_E_INVALIDOPERATION`, as the native peers of WinUI).
Tests: `a_refused_call_answers_the_result_code_of_the_exception_of_the_reference` (Win32), and
the peer tests of the scroll viewer, menu item, combo box, calendar and TreeDataGrid, which
asserted a panic and now assert the error.

### 5.5 OpenGL loading, making a context current, Vello and the catalog transitions

- **Entry points** (`ferroui-opengl` `entry_points.rs`): the generated `load` of a table returns
  `Result<_, OpenGlError>`; a required entry point that does not resolve is
  `Err("Unable to find an entry point named '<name>'.")`, the message of the panic before. New
  `try_` forms: `GlBasicInfoInterface::try_new`, `GlContextInfo::try_create`,
  `GlInterface::try_new`, `GlInterface::try_from_native_utf8_get_proc_address`,
  `EglInterface::try_new` and `Win32AngleEglInterface::try_from_loader`. The plain forms keep
  their signature and panic with the error, as before. Every framework caller uses the `try_`
  form: the EGL, GLX and WGL contexts (which make their context current again on the failure),
  `EglInterface::from_library`, the Wayland, DRM, Android and ANGLE EGL loaders, the macOS native
  GL, EAGL and the WebGL context of the browser. `WglPlatformOpenGlInterface` no longer catches a
  panic: the error of `WglDisplay::create_context` is logged and the start tries the next mode.
  `BrowserWebGlRenderTarget::new` and `WebGlContext::new` return `Result`; a WebGL target whose
  context cannot be used is left out with the error logged. Calling an absent optional entry
  point is a bug and still panics (callers check `is_<name>_available`).
- **Making a context current** (`IGlContext`): new provided methods `try_make_current` and
  `try_ensure_current`, `Result<Rc<dyn IDisposable>, OpenGlError>`, whose defaults ask
  `make_current` and `ensure_current`. The EGL, GLX, WGL, macOS native GL, EAGL and WebGL
  contexts override them; the loss of the context is an error too.
  `WglContext::try_make_current` returns the loss as an error instead of panicking. The release of
  a Skia snapshot (`gl_skia_external_objects_feature.rs`) uses `try_ensure_current` and no longer
  catches a panic.
- **Vello**: `binary_op` refuses paths whose coordinates are not finite before the sweep (the
  combination is empty, as when the sweep gives up); the catch of the sweep stays for the inputs
  it still refuses. `drop_retired` leaves (`std::mem::forget`) the swap chain of a lost device
  instead of releasing it, which `wgpu` cannot do without its context; the catch stays for a
  device that is not known to be lost.
- **Catalog transitions**: `Dispatcher::start_local_on_result_with_priority` (new, public) is
  `start_local_with_priority` for a future that returns `Result<T, JoinError>`, and the page
  transitions of `ferroui-base` use it. `mini_mvvm::start_async_on_result` wraps it for the
  samples; the six transitions of the catalog (composite, fade through, page slide, parallax
  slide, card stack and the custom transition of the `TransitioningContentControl` page) return
  the error of `AnimationTask::when_all` through it. Their checks of the visual parent are
  argument checks (bugs) and stay panics.

## 6. Deferred

### 6.1 The preview of an animation (caught in the Designer)

`Animation::apply_for_preview` rejects a target with a CSS `TransformOperations` value with a
`panic!` (`animation/animation.rs`), and `Animation::apply` and the animators validate their
setters, key frames and targets with panics too. The designer turned these into error values
with the `catch_unwind` around `apply_for_preview` (`src/FerroUI.Designer/animation_runtime.rs`).

Converted on 2026-11-19 without changing a signature: `Animation::try_apply_for_preview` (new)
checks, before anything is applied and in the order `apply_for_preview` meets them, every
rejection of `ferroui-base` itself (the CSS target, a setter without a property, a property no
animator is registered for, a transform animation of a target that is not a `Visual`, and the
timing an instance refuses: a negative speed ratio, delay, duration or delay between
iterations, an iteration count above `i64::MAX`) and returns it as `Err(AnimationError)` with the
message the panic has. The messages are shared with the panics (`NO_SETTER_PROPERTY`,
`CSS_TRANSFORM_TARGET`, `no_animator_message`, `not_a_visual_message`, and the `check_*`
functions `AnimationInstance::fetch_properties` and `validate_timing` both call). What stayed a
panic until 0.6 is what an animator checks itself: an animator given explicitly for a property
of another value type (`AnimatorBase::styled_property`) and the checks of an application's
animator. In 0.6 `IAnimator::apply` is fallible and these are errors too (11.2). The designer calls
`try_apply_for_preview`; its catch stays as the boundary (B) of those and of the application's
property changed handlers.

### 6.2 Production paths that break with abort

Each is described with its site in sections 8 and 9; the reason for deferring is the size of the
conversion, or that the panic comes from a third-party crate.

- **Panics of third-party crates** (`ferroui-vello` `combined_geometry_impl.rs:133`,
  `linesweeper`; `window_surface.rs:166`, the `Drop` of a `wgpu::Surface`): no `Result` exists on
  our side. Mitigated (5.5): paths that are not finite are refused before the sweep and the
  surface of a lost device is left. What remains is a finite input the sweep still gives up on,
  and a surface that cannot be released while its device is not known to be lost.
- **A rendering mode the backend cannot render with** (`ferroui-skia`
  `platform_render_interface.rs:115`, `create_gpu`): forcing the OpenGL mode of the macOS native
  backend (`FERROUI_SMOKE_RENDERING=opengl` of the catalog) with a Skia build without Ganesh GL
  (the default macOS build, Graphite on Metal) panics on the render thread with "Graphics context
  of type is not supported", which aborts with `panic = "abort"`. The default modes are not
  affected. A fallible `create_backend_context`, falling back to software, or a platform that
  offers only the modes the backend renders, is its own step.
- **Run-time loading of application XAML outside a scope** (5.1): converted in 0.6, the load
  has its own scope and returns the error (11.2).

### 6.3 Development tools that need a build that unwinds

The Designer, `DesignerSupport` (the previewer), hot reload (`ferroui-hot-reload`, debug builds
only), hot patch, DevTools, MCP and Inspect isolate the code of the previewed or inspected
application with `catch_unwind`, and two of them still turn expected errors into values with
it (section 10). They are built with the `dev`, `fast-release` or `release-unwind` profile or
`dist-unwind`. Not
converted, as decided for this step:

- hot reload: `catch_reload` (`lib.rs:671`) as the boundary of adapters and application
  callbacks; the `ReloadError`s of the resource lookups of a candidate and the errors the XAML
  runtime reports in a step no longer unwind (section 10, #567);
- the Designer: the preview of an animation (6.1) and the candidate stages
  (`preview_stages.rs:1665`, `:1760`), as the boundary of the application's animators,
  constructors and layout callbacks; the errors of the edited document and the rejections of the
  framework no longer unwind (2026-11-19);
- the previewer: the load and show of the edited document (`remote_designer_entry_point.rs:380`),
  as the boundary of the previewed application's code; the errors of the document no longer
  unwind (2026-11-19).

## 7. `ferroui-system` and the markup crates

### 7.1 `ferroui-system` (`src/FerroUI.System`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `task/executor.rs:248` | `poll_task` | catches a panic of the future of a task; the task fails with `JoinError::Panicked` | B | no change: a panicking future aborts instead of failing its task; a failure reported as a value (`spawn_local_on_result`, 5.2) is not a panic |
| `task/executor.rs:390` | `handle_failure` | raises a panic that came after the outcome was written, to the caller of the poll | R | no change (belongs to :248) |
| `task/executor.rs:508` | `Start::run` (of `spawn_on`) | catches a panic of the callback that makes the future | B | no change: a panicking callback aborts instead of failing the task |
| `task/executor.rs:650` | `block_on` | raises the panic of the future to the caller (`JoinError::resume_unwind`) | R | no change (belongs to :248) |
| `task/join.rs:113` | `JoinError::resume_unwind` | raises the payload of a panicked task again (public API, as in `tokio`) | R | no change: with abort a payload exists only when a future reported `JoinError::Panicked` as a value; raising it then panics, which aborts |
| `threading/tasks/local_executor.rs:69` | `spawn_local` (deprecated `Task` form) | raises a panic that came after the task ended | R | no change (belongs to `task/executor.rs:248`) |
| `threading/tasks/local_executor.rs:143` | `run` (worker thread) | catches a panic of the work at the top of the thread; the task fails with `TaskWorkError` | B | no change: the work reports its errors as `Err`; a panicking work aborts instead of failing the task |

Tests (T), 6 sites: `tests/executor_tests.rs` 1, `tests/slot_order_map_tests.rs` 1,
`tests/ferro_thread_local_tests.rs` 1, `tests/task_tests.rs` 2,
`tests/observable_collection_tests.rs` 1.

### 7.2 The markup crates (`src/Markup`)

No site outside tests is left. Before this step `xaml_error_scope.rs` of `ferroui-markup-xaml`
had 4: the `catch_unwind` of `with_xaml_errors` (E), its two `resume_unwind` of other panics and
of a token without an error (R), and the `resume_unwind` of the throw token (E). They are gone
(section 5.1). `ferroui-markup-xaml-loader` and `ferroui-markup-xaml-winui` have no site outside
tests; the loader never turned a panic into a load error.

Tests (T), 9 sites: `ferroui-markup-xaml`: `xaml_error_scope.rs` 3, `source_overlay.rs` 1,
`populate_overrides.rs` 1, `reload_validation.rs` 1 (all `cfg(test)` modules),
`xaml_il/runtime/xaml_il_runtime_helpers_tests.rs` 1; `ferroui-markup-xaml-winui`:
`importer_tests.rs` 1 (the run-time loader outside a scope still panics); `ferroui-markup`:
`tests/test_support.rs` 1.

## 8. Platform crates

| Crate | B | R | E | T | All |
|---|---:|---:|---:|---:|---:|
| `ferroui-x11` | 9 | 2 | 0 | 1 | 12 |
| `ferroui-wayland` | 1 | 2 | 0 | 0 | 3 |
| `ferroui-win32` | 5 | 2 | 1 | 5 | 13 |
| `ferroui-android` | 5 | 2 | 0 | 3 | 10 |
| `ferroui-native` | 2 | 4 | 0 | 10 | 16 |
| `ferroui-browser` | 2 | 0 | 0 | 1 | 3 |
| `ferroui-skia` | 2 | 0 | 1 | 6 | 9 |
| `ferroui-vello` | 0 | 0 | 2 | 3 | 5 |
| `ferroui-opengl` | 0 | 1 | 0 | 0 | 1 |
| `ferroui-media-video` | 1 | 0 | 0 | 0 | 1 |
| `ferroui-vulkan` | 0 | 0 | 0 | 7 | 7 |
| **All** | **27** | **13** | **4** | **36** | **80** |

Sites are counted per line, `use` lines left out. (T) includes the sites of `*_tests.rs` files,
which the site list leaves out. No sites: `src/Windows/FerroUI.WinUI` (its handlers run under the
guard of `ferroui-win32`), `src/Linux/FerroUI.LinuxFramebuffer`, `src/iOS/FerroUI.iOS`,
`src/FerroUI.Metal`, `src/FerroUI.FreeDesktop`, `src/FerroUI.Desktop`, and every
`src/FerroUI.Media*` crate other than `FerroUI.Media.Video`.

### FerroUI.X11 (`src/FerroUI.X11`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `x11_native_control_host.rs:93` | `X11NativeControlHost::create_new_attachment_with` | catches a panic of the application callback that creates the child | B | no change: a panicking callback aborts before the holder window is destroyed |
| `x11_native_control_host.rs:113` | `X11NativeControlHost::create_new_attachment_with` | raises it again after the holder and the attachment are disposed of | R | no change (belongs to :93) |
| `dispatching/g_lib_dispatcher_impl.rs:88` | `GlibDispatcherImpl::x11_source_callback` | catches a panic of the X11 event handlers and the queued jobs, in a GLib source callback | B | no change: a panicking handler aborts instead of ending the frame and being raised from `run_loop` |
| `dispatching/glib_dispatcher_impl_base.rs:277` | `GlibDispatcherImplBase::check_signaled` | catches a panic of the dispatcher jobs | B | no change: as :88 |
| `dispatching/glib_dispatcher_impl_base.rs:289` | `GlibDispatcherImplBase::timer_callback` | catches a panic of the dispatcher timers | B | no change: as :88 |
| `dispatching/glib_dispatcher_impl_base.rs:394` | `GlibDispatcherImplBase::request_background_processing` (idle callback) | catches a panic of the background jobs | B | no change: as :88 |
| `dispatching/glib_dispatcher_impl_base.rs:435` | `GlibDispatcherImplBase::run_loop` | raises the first panic kept by the frame | R | no change (belongs to :277, :289, :394 and `g_lib_dispatcher_impl.rs:88`) |
| `x11_platform_lifetime_events.rs:137` | `guarded` | catches a panic of a libSM handler (`extern "C"`) | B | no change: a panicking handler aborts instead of being logged |
| `interop/glib.rs:119` | `guarded` | catches a panic of a GLib callback (`extern "C"`) | B | no change: a panicking callback aborts instead of being logged |
| `interop/glib.rs:511` | `Glib::run_on_glib_thread` | catches a panic of the callback (application code through `GtkInteropHelper::run_on_glib_thread`) and fails the task with its message | B | no change: a panicking callback aborts instead of failing the task |
| `native_dialogs/gtk_native_file_dialogs.rs:283` | `run_dialog` | catches a panic of `show_dialog` and fails the dialog task | B | no change: the dialog reports its errors as values; a bug aborts instead of failing the task |

Tests (T): `dispatching/glib_dispatcher_impl_base.rs:550` (`cfg(test)` module), 1 site.

### FerroUI.Wayland (`src/FerroUI.Wayland`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `ferro_wayland_platform_extensions.rs:74` | `use_wayland_with_fallback` (windowing initializer) | catches a panic of the fallback backend (for example no X server either) to print why Wayland was refused | R | no change in outcome (the process ends either way); the extra line is not printed, the earlier warning log and the panic hook still say both reasons |
| `ferro_wayland_platform_extensions.rs:79` | `use_wayland_with_fallback` | raises it again | R | no change (belongs to :74) |
| `server/wayland_worker_render_timer.rs:256` | `WaylandRenderTimer::tick_render_loop_if_needed` | catches a panic of a render loop tick, logs it and asks for another tick | B | no change: a panicking render task aborts instead of being logged as "Exception in render loop" |

### FerroUI.Win32 (`src/Windows/FerroUI.Win32`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `wnd_proc_guard.rs:38` | `guard` | catches a panic of a window procedure body (and of the WinUI handlers that use the guard), keeps it, returns the default result | B | no change: a panicking handler aborts inside the window procedure instead of being raised in the message loop |
| `wnd_proc_guard.rs:77` | `resume_pending` | raises the kept panic once the system has returned to the loop | R | no change (belongs to :38) |
| `win32_storage_provider.rs:232` | `Win32StorageProvider::show_file_picker_async` (dialog thread) | catches a panic of `show_dialog` at the top of the dialog thread and fails the picker | B | no change: the dialog reports its errors as values; a bug aborts instead of failing the picker |
| `automation/text_range_node.rs:196` | `TextRangeNode::invoke` | catches a panic of a text range provider called by a UI Automation client, returns `E_FAIL` | B | no change: the framework providers report errors as values; a panicking custom provider aborts |
| `automation/automation_node.rs:578` | `AutomationNode::invoke_sync` | catches a panic of the peer code a UI Automation client calls, returns `E_FAIL` | B | no change since 5.4: a call the state of the control refuses is an `AutomationError` of the provider, mapped to its result code; a bug or a panicking application handler (a button's click under `invoke`) aborts |
| `win32_native_control_host.rs:102` | `Win32NativeControlHost::create_new_attachment_with` | catches a panic of the application callback that creates the child | B | no change: a panicking callback aborts before the holder window is destroyed |
| `win32_native_control_host.rs:117` | `Win32NativeControlHost::create_new_attachment_with` | raises it again after the holder and the attachment are disposed of | R | no change (belongs to :102) |
| `open_gl/wgl_platform_open_gl_interface.rs:90` | `WglPlatformOpenGlInterface::try_create_with_profiles` | caught the panic of a missing required OpenGL entry point and reported WGL as not available | E | converted (5.5): the creation of the context fails with the error, which is logged, and the start falls back to the next rendering mode; no catch |

Tests (T), 5 sites: `examples/win32_window.rs:2228`, `:2229` (a hook of the smoke run that ends
the process with code 101 after the default hook; it behaves the same with abort),
`input/imm32_context_tests.rs` 1, `automation/marshalling/safe_array_ref_tests.rs` 2.

**`automation/automation_node.rs:578`** (resolved, section 5.4). The automation peers of
`ferroui-controls` rejected client calls the state of the control does not allow with `panic!`,
the exceptions of the reference, and `invoke_sync` turned the panic into `E_FAIL` for the client;
with abort a screen reader query ended the application. The providers now return an
`AutomationError`, which the nodes map to the result code of the exception of the reference.
The catch stays as the boundary: `invoke` runs application handlers (a button's click). On
macOS the refusal is handed to the dispatcher as an error value (`raise_on_error` of
`frn_automation_peer.rs`), no longer as a panic through the `guard` of `ferroui-native`; the
dispatcher raises it from its run loop, as the reference rethrows an exception of a callback.

**`open_gl/wgl_platform_open_gl_interface.rs:90`.** `gl_entry_points!` of `ferroui-opengl`
(`entry_points.rs`, the `@check required` arm) calls `entry_point_not_found`, a `panic!`, when a
required entry point does not resolve while the table of a context is loaded. The WGL start
catches it and reports WGL as not available, so the platform tries the next rendering mode,
as the reference catches every exception of the creation. A `Result` conversion: the generated
load returns `Result<_, OpenGlError>` and `GlInterface::new` becomes fallible; it has 14
non-test call sites in 10 files (WGL, GLX, EGL, the macOS native GL, EAGL, WebGL, Skia). The
panics of the `@call` arms (calling an optional entry point that is absent) are bugs and stay.
Production path: the Windows start in an OpenGL mode. Done as described in 5.5.

### FerroUI.Android (`src/Android/FerroUI.Android`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `interop/natives.rs:83` | `guard` | catches a panic of a JNI native method body, logs it and throws a Java `RuntimeException` | B | no change: a panicking native method aborts instead of throwing into Java (which does not catch it either) |
| `interop/natives.rs:110` | `on_load` | takes the default hook to wrap it | B | no change: part of the crash reporter (:111) |
| `interop/natives.rs:111` | `on_load` | sets a hook that writes every panic to the log of the system | B | no change: the hook runs before the abort |
| `interop/natives.rs:118` | `on_load` | catches a panic of the body of `JNI_OnLoad` and fails the load | B | no change: a failed setup aborts instead of returning `JNI_ERR` |
| `interop/ndk.rs:281` | `run_callback` | catches a panic of a choreographer frame callback (`extern "C"`), logs it and aborts | B | no change: it aborts already |
| `platform/clipboard_impl.rs:28` | `completed` | catches a panic of a clipboard call (a Java exception) to raise it when the future is awaited | R | no change: the clipboard errors are values; a Java exception aborts at the call instead of faulting the awaiting task |
| `platform/clipboard_impl.rs:30` | `completed` (its future) | raises it again | R | no change (belongs to :28) |

Tests (T): `interop/mod.rs:36`, `:38`, `:40` (`cfg(test)` module), 3 sites.

### FerroUI.Native (`src/FerroUI.Native`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `callback_base.rs:25` | `guard` | catches a panic of a callback native code calls and hands it to the dispatcher implementation | B | no change: a panicking callback aborts instead of ending the run loop and being raised from it |
| `dispatcher_impl.rs:267` | `DispatcherImpl::run_loop` | raises the panic a callback handed over | R | no change (belongs to `callback_base.rs:25`) |
| `native_control_host_impl.rs:117` | `NativeControlHostImpl::create_new_attachment_with` | catches a panic of the application callback that creates the child | B | no change: a panicking callback aborts before the attachment is disposed of |
| `native_control_host_impl.rs:126` | `NativeControlHostImpl::create_new_attachment_with` | raises it again | R | no change (belongs to :117) |
| `clipboard_impl.rs:30` | `completed` | catches a panic of a clipboard call (a disposed clipboard) to raise it when the future is awaited | R | no change: the clipboard errors are values; a call on a disposed clipboard aborts at the call |
| `clipboard_impl.rs:32` | `completed` (its future) | raises it again | R | no change (belongs to :30) |

Tests (T), 10 sites: `callback_base.rs:55`, `:56`, `:58`, `frn_automation_peer.rs:743`, `:744`,
`:746`, `:936`, `:937`, `:939` (hooks that silence the output of expected panics in `cfg(test)`
modules), `ferro_native_gl_platform_graphics.rs:1084` (`cfg(test)` module).

### FerroUI.Browser (`src/Browser/FerroUI.Browser`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `rendering/render_worker.rs:245` | `RenderWorker::report_panics` | takes the hook to wrap it | B | no change: part of the crash reporter (:246) |
| `rendering/render_worker.rs:246` | `RenderWorker::report_panics` | sets a hook that counts the panics of the render thread and logs them on the console of the page | B | no change: the hook runs before the abort |

Tests (T): `examples/themed_view/main.rs:486` (a behaviour test page that checks the dispatcher
catches panics), 1 site. It needs a build that unwinds.

### FerroUI.Skia (`src/Skia/FerroUI.Skia`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `gpu/open_gl/gl_skia_external_objects_feature.rs:479` | `GlSkiaImportedImage::take_snapshot` (dispose callback of the snapshot bitmap) | caught the panic of `ensure_current` on a context that cannot be made current and dropped the image without it | E | converted (5.5): `try_ensure_current` fails and the image is dropped without a current context; no catch |
| `gpu/graphite/pipelined_layer.rs:885` | `BandWorker::run` (a band thread of the pipelined Graphite layer) | catches a panic while a band is played into its recording, and reports the band as not recorded | B | no change: a panic of the thread would leave the render thread waiting for an answer; with abort it ends the process (added after the first survey, 2026-11-19) |
| `gpu/graphite/pipelined_layer.rs:898` | same | catches a panic while a recording is inserted into the shared context, and reports it as not inserted | B | no change: as `:885` |

Tests (T), 6 sites: `tests.rs` 1, `gpu/metal/tests.rs` 3, `unit_tests/media/font_manager_tests.rs`
1, `unit_tests/media/text_formatting/text_collapsing_bidi_tests.rs` 1.

**`gpu/open_gl/gl_skia_external_objects_feature.rs:479`.** `IGlContext::make_current`, which
`ensure_current` calls, panics with the `OpenGlError` of the platform when the context cannot be
made current (`egl/egl_context.rs:511` of `ferroui-opengl`, `glx/glx_context.rs:230` of
`ferroui-x11`, `open_gl/wgl_context.rs:202` of `ferroui-win32`). The dispose callback catches it
and releases the image without a current context, as the reference ignores the exception of a
context that is likely dead. A `Result` conversion: a fallible `try_ensure_current` (or
`try_make_current`) on the GL contexts returning `Result<Rc<dyn IDisposable>, OpenGlError>`;
GLX and WGL already have `make_current_with(..) -> Result`, so the EGL, the macOS native GL and
EAGL contexts need one. This is the only catch of that panic in the listed crates. Production
path: the Skia OpenGL backend after a context loss. Done as described in 5.5.

### FerroUI.Vello (`src/Vello/FerroUI.Vello`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `combined_geometry_impl.rs:133` | `binary_op` | catches a panic of `linesweeper::binary_op` (the sweep gives up on an input) and makes the empty geometry | E | mitigated (5.5): paths that are not finite are refused first; a finite input the sweep gives up on still aborts (see below) |
| `gpu/window_surface.rs:166` | `VelloSurfaceGpu::drop_retired` | catches a panic of dropping a `wgpu` surface whose context cannot be made current, logs it | E | mitigated (5.5): the surface of a lost device is left without releasing it; a surface that cannot be released otherwise still aborts (see below) |

Tests (T), 3 sites: `effect_tests.rs` 1, `unit_tests/media/font_manager_tests.rs` 1,
`unit_tests/media/text_formatting/text_collapsing_bidi_tests.rs` 1.

**`combined_geometry_impl.rs:133`.** The error is a panic of a third-party crate:
`linesweeper` describes itself as early and may panic on an input. The catch turns it into the
empty geometry of the contract. No `Result` is available from the library; keeping the
behaviour with abort takes an upstream fix, a check of the inputs that the sweep refuses, or
another implementation of the boolean operations. One caller, `combine_paths`, with
up to three calls. Production path: every `CombinedGeometry` drawn with Vello. Mitigated as described in 5.5.

**`gpu/window_surface.rs:166`.** The error is a panic inside the `Drop` of a `wgpu::Surface`
(third party) when its context cannot be made current. The catch logs it and goes on; it also
runs from `<VelloSurfaceGpu as Drop>::drop`. No `Result` is available from a `Drop`; keeping the
behaviour takes leaking the surface (`std::mem::forget`) when the context is known to be lost,
or an upstream fix. Callers: `drop_retired` from the frame start (`window_surface.rs:475`) and
from `Drop`. Production path: closing a window of the Vello backend on Linux or Windows after a
device or context loss. Mitigated as described in 5.5.

### FerroUI.OpenGL (`src/FerroUI.OpenGL`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `composition/task_support.rs:32` | `join` | raises the panic of a failed task (`JoinError::Panicked`) in the task that awaits it | R | no change: the only joined task (`CompositionGlTexture::dispose_async`) fails only by a bug, and does not report a `JoinError` value (`spawn_local_on_result`); with abort its `JoinError::Panicked` is never made (belongs to the task boundary of `ferroui-base`, `TaskBody::poll`) |

### FerroUI.Media.Video (`src/FerroUI.Media.Video`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `native_source.rs:130` | `worker` (worker thread) | catches a panic of the preparation work at the top of the thread and fails it with `NativeFailure` | B | no change: the work reports its errors as values; a bug aborts instead of failing the preparation |

### FerroUI.Vulkan (`src/FerroUI.Vulkan`)

Tests (T): `live_tests.rs` 7 sites. No other sites.

## 9. Libraries

| Crate | B | R | E | T | All |
|---|---:|---:|---:|---:|---:|
| `ferroui-headless` | 8 | 8 | 1 | 12 | 29 |
| `ferroui-headless-vnc` | 3 | 0 | 0 | 0 | 3 |
| `ferrodock-ferro` | 2 | 1 | 0 | 15 | 18 |
| `ferrodock-ferro-diagnostics` | 0 | 0 | 0 | 1 | 1 |
| `ferrodock-model` | 3 | 3 | 0 | 14 | 20 |
| `ferrodock-model-ferro` | 0 | 0 | 0 | 1 | 1 |
| `ferrodock-model-reactiveui` | 0 | 0 | 0 | 66 | 66 |
| `ferrodock-model-reactiveui-services` | 6 | 0 | 0 | 12 | 18 |
| `ferrodock-serializer-system-text-json` | 1 | 1 | 0 | 0 | 2 |
| `ferrodock-settings` | 0 | 0 | 0 | 3 | 3 |
| `ferrodock-controls-proportional-stack-panel` | 0 | 0 | 0 | 2 | 2 |
| `ferroui-xaml-interactions` | 3 | 2 | 0 | 1 | 6 |
| `ferroui-xaml-interactions-draganddrop` | 2 | 0 | 0 | 0 | 2 |
| `ferroui-xaml-interactivity` | 0 | 0 | 0 | 3 | 3 |
| `ferroui-xaml-interactions-events` | 0 | 0 | 0 | 1 | 1 |
| `ferroui-xaml-interactions-responsive` | 0 | 0 | 0 | 1 | 1 |
| `ferroui-xaml-interactions-reactiveui` | 0 | 0 | 0 | 1 | 1 |
| `ferroui-xaml-behaviors-animations` | 0 | 0 | 0 | 1 | 1 |
| `ferroui-controls` | 1 | 2 | 0 | 46 | 49 |
| `ferroui-controls-repeater` | 0 | 0 | 0 | 25 | 25 |
| `ferroui-dynamic-data` | 1 | 0 | 0 | 2 | 3 |
| `ferroui-reactive` | 0 | 0 | 0 | 1 | 1 |
| `ferroui-reactive-testing` | 0 | 0 | 0 | 1 | 1 |
| `ferroui-reactiveui` | 1 | 2 | 2 | 0 | 5 |
| `ferroui-reactiveui-controls` | 2 | 1 | 0 | 2 | 5 |
| `ferroui-splat` | 4 | 1 | 2 | 1 | 8 |
| `ferroui-controls-treedatagrid` | 0 | 0 | 0 | 1 | 1 |
| `ferroui-treedatagrid-core` | 0 | 0 | 0 | 1 | 1 |
| **All** | **37** | **21** | **5** | **214** | **277** |

Sites found with `grep -rn "catch_unwind\|resume_unwind\|panic_any\|set_hook(\|take_hook("` over
the crates of this group, test files included, `use` lines (also the entries of multi-line `use`
lists) and comment lines left out. No sites in the other crates of the group:
`ferroui-controls-charts`, `-colorpicker`, `-inking`, `-livecharts`, the other `src/Dock` and
`src/Behaviors` crates, `ferroui-reactiveui-testing`, `src/ProTranslate`, `src/StaticViewLocator`,
`ferroui-dialogs`, `ferroui-diagnostics`, `ferroui-dependency-injection`. Line numbers are those
after the boundary comments were added.

A general note for the whole group: Cargo ignores the `panic` setting of a profile for tests and
benchmarks, which always unwind. Sites reached only from tests keep their behavior whatever the
shipped profile says.

### ferroui-headless (`src/Headless/FerroUI.Headless`)

The session (`headless_unit_test_session.rs`) is compiled into the library, not behind
`cfg(test)`, but its only users are test crates (`tests/Dock/*`,
`tests/FerroUI.RenderBackends.Comparison`). It runs application and test code on a session
thread and reports a panic of that code as the failure of the test, as the reference sets the
exception of the task. Classified as boundaries; in practice always built with unwinding.

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `headless_unit_test_session.rs:145` | `HeadlessUnitTestTask::take` | raises the panic of the dispatched test in the waiting thread | R | no change (belongs to :332/:354/:389) |
| `headless_unit_test_session.rs:332` | `HeadlessUnitTestSession::dispatch_core` (work item) | catches a panic of the setup of the application | B | no change in a test build; with abort a failing setup would end the test process instead of failing one test |
| `headless_unit_test_session.rs:354` | `HeadlessUnitTestSession::dispatch_core` (work item) | catches a panic of the test action and its future | B | as :332: a failed assertion would abort instead of failing the test |
| `headless_unit_test_session.rs:389` | `HeadlessUnitTestSession::dispatch_core` (work item) | catches a panic of the cleanup of the application | B | as :332 |
| `headless_unit_test_session.rs:418` | `HeadlessUnitTestSession::dispose` | raises a panic that ended the session thread (`join`) | R | no change (the top of the session thread) |
| `headless_unit_test_session.rs:471` | `HeadlessUnitTestSession::start_new_with_isolation` (thread) | catches a panic of the entry point (`fn() -> AppBuilder`) | B | as :332 |
| `headless_unit_test_session.rs:508` | `HeadlessUnitTestSession::start_new_with_isolation` | raises it in the starting thread | R | no change (belongs to :471) |
| `headless_unit_test_session.rs:606` | `SessionThread::ensure_shared_application` (cleanup) | catches a panic of the jobs left by the test | B | no change: try/finally; the priority restore is skipped on abort |
| `headless_unit_test_session.rs:612` | same | raises it again | R | no change (belongs to :606) |
| `headless_unit_test_session.rs:622` | `SessionThread::ensure_isolated_application` | catches a panic of the setup | B | no change: try/catch that disposes of the locator scope |
| `headless_unit_test_session.rs:627` | same | raises it again | R | no change (belongs to :622) |
| `headless_unit_test_session.rs:634` | `SessionThread::ensure_isolated_application` (cleanup) | catches a panic of the disposal of the services | B | no change: nested try/finally |
| `headless_unit_test_session.rs:649` | same | catches a panic of the disposal of the scope | B | no change: nested try/finally |
| `headless_unit_test_session.rs:659` | same | raises the panic of the scope | R | no change (belongs to :649) |
| `headless_unit_test_session.rs:662` | same | raises the panic of the services | R | no change (belongs to :634) |
| `headless_unit_test_session.rs:695` | `result_of` | raises the `JoinError::Panicked` of the future of the test | R | no change (belongs to :354) |
| `headless_unit_test_session.rs:696` | `result_of` | raises `Canceled` as a panic when the task of the test was canceled | E | no change in a test build; see below |

Tests (T), all in the `cfg(test)` module `unit_tests` (`lib.rs:44`):

| File | Sites | What |
|---|---:|---|
| `unit_tests/test_application.rs` | 4 | a panic hook that records the panics of the session threads for the failed test (`take_hook`/`set_hook`), and the wait of a test task |
| `unit_tests/headless_unit_test_application.rs` | 2 | the test application of the tests |
| `unit_tests/async_setup_tests.rs` | 2 | tests |
| `unit_tests/threading_tests.rs` | 2 | tests |
| `unit_tests/headless_probe_tests.rs` | 1 | tests |
| `unit_tests/headless_unit_test_session_tests.rs` | 1 | tests |

**(E) `headless_unit_test_session.rs:696`.** A canceled task of a test (`JoinError::Canceled`,
the dispatcher dropped it) is raised as `panic_any(Canceled)`; the boundary at :354 catches it,
the work item stores it with `tcs.set_exception`, and `HeadlessUnitTestTask::take` raises it in
the waiting thread. The reference does the same (`await` of a canceled task throws, the catch
sets the exception). A `Result` form: `result_of` returns `Result<T, Canceled>` and its two
callers in `dispatch_core` set `should_cancel` (then `tcs.set_canceled()`, which `wait` already
returns as `Err(Canceled)`); no public signature changes. Test harness only.

### ferroui-headless-vnc (`src/Headless/FerroUI.Headless.Vnc`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `remote_viewing/utility/periodic_thread.rs:112` | `PeriodicThread::start` (thread) | catches a panic of the update action; the thread ends | B | no change: a panicking update aborts the process instead of ending the update thread |
| `remote_viewing/vnc/server/vnc_server_session.rs:656` | `VncServerSession::framebuffer_send_changes` | catches a panic of the framebuffer source (`capture`, on the UI thread for the headless source) and logs it | B | no change: a panicking source aborts instead of being logged |
| `remote_viewing/vnc/server/vnc_server_session.rs:1091` | `VncServerSession::thread_main` | catches a panic of the session thread (protocol, event handlers of the host); logs and closes | B | no change: protocol and I/O errors are `VncError` values; a panic aborts instead of closing the session |

### ferrodock-ferro (`src/Dock/FerroDock.Ferro`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `internal/dock_helpers.rs:119` | `DockHelpers::get_control` | catches a panic of the hit test (custom hit tests of controls) and prints it | B | no change: a panicking hit test aborts instead of the drop search finding nothing |
| `controls/host_window.rs:557` | `HostWindow::try_begin_move_drag` | catches a panic of the platform move drag (may run a modal loop) | B | no change: catch-and-rethrow; the drag state cleanup is skipped on abort |
| `controls/host_window.rs:564` | same | raises it again | R | no change (belongs to :557) |

Tests (T): 15 sites in test files: `controls/container_control_tests.rs` 3,
`controls/pinned_dock_control_tests.rs` 2, `controls/overlays/tests.rs` 2, and 1 each in
`internal/dock_data_template_helper_tests.rs`, `internal/item_drag_helper_tests.rs`,
`internal/drag_preview_helper_tests.rs`, `mdi/mdi_tests.rs`,
`controls/delayed_uniform_tab_panel_tests.rs`, `controls/managed_dockable_base_tests.rs`,
`model_items_tests.rs`, `diagnostics/dock_diagnostic_events_tests.rs`.

### ferrodock-ferro-diagnostics (`src/Dock/FerroDock.Ferro.Diagnostics`)

Tests (T): 1 site in `selection_overlay_tests.rs`.

### ferrodock-model (`src/Dock/FerroDock.Model`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `factory_query.rs:107` | `DockableQueryEnumerator::move_next` | catches a panic of the query iterator (application predicates) | B | no change: try/finally; the state is not finished on abort |
| `factory_query.rs:115` | same | raises it again | R | no change (belongs to :107) |
| `dock_manager.rs:539` | `DockManager::validate_dockable` | catches a panic of the dock operation (factory and application code) | B | no change: try/finally; the observer scope is not disposed of on abort |
| `dock_manager.rs:553` | same | raises it again | R | no change (belongs to :539) |
| `adapters/host_adapter.rs:121` | `HostAdapter::exit` | catches a panic of saving and closing the host (native host, closing handlers) | B | no change: catch/finally; the tracked flag is not restored on abort |
| `adapters/host_adapter.rs:141` | same | raises it again | R | no change (belongs to :121) |

Tests (T): 14 sites in test files: `collections/list_tests.rs` 9, `factory_base_tests.rs` 2,
`collections/read_only_tests.rs` 2, `core/model_contract_tests.rs` 1.

### ferrodock-model-ferro (`src/Dock/FerroDock.Model.Ferro`)

Tests (T): 1 site in `factory_operations_tests.rs`.

### ferrodock-model-reactiveui (`src/Dock/FerroDock.Model.ReactiveUI`)

Tests (T): 66 sites, all in `*_tests.rs` files: `window_tests.rs` 8, `dock_workspace_tests.rs` 7,
`fluent_tests.rs` 7, `factory_metadata_tests.rs` 4, `factory_state_tests.rs` 4,
`factory_window_tests.rs` 4, `factory_mutation_tests.rs` 4, `query_cursor_tests.rs` 4,
`factory_event_tests.rs` 3, `nullable_factory_tests.rs` 2, `nullable_windows_tests.rs` 2, and 1
each in `nullable_state_tests.rs`, `document_dock_tests.rs`, `nullable_mutation_tests.rs`,
`nullable_target_tests.rs`, `nullable_preview_tests.rs`, `dock_state_tests.rs`,
`nullable_window_options_tests.rs`, `nullable_focus_tests.rs`, `nullable_floating_tests.rs`,
`factory_close_tests.rs`, `dock_manager_tests.rs`, `nullable_search_tests.rs`,
`nullable_move_tests.rs`, `nullable_hidden_tests.rs`, `factory_init_tests.rs`,
`factory_split_tests.rs`, `factory_float_tests.rs`.

### ferrodock-model-reactiveui-services (`src/Dock/FerroDock.Model.ReactiveUI.Services`)

These catches reproduce the C# rule that an exception of an `async` method, synchronous or not,
faults the returned task: a panic of the application code becomes an `ObservableError`
(`TaskWorkError`) in the task. Errors of the services themselves are `Result` values.

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `overlays/services/dock_busy_service.rs:124` | `DockBusyService::run_async` | catches a panic of `begin` (global busy service, change observers); faults the task | B | no change: a panicking observer aborts instead of faulting the task |
| `overlays/services/dock_busy_service.rs:146` | `DockBusyService::finish_run` | catches a panic of the scope cleanup; faults the task in place of the work outcome | B | as :124 |
| `threading/task_action.rs:31` | `invoke_task` | catches a panic of a busy or reload handler; faults the task | B | no change: a panicking handler aborts instead of faulting the task |
| `threading/task_action.rs:51` | `spawn_catching` (`Catch::poll`) | catches a panic of the future of a dialog, confirmation or reload; faults the task | B | as :31 |
| `threading/service_dispatcher.rs:27` | `ServiceDispatcher::invoke_async` | catches a panic of the dispatched action; faults the invocation | B | as :31 |
| `threading/main_thread_dispatcher.rs:27` | `MainThreadDispatcher::invoke_async` | catches a panic of the dispatched action; faults the task | B | as :31 |

Tests (T): 12 sites: `threading/action_disposable.rs:73` (`cfg(test)` module, line 29) and the
test files `navigation/navigation_tests.rs` 3, `overlays/hosting/hosting_tests.rs` 2,
`overlays/dependency_injection/dependency_resolver_extensions_tests.rs` 2, and 1 each in
`overlays/controls/busy_root_dock_tests.rs`, `overlays/services/overlay_stack_tests.rs`,
`lifecycle/dock_activation_bridge_tests.rs`, `threading/main_thread_dispatcher_tests.rs`.

### ferrodock-serializer-system-text-json (`src/Dock/FerroDock.Serializer.SystemTextJson`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `dock_serializer.rs:113` | `using_stream` | catches a panic of the serialization around the stream of the application | B | no change: the `using` of the reference; errors are `SerializationResult` values, and the stream is not closed on abort |
| `dock_serializer.rs:117` | same | raises it again after closing the stream | R | no change (belongs to :113) |

### ferrodock-settings (`src/Dock/FerroDock.Settings`)

Tests (T): 3 sites in `settings_tests.rs`.

### ferrodock-controls-proportional-stack-panel (`src/Dock/FerroDock.Controls.ProportionalStackPanel`)

Tests (T): 2 sites in `tests.rs`.

### ferroui-xaml-interactions (`src/Behaviors/FerroUI.Xaml.Interactions`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `core/reversible_action_execution.rs:41` | `ReversibleActionExecution::execute` | catches a panic of an action; the applied reversible actions are reverted | B | no change: a panicking action aborts; the revert does not run |
| `core/reversible_action_execution.rs:58` | same | catches and drops a panic of a revert, so that the panic of the action is the one raised | B | no change |
| `core/reversible_action_execution.rs:66` | same | raises the panic of the action again | R | no change (belongs to :41) |
| `core/async_action_group.rs:136` | `await_action` | raises the `JoinError::Panicked` of an action task in the future of the group (`await` of a faulted task) | R | no change: the payload is a panic of application action code, caught by the executor of the task |
| `action_task.rs:351` | `CatchUnwind::poll` | catches a panic of an awaited operation in clean-up continuations (`try { await } catch { }`; used by `TaskCompletedTrigger` and `PickerActionBase::track_picker_operation`) | B | no change: awaiting an `ActionTask` gives its failure as a value; only a bug reaches the catch |

Tests (T): 1 site in `tests/support.rs`.

### ferroui-xaml-interactions-draganddrop (`src/Behaviors/FerroUI.Xaml.Interactions.DragAndDrop`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `managed_drag_drop/drag_preview_service.rs:85` | `DragPreviewService::hide` | catches and ignores a panic of closing the preview window (closing handlers) | B | no change: a panicking closing handler aborts instead of being ignored |
| `managed_drag_drop/managed_context_drop_behavior.rs:232` | `ManagedContextDropBehavior::on_drag_started` | catches a panic of the drop handler's `validate`; treated as not valid | B | no change: a panicking handler aborts instead of rejecting the drop |

### ferroui-xaml-interactivity (`src/Behaviors/FerroUI.Xaml.Interactivity`)

Tests (T): 3 sites: `tests/condition_tests.rs` 2, `tests/support.rs` 1.

### Other behaviors crates

Tests (T): 1 site each in `tests/support.rs` of `ferroui-xaml-interactions-events`
(`src/Behaviors/FerroUI.Xaml.Interactions.Events`), `ferroui-xaml-interactions-responsive`
(`src/Behaviors/FerroUI.Xaml.Interactions.Responsive`) and `ferroui-xaml-behaviors-animations`
(`src/Behaviors/FerroUI.Xaml.Behaviors.Animations`), and in
`tests/interaction_trigger_behavior_tests.rs` of `ferroui-xaml-interactions-reactiveui`
(`src/Behaviors/FerroUI.Xaml.Interactions.ReactiveUI`).

### ferroui-controls (`src/FerroUI.Controls`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `utils/clipboard_helper.rs:103` | `CatchUnwind::poll` | catches a panic of a clipboard operation (pasting handlers, platform clipboard), for `ClipboardHelper::start` (raised on the dispatcher) and `PrimarySelectionHelper::publish_text_async` (logged) | B | no change: clipboard failures are `ClipboardError` values; a panic aborts instead of being raised from a dispatcher job or logged |
| `utils/clipboard_helper.rs:50` | `ClipboardHelper::start` | raises the panic of the operation from a dispatcher job (the exception of an async void method) | R | no change (belongs to :103) |
| `text_box.rs:2461` | `TextBox::paste` | raises the `JoinError::Panicked` of the paste task into the clipboard operation | R | no change (belongs to :103) |

Not a site of the grep, for the record: `ClipboardHelper::throw` (`utils/clipboard_helper.rs:60`)
raises an unexpected `ClipboardError` with `panic!` from a dispatcher job, as an unhandled
exception of the reference. With abort that ends the process without the unhandled-exception
filter of the dispatcher; it is a crash in the reference too unless the application handles it
there.

Tests (T): 52 sites: `utils/clipboard_helper.rs:147`, `:162`, `:173` (`cfg(test)` module, line
117) and 49 in test files: `date_time_pickers/date_picker_tests.rs` 5,
`selectable_text_block_tests.rs` 5, `date_time_pickers/time_picker_tests.rs` 4, 2 each in
`page/navigation_page_tests.rs`, `page/page_navigation_host_tests.rs`, `text_box_tests.rs`,
`window_tests.rs`, `styled_element_tests.rs`, `calendar_date_picker/calendar_date_picker_tests.rs`,
`automation/peers/items_automation_peer_tests.rs`, `automation/peers/menu_item_automation_peer_tests.rs`,
`context_menu_tests.rs`, `grid_tests.rs`, and 1 each in `page/content_page_tests.rs`,
`calendar/calendar_tests.rs`, `auto_complete_box/auto_complete_box_tests.rs`, `border_tests.rs`,
`menu_item_tests.rs`, `primitives/selecting_items_control_tests.rs`, `list_box_tests.rs`,
`loaded_tests.rs`, `layoutable_tests.rs`, `selection/selection_model_tests_multiple.rs`,
`native_menu_tests.rs`, `automation/peers/combo_box_automation_peer_tests.rs`,
`automation/peers/complex_control_automation_peer_tests.rs`, `visual_tests.rs`,
`tray_icon_tests.rs`.

### ferroui-controls-repeater (`src/FerroUI.Controls.Repeater`)

Tests (T): 25 sites in test files: `conformance_tests.rs` 10, `items_repeater_tests.rs` 9,
`layouts/layout_tests.rs` 2, `source_view_tests.rs` 2, `acceptance_tests.rs` 1,
`layout_contract_tests.rs` 1.

### ferroui-dynamic-data (`src/FerroUI.DynamicData`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `internal/keyed_disposable.rs:103` | `KeyedDisposable::dispose` | catches a panic of each disposable; all are disposed, then one aggregate panic | B | no change: the first panicking disposable aborts; the others are not disposed |

Tests (T): 2 sites: `tests/internal/mod.rs` 1, `tests/source_cache_tests.rs` 1.

### ferroui-reactive (`src/FerroUI.Reactive`)

Tests (T): 1 site in `tests/fast_immediate_observer_test.rs`. No site outside tests: the
operators of the crate do not catch panics of observers.

### ferroui-reactive-testing (`src/FerroUI.Reactive.Testing`)

Tests (T): 1 site, `lib.rs:401` (`assert_throws`, `ReactiveAssert.Throws`), in a testing crate.

### ferroui-reactiveui (`src/FerroUI.ReactiveUI`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `internal/panic_exception.rs:60` | `PanicError::catch` | catches a panic of user code and gives it as an `ObservableError`; the shared catch of the crate | B | no change in correct operation: a panicking selector, factory, handler, command body or subscriber aborts instead of reaching `OnError` or `ThrownExceptions`; an error returned as a faulted observable is unaffected |
| `reactive_object/extension_state.rs:157` | `ExtensionState::notify_observable` | raises the panic of a subscriber again when no `ThrownExceptions` observer exists | R | no change (belongs to `panic_exception.rs:60`, called at :150) |
| `rx_state.rs:67` | `RxState::initialize_default_exception_handler` | raises an unhandled error (`UnhandledError`) on the main thread scheduler on purpose | R | no change in correct operation: an error nobody handled ends the process; with abort the unhandled-exception filter of the dispatcher (the boundary of `ferroui-base` that receives it) can no longer mark it handled |
| `binding/bindings/binding_errors.rs:47` | `BindingObserver::on_error` | gives a fault of a binding source with an inner error, as `TargetInvocationError`, to the notifying object's `ThrownExceptions` or the default exception handler | E | converted (#567): see below |
| `binding/view/view_locator.rs:17` | `ViewLocator::current` | raises `ViewLocatorNotFoundError` when no `dyn IViewLocator` is registered | E | no change: no framework code catches it; a misconfiguration crash either way |

`PanicError::catch` has 11 callers outside tests, all wrapping user code where the reference
catches an `Exception`: `mixins/switch_subscribe_mixins.rs:197` (selector),
`mixins/observable_mixins.rs:118` (factory), `reactive_property/reactive_property.rs:915`
(selector), `primitives/start_signal.rs:34` (function), `binding/interactions/interaction.rs:202`
(handler), `reactive_command/reactive_command_t_param_t_result.rs:500` (execute),
`mixins/observable_logging_mixins.rs:201`, `:209`, `:217`, `:270` (logging callbacks and the
error handler), `reactive_object/extension_state.rs:150` (subscriber). Two more callers are in
`ferroui-reactiveui-testing` (`rx_test.rs:153`, `scheduler_extensions.rs:73`), test support.

**(E) `binding/bindings/binding_errors.rs:47`.** When the sequence that feeds the write of a
binding faults with an inner error (a setter failed), the observer logs it and raises a
`TargetInvocationError` panic, as the reference throws `TargetInvocationException`. The panic
unwinds to whoever pushed the fault; when that is the change notification of a reactive object,
`ExtensionState::notify_observable` catches it (`PanicError::catch`) and delivers it to the
object's `ThrownExceptions` as an error value. With abort that delivery stops: the process aborts.
Where nothing catches it, it ends the process in both builds, as in the reference. A `Result`
form cannot travel through `IObserver::on_error` (returns `()`). Converted (owner decision of
2026-11-07): the observer raises no panic. It gives the `TargetInvocationError` to the place the
panic ended: the `ThrownExceptions` of the reactive object whose change notification runs
(`ExtensionState::notify_observable` records it on a thread-local stack, and
`deliver_subscriber_error` logs and delivers as the catch did), or else
`RxState::default_exception_handler()`, which an application replaces and which by default raises
it on the main thread. The same in both builds. One function, 8 callers of
`BindingErrors::subscribe` (in `binding/bindings/property_binder.rs` and `binding/mixins/`).

**(E) `binding/view/view_locator.rs:17`.** A missing `dyn IViewLocator` registration is raised
as a typed panic payload. Nothing in the framework catches it, so abort changes nothing; listed
because the API reports a lookup failure as a panic. A `Result` or `Option` form of
`ViewLocator::current` reaches 2 callers (`platform/view_host_resolution.rs:34`, and
`view_host_navigation.rs:83` in `ferroui-reactiveui-controls`), both of which already return
`Option<Rc<dyn IViewFor>>`. Production path, misconfiguration only.

### ferroui-reactiveui-controls (`src/FerroUI.ReactiveUI.Controls`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `ferro_object_reactive_extensions.rs:226` | `FerroPropertySignal::subscribe` | catches a panic of subscribing (the observer of the caller); the count is restored | B | no change: try/catch-rethrow; the count is not restored on abort |
| `ferro_object_reactive_extensions.rs:231` | same | raises it again | R | no change (belongs to :226) |
| `view_registrar.rs:154` | `ViewRegistrar::create_view` | catches a panic of the service locator (registered factories, a disposed resolver); logs and falls back to the constructor | B | no change in correct operation: a panicking factory or a disposed resolver aborts instead of the logged fallback |

Tests (T): 2 sites in `tests/ferro_object_reactive_extensions_behavior_tests.rs`.

### ferroui-splat (`src/FerroUI.Splat`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `logging/memoizing_mru_cache.rs:239` | `MemoizingMRUCache::invalidate_all_with` | catches a panic of each release callback; aggregated into `AggregateReleaseError` | B | no change: the first panicking release aborts instead of being aggregated |
| `mode_detection/default_mode_detector.rs:98` | `DefaultModeDetector::detect_unit_test_runner_safely` | catches a panic of the detection and logs it | B | no change: nothing in the detection panics (environment reads return values); the catch is redundant |
| `application_performance_monitoring/mod.rs:89` | `run_in_session` | catches a panic of the tracked action; recorded with `on_exception`, session disposed | B | no change: the session is not disposed on abort |
| `application_performance_monitoring/mod.rs:93` | same | raises it again | R | no change (belongs to :89) |
| `service_location/common/resolver_exception_helpers.rs:12` | `run_swallowing_exceptions` | catches and drops a panic of a resolver callback or disposal (5 callers in the resolvers) | B | no change: a panicking callback or disposal aborts instead of being skipped |
| `logging/log_host.rs:17` | `LogHost::default_logger` | raises `LoggingError` when the resolver has no `dyn ILogManager` | E | no change: no framework code catches it |
| `logging/default_log_manager.rs:28` | `DefaultLogManager::new` (cache factory) | raises `LoggingError` when the resolver has no `dyn ILogger` | E | no change: no framework code catches it |

Tests (T): 1 site in `tests/service_location/global_generic_first/array_helpers_tests.rs`.

**(E) `logging/log_host.rs:17` and `logging/default_log_manager.rs:28`.** A broken resolver
(no log manager, no logger) is raised as a typed `LoggingError` panic, as the reference throws
`LoggingException` ("This should never happen, your dependency resolver is broken"). No
framework code catches it (no `downcast` of `LoggingError` outside tests), so abort changes
nothing; listed because the payload is an error type meant to be caught. A `Result` form of
`LogHost::default_logger` reaches its callers across ReactiveUI and Dock (logging call sites);
`DefaultLogManager` would need a fallible `get_logger` in `ILogManager`, which every log manager
implements. Not worth converting: both are misconfiguration crashes. Production path.

### ferroui-controls-treedatagrid (`src/TreeDataGrid/FerroUI.Controls.TreeDataGrid`)

Tests (T): 1 site in `primitives/tests/support.rs`.

### ferroui-treedatagrid-core (`src/TreeDataGrid/FerroUI.TreeDataGrid.Core`)

Tests (T): 1 site in `tests/support.rs`.

## 10. Tools

| Crate | B | R | E | T | All |
|---|---:|---:|---:|---:|---:|
| `ferroui-view` | 1 | 0 | 0 | 0 | 1 |
| `ferroui-remote-protocol` | 1 | 0 | 0 | 0 | 1 |
| `ferroui-inspect` | 1 | 0 | 0 | 0 | 1 |
| `ferroui-mcp` | 2 | 0 | 0 | 0 | 2 |
| `ferroui-devtools` | 1 | 0 | 0 | 0 | 1 |
| `ferroui-hot-reload` | 1 | 0 | 1 | 6 | 8 |
| `ferroui-hot-patch` | 1 | 1 | 0 | 1 | 3 |
| `ferroui-designer-support` | 4 | 0 | 0 | 0 | 4 |
| `ferroui-designer` | 7 | 0 | 0 | 0 | 7 |
| **All** | **19** | **1** | **1** | **7** | **28** |

`src/FerroUI.Designer.*` (Animation, Collaboration, Collaboration.Host, Core, Language, Project,
Tools), `src/FerroUI.Build.Tasks`, `src/FerroUI.Build.Scan`, `src/tools` and `src/FerroUI` have no
site (no `catch_unwind`, `resume_unwind`, `panic_any`, `set_hook` or `take_hook`), so they have no
section. None of the crates of this group installs or takes a panic hook. Not counted: the doc
comment `error_boundary.rs:6` of `ferroui-view` (it names `catch_unwind`, it is no call), and
`root_binding_tests.rs:288` of `ferroui-hot-reload` (a `_tests.rs` file the grep leaves out; T).

Most sites are development tools. A (B) site of a tool that isolates user code is marked as
needing a build that unwinds (`dev`, `fast-release`, `release-unwind` or `dist-unwind` profile). Three (E) families reached the
catches of this group: the `panic_any(ReloadError)` of hot reload, the validation panics of
`Animation::apply_for_preview` and the animators (`ferroui-base`), and the "throw" of the XAML
runtime of `ferroui-markup-xaml` (`crate::throw`, `xaml_error_scope::throw`: a failing deferred
resource, template, include or markup extension, which panics with its `XamlLoadError` outside a
`with_xaml_errors` scope). All three are converted (#876 and 2026-11-19): the steps of hot
reload, the previewer's load and show and the Designer's stage layouts run in a
`with_xaml_errors` scope, and the Designer's animation preview calls
`Animation::try_apply_for_preview`. The catches stay as boundaries (B) of the application's
code, and of the members of the runtime that keep the fatal throw (5.1).

### View (`src/FerroUI.View`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `hooks.rs:100` | `render_catching` | catches a panic of the function of a component below an error boundary | B | no change in correct operation; the error boundary does nothing: a panicking component aborts instead of showing the fallback |

### Remote Protocol (`src/FerroUI.Remote.Protocol`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `error.rs:93` | `catch_handler` | catches a panic of the message handlers (used by `BsonStreamTransport::read_messages` and `EventStash::add`) | B | no change: a panicking handler aborts instead of ending the reader loop with `Error::Handler` or reaching the exception handler of the stash |

### Inspect (`src/FerroUI.Inspect`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `lib.rs:930` | `catch` (for `Inspector::execute` and `execute`) | catches a panic of a tool, which reaches application handlers (input, property changed) | B | no change: arguments, conversion and validation are checked before and return `Err`; a panicking application handler aborts the application instead of answering the tool with an error |

### MCP (`src/FerroUI.Mcp`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `lib.rs:294` | `Client::call` | catches a panic of a service tool provider (application code) on the request thread | B | no change: a panicking provider aborts the process instead of answering "Service operation panicked" |
| `lib.rs:335` | `Client::call` (dispatcher job) | catches a panic of an extension tool or of `ferroui_inspect::execute` | B | no change: a panicking tool aborts the application instead of answering "UI operation panicked" |

### DevTools (`src/FerroUI.DevTools`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `bridge.rs:51` | `guarded` | catches a panic of a command in a JavaScript callback (FFI) | B | no change: `wasm32-unknown-unknown` does not unwind by default, so a panic already aborts the module there |

### Hot reload (`src/FerroUI.HotReload`)

The crate is `#![cfg(debug_assertions)]`: it is not compiled into `release` or `dist`.

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `lib.rs:671` | `catch_reload` | catches a panic of an adapter or of an application callback, and turns its payload into `ReloadError::Failed`; the errors the XAML runtime reports in the step come from its `with_xaml_errors` scope | B | no change: a panicking callback aborts the application instead of giving a `ReloadOutcome`; a failing deferred resource, include or template of the candidate gives `ReloadError::Failed` without unwinding (since 2026-11-19) |
| `lib.rs:651` | `raise_reload_error` | raises a `ReloadError` as a panic when no step of a reload is open | E | no change: the resource previews of a candidate exist only inside a step, so nothing reaches it in correct operation |
| `resource_preparation.rs:40` | `ResourcePreparation::register` (preview factory) | raised the `ReloadError` of `resolve` with `panic_any` | E | converted: raised in the step (`raise_reload_error`), the lookup goes on in the original node; no panic |
| `root_resources.rs:50` | `OwnedResources::preview_resource` | raised `ReloadError::RestartRequired` with `panic_any` | E | converted: raised in the step, the key is not found; no panic |
| `root_styles.rs:146` | `OwnedStyles::preview_dictionary` | raised `ReloadError::RestartRequired` with `panic_any` | E | converted: as `root_resources.rs:50` |
| `root_styles.rs:151` | `OwnedStyles::preview_resource` | raised the `ReloadError` of `current()` with `panic_any` | E | converted: as `root_resources.rs:50` |
| `restart_handoff.rs:143` | test in `#[cfg(test)] mod tests` | catches a panicking participant | T | untouched |
| `root_preview.rs:152` | test in `#[cfg(test)] mod tests` | catches a test panic | T | untouched |
| `root_preview.rs:174` | test in `#[cfg(test)] mod tests` | catches a test panic | T | untouched |
| `tests.rs` | tests | 3 sites | T | untouched |

**`ReloadError` through the resource lookup of a candidate** (`resource_preparation.rs:40`,
`root_resources.rs:50`, `root_styles.rs:146`, `root_styles.rs:151`). A `ReloadError` (mostly
`RestartRequired`, an expected outcome of a reload) arises in lookups that have no error channel:
`Preview::try_get_resource` (`root_preview.rs`, `IResourceNode::try_get_resource ->
Option<ResourceValue>`) calls `preview_resource`/`preview_dictionary`, and the factory registered
in `ResourcePreviewScope` is `Rc<dyn Fn() -> Rc<dyn IResourceNode>>`. It travelled as a panic
(`panic_any`) to `catch_reload`. Converted (#567) with a side channel, option (a) of the first
survey, rather than a fallible lookup through every node: `catch_reload` opens a scope of the
thread, `raise_reload_error` records the first error of the innermost scope and the lookup returns
its fallback (the key is not found; the factory gives the original node), and `catch_reload`
returns that error when the action ends, before the result of the action or of a panic. Nested
steps (`ResourcePreparation::resolve` inside a lookup of an outer step) have their own scopes.
Outside every step the error is still raised as a panic. Under unwinding the outcome of a reload
is the same `ReloadOutcome`; what differs is that the code after the failed lookup runs with the
fallback until the step ends, instead of unwinding at once. The candidate of a failed step is
not committed. Development tool only, never a production path. Test:
`an_error_raised_in_a_reload_step_is_its_error_without_unwinding`.

**The throws of the XAML runtime in a step** (`lib.rs:671`, 2026-11-19). A deferred resource,
include or template of the candidate that fails while it is populated (or while a prepared
change is committed) threw: a panic with the message of its `XamlLoadError`, which `catch_reload`
caught and turned into `ReloadError::Failed(message)`. Converted: `catch_reload` runs its action
in `with_xaml_errors` (5.1). The failing member reports its error and builds nothing, the loader
stops at its next node, and the step fails with `ReloadError::Failed` and the same message.
Precedence is as before: a `ReloadError` raised before the report wins; one raised after it is
left out (`raise_reload_error` asks the step's own scope, `xaml_error_scope::has_error_at`, new
and `doc(hidden)`), as when the throw ended the step; a scope opened inside the step keeps its
own error. What differs under unwinding is that the code after the failing member runs with its
fallback up to the next node of the loader instead of unwinding at once. `catch_reload` stays
as the boundary (B) of adapters, application callbacks and the members of the runtime that keep
the fatal throw, which still need a build that unwinds. Test:
`an_error_the_xaml_runtime_reports_in_a_reload_step_is_its_error_without_unwinding`.

### Hot patch (`src/FerroUI.HotPatch`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `supervisor.rs:194` | `PatchSupervisor::apply_with` | catches a panic of the view transaction (application code) | B | no change: the process ends, so restoring the dispatch table is moot |
| `supervisor.rs:205` | `PatchSupervisor::apply_with` | raises that panic again after the rollback | R | kept (belongs to `:194`) |
| `lib.rs:110` | test in `#[cfg(all(test, debug_assertions))] mod callback_tests` | catches a panicking handler | T | untouched |

### Designer support (`src/FerroUI.DesignerSupport`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `remote/html_transport/html_transport.rs:373` | `HtmlWebSocketTransport::raise_message` | catches a panic of the message handlers | B | no change: a panicking handler aborts the previewer instead of becoming `Error::Handler` |
| `remote/previewer_windowing_platform.rs:109` | `PreviewerWindowingPlatform::create_embeddable_window` | catches a panic of the dispose of the last preview window (the reference ignores the exception) | B | no change: a panicking closed handler aborts the previewer |
| `remote/remote_designer_entry_point.rs:380` | `RemoteDesignerEntryPoint::on_transport_message` | catches a panic of the previewed application's code while the document is loaded and its window shown, and answers `UpdateXamlResultMessage` with its message; the errors of the document come from a `with_xaml_errors` scope | B | no change: a panicking constructor or handler of the application aborts the previewer; an error of the edited document that the XAML runtime reports is answered as a load error without unwinding (since 2026-11-19) |
| `remote/remote_designer_entry_point.rs:397` | `RemoteDesignerEntryPoint::on_transport_message` | catches a panic of closing the previous preview window | B | no change: a panicking closing handler aborts the previewer |

**`remote_designer_entry_point.rs:380`.** `DesignWindowLoader::load_designer_window_with_scaling`
returns `Result<Ref<Window>, XamlLoadError>` for the errors of the loader itself, but the members
the XAML runtime calls through metadata (deferred content, templates, `StyleInclude` and
`ResourceInclude::loaded`, markup extensions) have no error channel and threw: a panic with the
message of their `XamlLoadError`, which this catch answered as an error of type `Panic`. The
reference catches the exception of the whole load and show (`catch (Exception e)`). Converted
(2026-11-19): the load and the show run in `with_xaml_errors` (5.1), so such an error is the
`Err` of the load, answered with the same message and the details of a load error
(`exception_details`: the type and position of the inner `XamlError`, else
`XamlLoadException`). The window of a failed load is not kept as the current preview, as when
the throw unwound out of it; what differs under unwinding is that the show runs to its end with
the fallbacks (templates without content) before the error is answered. The catch stays as the
boundary (B) of the previewed application's constructors and handlers, and of the members that
keep the fatal throw. Development tool (the previewer process). Test:
`an_error_of_the_xaml_runtime_is_answered_as_a_load_error_without_unwinding`.

### Designer (`src/FerroUI.Designer`)

| File:line | Function | Catches or raises | Class | With abort |
|---|---|---|---|---|
| `animation_runtime.rs:190` | `NativeAnimations::refresh` | catches a panic of `ResourceDictionary::get` (a deferred animation resource is built, running constructors of the application) and skips the key | B | no change: since this step the failure of the build goes to an error scope (`with_xaml_errors`) and the key is left out without unwinding; a panicking constructor of the application aborts the designer |
| `animation_runtime.rs:328` | `NativeAnimations::seek` | catches a panic of an animator's own checks or of a property changed handler while `Animation::try_apply_for_preview` applies, and returns "Native animation rejected the program" | B | no change: an animator given for a property of another type or a panicking animator or handler of the application aborts the designer; the programs the framework rejects (no setter property, no animator for the property, a CSS `TransformOperations` target, a target that is not a `Visual`, negative timing) are `Err` without unwinding (since 2026-11-19) |
| `animation_runtime.rs:380` | `pulse` | catches a panic of a frame of the preview clocks | B | no change: the animators validate on apply (`:328`), and a timing error of a frame is published as an observable error, not a panic; a panicking property changed handler aborts the designer |
| `preview_stage_export.rs:103` | `capture_preview_stage_png` | catches a panic while the stage is rendered to a bitmap | B | no change: size and backend are checked before and return `Err`; a panicking render of a previewed control aborts the designer |
| `preview_stages.rs:1051` | `PreviewStageBoard::prototype_hotspot_bounds` | catches a panic while the native geometry of a hotspot is read | B | no change: every expected failure is an `Err` of `resolve_prototype_hotspot_bounds` |
| `preview_stages.rs:1665` | `PreviewStageBoard::build_candidate` | catches a panic of the application's constructors and layout overrides while a candidate stage is built, laid out and given its states | B | no change: a panicking constructor aborts the designer; an error of the edited document (templates and resources built by `update_layout`, `apply_stage_states`) fails the stage without unwinding (`with_runtime_errors`, since 2026-11-19) |
| `preview_stages.rs:1760` | `PreviewStageBoard::apply_layout` | catches a panic of a layout callback of the application | B | no change: as `:1665`, for the templates and resources the new layout builds |

**`animation_runtime.rs:190`.** Was (E): `ResourceDictionary::get` builds a deferred resource
through `IDeferredContent::build`, and the XAML implementation threw (panicked) when the builder
failed. Converted in this step (section 5.3): the get runs in `with_xaml_errors`, the dictionary
builds with `IDeferredContent::try_build`, the failure goes to the scope and the key is left out,
without unwinding. The catch stays as the boundary of the constructors of the application the
builder runs. Development tool.

**`animation_runtime.rs:328`.** `Animation::apply_for_preview(&self, &Animatable,
Option<Rc<dyn IClock>>, Option<Rc<dyn Fn()>>) -> Rc<dyn IDisposable>` (`ferroui-base`) reports a
rejected program by panicking: its own `TransformOperations` check, `interpret_keyframes` (no
setter property, no animator registered), the animators' `apply` (`IAnimator::apply -> Option<..>`;
`TransformAnimator` target and property checks, `AnimatorBase::styled_property` type check) and
`AnimationInstance::new`, which panics on the `Err(AnimationError)` of `fetch_properties`.
Converted (2026-11-19, 6.1): the seek calls `Animation::try_apply_for_preview`, which returns the
rejections of `ferroui-base` as `Err(AnimationError)` with the message of the panic before
anything is applied; the message of the seek is the same ("Native animation rejected the
program: .."). What an animator checks itself still panics (a fallible `IAnimator::apply` would
change a public trait), so the catch stays as the boundary (B) of those, of the application's
animators and of its property changed handlers. The cleanup-and-rethrow inside
`apply_for_preview` is the (R) of `ferroui-base`. Development tool. Test (`ferroui-base`):
`animation_preview_rejections_are_errors_with_the_message_of_the_panic`.

**`preview_stages.rs:1665` and `:1760`.** The candidate document is populated in a reload session
(`RuntimePreview::new_isolated`, caught by `catch_reload` of `ferroui-hot-reload`, which has its
own error scope since 2026-11-19), but the layout of the stage (`update_layout` here and in
`apply_layout`), `rebuild_map` and `apply_stage_states` run outside it and build templates,
deferred resources and includes, whose failures the XAML runtime threw as panics. These catches
turned them into a failed stage (`Err("Native preview construction failed: ..")`,
`PreviewStageStatus::Failed`). Converted (2026-11-19): that work runs in `with_runtime_errors`,
a `with_xaml_errors` scope (5.1) whose first error fails the stage with the same message
(`"Native preview {construction|layout} failed: {error}"`), without unwinding; the stage is
removed as before. Both catches stay as the boundary (B) of the registered constructors and
layout overrides of the application, as the comment at `:1658` says. Development tool. Test:
`an_error_of_the_xaml_runtime_in_a_stage_layout_is_its_error_without_unwinding`.

## 11. Errors of application code as values (0.6)

0.6 allows breaking changes. The (B) sites of sections 4 to 10 that isolate the application's
own code (its constructors, handlers, layout callbacks and animators, run by the framework or
previewed by a tool) caught a panic because a callback had no other way to fail. In 0.6 the
callbacks get fallible forms, their errors go to one place that exists already, and a panic is
what it is with `panic = "abort"`: a bug that ends the process after the panic hook.

### 11.1 The defined place: the unhandled-exception events of the dispatcher

- **`Dispatcher::raise_unhandled_exception(error)`** (`impl Into<Box<dyn Error + Send + Sync>>`)
  gives an error of application code that has no caller to return it to to
  `unhandled_exception_filter` and `unhandled_exception`, the events the panic of a posted
  callback reaches (`try_catch_when`). The payload of the event is the error
  (`Box<dyn Error + Send + Sync>`): `DispatcherUnhandledExceptionEventArgs::error()` (and the
  filter's) return it, `exception_message()` its text. Off the dispatcher thread it is posted.
- An error no handler handles is fatal, as an unhandled exception is in the reference: under
  `panic = "unwind"` it is logged and unwinds with its message as the payload, marked as seen
  by the operations it unwinds through (no second filtering); under `panic = "abort"` the
  process ends with a panic with the message.
- **Fallible callbacks**: `Dispatcher::post_fallible`, `post_local_fallible`,
  `Interactive::add_fallible_handler` and `add_fallible_handler_with` take a callback that
  returns `Result<(), E>` and give its error to `raise_unhandled_exception`. The infallible
  methods keep their signatures: a generic result type there would make every closure that only
  panics depend on the never type fallback, which the compiler rejects.
- **`Dispatcher::capture_unhandled_exceptions(work) -> Result<T, String>`** runs `work` and
  returns the message of the first exception that reaches the events while it runs and that no
  other handler handles (marked handled); only the innermost capture of a thread records. This
  is how the development tools fail a step with the error of the code they preview.
- ReactiveUI: `RxState::default_exception_handler` of the core library still panics with an
  `UnhandledError` on the main thread scheduler (the library knows no dispatcher, `rx_state.rs`,
  (R)); a FerroUI application (`with_ferro`) gets a handler that posts a job returning the
  `UnhandledError` to the dispatcher of the user interface thread, unless the builder has one
  (`ReactiveUIBuilder::exception_handler`, `UnhandledError::of_broken_pipeline`, new).

### 11.2 The converted flows

| Flow | Before | After | Test |
|---|---|---|---|
| posted jobs (`Dispatcher::post_fallible`, `post_local_fallible`) | a job could only panic | a job returns `Result`; the error reaches the events | `an_error_a_posted_job_returns_reaches_the_unhandled_exception_events_without_a_panic`, `an_error_no_handler_handles_unwinds_with_its_message_once`, `an_error_raised_off_the_dispatcher_thread_is_posted_to_it` |
| routed event handlers (`add_fallible_handler`, `add_fallible_handler_with`) | a handler could only panic, which stopped the route | the error reaches the events; handled, the route goes on | `an_error_a_fallible_handler_returns_reaches_the_dispatcher_and_the_route_goes_on` |
| markup event handlers and method bindings (`MarkupDelegate::invoke`) | the failure of a `try fn` handler was discarded | it reaches the events | `delegates_of_declared_methods_know_their_method_and_target` |
| animators (`IAnimator::apply`, `Animator::apply`) | the type of the property (`AnimatorBase::styled_property`), the target of `TransformAnimator` and the timing were panics when the animation ran | `apply` returns `Result<Option<..>, AnimationError>` and checks first (`try_styled_property`, `validate_timing`); `Animation::try_apply` (new) removes the steps applied and returns the error; `Animation::apply` raises it on the dispatcher; `try_apply_for_preview` returns it | `an_animator_that_rejects_its_property_is_an_error_of_apply_without_a_panic` |
| the members of the XAML runtime outside a load (deferred resources, includes, templates built on first use) | a panic with the message (`throw_or` outside a scope) | the error reaches the events (`xaml_error_scope::report_or_raise`, new); the member goes on with its fallback | `a_failing_builder_outside_a_load_raises_its_error_on_the_dispatcher`, `without_a_scope_a_failing_member_raises_its_error_on_the_dispatcher` |
| the fatal throw of the XAML runtime (5.1) | `xaml_error_scope::throw` and `crate::throw` | removed: `deferred_transformation_factory_v1` to `_v3` return `Result`; the panicking `deferred_transformation_factory_for`, `StyleInclude::loaded`, `ResourceInclude::loaded`, `RelativeSourceExtension::provide_value`, `OnFormFactorExtension::should_provide_option` and `PropertyInfoAccessorFactory::create_ferro_property_accessor` are removed (their `try_` forms); `WindowDrawnDecorationsTemplate::build` reports and builds empty decorations | the tests of the `try_` forms |
| run-time loading (`FerroRuntimeXamlLoader::load_document`, `load_group`) | a failing deferred resource of the load panicked | the load runs in a scope; the error is its `Err` | `fresh_root_deferred_resource_error_is_returned_by_the_importer_and_the_default_loader` |
| ReactiveUI pipelines nothing handles, in a FerroUI application | a panic on the main thread scheduler | a failing job on the dispatcher | `with_ferro_raises_unhandled_pipeline_errors_on_the_dispatcher_without_a_panic` |
| hot reload (`catch_reload`) | the errors of the candidate's code were panics | captured from the dispatcher: `ReloadError::Failed`; `raise_reload_error` outside a step raises on the dispatcher instead of `panic_any` (`ReloadError` implements `Error`) | `an_error_the_application_raises_on_the_dispatcher_in_a_reload_step_is_its_error_without_unwinding` |
| the Designer's stages (`with_runtime_errors`) and animation preview (`seek`) | panics of the previewed code | captured: the stage fails, the seek answers "Native animation rejected the program" | `an_error_the_previewed_code_raises_on_the_dispatcher_fails_the_stage_without_unwinding` |
| the previewer's load and show | panics of the previewed code, answered as `Panic` | captured, answered as an `Exception` | `an_error_the_previewed_code_raises_on_the_dispatcher_is_answered_without_unwinding` |

Constructors: a markup type declares a fallible constructor with `try (..) => ..` in
`ferro_markup_type!` (this existed before 0.6); the run-time loader and the compiled markup
turn its error into a load error, which the load returns. Layout and property
changed callbacks (`measure_override`, `arrange_override`, property changed handlers) keep their
signatures: one that fails raises its error with `Dispatcher::raise_unhandled_exception` and
returns its fallback (a size, nothing), as the XAML runtime does. `XBinding::create_instance`
keeps a panic for a missing target property, which the loader and the property system always
pass: an assertion of a bug. Class handlers (`RoutedEvent::add_class_handler`) keep their
signature: they are the framework's own (244 call sites name their target type explicitly);
one that fails raises its error the same way.

### 11.3 Breaking changes

`IAnimator::apply` and `Animator::apply` return `Result`; `deferred_transformation_factory_v1`
to `_v3` return `Result`; the panicking members of 11.2 are removed; a failing markup delegate and a failing member of the XAML runtime outside a load are errors on
the dispatcher, fatal unless a handler handles them.

### 11.4 What still catches a panic, and why

Every other (B) and (R) site of sections 4 to 10 stays, and none is needed for correct
operation: each isolates code that can only fail by a bug, and does so only in a build that
unwinds (`dev`, `fast-release`, `release-unwind`, `dist-unwind`); with `panic = "abort"` it
never sees a panic. They are: the FFI guards (a panic must not unwind into native code: Win32,
X11/GLib, Wayland, the native backend, Android, the browser, DevTools); the dispatcher's own
try/finally sites and the top of its loop, the render loops and the task executors (they turn a
panic into the exception of an operation, as the reference does, and now also carry the errors
of 11.1); the native control hosts and the destructors and cleanup-and-rethrow sites (Dock,
Behaviors, DynamicData, Splat, `LocalBindingReload`, `apply_for_preview`); the headless test
session and the testing crates (tests always unwind); and the catches of the development tools
(Designer, previewer, hot reload, hot patch, Inspect, MCP), which keep a previewed application's
bug from ending the tool in a build that unwinds. The (E) sites left are those of section 2,
less hot reload's `raise_reload_error`, converted here: third-party panics (Vello), the
`Canceled` of a test task, and misconfiguration panics (a missing view locator or log manager).
