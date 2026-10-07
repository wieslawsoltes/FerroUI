# 02. Cheaper property change notifications

Status: design, not implemented.

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

## Expected gain

5 to 10 % **[E]**: the fixed cost is a large part of the 118 ms, but the work of the listeners that
do run stays.

## Risks

Medium. The property system is the base of everything; step 1 must be exact about which classes
handle which properties, including handlers added by derived classes in other crates. Each step
lands separately behind the full property system suite.

## Verification

- The property system and binding suites of `ferroui-base`; the control suites.
- A test per step that records the notification sequence of a scripted scenario (set, bind, clear,
  inherit, animate) and compares it with the sequence recorded before the change.
- `scroll-profile.mjs` and the start-up measurement (`first-frame.mjs`), since start-up raises the
  most changes.
