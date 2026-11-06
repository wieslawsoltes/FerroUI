# Panics as exceptions: the sites of `ferroui-base`

Status: step 1 of #567 for `ferroui-base`, 2026-11-06. Other crates follow.

Issue #567 replaces panics that carry an error between framework functions with `Result`, and
keeps `catch_unwind` only at true boundaries. This page classifies every use of `catch_unwind`,
`resume_unwind` and `panic_any` in `src/FerroUI.Base` (found with
`grep -rn "catch_unwind\|resume_unwind\|panic_any" src/FerroUI.Base`, `use` lines left out) and
records what happens to each.

## 1. Classes

- **(B)** Boundary, kept: an FFI callback, the top of the dispatcher loop, or application code
  whose panic must not take the loop down, where the reference isolates the exception too (an
  exception in one subscriber does not stop the others, or is rethrown later on the dispatcher).
  The try/finally of the reference around such code is part of the boundary.
- **(E)** Exception-like flow, to become `Result`: a panic used to carry an error between
  framework functions (a conversion or validation failure, a binding error, a parse error) that
  framework code catches and turns into an error value or a fallback.
- **(T)** Test or diagnostics support behind `cfg(test)` or a testing feature, untouched.
- **(R)** Rethrow: a `resume_unwind` that raises a caught panic again on purpose. Kept when it
  belongs to a (B) site.

## 2. Counts

| Class | Sites | Change |
|---|---:|---|
| (B) | 15 | kept, each marked `// Boundary (#567): ...` |
| (R) | 14 | kept: 13 belong to (B) sites, 1 is the cleanup-and-rethrow of `Animation::apply_for_preview` |
| (E) | 1 | deferred (section 4) |
| (T) | 75 | untouched |
| **All** | **105** | |

Outside tests the crate has 30 sites, all in the dispatcher (24), the render loop and the
custom visual handler (2), the binding reload (1) and the animations (3). None of them is a
panic that framework code of this crate catches and turns into an error value or a fallback:
every `catch_unwind` outside tests either isolates application code as the reference does, or
runs a finally block and raises the same panic again. No function changed its signature, and
no public API changed. The one exception-like flow of the crate is caught in `ferroui-system`
and is deferred.

## 3. The sites outside tests

Line numbers are those after the boundary comments were added.

### 3.1 Dispatcher (`threading/`)

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

### 3.2 Rendering, data and animations

| File:line | Function | Catches or raises | Class | Change |
|---|---|---|---|---|
| `rendering/render_loop.rs:107` | `DefaultRenderLoop::timer_tick` | catches a panic of a render loop task | B | kept: the top of the render loop; logged as "Exception in render loop", as the reference does |
| `rendering/composition/server/server_custom_composition_visual.rs:30` | `guarded` | catches a panic of a custom visual handler | B | kept: application code on the render thread; logged as the reference logs the exception of a handler |
| `data/local_binding_reload.rs:252` | `<LocalBindingReload<T> as Drop>::drop` | catches a panic of the rollback | B | kept: application observers run from a destructor, where a second panic during unwinding would abort |
| `animation/animation.rs:415` | `Animation::apply_for_preview` | catches a panic of the preview setup | R | kept: cleanup and the same panic raised again, not an error value (see 4.2) |
| `animation/animation.rs:451` | `Animation::apply_for_preview` | raises it again after disposing of the setup steps | R | kept (belongs to :415) |
| `animation/composite_page_transition.rs:58` | `CompositePageTransition::start` (its future) | raises the `JoinError::Panicked` of a child transition so that the task of the composite fails with it | E | deferred (4.1) |

## 4. Deferred

### 4.1 Failures of page transitions

`IPageTransition::start` returns a `JoinHandle<()>`, and its contract reports a failure to run an
animation as `JoinError::Panicked`, a value of the handle. The transitions make that value with
a panic, which the executor of `ferroui-system` catches (`task/executor.rs`) and stores:

- `animation/composite_page_transition.rs:58`: `resume_unwind` of the payload of a failed child
  transition;
- `animation/composite_page_transition.rs:63`: `panic!` with `Canceled` when a child was
  canceled;
- `animation/i_page_transition.rs:71`: `when_all` panics with the error of a failed
  `AnimationTask` (used by `CrossFade`, `PageSlide` and `Rotate3DTransition`).

Deferred: the executor of `ferroui-system` makes `JoinError::Panicked` only from a panic, and
`ferroui-system` is outside this step. The alternative, a `Result` output for
`IPageTransition::start`, ripples through more than 15 call sites and implementations in other
crates (an implementation and the callers in `ferroui-controls`, with their tests, and seven
implementations in the control catalog). The way out is an executor primitive
that completes a task with a `JoinError` value, or a future whose `Result` output the dispatcher
flattens into the outcome of the handle.

### 4.2 The preview of an animation (caught in another crate)

`Animation::apply_for_preview` rejects a target with a CSS `TransformOperations` value with a
`panic!` (`animation/animation.rs:431`), and `Animation::apply` and the animators validate
their setters, key frames and targets with panics too. The designer turns these into error
values (`src/FerroUI.Designer/animation_runtime.rs`, the `catch_unwind` around
`apply_for_preview`). That is an exception-like flow whose catch is outside this crate; a
`try_apply_for_preview` alone would not let the designer drop its catch while `apply` and the
animators still panic, and a `Result` form of `apply` reaches every animator. It belongs to the
step for the designer.

## 5. Tests and test support (T)

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

## 6. Beyond this crate

For the later steps, sites outside this crate to classify first, because they turn panics into
error values: the throw token of the XAML loader
(`src/Markup/FerroUI.Markup.Xaml/xaml_error_scope.rs`) and the designer
(`src/FerroUI.Designer/animation_runtime.rs`, `preview_stages.rs`). Many of the sites of the
platform crates are FFI callbacks (Win32 window procedures, GLib sources, JNI entry points).
