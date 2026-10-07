# 05. Bindings while a container is recycled

Status: design, not implemented.

## Finding

- **[M]** Bindings account for 96 ms of the scrolling profile (17 %): template bindings 59 ms,
  dynamic resource bindings 21 ms, value bindings the rest.
- **[M]** Clearing a cell (`TableViewCell::set_column(None)`) costs 48 ms in total; 35 ms of it are
  template bindings publishing the cleared content into the template of the cell, which removes
  the text block of its content presenter.
- **[M]** Setting the column again costs 19 ms, 14 ms of it in `bind_binding`.

## Cause

A cell binds its content when its column is set and clears the binding when the column is cleared;
the content flows through a template binding into the content presenter, which drops its child for
empty content and builds a new one for the next content. Upstream does exactly this, and the
sequence stays.

**[H]** What can cost less is the machinery of each step:

1. A template binding publishes through the general binding expression path
   (`UntypedBindingExpressionBase::publish_value`, a sink, a property set with a priority), with
   boxed values, although source and target are properties of two known objects.
2. `bind_binding` builds a binding expression from a binding description every time: parsing or
   cloning the path, creating the observer nodes, subscribing. The description is the same for
   every cell of a column.
3. A dynamic resource binding repeats its lookup on every attachment (see design 04 for the lookup
   itself).

## What must not change

The values published, their order and priority, when a binding is instantiated and when it is
disposed, and the error and fallback behaviour of bindings.

## Design

1. **Measure**: per recycled row, bindings created and disposed, values published, and the time in
   expression construction against the time in publishing.
2. **Template bindings on the typed path.** When source and target property have the same value
   type and the binding has no converter, copy the value through the typed property route instead
   of the boxed one. The observable behaviour is the same value at the same priority.
3. **Cheaper instantiation of a compiled binding.** Keep the immutable part of a binding (the
   parsed path and its accessors) shared behind a reference count and build only the per-instance
   state on `bind_binding`. **[H]** Check how upstream shares the compiled path between instances
   and follow it.
4. **Dynamic resource bindings** take the cached lookup of design 04 when it lands.

## Expected gain

4 to 8 % **[E]**: the publishing itself has to stay, but the template binding path and the
instantiation are most of the 96 ms.

## Risks

Medium. Bindings have many edge cases (fallback values, null handling, priorities, data
validation); steps 2 and 3 apply only to the plain cases and leave the others on the present path.

## Verification

- The binding suites of `ferroui-base` and of the markup crates, the control template tests.
- A recorded sequence of published values for a cell that is cleared and set again, compared
  before and after.
- `scroll-profile.mjs`.
