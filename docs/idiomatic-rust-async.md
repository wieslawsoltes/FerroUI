# Async: from `Task` and `SynchronizationContext` to futures on the dispatcher (C4 of #423)

The plan of phase C4 of [idiomatic-rust.md](idiomatic-rust.md): `Task`, `TaskCompletionSource` and
`SynchronizationContext` become futures on the dispatcher's executor, keeping the dispatcher's
ordering guarantees, and the workspace gets one cancellation type. It is the riskiest phase of
#423: every asynchronous step of the user interface runs through the dispatcher, and the order in
which continuations run is behaviour that controls, tests and applications see. The first step
(A0) is this page and the tests of section 2, and changes no behaviour and no API. The measures
are of main at 0.5.0-preview.1 (2026-11-02).

## 1. What there is

### 1.1 Counts

From `scripts/idioms/inventory.py` (over `src/`, comments and literals blanked):

| Pattern | Occurrences | Files | Crates |
|---|---:|---:|---:|
| `Task<` | 122 | 40 | 10 |
| `TaskCompletionSource` | 92 | 27 | 11 |
| `SynchronizationContext` | 88 | 24 | 7 |
| `CancellationToken`, `...Source`, `...Registration` | 602 | 126 | 27 |

Per type, with the same blanking: occurrences in production code and in tests (files under
`tests/` or named `..._tests.rs`), and the `pub fn` signatures (outside of tests) that take or
return the type.

| Type | Production | Tests | `pub fn` signatures | Crates with the most |
|---|---:|---:|---:|---|
| `Task<T>` (`ferroui-system`) | 115 | 7 | 23 | dock ReactiveUI services, `ferroui-dependency-injection`, `ferroui-reactiveui`, `ferroui-system` |
| `TaskCompletionSource` (all definitions) | 76 | 16 | 1 | `ferroui-x11` 14, `ferroui-system` 13, `ferroui-remote-protocol` 12 |
| `SynchronizationContext` (the trait) | 62 | 26 | 17 | dock ReactiveUI services 29, `ferroui-reactive` 26, `ferroui-system` 25 |
| `FerroSynchronizationContext` | 82 | 31 | 2 | `ferroui-base` 68, `ferroui-controls` 24, `ferroui-headless` 14 |
| `DispatcherTask` | 138 | 28 | 32 | `ferroui-controls` 114 (22 signatures: `NavigationPage::*_async`, `Window::show_dialog`), `ferroui-base` 34 |
| `DispatcherOperation` | 27 | 6 | 6 | `ferroui-base` |
| `DispatcherFrame` | 27 | 7 | 3 | `ferroui-base`, `ferroui-headless`, `ferroui-x11` |
| `CancellationToken` | 281 | 152 | 42 | `ferroui-reactive` 106, `ferroui-base` 94, `ferroui-dynamic-data` 55, dock 52 |
| `CancellationTokenSource` | 122 | 34 | 1 | `ferroui-controls` 50, `ferroui-base` 32, `ferroui-reactive` 16 |
| `CancellationTokenRegistration` (with the local one) | 24 | 0 | 2 | `ferroui-system` 13, `ferroui-reactive` 6 |
| `OperationCanceledError` | 38 | 14 | 13 | `ferroui-base` 40 |
| `TaskCanceledError` | 14 | 5 | 0 | `ferroui-system` 13 |

The calls of the dispatcher that the phase touches (all of `src/`, tests included):

| Call | Occurrences | Files | Crates |
|---|---:|---:|---:|
| `invoke_async_task` and its variants | 72 | 36 | 11 |
| `to_task_scheduler` and `DispatcherTaskScheduler` | 71 | 40 | 11 |
| `invoke_async` and its variants | 82 | 24 | 8 |
| `resume`, `yield_now`, `await_with_priority` and their awaitables | 48 | 14 | 4 |
| `ferroui_system::threading::tasks::spawn_local` (`Task::spawn_local`) | 33 | 21 | 11 |
| `block_on` (of `ferroui-system`; the one of `ferroui-vello` is a `wgpu` poll loop, not in scope) | 17 | 10 | 7 |
| `post`, `post_local` | 332 | 164 | 33 |
| `invoke`, `invoke_local` and their variants (synchronous) | 401 | 125 | 23 |
| `run_jobs` (mostly tests) | 1372 | 278 | 42 |
| `push_frame`, `main_loop` | 42 | 15 | 8 |

The platform-facing async APIs are already plain futures: 452 lines name
`Pin<Box<dyn Future<..>>>` or `LocalBoxFuture` in 112 files (storage, clipboard, launcher, input
methods, inking), and there are 234 `async fn` outside of tests. They are the target shape.

### 1.2 The types

**`ferroui-system`, `threading`** (the ports of `System.Threading` and `System.Threading.Tasks`;
docs/porting/reactive.md, section 5.8):

- `Task<T>`: a shared, single-threaded (`Rc`) handle to one outcome, `Result<T, ObservableError>`
  with `T: Clone`, with `TaskStatus`, `continue_with` (continuations run synchronously when the
  task ends) and `impl Future`. `ContinuationId`, `TaskCanceledError`, `TaskWorkError`.
- `TaskCompletionSource<T>`: the producer side; `set_result`, `set_exception`, `set_canceled` and
  their `try_` forms.
- The local executor: `spawn_local`, `block_on`, `run` (`Task.Run`: a `Send` closure on a new
  thread whose task ends on the calling thread), `run_pending_wakes`, `set_thread_poster` and
  `ThreadPoster`, `canceled()`.
- `SynchronizationContext` (a trait: `post`, `send`, `post_local`, `send_local`,
  `operation_started`, `operation_completed`) with `current_synchronization_context` and
  `set_synchronization_context`, which also sets the thread poster.
- `CancellationTokenSource`, `CancellationToken`, `CancellationTokenRegistration`,
  `LocalCancellationTokenRegistration` (`register_local`).

**`ferroui-base`, `threading`** (the port of the upstream dispatcher):

- `Dispatcher` with `post`/`post_local`, `invoke*` (synchronous), `invoke_async*` (returns
  `DispatcherOperation<R>`), `invoke_async_task*` (returns `DispatcherTask<T>`), `resume`,
  `resume_with_priority`, `yield_now`, `yield_with_priority`, `await_with_priority`, `run_jobs`,
  `push_frame`, `main_loop`, `disable_processing`, the shutdown and the unhandled-exception events.
- `DispatcherOperation<R>`: a queued job; `wait`, `wait_with_timeout`, `result`, `abort`,
  `set_priority`, the `aborted` and `completed` events, `impl Future`.
- `DispatcherTask<T>`: the outcome of a future running on the dispatcher; `wait`, `result`
  (single take), `impl Future`.
- `DispatcherPriorityAwaitable`, `DispatcherPriorityTaskAwaitable`.
- `FerroSynchronizationContext` (a dispatcher and a priority; the current one of a thread is the
  one of the priority of the job that runs), `RestoreContext`, `DispatcherTaskScheduler`
  (`spawn`, `spawn_local`, `start_local`). `FerroSynchronizationContext` implements the trait of
  `ferroui-system`.
- `DispatcherFrame`, `DispatcherProcessingDisabled`, `DispatcherTimer`, `OperationCanceledError`.
- The platform side: `IDispatcherImpl` and its extensions (`IDispatcherImplWithPendingInput`,
  `IDispatcherImplWithExplicitBackgroundProcessing`, `IControlledDispatcherImpl`, whose
  `run_loop` takes a `CancellationToken`). Implementations: the managed one of `ferroui-base`,
  macOS (`ferroui-native`), Win32, X11 (its own and the GLib one, which Wayland shares), the Linux
  framebuffer (epoll), Android, iOS and the browser; seven of them are controlled (run loops).

**Local copies.** Ten one-shot completions beside the one of `ferroui-system`, each with its own
waker handling: `ferroui-x11` (`TaskCompletionSource` and `TaskCompletionFuture` of the
selections, public; `GlibTaskCompletion` and `GlibTask`; `StartGtkTask`),
`ferroui-remote-protocol` (`Task`, `TaskCompletionSource`, blocking), `ferroui-dialogs`
(`TaskCompletionSource`, `CompletionTask`), `ferroui-headless` (`TaskCompletionSource`,
`HeadlessUnitTestTask`), `ferroui-ios` (`Completion`, `CompletionFuture`), `ferroui-android`
(`Completion`, `CompletionFuture`), `ferroui-freedesktop` (`CompletionSender`,
`CompletionReceiver`), `ferroui-winui` (`Completion`). Other task-like types are handles of their
own subsystem and stay: `ServerJobTask` (compositor), `AnimationTask`, `TaskValue` and
`TaskStreamPlugin` (the `^` stream operator of bindings), `TaskHandle` of the behaviors (a wrapper
of `DispatcherTask`).

**Cancellation types.** Besides the four of `ferroui-system`: `LocalCancellationRegistration` of
`ferroui-base::animation` (a second `register_local`), `CancellationFlag` and `Cancelled` of
`ferroui-freedesktop` (a flag with a future), `AiCancellationToken` of ProTranslate,
`LoopCancellation` of `ferroui-native`, `CancelOnDrop` of `ferroui-headless`; and the error types
`OperationCanceledError` (`ferroui-base`) and `TaskCanceledError` (`ferroui-system`).
`CancellationDisposable` and `ICancelable` of `ferroui-reactive` are Rx API and keep their names.

### 1.3 How the dispatcher runs work

- **One queue per dispatcher thread**, guarded by the instance lock so that any thread can add to
  it: a chain per priority (`Inactive`, `SystemIdle` to `Background`, `Input`, `Default` to
  `Normal`, `Send`) and a chain in the order of arrival. The highest non-empty chain is served
  first; within a chain the order of arrival is kept, also when an operation changes its priority
  (`set_priority` inserts it by its place in the order of arrival).
- **Foreground and background.** Jobs above `Input` run without a pause. Jobs at `Input` and below
  yield to the platform: they run when it reports no pending input, when it calls back for
  explicit background processing, or for at most 50 ms before a timer continues them.
- **Queueing.** `post`, `post_local`, `invoke_async*` and `invoke_async_task*` always queue, also at
  `Send` priority and on the dispatcher thread. `invoke` at `Send` on the dispatcher thread runs
  inline; any other `invoke` queues and waits: on another thread it blocks; on the dispatcher
  thread it pumps the queue (`run_jobs` down to the operation's priority for a foreground
  operation, a nested frame for a background one where the platform has run loops) until the
  operation has run.
- **Timers.** A started `DispatcherTimer` queues an `Inactive` operation; when it is due (the
  platform timer, and after every job) it is promoted to its priority and keeps its place in the
  order of arrival.
- **Frames.** `push_frame` runs a platform loop (`IControlledDispatcherImpl::run_loop`) that
  processes the same queue; the job that pushed it continues when it exits. `main_loop` is a frame
  that a token exits.
- **Disabled processing.** `disable_processing` returns `DispatcherProcessingDisabled`; while one
  is alive `run_jobs`, `push_frame` and the waits that pump panic. The signal of the platform does
  not check it (as upstream).
- **The current context.** While a job runs, the current `FerroSynchronizationContext` is the one
  of the job's priority, and it is restored afterwards.
- **Panics.** A panic of a `post` or `send` callback goes through `unhandled_exception_filter` and
  `unhandled_exception`, and leaves `run_jobs` unless handled; the rest of the queue stays queued.
  A panic of an `invoke_async` callback is kept and raised again to whoever observes the
  operation.
- **Shutdown** aborts the queued operations and drops the futures running on the dispatcher, which
  cancels their tasks.

### 1.4 How futures run: two executors on one thread

- **The dispatcher's** (`DispatcherLocal::tasks`). A future started with `invoke_async_task*` or a
  `DispatcherTaskScheduler` is stored on the dispatcher thread with a fixed priority. Its first poll
  happens inside the job that starts it (`invoke_async_task*`, `start_local`) or in a queued job
  (`spawn`, `spawn_local`). Its waker queues one poll at the task's priority (at most one queued
  poll at a time). So a continuation never runs inside the call that completes what it awaited: it
  runs in a later job, behind the work of its priority that was queued before the wake.
  `resume_with_priority`, `yield_now` and `await_with_priority` post one job at another priority
  that polls the task directly.
- **`ferroui-system`'s** (`threading::tasks::spawn_local`, used by the reactive libraries,
  `FerroUI.View`'s async data and the dependency injection). A future is polled when it is spawned
  and again inside the call that wakes it, on the same thread; a wake during its own poll polls it
  again after that poll. A wake from another thread goes through the thread poster, which
  `FerroScheduler::install_on_ui_thread` sets to the dispatcher's context (a job at that
  context's priority), or waits for `run_pending_wakes`.

`Task::continue_with` continuations run synchronously when the task ends, before the awaiting
futures are woken. The test
`a_completion_source_runs_its_continuations_and_system_tasks_inline_and_queues_dispatcher_tasks`
shows the two executors side by side.

For comparison, .NET posts an `await` continuation to the captured `SynchronizationContext`, and
may run it inline when the completing thread is already in that same context. The port does
neither exactly: the dispatcher always queues, `ferroui-system` always runs inline.

### 1.5 Cancellation

`CancellationTokenSource::cancel` runs the registered callbacks synchronously, on the cancelling
thread, once, in the order of registration; a callback registered after the cancellation runs at
once on the registering thread; `register_local` brings the callback back to the registering
thread (at once on it, otherwise through its poster or `run_pending_wakes`). Dropping a
registration keeps it (the `IDisposable` contract); `dispose()` removes it. A canceled
`invoke_async` operation is aborted inside `cancel`, its `aborted` event runs on the cancelling
thread, and its awaiters are woken from there.

### 1.6 Findings

What the inventory found that is not asserted by the tests, because it is a defect or an open
question; each is decided in the step named.

| # | Finding | Step |
|---|---|---|
| F1 | **Lost wake of a dispatcher task polled in a nested loop.** `poll_task` takes the task out of the table while it polls it and clears its `scheduled` flag. If the future pumps the queue during that poll (a synchronous `invoke_local_with_priority` at a priority other than `Send`, `DispatcherOperation::wait`, `push_frame`), and something it awaits completes there, the queued poll runs inside the pump, finds no task and does nothing, and `scheduled` stays set: every later wake is swallowed and the task never resumes. The executor of `ferroui-system` handles the same case (a wake during a poll repolls after it). | A2 fixes it, with a test |
| F2 | **A panic in a future of `ferroui-system` strands it.** `poll_task` does not catch: the panic leaves through the call that woke the future (often `set_result`), the task stays `Running` in the table with its `polling` flag set, and the awaiters of the same task that were not woken yet never are (the panic leaves `finish` with the rest of its wakers). The dispatcher's executor catches and faults the task. | A1 (the panic faults the task, as on the dispatcher): done |
| F3 | **Continuations inline or queued.** Section 1.4: the two executors of a dispatcher thread order continuations differently, and neither is the .NET rule. | A2 keeps the dispatcher's rule; A5 decides the rule for the tasks of the reactive libraries on a dispatcher thread |
| F4 | **Order of cancellation callbacks.** The port runs them in the order of registration; .NET runs them in reverse order. Not recorded in DEVIATIONS. | A4 keeps the order of registration and records it |
| F5 | **The platform signal runs jobs while processing is disabled** (as upstream: only `run_jobs`, `push_frame` and the pumping waits check the counter). A native modal loop that delivers the signal while a layout pass has disabled processing would run jobs inside it. Pinned by `a_platform_signal_runs_jobs_while_processing_is_disabled`. | Open: an owner decision; not changed without a DEVIATIONS row |
| F6 | **A dropped `TaskCompletionSource` leaves its awaiters pending for ever** (as in .NET); the D-Bus call queue already resolves such a task as canceled (DEVIATIONS, row of `DBusCallQueue`). Pinned by `a_future_awaiting_a_dropped_completion_source_stays_pending`. | A3: the one-shot channel cancels when its sender is dropped |
| F7 | **`DispatcherTask` has one observer.** Its result is taken once; a second `await` or `result()` of a clone panics. `Task<T>` of `ferroui-system` is multi-observer (`T: Clone`). | A1: `JoinHandle` (one owner) and `Shared` (several): done |

## 2. The ordering the dispatcher guarantees (A0)

The tests that pin the current behaviour are in `src/FerroUI.Base/threading/dispatcher_ordering_tests.rs`
(a child module of `dispatcher_tests.rs`, so that it uses the simulated platform of those tests)
and `src/FerroUI.System/tests/async_ordering_tests.rs`, beside the existing dispatcher, task and
synchronization-context tests, which pin the rest (priorities, changing a priority, frames and
shutdown, the unhandled-exception events, timeouts, `resume` and `yield`, the current context of a
job). Every step of C4 runs them unchanged; a step that changes one of these orders on purpose
changes its test in the same pull request and adds a DEVIATIONS row.

| # | Guarantee | Test |
|---|---|---|
| G1 | Jobs of one priority run in the order they were queued, whatever queued them (`post`, `post_local`, `invoke_async`, `invoke_async_local`); nothing queued runs inside the call that queues it | `jobs_of_one_priority_run_in_the_order_they_were_queued_whatever_queued_them` |
| G2 | The highest priority runs first, also for jobs queued while the queue runs; `invoke_async` at `Send` is queued, not inline | `the_highest_priority_runs_first_also_for_jobs_queued_while_the_queue_runs` |
| G3 | Work from other threads joins the same queue in the order of arrival | `work_queued_from_another_thread_joins_the_same_queue_in_the_order_of_arrival` |
| G4 | A synchronous `invoke` from another thread is a `Send` job that waits for the dispatcher thread | `a_synchronous_invoke_from_another_thread_is_queued_at_send_priority_and_waits_for_the_queue` |
| G5 | A wait on the dispatcher thread runs only the jobs ahead of the operation | `waiting_on_the_dispatcher_thread_runs_only_the_jobs_ahead_of_the_operation` |
| G6 | A task awaiting an operation resumes in a job queued at its priority when the operation completes, behind what was queued before | `a_task_awaiting_an_operation_resumes_in_a_job_queued_at_its_priority_when_the_operation_completes` |
| G7 | Tasks woken together resume in the order they awaited, each at its priority, with the current context of that priority | `tasks_woken_together_resume_in_the_order_they_awaited_each_at_its_own_priority` |
| G8 | `resume_with_priority` changes the priority of one continuation; later wakes return to the task's priority | `a_resumption_at_another_priority_changes_the_priority_of_one_continuation_only` |
| G9 | A completion runs `continue_with` continuations first, then the awaiting futures in the order they awaited; a `ferroui-system` task resumes inside the completing call, a dispatcher task in a queued job | `a_completion_source_runs_its_continuations_and_system_tasks_inline_and_queues_dispatcher_tasks`, `a_completion_runs_the_continuations_then_resumes_the_awaiting_futures_inside_the_call` |
| G10 | A nested frame runs the queue in priority order, background work included, before the job that pushed it continues | `a_nested_frame_runs_the_queue_in_priority_order_before_the_job_that_pushed_it_continues` |
| G11 | Work queued while processing is disabled waits and keeps its order; a pumping wait panics | `work_queued_while_processing_is_disabled_waits_and_keeps_its_order` |
| G12 | The platform signal runs jobs while processing is disabled (F5) | `a_platform_signal_runs_jobs_while_processing_is_disabled` |
| G13 | A due timer runs in the place where it was started among the jobs of its priority; due timers are promoted after every job | `a_due_timer_runs_in_the_place_where_it_was_started_among_the_jobs_of_its_priority`, `a_timer_that_becomes_due_during_a_job_runs_before_the_next_job_of_its_priority` |
| G14 | Canceling a queued operation removes it and keeps the order of the rest; its `aborted` event runs on the cancelling thread, its awaiter resumes in its own job | `a_canceled_operation_leaves_the_queue_and_its_awaiter_resumes_in_its_own_job` |
| G15 | An unhandled panic of a posted job leaves `run_jobs`, and the rest of the queue stays in order | `an_unhandled_panic_of_a_posted_job_leaves_the_rest_of_the_queue_in_order` |
| G16 | A panic of an awaited `invoke_async` operation faults the awaiting task in its own job; the queue goes on | `a_panic_of_an_awaited_operation_faults_the_awaiting_task_in_its_own_job` |
| G17 | The `completed` handlers of an operation run on the dispatcher thread, in the order they were added (from any thread), after its wakers | `the_completed_handlers_of_an_operation_run_on_the_dispatcher_thread_in_order_after_its_wakers` |
| G18 | A cancellation runs its callbacks once, in the order of registration, on the cancelling thread; a late registration runs at once | `a_cancellation_runs_its_callbacks_once_in_registration_order_on_the_cancelling_thread` |
| G19 | Callbacks that other threads send to a thread (`register_local`) run there in the order they arrived | `callbacks_that_other_threads_send_to_a_thread_run_in_the_order_they_arrived` |
| G20 | A future awaiting a dropped `TaskCompletionSource` stays pending (F6) | `a_future_awaiting_a_dropped_completion_source_stays_pending` |

## 3. The target

### 3.1 One executor

The task table, the wakers and the poll loop live once, in `ferroui-system` (`ferroui_system::task`,
named after `std::task`), with the reentrancy handling of today's local executor (a wake during a
poll repolls after it, never inside it) and the panic handling of the dispatcher's (a panic faults
the task). Where a woken task is polled is the task's **schedule**, fixed when it is spawned:

```rust
/// Where the polls of a task run: called from any thread, it queues `poll` to run on the task's
/// thread, in order with the other work of that thread.
pub trait Schedule: Send + Sync + 'static {
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>);
}

pub fn spawn_local<F: Future + 'static>(future: F) -> JoinHandle<F::Output>;      // the thread's schedule
pub fn spawn_local_on<F: Future + 'static>(future: F, schedule: Arc<dyn Schedule>, first_poll: FirstPoll) -> JoinHandle<F::Output>;
pub fn set_thread_schedule(schedule: Option<Arc<dyn Schedule>>);                   // replaces set_thread_poster
pub fn block_on<F: Future + 'static>(future: F) -> F::Output;                      // threads without a dispatcher
```

- **The dispatcher** is a schedule per priority: `schedule` is `post(poll, priority)`, which is
  what `TaskWaker` does today. `Dispatcher::spawn_local`, `spawn_local_with_priority` and `spawn`
  (a `Send` constructor of a future that need not be `Send`, which is what `invoke_async_task`
  does) spawn on it; the dispatcher keeps the ids of its tasks to abort them at shutdown.
  `DispatcherLocal::tasks` goes away.
- **A thread without a dispatcher** (a console program, a test, a worker) has the queue of
  `ferroui-system`: a same-thread wake polls at once (today's rule), a wake from another thread
  waits for `run_pending` or `block_on`.
- **The thread's schedule.** The dispatcher installs itself as the schedule of its thread
  (`set_thread_schedule`, which replaces `set_thread_poster` and `set_synchronization_context`),
  at the priority of the current job, `Default` outside of a job. `spawn_local` on a dispatcher
  thread then queues its polls like the dispatcher's own tasks; which code keeps the inline rule is
  step A5 (F3).

### 3.2 `Task<T>` and `DispatcherTask<T>`: a future and a handle

`async` code is a plain future; spawning it gives a handle:

- `JoinHandle<T>`: `impl Future<Output = Result<T, JoinError>>`, `is_finished`, `abort` (drops the
  future; the outcome is `Canceled`). Dropping the handle detaches the task, which keeps running:
  today's behaviour of a dropped `DispatcherTask` or `Task`. The handle crosses threads as
  `DispatcherTask` does; a result that is not `Send` is read on its thread.
- `JoinError`: `Canceled` (aborted, never started, or its dispatcher shut down) and
  `Panicked(Box<dyn Any + Send>)` (`into_panic`, `resume_unwind`), with `std::error::Error`. A
  panic is a value here; the deprecated `DispatcherTask` keeps raising it again when observed.
- `Shared<T>` (`JoinHandle::shared`, `T: Clone`): several observers, the role of `Task<T>` of
  `ferroui-system` in the reactive libraries, with `on_finished(callback)` for the synchronous
  continuations of Rx (`ContinueWith(ExecuteSynchronously)`), which run before the awaiting
  futures are woken (G9).

The outcome of a future that can fail is part of `T` (`Result<T, ObservableError>` for the Rx
forms); a future that returns `Err` of the cancellation error (section 3.6) is how Rx code stops
on a token, as `canceled()` does today.

`DispatcherOperation<R>` stays: it is a queued job with a priority, an abort and events, not a
future that is polled, and it already is a `Future` of its result. `DispatcherFrame` stays.

### 3.3 `TaskCompletionSource` to a one-shot channel

```rust
pub mod oneshot {
    pub fn channel<T>() -> (Sender<T>, Receiver<T>);
    // Sender::send(self, T) -> Result<(), T>; dropping an unsent sender cancels the receiver.
    // Receiver: Future<Output = Result<T, Canceled>>, try_recv, shared() for several waiters.
}
```

One type for the user-interface thread and for other threads (`Send` when `T` is): the waker of
the awaiting task decides where the continuation runs, so the channel needs no thread affinity.
`set_result` is `send(value)`, `set_exception` is `send(Err(error))` on a channel of a `Result`,
`set_canceled` is dropping the sender, `try_set_*` is `send` that reports a gone receiver. The ten
local copies (section 1.2) move to it. A dropped sender cancels its receiver (F6): a behaviour
change for the code that moves, recorded where a copy did not already do so.

### 3.4 `SynchronizationContext` to spawning on the dispatcher

The trait of `ferroui-system` had two jobs, and both move:

- **Where a wake goes:** the `Schedule` of a task (section 3.1). The thread poster and the
  current context of `ferroui-system` become the thread's schedule.
- **Where posted work runs** (`post`, `post_local`): `Dispatcher::post` and `post_local` on a user
  interface thread; for code that does not know the dispatcher (the reactive libraries), the
  thread's schedule, which runs a closure the way it runs a poll. `send` and `send_local` (blocking)
  are not part of it: blocking on the dispatcher stays `Dispatcher::invoke`, which is upstream API.
  `operation_started` and `operation_completed` have no implementation that does anything and go.

`FerroSynchronizationContext` keeps one role, the priority of the running job, which becomes
`Dispatcher::current_priority()` (`None` outside of a job); `RestoreContext` becomes internal.
`DispatcherTaskScheduler` is replaced by the `spawn` methods of the dispatcher.

### 3.5 How `await` keeps the dispatcher's priority and order

These rules are the contract of the executor, and the tests of section 2 check them:

1. A task of a dispatcher has a priority, fixed when it is spawned: the given one, or the one of
   the job that spawns it (G7).
2. A wake, from any thread, queues one poll at that priority, at the end of its chain; wakes of the
   same priority resume in the order they came (G6, G7, G14).
3. A continuation never runs inside the call that completed what it awaited (G6, G9 for dispatcher
   tasks); `Shared::on_finished` callbacks do, before the wakes (G9).
4. `resume_with_priority`, `yield_now` and `await_with_priority` move one continuation to another
   priority (G8), as today.
5. While a task is polled, the current priority is the one of the job that polls it (G7, G8).
6. A first poll inside the spawning job (`invoke_async_task`, `start_local`) stays available as
   `FirstPoll::Now`, for the `async void` methods of upstream that run up to their first await
   inside the call (DEVIATIONS, the rows of the D-Bus and X11 code).

### 3.6 Nested frames and `DispatcherProcessingDisabled`

- A poll is a job, so a nested frame or a pumping wait polls the tasks whose polls are queued, in
  priority order like any job (G10, G5).
- A task is never polled inside its own poll: a poll job that finds its task being polled marks it,
  and the running poll schedules it again when it returns (F1).
- While processing is disabled, polls are queued and not run by `run_jobs` or `push_frame`, which
  refuse (G11); the platform signal runs them (G12, F5, unchanged unless the owner decides).
- Waiting for a `JoinHandle` synchronously on the dispatcher thread panics, as waiting for a
  `DispatcherTask` does today; on another thread it blocks.

### 3.7 One cancellation type

One `CancellationToken` in `ferroui-system`, in the shape of `tokio_util::sync::CancellationToken`
(the ecosystem's type of the same name), implemented in the crate (no `tokio` dependency):

| Today | Target |
|---|---|
| `CancellationTokenSource::new()`, `.token()` | `CancellationToken::new()`, `.clone()` |
| `source.cancel()` | `token.cancel()` |
| `is_cancellation_requested()` | `is_cancelled()` |
| `CancellationToken::none()`, `can_be_canceled()` | `CancellationToken::none()`, `can_be_cancelled()` |
| `register(callback) -> CancellationTokenRegistration` (dropping keeps it) | `register(callback) -> CancellationGuard`: `#[must_use]`, `Send`, dropping it unregisters, `detach()` keeps it (the shape of `Subscription`, C3b) |
| `register_local(callback)` | `register_local(callback)`, the same guard; `animation::cancellation` uses it |
| a future of the cancellation (`CancellationFlag` of `ferroui-freedesktop`, `AiCancellationToken`) | `token.cancelled()` |
| linked sources | `child_token()` |
| `CancelOnDrop` of `ferroui-headless`, `LoopCancellation` of `ferroui-native` | `drop_guard()`, the token |
| `OperationCanceledError`, `TaskCanceledError` | one error type, `ferroui_system::task::Canceled` (`std::error::Error`); `JoinError::Canceled` and `oneshot::Canceled` are it |

The callbacks keep running synchronously on the cancelling thread, once, in the order of
registration (G18; F4 is recorded in DEVIATIONS). `IControlledDispatcherImpl::run_loop` keeps
taking the token. `CancellationDisposable` and `ICancelable` (Rx) wrap the new token.

### 3.8 The reactive libraries and `FerroUI.View`

- **Rx schedulers** (`IScheduler`, `CurrentThreadScheduler`, `SynchronizationContextScheduler`,
  the virtual-time schedulers) keep their names and contracts (upstream API). Their work is not a
  future; only `SynchronizationContextScheduler`, `ContextDisposable`, `ObserveOn` and
  `SubscribeOn` of a context, `FromEvent` and `AsyncSubject` (which capture the current context)
  and `ScheduleAsync`, `Sleep` and `Yield` take the thread's schedule instead of a
  `SynchronizationContext`.
- **The `Task` forms of Rx** (`from_async`, `start_async`, `to_task`, `run_async`, `get_awaiter`,
  `for_each_async`, the task overloads of `select_many`, `create`, `defer`, `using`, `merge`,
  `concat`, `switch`) keep their names; they take futures and give `Shared<T>`.
- **`FerroScheduler`** of `ferroui-reactiveui-controls` already posts to the dispatcher; its
  `install_on_ui_thread` installs the dispatcher as the thread's schedule.
- **The services of the dock's ReactiveUI layer** (`ServiceDispatcher`, `MainThreadDispatcher` and
  the overlay services, 29 uses of the context) move to the dispatcher's `spawn` and `post`.
- **`FerroUI.View`**: `use_future` spawns with `ferroui-system`'s `spawn_local` and abandons a
  stale future with a flag. It moves to `spawn_local` on the dispatcher, abandons with
  `JoinHandle::abort`, and writes its signal in the dispatcher job of the task's priority rather
  than inside the waking call: an order change of that step, with its test.

### 3.9 What keeps its name

The dispatcher and its methods (`Dispatcher`, `invoke`, `invoke_async`, `post`, `run_jobs`,
`push_frame`, `disable_processing`, `DispatcherPriority`, `DispatcherOperation`,
`DispatcherFrame`, `DispatcherTimer`): upstream API of the framework. The `..._async` methods of
the controls (`NavigationPage::push_async`, ...). The Rx, DynamicData and ReactiveUI names.

## 4. Deprecation policy for 0.5

As [idiomatic-rust.md](idiomatic-rust.md), section 3, has it: the old types remain one release.

- `Task<T>`, `TaskCompletionSource<T>`, `TaskStatus`, `ContinuationId`, `TaskCanceledError`,
  `threading::tasks::{spawn_local, block_on, run_pending_wakes, set_thread_poster, ThreadPoster}`,
  the `SynchronizationContext` trait with `current_synchronization_context` and
  `set_synchronization_context`, `CancellationTokenSource`, `CancellationTokenRegistration`,
  `LocalCancellationTokenRegistration` (`ferroui-system`); `DispatcherTask<T>`,
  `DispatcherTaskScheduler`, `FerroSynchronizationContext`, `RestoreContext`, `invoke_async_task*`,
  `to_task_scheduler*`, `OperationCanceledError` (`ferroui-base`): `#[deprecated(since = "0.5.0")]`
  wrappers over the new types, with the behaviour they have today (a `DispatcherTask` raises the
  panic of its future again; a `TaskCompletionSource` that is dropped leaves its task pending;
  `register` returns a registration that dropping keeps), and `From` conversions to the new types.
- The public signatures that return `DispatcherTask` (`NavigationPage::*_async`,
  `Window::show_dialog`, the behaviors' `TaskHandle`) return `JoinHandle` in A6; their old form
  stays as a deprecated `..._task` variant where an application may depend on it.
- Each step adds its rows to `docs/porting/data/upstream-names.toml`
  (`System.Threading.Tasks.Task` to `ferroui_system::task::JoinHandle`, ...) and its migration
  notes to `CHANGELOG.md`.

## 5. Steps

Each step is one pull request from main, built and tested on its own; A2, A4 and A6 split by area
where they are large.

| Step | Content | Public API | Risk |
|---|---|---|---|
| A0 (done, #498) | This plan; the ordering tests of section 2 | unchanged | none |
| A1 (done) | The executor core in `ferroui-system` (`task::{spawn_local_on, JoinHandle, JoinError, Shared, Schedule, block_on}`), with its tests; `threading::tasks` (`Task`, `spawn_local`, `block_on`, `run`) on it with today's inline rule; a panic faults the task (F2) | additions | medium: every task of the reactive libraries; their ported tests and G9, G19, G20 guard it |
| A2 | The dispatcher on the core: its schedule per priority, `Dispatcher::spawn_local`, `spawn_local_with_priority` and `spawn`; `DispatcherTask`, `DispatcherTaskScheduler` and `invoke_async_task*` as wrappers over them; `resume`, `yield_now`, `await_with_priority` on the core; the lost wake (F1) fixed, with a test | additions | high: every asynchronous step of the user interface; G1 to G17 unchanged |
| A3 | `task::oneshot`; the ten local completions move to it, by crate (X11, remote protocol, dialogs, headless, iOS, Android, FreeDesktop, WinUI) | additions; `ferroui-x11`'s public `TaskCompletionSource` deprecated | low per crate; a dropped sender now cancels (F6, a DEVIATIONS row where the copy did not) |
| A4 | One cancellation type: `CancellationToken` reshaped (section 3.7), `CancellationGuard`, `cancelled()`, `child_token()`, `drop_guard()`; `task::Canceled` for `OperationCanceledError` and `TaskCanceledError`; the local cancellation types replaced; F4 recorded | 0.5: the old names deprecated | medium: 602 uses; registrations that were dropped get `detach()` (the compiler lists them, as in D4) |
| A5 | `SynchronizationContext` to the thread's schedule: `set_thread_schedule`, `Dispatcher::current_priority`, `FerroScheduler::install_on_ui_thread`, the Rx forms that capture a context, the dock services; the rule for the tasks of the reactive libraries on a dispatcher thread decided (F3) and its order tested | 0.5: the trait, `FerroSynchronizationContext` and `RestoreContext` deprecated | high where F3 changes an order; the Rx, ReactiveUI and dock tests guard it |
| A6 | The asynchronous public APIs of the controls and platforms return `JoinHandle` or `impl Future` (navigation, dialogs, the behaviors' `TaskHandle`, the callers of `invoke_async_task*` and `to_task_scheduler`), by area | 0.5 | medium: callers visited; deprecated `..._task` variants where needed |
| A7 | The `Task` forms of Rx, DynamicData and ReactiveUI on futures and `Shared<T>` (section 3.8) | 0.5 (names kept) | medium: the ported upstream tests drive them through completion sources |
| A8 | `FerroUI.View`'s async data on the dispatcher, abandoned with `abort` | behaviour: the order of the signal write (a test) | low |
| A9 | The inventory (`CancellationToken` becomes the target; `Task`, `TaskCompletionSource`, `SynchronizationContext`, `CancellationTokenSource` counted until removed), `upstream-names.toml`, the migration notes, docs/porting/reactive.md section 5.8 and PORTING-GUIDE | | |

Status: A0 (this page and the tests) is done (#498). A1 is done: the executor core is
`ferroui_system::task` (`executor.rs`: the task table, the poll loop, the queue of a thread and the
callbacks that other threads send to it; `join.rs`: `JoinHandle`, `JoinError`; `shared.rs`:
`Shared`; `schedule.rs`: `Schedule`, `FirstPoll`), with its tests in
`src/FerroUI.System/tests/executor_tests.rs`. `threading::tasks` runs on it: `spawn_local` spawns
a task in the queue of the thread whose future completes the `Task` (a panic faults it with a
`TaskWorkError`, F2), `block_on` is `task::block_on`, `run_pending_wakes` is `task::run_pending`,
and `set_thread_poster` sets the thread's schedule; `Task`, `TaskCompletionSource` and their
continuations keep their own state and behaviour, and nothing is deprecated yet (section 4: the
old types become deprecated wrappers with the steps that replace their uses).

## 6. Not in scope

- The dispatcher's queue, priorities, timers and platform implementations: C4 changes how futures
  are polled from it, not how it runs jobs.
- Threads and thread pools (`Task.Run` stays `ferroui_system`'s `run`, a thread per call, until a
  need is shown), and `async` runtimes: no `tokio`, `async-std` or `smol` in the workspace.
- The `block_on` of `ferroui-vello` (a poll loop over `wgpu` futures that are complete when asked)
  and the blocking transports of the remote protocol.
- The names of the Rx, DynamicData and ReactiveUI API, and `DispatcherOperation`'s events.
