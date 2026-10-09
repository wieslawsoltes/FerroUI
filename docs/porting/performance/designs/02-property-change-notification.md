# 02. Cheaper property change notifications

Status: the hypotheses are verified against the code, upstream and the counters of design 09 (see
"Hypotheses, verified"). Two of the three are contradicted by the code and dropped, and the figures
contradict the premise of the fast path; none of the four steps of the design is implemented as it
was written. What the reading left of the fixed cost is the port's own machinery in two places, and
both are changed: the dispatch of the overrides (design 01, implemented for this path) and a copy of
the value in the effective value (see "What is implemented"). Written without a build: the session
that validates builds, runs the suites and fills the "measured" column of "Counters".

## Finding

- **[M]** `raise_property_changed` and what it calls account for 118 ms of the scrolling profile
  (21 %); the `on_property_changed` overrides alone for 89 ms.
- **[M]** The value store and the effective values have 56 ms of self time (10 %).
- **[M]** Clearing the cells of recycled rows costs 48 ms, of which 43 ms are one `clear_value`
  per cell that raises a change which a template binding publishes onward.

## Cause

Every effective value change builds a `FerroPropertyChangedEventArgs`, calls
`on_property_changed_core` through the virtual table (one call per class level, see design 01, each
level comparing the property against the ones it handles), then `property.notify_changed`, then
the instance listeners from a snapshot of the listener list (`ferro_object.rs`). The managed
original does the same steps, but each is a few instructions there. Here the fixed cost per change
is high, and a recycled row raises hundreds of changes, most of which nobody listens to.

**[H]** Three parts of the fixed cost can go without changing what is notified:

1. The snapshot of the instance listeners is taken even when the list has one entry; the class
   handlers of a property are looked up per change.
2. Each class level of `on_property_changed` compares the changed property with a list of
   properties one by one (property identity is a pointer or an id comparison per candidate).
3. The event arguments carry boxed or dynamically typed old and new values even when no listener
   reads them.

## What must not change

The set of notifications, their order (class override first from the most derived class, then the
property's class handlers, then instance listeners in subscription order), re-entrancy (a listener
that sets a property during a notification), and the values a listener sees.

## Design

Measure first: count, per scrolled row, the changes raised, the changes with at least one listener
beyond the class overrides, and the time in each of the three parts above. Then, in the order the
counts justify:

1. **No-listener fast path.** A per-class bit set, built at class registration, of the property
   ids that any class level handles in `on_property_changed`. A change of a property outside the
   set skips the virtual chain. Together with an empty class-handler list and an empty listener
   list, such a change costs a few loads.
2. **Dispatch by property id.** Where a class handles many properties, a `match` on the property id
   (a dense integer) instead of a chain of comparisons. This is a mechanical rewrite of each
   override; the bodies stay.
3. **Listener iteration without a snapshot** when the list is not modified during the call (a
   generation counter detects modification and falls back to the snapshot).
4. **Lazy old and new values** in the event arguments, typed where the caller is typed.

## Hypotheses, verified

Against `FerroObject::raise_property_changed`, the virtual members of `FerroObject` and their
overrides, `PropertyChangedObservable::notify`, `HandlerList`, `FerroPropertyChangedEventArgs`,
`EffectiveValue` and `ValueStore` of the port; `AvaloniaObject.RaisePropertyChanged`,
`OnPropertyChangedCore`, `AvaloniaProperty.NotifyChanged`, `LightweightObservableBase.PublishNext`,
`EffectiveValue<T>.SetAndRaiseCore` and the `OnPropertyChanged` overrides of upstream; and the
counters per recycled row of design 09 (20 pixels per step).

**The sequence is upstream's, statement by statement.** `RaisePropertyChanged` builds the arguments,
calls `OnPropertyChangedCore`, and for a change of the effective value calls
`property.NotifyChanged`, then the `PropertyChanged` handlers of the object.
`raise_property_changed` is the same four statements in the same order. The port has no
`INotifyPropertyChanged` event on the object, which upstream raises last. Neither side has a batch
update any more: a change is raised when the effective value is set.

**The figures. [M]** Per recycled row 459 changes are raised, 439 of them of an effective value (the
other 20 are changes of a base value, which reach `on_property_changed_core` and nothing else). Of
the 439, 216 (49 %) have at least one subscriber of the property and 421 (96 %) have at least one
handler on the object. So the premise of the cause, "most of which nobody listens to", does not
hold for this scenario: 18 changes per row have no handler on the object, and fewer have neither
kind. The counters do not say how many handlers a change calls, nor what they do; nearly all of
them belong to bindings (design 05), which subscribe to the object as upstream's do.

**Hypothesis 1, the snapshot of the listeners and the lookup of the class handlers: contradicted by
the code, dropped.** `HandlerList` (`utilities/handler_list.rs`) keeps its handlers in a shared,
immutable list; `snapshot()` is one more reference to that list and copies nothing, whatever its
length. The list is copied when a handler is added or removed while a notification still holds the
old one, which is the only time a copy is needed. The handlers of a property are not looked up:
`FerroProperty::changed` is a field of the property, and `notify` tests it for emptiness and walks
it. Upstream copies more than the port here: `PublishNext` reads up to three observers into locals
under a lock and rents an array for more, and `_propertyChanged?.Invoke` walks the invocation list
of an immutable delegate. Step 3 has nothing to remove.

**Hypothesis 2, the comparisons in each override: confirmed as a description, and the remedy does
not exist.** Each override calls its base and then compares `change.property()` with the properties
it handles, one after the other, as the upstream override does with its static fields; a comparison
is the read of the accessor's cell and of two IDs. Where upstream switches on the name of the
property (`TextBlock`, `ContentPresenter`, `Border`), the port matches on the name. A `match` on the
ID (step 2) cannot be written: an ID is assigned when the property is registered, at run time and in
the order of registration (`NEXT_ID` in `ferro_property.rs`), so it is not a constant a `match` can
name. A table from ID to handler per class would be a registry upstream does not have, which is
logic, not machinery. Dropped.

**Hypothesis 3, boxed values in the arguments: contradicted by the code, dropped.**
`FerroPropertyChangedEventArgs` is six fields on the stack; the old and the new value are borrowed
(`&dyn Any`), and nothing is boxed or copied unless a handler asks for it (`get_new_value` copies
the value it reads; `to_owned_args`, for a handler attached from markup, boxes both). Upstream
allocates an object per change. Step 4 has nothing to remove.

**Step 1, the fast path: not allowed, and the figures leave it nothing.** An override is arbitrary
code: the core override of `Animatable` acts on any property that has a transition, the override of
`TextBlock` matches by name, and a class of another crate can override either member. Skipping the
call for "a property no level handles" needs every override to declare its properties, and a wrong
or missing declaration drops a notification silently, where upstream always calls
`OnPropertyChangedCore`. That is different logic and is not implemented. By the figures it would
also apply to the 4 % of changes without a handler on the object, at most.

**Where the fixed cost is, then.** Not in the lists and not in the arguments, but in two pieces of
the port's own machinery:

- **The dispatch of the two overrides.** `on_property_changed_core` is overridden by `Animatable`
  alone (upstream seals it there), and its slot in the table of an element was a function of the
  element's class that forwarded to the table of the base class, which forwarded again: for a text
  block seven such functions (`TextBlock`, `Control`, `InputElement`, `Interactive`, `Layoutable`,
  `Visual`, `StyledElement`) before the override of `Animatable` ran, each of them a read of a
  lazily built table and an indirect call, for each of the 459 changes of a row. **[E]** About
  3,200 forwarding calls per recycled row for this member alone (459 times at least 7; a cell of
  the table is nine classes below `Animatable`), and two more per effective change in the base
  calls of `on_property_changed` (through `Interactive` and `Animatable`, which do not override
  it). This is the finding of design 01 on the path of this design; see "What is implemented".
- **Copies of the value.** `EffectiveValue::set_and_raise_core` copied the value it was given
  before comparing it, and dropped the original unread, where upstream assigns a reference
  (`var v = value`). For a text that is an allocation and a release per set.

## What is implemented

Two changes of machinery. Neither alters a statement of the ported logic, and no notification, its
order, its arguments or its behaviour under re-entrancy changes.

### The overrides are reached without forwarding functions (design 01)

Described in [design 01](01-virtual-dispatch.md), which this design shares the path with: the table
of a class takes the slot of its base class for a member the class states it does not override. For
this design it is applied to the root class members (`constructed`, `on_property_changed_core`,
`on_property_changed`, `update_data_validation`):

- Files: `src/FerroUI.Base/ferro_object.rs` (`FerroObjectImpl::__OVERRIDES`, `build_vtable` of the
  root class), `src/FerroUI.Base/type_system.rs` (`__forwards_to_parent`, `ferro_impl_classes!`,
  `ferro_overrides!`); the implementations of `FerroObjectImpl` of `Animatable`, `StyledElement`,
  `Visual`, `Layoutable`, `InputElement`, `Control`, `ContentControl`, `TextBlock`,
  `ContentPresenter`, `Border` and `Panel`, which now state what they override; 59 empty
  implementations of `FerroObjectImpl` of the base and the controls crate (`TemplatedControl` and
  `Decorator` among them), which now state that they override nothing.
- Machinery: for an element, the slot of `on_property_changed_core` is the implementation of
  `Animatable` itself, and the base call of an `on_property_changed` override whose base class does
  not override it reaches the next override directly.
- Upstream statements preserved: `OnPropertyChangedCore(e)` is called once per change, on the most
  derived override (`Animatable`'s, which calls `base.OnPropertyChangedCore`, which calls
  `OnPropertyChanged` for a change of the effective value); each `OnPropertyChanged` override calls
  its base first and then compares the property, from `StyledElement` up to the class of the
  object, in the order the overrides are written.
- Why behaviour is identical: a member a class does not override is the default of the
  implementation trait, which calls the same slot of the table of the base class and does nothing
  else. Putting that slot into the table of the class reaches the same function with the same
  arguments; the functions that ran before run now, in the same order, and the ones removed from
  the path did nothing but pass the call on. The public function of a virtual member, where the
  counters count, is untouched.

### One copy of the value less per set

- File: `src/FerroUI.Base/property_store/effective_value.rs`, `EffectiveValue::set_and_raise_core`.
- Machinery: without a coercion the value given is itself the value compared and stored (it was
  copied, and the original dropped at the end of the function without being read). With a coercion
  the value given is kept, as before, for the two places that store it uncoerced.
- Upstream statements preserved (`SetAndRaiseCore`): `var oldValue = Value`; the two flags set;
  `v = coerce(owner, value)` where there is a coercion and the default is not being coerced; where
  the priority allows, the comparison with the current value, `Value = v`, the priority, the
  uncoerced value; the same for the base value; then `NotifyValueChanged` or
  `NotifyBaseValueChanged`.
- Why behaviour is identical: the comparisons are made with an equal value, the same values are
  stored in the same order, and the coercion is called at the same point with a copy of the value
  as before. What differs is that one copy of the value is not made and not dropped. A value type
  whose copy or drop runs code would see one of each less per set; the framework has no such type,
  and the copies of a value are not something upstream defines (see "Doubts").

## What is not implemented, and why

- **Step 1**, the fast path: different logic; see above.
- **Step 2**, the `match` on the ID: IDs are not constants; see above.
- **Step 3**, iteration without a snapshot: the snapshot copies nothing; see above.
- **Step 4**, lazy values: the values are borrowed already; see above.

### Left for a profile

Not changed; each would be machinery only, and each has a reason to wait.

- `ValueStore::reevaluate_effective_values` copies the list of effective values, with a handle of
  each, to end the reevaluation and remove the unset ones, where upstream walks its list by index
  from the end. `end_reevaluation` and `dispose_and_raise_unset` raise changes, and a handler can
  add or remove effective values: a walk by index visits a list that changed under it differently
  from a walk over a copy. The two agree when no handler touches the store; when one does, the
  port's present behaviour is the copy's. Making it upstream's is a change of behaviour under
  re-entrancy that needs a test of what upstream does there first.
- `EffectiveValue::notify_value_changed` copies the new value for the notification. It has to: the
  arguments borrow the value, and a handler may set the property while they are alive.
- `EffectiveValue::raise_inherited_value_changed` copies the old and the new value before comparing
  them (design 03, "Left for a profile").
- The base call of an override (`parent_on_property_changed`) reads the table of the base class
  (a lazily initialised static) and calls through it, where upstream's `base.OnPropertyChanged` is
  a direct call. With the slots resolved it now reaches the next override in one call. A direct
  call would need the implementation trait to state that the base class implements it; not tried
  without a build.
- `raise_property_changed` and `PropertyChangedObservable::notify` borrow the cell of the list
  twice (emptiness, then the snapshot).
- The handlers a change calls are not counted. A counter of handlers called per change (property
  subscribers and object handlers apart) would say how much of the 118 ms is the handlers
  themselves; most belong to bindings (design 05).

## Expected gain

To be measured. The 5 to 10 % **[E]** of the design came from the four steps, none of which
survived. **[E]** What is implemented removes about 3,200 forwarding calls and one copy of a value
per set, per recycled row; the work of the overrides and of the handlers that do run stays, and it
is the larger part of the 118 ms (the overrides alone had 89 ms, most of it in what they call).

## Risks

Medium, as the design said, for the dispatch: it is the base of everything. It is held by the
tests of the table (design 01) and by the sequence tests below. Low for the copy.

## Verification

- The property system and binding suites of `ferroui-base`; the control suites. All four tests of
  upstream's `AvaloniaObjectTests_OnPropertyChanged.cs` were already ported
  (`tests/ferro_object_tests_on_property_changed.rs`), as were the tests of the `Changed` observable
  of `AvaloniaPropertyTests.cs` (`tests/ferro_property_tests.rs`) and the 19 of
  `AvaloniaObjectTests_Coercion.cs`; `tests/ferro_object_tests_reentrancy.rs` holds re-entrancy.
- New, in `tests/ferro_object_tests_notification_order.rs` (they hold before the changes as well;
  each records a whole sequence, the "recorded notification sequence" the design asked for):
  - `change_is_raised_to_the_overrides_then_the_property_subscribers_then_the_object_handlers`:
    the core override around the overrides of the derived and the base class, then class handlers
    and subscribers of the property in the order they subscribed, then the handlers of the object.
  - `class_handler_is_called_for_objects_of_its_class_only`.
  - `change_of_a_base_value_is_raised_to_the_core_override_only`.
  - `change_made_by_a_property_subscriber_is_raised_completely_before_the_next_subscriber`: a
    handler that sets another property.
  - `change_of_the_same_property_made_by_a_handler_reaches_later_handlers_first`: a handler that
    sets the same property; the outer change keeps its values.
  - `property_subscribers_called_for_a_change_are_the_ones_there_were_when_it_began` and
    `object_handlers_called_for_a_change_are_the_ones_there_were_when_it_began`: a handler that adds
    one and removes the next during the notification (the removed one is still called for this
    change, the added one is not), as upstream's copy of the observers and its immutable delegate
    behave.
  - `handler_added_inside_a_nested_change_is_called_for_neither_change`.
  - `inherited_change_is_raised_completely_on_an_object_before_its_children` and
    `inherited_change_stops_at_an_object_that_sets_the_property`.
- New, in `tests/virtual_dispatch_tests.rs` (design 01):
  `property_change_reaches_the_overrides_through_every_form_of_class`.
- `scroll-profile.mjs` and the start-up measurement (`first-frame.mjs`), since start-up raises the
  most changes.

## Counters: before, expected after

Per recycled row, 20 pixels per step. "Before" is the table of design 09, the allocations as
measured after designs 03 and 04. No counter of design 09 is expected to move: the counters of this
design count what the framework does, and the virtual calls are counted in the public function of
a member, once per call, not in the forwarding functions. One that moves is a change of logic and a
defect of this work.

| Counter | Before | Expected after | Measured after |
|---|---:|---|---|
| property changes raised | 459.24 | 459.24 | to be measured |
| property changes of an effective value | 439.24 | 439.24 | to be measured |
| property changes with handlers of the property | 215.51 | 215.51 | to be measured |
| property changes with listeners of the object | 421.27 | 421.27 | to be measured |
| virtual calls (all members) | 2835.50 | 2835.50 | to be measured |
| `FerroObject::on_property_changed_core` | 459.24 | 459.24 | to be measured |
| `FerroObject::on_property_changed` | 439.24 | 439.24 | to be measured |
| binding values published | 86.42 | 86.42 | to be measured |
| every other counter of design 09 | as in its table | unchanged | to be measured |
| allocations | 1175.0 | lower or equal **[E]**: one copy less per set of a value, which is an allocation only for a value that allocates when copied (a text) | to be measured |
| bytes | 104865 | lower or equal **[E]** | to be measured |
| layout microseconds (`release`) | 411.3 (before designs 03 and 04; not measured since) | lower | to be measured |

### Commands

The suites:

```sh
cargo test -p ferroui-base --lib
cargo test -p ferroui-base --features perf-counters --lib perf_counters
cargo test -p ferroui-controls --lib
cargo test -p control-catalog --lib
```

The new tests alone:

```sh
cargo test -p ferroui-base --lib ferro_object_tests_notification_order
cargo test -p ferroui-base --lib virtual_dispatch_tests
```

The figures, with the commands of design 09 (the counters, then the allocations, then the times
without a feature), on each commit and on its parent:

```sh
cargo test -p control-catalog --release --features perf-counters --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --profile dist --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

## Doubts

Most likely first.

1. Nothing was built or run. The macros were written against the grammar by hand, and the
   sequences the new tests assert were derived by reading `raise_property_changed`,
   `EffectiveValue::notify_value_changed` and `ValueStore::on_ancestor_inherited_value_changed`.
2. The gain may be small: the overrides and the handlers do the work they did, and a forwarding
   call is a few instructions on the desktop. The browser, where the profile was taken, is where
   it should show.
3. The copy of the value: a property value type whose `Clone` or `Drop` runs code sees one copy and
   one drop less per set. The test type `Droppy` of the re-entrancy tests is such a type and its
   test asserts values, not counts. If a count of copies is ever to be held, this change is the
   one to revert.
4. Whether the walk of `reevaluate_effective_values` should become upstream's walk by index (see
   "Left for a profile"): it needs a decision about behaviour under re-entrancy, not a measurement.

### Measured on 2026-10-09

The whole workspace passes with the changes of designs 01 and 02 together (12080 tests), and the desktop catalog starts and runs. The run with `perf-counters` gives the same value for every one of the 70 counter lines of the benchmark as before: the same notifications, virtual calls by member and everything else. A recycled row costs 5 fewer allocations (1170.0 against 1175.0, from the value no longer copied per set). What these changes are for is processor time (the forwarding functions a virtual call no longer passes through), and that was not measured: the machine was under heavy load, and the time per row moved by more than the expected gain between runs. To be measured on an idle machine and in the browser (`scripts/browser/scroll-profile.mjs`), where the indirect calls cost most.
