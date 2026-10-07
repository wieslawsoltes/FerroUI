# 03. Inherited values on a parent change

Status: design, not implemented.

## Finding

- **[M]** `set_inheritance_parent` accounts for 46 ms of the scrolling profile (8 %) and
  `inherited_value_changed` for 54 ms (10 %; the two overlap).
- **[M]** `try_get_inherited_value` and `get_value` of inherited properties (flow direction, font
  properties) have about 20 ms of self time between them.

## Cause

A recycled row is removed from the panel and added again, so the inheritance parent of the row
changes twice, to nothing and back to the same parent. `ValueStore::set_inheritance_parent`
(`property_store/value_store.rs`) collects the inherited values of the old and of the new ancestor
chain, pairs them with a linear search per property, and for each property whose value differs
notifies the store, which walks the whole subtree of the row and reevaluates the value at every
descendant: once per property, so a row with five cells and their presenters and text blocks is
walked once for each of the inherited properties that the chain sets (data context, font family,
font size, foreground, flow direction, theme variant and others).

This is the upstream algorithm. In the common recycling case the row returns to the parent it left,
and every inherited value ends up as it was.

## What must not change

The notifications raised: a descendant whose effective inherited value changes raises a change, in
the order upstream raises them. The removal and the addition stay two separate operations with
their own notifications (a listener may observe the detached state).

## Design

1. **Measure**: per recycled row, the number of properties in the old/new comparison, the number
   that differ, the size of the subtree walked and the number of walks.
2. **One subtree walk for all properties.** Collect the changed properties first (as now), then walk
   the subtree once, applying all of them at each descendant, instead of one walk per property. The
   order of notifications at one object stays the order of the property list; the order across
   objects changes from property-major to object-major. **[H]** Upstream order is property-major;
   if a test or a listener depends on it, this step is dropped rather than recorded as a deviation.
3. **Cheaper comparison.** Replace the linear search that pairs old and new values by a small map
   keyed by property id; skip properties whose old and new effective value are the same object
   before any notification is built.
4. **Subtree pruning that upstream already implies.** A descendant that sets the property locally
   stops the propagation below it; check that the port stops there for every property and does not
   continue the walk for the remaining ones.

## Expected gain

4 to 7 % **[E]**, mostly from step 2 if it holds, since the subtree of a row is walked several
times per parent change and there are two parent changes per recycled row.

## Risks

Medium: step 2 changes an order that upstream does not document but tests may observe.

## Verification

- The inheritance tests of the property system; the styling tests that rely on inherited values.
- A recorded notification sequence of a subtree whose parent changes, compared before and after
  (for step 2 the comparison decides whether the step is taken).
- `scroll-profile.mjs`.
