# 01. Virtual tables without forwarding chains

Status: design, not implemented.

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

## Expected gain

5 to 8 % of the scrolling time **[E]**: the 46 ms of self time are mostly the forwarding itself, and
the callers lose the call overhead around it. Code size should fall slightly, since thousands of
forwarding closures disappear (they are among the largest generic families of the module, see
`browser-platform.md`, section 18).

## Risks

Low. The change is inside one macro and one module. A member whose default is not a pure forward
must keep its body; step 2 only applies to pure forwards.

## Verification

- The class system tests of `ferroui-base` (overrides, base calls, tables of deep hierarchies).
- A test that counts calls: an override at depth 1 of a hierarchy of depth 6 is entered once per
  call and its parent once.
- `scripts/browser/scroll-profile.mjs` before and after; the module size before and after.
