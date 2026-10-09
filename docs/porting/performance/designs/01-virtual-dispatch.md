# 01. Virtual tables without forwarding chains

Status: the hypotheses are verified against the code (see "Hypotheses, verified"). Step 1 is
dropped: inlining cannot remove a call through a table. Step 2 is implemented in the form the code
allows: the table of a class takes the slot of its base class for every member the class states it
does not override, and a class states that where its implementation is written (see "What is
implemented"). It is applied to every implementation that overrides nothing and says so, and to the
root class members of the classes on the path of a property change (design 02); the other
implementations with overrides are as they were and can follow (see "What is left"). Written
without a build: the session that validates builds, runs the suites and fills the "measured"
column of "Counters".

## Finding

- **[M]** The closures of the virtual tables (`Subclassable::build_vtable::{closure}`) have 46 ms of
  self time in the scrolling profile, 8 % of the total: the largest single entry.
- **[M]** One virtual call shows up as a chain of frames, one per class between the class of the
  object and the class that implements the member. A `measure_core` call on a text block is nine
  frames deep before the implementation runs.

## Cause

`ferro_class!` (`src/FerroUI.Base/type_system.rs`) fills a slot of the table of class `T` with
`|this, args| T::method(cast_this(this), args)`. When `T` does not override the member, `T::method`
is the default of its implementation trait, which calls the same member of the parent
implementation, which is again a default, and so on up to the class that implements it. Each level
is a separate function. At `opt-level = "z"` they are not inlined, so a call that is one indirect
jump in the managed original is up to nine calls here. **[H]** At level 3 on the desktop the chain
collapses; this needs a look at the generated code, since it decides whether the design matters
outside the browser.

## What must not change

Which implementation a call reaches, and the order of base calls inside an override (an override
that calls its parent before or after its own work keeps doing so). The table layout (`repr(C)`,
the base table first) that `parent_vtable` relies on.

## Design

Resolve the chain when the table is built instead of on every call.

1. Mark the generated trait defaults and the slot closures `#[inline(always)]`, so that the slot of
   a class that does not override a member compiles to the implementation of the nearest ancestor
   that does. This is the smallest change and may be all that is needed.
2. If the first step leaves chains (a default that is not a plain forward cannot be inlined through
   a trait object boundary): have `build_vtable` copy the slot of the parent table for members `T`
   does not override, so that the pointer in the table is the implementation itself. The macro
   needs to know whether `T` overrides a member; an associated constant per member on the
   implementation trait (`const OVERRIDES_MEASURE_CORE: bool`), set by the `impl` macro, gives it
   without specialisation.

## Hypotheses, verified

Against `ferro_class!`, `parent_vtable`, `FerroObject::vtable_of` and the hand-written table of the
root class (`ferro_object.rs`), and the counters of design 09.

**The cause: confirmed, with one correction.** The slot of class `T` is a function that calls
`T::method`; the default of the implementation trait calls `parent_<member>`, which reads the table
of the parent class (`parent_vtable`: `<T::Parent as ObjectType>::vtable()`, a table built on first
use behind a `OnceLock`) and calls the slot there. So a forwarding step is not a plain call of the
parent's function: it is a load of a lazily built table, a test that it is built, and an indirect
call. The chain is therefore one indirect call per class, not one function per class that an
optimiser could merge.

**The [H], the chain collapses at level 3: dropped by the code.** What inlining can merge is the
layers of one step (the slot function, the default, `parent_<member>`, `parent_vtable`), and
`parent_<member>` is `#[inline]` already. It cannot merge two steps: the target of the call at the
end of a step is a pointer read from a table that exists only at run time, which no optimisation
level resolves. The chain is as long on the desktop as in the browser; only the cost of a step
differs. Not confirmed in the generated code, since this session does not build.

**Step 1, `#[inline(always)]`: dropped.** For the reason above it would leave every chain as long
as it is.

**Step 2, whether the macro can know what `T` overrides: only where the implementation says so.**
The design assumed an `impl` macro that sets a constant per member; there is none, and the classes
implement their traits with plain `impl` blocks. Nothing on the side of the trait can tell whether
a default was replaced: the default and the override have the same path, and a constant of the
trait cannot depend on which functions an `impl` contains. So the knowledge has to come from where
the implementation is written, and there are two places where it is exact by construction:

- `ferro_impl_classes!(Class: TraitImpl, ..)` writes implementations with no function at all. It
  was already what most classes use for the traits they override nothing of.
- A macro that wraps an implementation sees its functions and can list them.

A constant that an implementation sets by hand was considered and rejected: a class that overrides
a member and does not say so would have its override skipped silently.

**The counters. [M]** 2836 virtual calls per recycled row, counted at the public function of a
member. They do not count the forwarding functions a call passes through, so they neither confirm
nor move with this design (see "Counters").

## What is implemented

### The mechanism

- Files: `src/FerroUI.Base/type_system.rs` (`__forwards_to_parent`; `ferro_class!`: the constant
  `__OVERRIDES` of a generated implementation trait and the slots of `build_vtable`;
  `ferro_impl_classes!`; the new `ferro_overrides!`), `src/FerroUI.Base/ferro_object.rs` (the same
  for the root class, whose trait and table are written by hand).
- Machinery: every implementation trait has a hidden constant, `__OVERRIDES`, the names of the
  members an implementation overrides where it states them, and `None` where it does not (the
  default, so a plain `impl` is what it was). `ferro_impl_classes!` sets it to the empty list.
  `ferro_overrides! { impl TraitImpl for Class { fn .. } }` writes the implementation it wraps
  and sets the constant to the names of the functions in its block. When the table of a class is
  built, the slot of a member is the slot of the table of the base class if the implementation
  states its overrides and the member is not among them; otherwise it is the forwarding function
  of the class, as before. The decision is a constant function evaluated in a constant block, per
  member and class, at compile time.
- What is preserved, and why behaviour is identical: a member that is not overridden is the default
  of the implementation trait, which calls that same slot of the base table with the same
  arguments and does nothing else, in the traits the macro generates and in the hand-written
  `FerroObjectImpl` alike. Taking the slot gives the call the same destination with one step
  less. Every override that ran before runs now, once, in the same order; `parent_<member>` of an
  override reads the table of its base class as before, and finds there either the base's own
  function or the slot the base took from further up, which is where the base's forwarding
  function would have led. The layout of the tables is unchanged: a slot holds another pointer of
  the same type.
- The class that declares a member keeps its own slot whatever its implementation states: its
  base class has no such slot, and its default panics when it is called
  (`virtual member has no base implementation`), not when the table is built. The test is the one
  `parent_vtable` makes, on the sizes of the tables.
- The list of `ferro_overrides!` is taken from the functions of the block and cannot differ from
  them. A function behind a `#[cfg]` that is off is listed and absent, which keeps the forwarding
  function for it: the slower form, never the wrong one.
- How it is known that every class of the workspace still compiles: the constant has a default, so
  no plain `impl` changes; `ferro_impl_classes!` now writes the constant into the implementations
  it generates, which needs every trait it is given to have it, and a scan of the workspace
  (`src`, `samples`, `tests`, `external`) finds 542 uses naming 40 distinct traits, each of them a
  trait declared by a `virtuals` block of `ferro_class!` or `FerroObjectImpl`; the slot of a member
  is built from the same closure with the type it had as a field. No class is named in the macro.

### Where it applies

- **Every implementation written by `ferro_impl_classes!`**, by the change of the macro alone: 542
  uses, 2,469 implementations of a trait, in every crate.
- **Empty implementations written as `impl TraitImpl for Class {}`** in the base and the controls
  crate now use `ferro_impl_classes!`: 83 in 65 files (`FerroObjectImpl` 59, `StyledElementImpl` 9,
  the others 15). The automation peers (88), the directories other work is under way in (`data`,
  `styling`, `rendering`, `media/text_formatting`, the resources) and the other crates are left;
  they are correct as they are and gain when they are converted.
- **The root class members on the path of a property change** (design 02): the implementations of
  `FerroObjectImpl` of `Animatable`, `StyledElement`, `Visual`, `Layoutable`, `InputElement`,
  `Control`, `ContentControl`, `TextBlock`, `ContentPresenter`, `Border` and `Panel` are wrapped in
  `ferro_overrides!`. The functions are untouched; the change is the first and the last line of
  each block.

**[E]** For a text block the slot of `on_property_changed_core` was reached through seven
forwarding functions and is now the implementation of `Animatable`; `styling_parent` of a text
block passed through six and now passes through two (`Layoutable` and `Visual` implement
`StyledElementImpl` with plain blocks that override other members).

## What is left

- **Implementations with overrides, written as plain `impl` blocks**: 919 in the workspace by a
  scan, 201 of them of `FerroObjectImpl`. Each keeps a forwarding function for every member it
  does not override. Wrapping one in `ferro_overrides!` is two lines and cannot change what it
  does; which to wrap next should follow the profile and the ten most called members of design 09
  (`StyledElement::styling_parent`, the members of `StyledElementImpl`, `VisualImpl`,
  `LayoutableImpl` and `InputElementImpl` of the base classes first, since every control passes
  through them).
- **The base call itself** (`parent_<member>`) still reads the table of the base class and calls
  through it, where upstream's `base.Member()` is a direct call; see design 02, "Left for a
  profile".

## Expected gain

To be measured. The 5 to 8 % **[E]** of the design assumed every chain gone; what is implemented
removes the forwarding functions of the implementations that state their overrides, which is all
of the empty ones and eleven with overrides. Code size should fall slightly where the forwarding
functions are no longer referenced by any table.

## Risks

Low to medium. The change is inside one macro and the root class, and what it rests on (a default
is a pure forward) holds by construction of the macro. It is the path of every virtual call of
every class, and it was written without a build.

## Verification

- The class system tests of `ferroui-base` (`ferro_object_tests.rs`,
  `tests/class_registration_tests.rs`) and every suite of the workspace: each test of a control
  goes through the tables.
- New, in `src/FerroUI.Base/tests/virtual_dispatch_tests.rs`: a hierarchy six classes deep under
  the class that declares two members, which mixes the three forms (plain `impl` with and without
  overrides, `ferro_impl_classes!`, `ferro_overrides!`):
  - `call_reaches_the_most_derived_override_and_each_base_once`: the test of the design that
    counts calls (an override at depth 1 and one at depth 6, the first calling its base before its
    own work and the second after: each is entered once, in that order).
  - `call_through_a_base_class_reaches_the_same_override`.
  - `override_in_the_middle_is_inherited_by_the_classes_below_it`.
  - `constructed_runs_the_override_of_the_class_once`.
  - `property_change_reaches_the_overrides_through_every_form_of_class`.
  - `implementation_states_the_members_it_overrides`: what each form sets `__OVERRIDES` to.
  - `slot_of_a_member_stated_as_not_overridden_is_the_slot_of_the_base_class`: the tables
    themselves. This one tests the machinery and does not hold before the change.
  - `slot_is_not_taken_from_a_base_class_that_has_no_such_member`: the class that declares a
    member.
- `scripts/browser/scroll-profile.mjs` before and after; the module size before and after
  (`scripts/browser/module-sizes.mjs`).

## Counters: before, expected after

Per recycled row, 20 pixels per step. "Before" is the table of design 09. The counter of virtual
calls is incremented in the public function of a member (`Class::member`), once per call; the
forwarding functions were never counted. So no counter of design 09 moves with this design, by
member or in total; one that does is a defect.

| Counter | Before | Expected after | Measured after |
|---|---:|---|---|
| virtual calls (all members) | 2835.50 | 2835.50 | to be measured |
| `StyledElement::styling_parent` | 764.00 | 764.00 | to be measured |
| `FerroObject::on_property_changed_core` | 459.24 | 459.24 | to be measured |
| `FerroObject::on_property_changed` | 439.24 | 439.24 | to be measured |
| `ResourceProvider::try_get_resource` | 343.00 | 343.00 | to be measured |
| every other counter of design 09 | as in its table | unchanged | to be measured |
| allocations | 1175.0 | 1175.0 from this design (the tables allocate nothing; design 02 lowers it) | to be measured |
| forwarding functions entered | not counted | lower: about 3,200 fewer for `on_property_changed_core` alone **[E]** (design 02) | a profile: the self time of `build_vtable::{closure}` |
| self time of the forwarding closures, browser profile | 46 ms of 574 | lower | to be measured |
| size of the browser module | as published | equal or slightly lower | to be measured |
| layout microseconds (`release`) | 411.3 (before designs 03 and 04; not measured since) | lower | to be measured |

### Commands

The suites:

```sh
cargo test -p ferroui-base --lib
cargo test -p ferroui-base --lib virtual_dispatch_tests
cargo test -p ferroui-base --features perf-counters --lib perf_counters
cargo test -p ferroui-controls --lib
cargo test -p control-catalog --lib
cargo test --workspace
```

The figures, with the commands of design 09, on each commit and on its parent:

```sh
cargo test -p control-catalog --release --features perf-counters --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --profile dist --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

In the browser: `scripts/browser/scroll-profile.mjs` and `scripts/browser/module-sizes.mjs`, on a
build of the branch and of its base.

## Doubts

Most likely first.

1. Nothing was built. The parts most likely to need a correction: the constant block inside the
   generic `build_vtable` (it names the type parameter of the surrounding `impl`), the closure
   bound to a variable with the type of the slot before it is chosen (it was given directly to the
   field), and the pattern of `ferro_overrides!` (attributes, a parameter list, an optional return
   type and a block, repeated). Should the constant block be refused, calling
   `__forwards_to_parent` without it is equivalent and only moves the decision to the first use of
   the class.
2. Whether an optimiser removes the forwarding function of a slot taken from the base class (it is
   still written, and no longer referenced by the table). If it does not, the module does not
   shrink; the calls are shorter either way.
3. The gain on the desktop may be within the noise: a forwarding step there is a load, a test and
   an indirect call that is predicted well.
4. `ferro_overrides!` changes how an implementation with overrides is written, in eleven places so
   far. It is two lines and the block inside is unchanged, but it is a form upstream has no
   counterpart of, and the formatter does not format inside it. Whether the rest of the 919 should
   follow is a decision for after the measurement.

### Measured on 2026-10-09

The whole workspace passes with the changes of designs 01 and 02 together (12080 tests), and the desktop catalog starts and runs. The run with `perf-counters` gives the same value for every one of the 70 counter lines of the benchmark as before: the same notifications, virtual calls by member and everything else. A recycled row costs 5 fewer allocations (1170.0 against 1175.0, from the value no longer copied per set). What these changes are for is processor time (the forwarding functions a virtual call no longer passes through), and that was not measured: the machine was under heavy load, and the time per row moved by more than the expected gain between runs. To be measured on an idle machine and in the browser (`scripts/browser/scroll-profile.mjs`), where the indirect calls cost most.
