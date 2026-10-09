# 03. Inherited values on a parent change

Status: the hypotheses are verified against the code, upstream and the counters of design 09 (see
"Hypotheses, verified"). One change of the port's machinery is implemented: the list of old and new
values comes from a pool, as upstream's does (see "What is implemented"). Step 2 is not implemented:
it changes the order of notifications, which is logic. Step 3 has nothing to gain by the figures,
and step 4 is what the port already does. An order in which the port differs from upstream was
found and is recorded, not changed (see "A difference found"). Written without a build: the session
that validates builds, runs the suites and fills the "measured" column of "Counters".

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

## Hypotheses, verified

Against `ValueStore::set_inheritance_parent`, `inherited_value_changed` and
`on_inheritance_ancestor_changed` of the port, `ValueStore.SetInheritanceParent`,
`InheritedValueChanged` and `OnInheritanceAncestorChanged` of upstream
(`PropertyStore/ValueStore.cs`), and the counters per recycled row of design 09 (20 pixels per step).

**Step 1, the figures. [M]** 12 changes of the inheritance ancestor per recycled row; 84 inherited
values compared, 7.0 per change; 84 of them differ; 228 objects visited by the walks, 2.7 per
differing value. **[E]** The 12 changes are the 2 of the row (removed, added) and the 10 of the text
blocks a content presenter replaced (5 removed when the cells are cleared, 5 added when they are
prepared; the counter of replaced children is 10.00). A text block is a leaf, so its 7 values are 7
visits; that leaves 158 visits for the 14 walks of the row, about 11 objects per walk. The counters
do not give the size of the subtree of a row as such (design 09, "what is not counted yet"), so the
figures alone do not say how much of it a walk skips; step 4 below was verified in the code.

**The cause as written: confirmed, with one correction.** The algorithm is upstream's, statement by
statement: the old values from the old ancestors, the new ones from the new ancestors,
`OnInheritanceAncestorChanged`, then one `InheritedValueChanged` per pair that differs, each of
which walks the subtree. "Every inherited value ends up as it was" is true of the two operations
together and of neither alone: on the removal every pair has an old value and no new one, on the
addition a new one and no old one, so all 84 pairs differ and none can be skipped. The removal and
the addition must stay two operations (see "What must not change").

**Step 2, the [H] that upstream's order is property-major: confirmed, and the step is dropped.**
`SetInheritanceParent` loops over the pairs and calls `InheritedValueChanged(property, oldValue,
newValue)` for each; that call raises the change on the object and recurses into the inheritance
children. So the change of one property is raised over the whole subtree before the next property
is raised anywhere. A walk per object would raise the same notifications in another order, and the
order of notifications is behaviour: a handler of the first property on a descendant would see the
ancestor already changed for the second. This needs a decision as a deviation and is not
implemented. The order is now held by a test
(`changing_inheritance_parent_raises_the_changes_of_one_property_over_the_subtree_before_the_next_property`).

**Step 3, the cheaper comparison: dropped by the figures, except for what upstream has and the port
lacked.** With 7 pairs per change, the linear search that pairs old and new values compares a
handful of numbers; a map would cost more than it saves. Pairs whose old and new value are the same
object are already skipped before anything is built (the comparison of the two handles), and the
figures show there are none. What upstream has here and the port did not: the dictionary of pairs
comes from a pool (`AvaloniaPropertyDictionaryPool`, four at most) and is released at the end,
where the port made a new list and grew it for every change. That is implemented.

**Step 4, the pruning: the port does what upstream does.** `inherited_value_changed` returns before
anything else when the store has an effective value for the property (`is_set`, which is upstream's
`_effectiveValues.ContainsKey(property)`), for that property only; the next pair starts again at the
object whose parent changed. A test holds it
(`changing_inheritance_parent_stops_at_a_local_value_for_that_property_only`).

**The second finding, `try_get_inherited_value` and `get_value`: upstream's walk.** Both follow the
chain of inheritance ancestors to the first store that has the property, as `TryGetInheritedValue`
does. The port upgrades a weak handle per hop where upstream reads a field; see "Left for a
profile".

## A difference found

Upstream's dictionary of pairs is an `AvaloniaPropertyDictionary`, which is sorted by property ID,
and the final loop reads it by index: the properties are raised in the order of their IDs. The
port's list is in the order the pairs were found: the nearest old ancestor first, each store by
property ID, then the properties that only the new ancestors hold. The two orders are the same when
all the values come from one store, and differ when more than one ancestor store contributes (a
data context set near the row, the font properties set at the window). Within one property the
order over the objects is upstream's in both.

This was there before this work and is not changed by it: sorting the pairs would change the order
in which the port raises the notifications, which needs a decision. Property IDs follow the order
of registration, which is not the same in the port as in upstream, so neither order reproduces
upstream's sequence for a given set of properties. It is recorded in `DEVIATIONS.md` (Property
system) with a comment at the site.

## What is implemented

One change, one commit.

### The pool of the lists of old and new values

- File: `src/FerroUI.Base/property_store/value_store.rs`: `OldNewValue`, `OLD_NEW_VALUES_POOL`,
  `MAX_OLD_NEW_VALUES_POOL_SIZE`, `ValueStore::set_inheritance_parent`.
- Machinery: the list in which a change pairs the old and the new inherited values is taken from a
  pool of the thread (at most four lists, the size of upstream's pool) and given back empty, with
  its capacity, at the end. The pairs leave the list one by one as their turn comes (`drain`), as
  they left the consumed list before.
- Upstream statements preserved: `var values = AvaloniaPropertyDictionaryPool<OldNewValue>.Get()`;
  the old values from the old ancestor chain (`TryAdd`: the nearest ancestor wins); the new values
  from the new chain (the first new value found for a property is kept, a property without an old
  value is added); `OnInheritanceAncestorChanged(newAncestor)`; `InheritedValueChanged` for every
  pair whose old and new value are not the same object;
  `AvaloniaPropertyDictionaryPool<OldNewValue>.Release(values)`. (Upstream takes the dictionary
  before it tests whether the ancestor is the same and leaves it to the collector when it is; the
  port takes the list after the test. Nothing can observe that.)
- Why behaviour is identical: the pairs, their order and the calls made for them are the ones of
  the list before; only the storage of the list outlives the call. A change of the inheritance
  parent made by a handler inside the notifications of another change takes another list (the one
  in use is not in the pool), so the two never share one; a list is given back only after its last
  pair was raised; a list whose change ended in a panic is not given back. The pool holds empty
  lists and no values, so it keeps no object alive.

## What is not implemented, and why

- **Step 2**: a different order of notifications; see above.
- **Step 3's map**: nothing to gain at 7 pairs per change.
- **Merging the removal and the return of a recycled row** would remove all of this work, and is
  exactly what "What must not change" rules out.

What remains of the cost is upstream's own logic: per recycled row, 84 pairs that all differ and
228 visits, each of which raises a property change (design 02) on an object that does not set the
property itself.

### Left for a profile

Not changed; each would be machinery only, and nothing measured points at them yet.

- `EffectiveValue::raise_inherited_value_changed` clones the old and the new value for every object
  visited (228 per row) before it compares them, where upstream reads two references. The clones
  are needed while a handler runs, since a handler may change the ancestor's value; all pairs
  differ here, so comparing before cloning would save nothing in this scenario.
- `ValueStore::try_get_inherited_value` upgrades two weak handles per hop of the ancestor chain.

## Expected gain

Small, and to be measured. The 4 to 7 % **[E]** of the design came mostly from step 2, which is not
implemented. **[E]** The pool takes away one allocation and one reallocation per change of the
inheritance ancestor (a list of 7 pairs is made with room for 4 and grown once), 12 of each per
recycled row.

## Risks

Low: the pool holds empty lists. The order of notifications is untouched and now has a test.

## Verification

- The inheritance tests of the property system (`tests/ferro_object_tests_inheritance.rs`,
  `tests/property_store/value_store_tests_inheritance.rs`, the reentrancy tests of
  `tests/ferro_object_tests_reentrancy.rs`); the styling tests that rely on inherited values. All
  tests of upstream's `AvaloniaObjectTests_Inheritance.cs` and `ValueStoreTests_Inheritance.cs` are
  ported.
- New tests, in `tests/ferro_object_tests_inheritance.rs` (they hold before the change as well):
  - `changing_inheritance_parent_raises_the_changes_of_one_property_over_the_subtree_before_the_next_property`:
    the recorded sequence of a subtree whose parent is set, removed and set again, each a change of
    its own. This is the "recorded notification sequence" the design asked for.
  - `changing_inheritance_parent_stops_at_a_local_value_for_that_property_only`.
  - `handler_of_an_inherited_change_can_change_the_inheritance_parent_of_another_subtree`: a change
    inside the notifications of another, the sequence of both, and the next changes after them (the
    lists of the pool used again).
- `scroll-profile.mjs`.

## Counters: before, expected after

Per recycled row, 20 pixels per step. "Before" is the table of design 09. No counter of this design
is expected to move; one that does is a change of logic and a defect of this work.

| Counter | Before | Expected after | Measured after |
|---|---:|---|---|
| inheritance ancestor changes | 12.00 | 12.00 | to be measured |
| inherited values compared | 84.00 | 84.00 | to be measured |
| inherited values differing | 84.00 | 84.00 | to be measured |
| objects visited by inherited value walks | 228.00 | 228.00 | to be measured |
| property changes raised | 459.24 | 459.24 | to be measured |
| allocations | 1224.0 | lower by 12 **[E]** (with the changes of design 04 applied, lower by those as well) | to be measured |
| reallocations | not in the table of design 09 (the benchmark prints them) | lower by 12 **[E]** | to be measured |
| layout microseconds (`release`) | 411.3 | lower | to be measured |

### Commands

The suites:

```sh
cargo test -p ferroui-base --lib
cargo test -p ferroui-base --features perf-counters --lib perf_counters
cargo test -p ferroui-controls --lib
cargo test -p control-catalog --lib
```

The figures, with the commands of design 09 (the counters, then the allocations, then the times
without a feature), on the commit of the change and on its parent:

```sh
cargo test -p control-catalog --release --features perf-counters --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --profile dist --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

## Doubts

Most likely first.

1. Nothing was built or run: the new tests were written against the signatures in the source. The
   sequences they assert were derived by reading `set_inheritance_parent` and
   `inherited_value_changed`; they do not assume which of the two properties has the lower ID.
2. The gain may be within the noise of the benchmark: 12 allocations of about 1200 per row.
3. Whether the difference found (the order across properties) should be corrected: it is the one
   place on this path where the port does not raise what upstream raises in the order upstream
   raises it. It needs a decision, not a measurement.
4. A list of the pool keeps the largest capacity it ever had: four lists of a few dozen pairs at
   most, per thread.
