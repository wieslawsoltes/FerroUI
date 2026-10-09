# 04. Tree attachment: styles, implicit themes, resources

Status: the hypotheses are verified against the code and the counters of design 09 (see
"Hypotheses, verified"). Step 4 is implemented for the resource dictionary, with two more changes of
the port's machinery that the reading of the code turned up (see "What is implemented"). Steps 2 and
3 are not implemented: they are caches upstream does not have, which is different logic, and the
figures show that step 3 would gain nothing where it was meant to (see "What is not implemented").
Written without a build: the session that validates builds, runs the suites and fills the "measured"
column of "Counters".

## Finding

- **[M]** Attaching elements to the logical tree accounts for 53 ms of the scrolling profile (9 %),
  detaching for 18 ms, the visual tree for another 24 ms.
- **[M]** Applying styles accounts for 34 ms (6 %), resource lookups for 42 ms (7 %), of which the
  lookup of the implicit control theme is 14 ms.
- **[M]** A content presenter that replaces its child accounts for 76 ms (13 %): 24 ms of it attach
  the new text block to the logical tree, and 7 ms plus 7 ms of that apply its styling.

## Cause

Per recycled row, the row enters the logical tree again and five new text blocks enter it for the
first time. Entering the tree means, for every element of the subtree
(`styled_element.rs`, `on_attached_to_logical_tree_core`): finding the logical root, resolving the
implicit theme by a resource lookup that walks up the tree and through the merged dictionaries of
every host on the way (`get_effective_theme`, `try_find_resource`), and applying the styles of
every style host between the element and the root (`apply_styles`), each of which evaluates its
selectors against the element. Dynamic resource bindings look their values up again on the same
walk. The managed original does all of this as well.

**[H]** Two things make it cost more here than it has to, with the same results:

1. The same lookup is repeated for every element of the same class in the same place. Five text
   blocks attached under five cells of one row resolve the same implicit theme and match the same
   styles; the next row does it again.
2. The lookup walks dictionaries with string and type keys through dynamically dispatched
   providers; each step is a hash lookup with the default hasher (hashing has 9 ms of self time).

## What must not change

Which theme and which styles an element gets, in which order they are applied, and when: styling
still happens on attachment. A cache must give the answer the walk would give; anything that can
change the answer (a resource added, removed or replaced, a style added, a class or a pseudo-class
changed, a theme variant changed) invalidates it. The resource change notifications upstream raises
stay as they are.

## Design

1. **Measure**: per attached element, the number of style hosts walked, selectors evaluated,
   selectors matched, dictionaries probed, and the time in each.
2. **Implicit theme cache per host and style key.** The result of the implicit theme lookup for a
   style key, as seen from a given resource host, is the same for every element below that host
   until a resource changes somewhere on the path to the root. Cache it at the nearest host, keyed
   by style key and theme variant, and clear the cache of a host and of the hosts below it on the
   resource change notification that upstream already propagates down the tree.
3. **Selector prefilter per style host.** For each style host, an index from the type of an element
   to the styles whose selector can match that type at all (the type test is the outermost test of
   nearly every selector of the themes). Styles outside the index are skipped without evaluating
   them; the ones inside are evaluated as now, in the same order. The index is rebuilt when the
   styles collection changes. **[H]** Upstream has a comparable cache of "no style matches this
   type" per host; check whether the port carries it and whether it is hit.
4. **A faster hasher for the internal dictionaries** whose keys are not attacker controlled
   (resource keys, property ids, type ids): an `FxHash`-style hasher instead of SipHash.

## Hypotheses, verified

Against the code of the port, the code of upstream (`StyledElement.cs`, `Styles.cs`, `StyleBase.cs`,
`ControlTheme.cs`, `Style.cs`, `ResourceDictionary.cs`, `ResourceNodeExtensions.cs`) and the counters
per recycled row of design 09 (the scenario of 20 pixels per step; the other one differs in the
second decimal).

**Hypothesis 1, the same lookup repeated: confirmed for the implicit theme and the resource walks,
dropped for the styles.**

- **[M]** 18 elements enter the logical tree per recycled row and 17 implicit theme lookups are
  made: one for every element without a `Theme` set. Upstream makes the same lookups:
  `OnAttachedToLogicalTreeCore` calls `ReevaluateImplicitTheme`, which sets `_implicitTheme` to null
  and calls `GetEffectiveTheme`, which looks the style key up with `TryFindResource`. The only cache
  upstream has is the field `_implicitTheme` of the element, valid until the element enters a tree
  again; the port has the same field (`implicit_theme`) with the same life.
- **[M]** 43 resource lookups per row probe 467 hosts (10.9 per lookup) and reach 343 calls of
  `ResourceProvider::try_get_resource` (8.0 per lookup). Upstream's `TryFindResource` is the same
  loop: `TryGetResource` of the host, then of every `StylingParent`; it does not consult
  `HasResources`, and a host without resources answers after reading two fields, in the port as in
  upstream. So there is no host the port probes that upstream skips.
- **[M]** 0 styles are evaluated and 0 matched per recycled row; 4 control themes are evaluated and
  2 matched. "Match the same styles" does not happen in this scenario: the 261 style hosts walked
  (14.5 per attached element) are the walk of `ApplyStyles` to the root and back, which finds no
  style with setters on the way. What is repeated is the walk, not a match.

**Hypothesis 2, dynamically dispatched providers and the default hasher: confirmed.**

- The table of a resource dictionary was a `HashMap<ResourceKey, ResourceItem>` with the default
  hasher of the standard library (SipHash with a random seed), and every provider on the way is
  reached through `Rc<dyn IResourceProvider>` and the virtual `try_get_resource`: **[M]** 343 such
  calls per row. A table that is empty is not hashed into (the table of the standard library
  answers before it hashes).
- The walk asks `StyledElement::styling_parent` at every step: **[M]** 764 calls per row, the most
  called virtual member. **[E]** 261 of them are the style hosts walked and at most 467 the hosts
  probed for resources, each of which asks once for the next host; the rest is the search for the
  logical root. The base implementation made a second handle of the inheritance parent and dropped
  the first at every call.

**The [H] of step 3, a cache of "no style matches this type" in upstream: dropped.** Upstream has no
such cache. There is no `StyleCache` in the reference checkout; `Styles.TryAttach`, which returns
the `SelectorMatchResult` such a cache would be built from, has no caller there, and `ApplyStyles`
evaluates every style of every host for every element. The port's `Styles::try_attach` is likewise
not called. There is nothing missing to port.

**What the reading added.** The end of every styling pass is `ValueStore.EndStyling`, which
reevaluates all effective values of the element, in upstream as in the port, whether or not a frame
changed: once for each of the 18 elements attached (`ApplyStyling`) and once for each of the 18
detached (`InvalidateStyles`). Upstream's `ReevaluateEffectiveValues` indexes its list to call
`BeginReevaluation`; the port copied the list into a new `Vec`, with a handle of every value, for
that visit.

## What is implemented

Three changes, one commit each, none of which changes a statement of the ported logic.

### The hasher of the table of a resource dictionary (step 4)

- Files: `src/FerroUI.Base/controls/resource_key.rs` (`ResourceKeyHasher`,
  `ResourceKeyBuildHasher`), `src/FerroUI.Base/controls/resource_dictionary.rs` (the field `inner`
  of `ResourceDictionary`).
- Machinery: the table keeps `HashMap` and changes its hasher. `ResourceKeyHasher` takes a word at a
  time, multiplies, and folds the upper half of the product into the lower one; the rest of a text
  shorter than a word carries its length. A type key is two such steps and a name of 20 to 30
  characters four or five, where SipHash runs its rounds over the same bytes and a finalization.
  (The plain multiply-and-rotate scheme the design named was tried on paper first and rejected: a
  difference in the last byte of one word is cancelled by a difference in the first byte of the
  next, so that names with a number across a word boundary collide in all 64 bits. With the fold
  they do not, and the buckets and tags of numbered names, of addresses and of small numbers are
  spread as those of random values are.)
- Upstream statements preserved: `ResourceDictionary.TryGetResource` (the dictionary itself with
  `TryGetValue`, then the theme dictionaries for the variant, the variants it inherits and the
  default, then the merged dictionaries from the last to the first) and `TryGetValue` (the deferred
  item built on demand) are untouched; only the hasher behind `inner.get`, `insert`, `remove` and
  `contains_key` differs.
- Why behaviour is identical: equality of keys is `ResourceKey::eq` as before and equal keys hash
  equally (the hasher has no state of its own), so every lookup, insertion and removal finds what
  it found. No notification depends on the table. The only thing a hasher can change is the order
  in which `ResourceDictionary::keys` lists the keys, and that order followed the random seed of
  the default hasher: it differed from run to run and from dictionary to dictionary, so nothing can
  rely on it (its callers are tests that search or sort the keys). The keys are written in the
  markup and the code of the application, so the protection against chosen keys that SipHash gives
  is not needed here.
- Not changed: the theme dictionaries are a `FerroDictionary`, the observable dictionary of the
  collections, which keeps the default hasher; see "Left for a profile".

### The handle of the base styling parent

- File: `src/FerroUI.Base/styled_element.rs`, `StyledElementImpl::styling_parent` of
  `StyledElement`.
- Machinery: `Ref::downcast`, which turns the handle of the inheritance parent into the handle of
  the host after the type test, instead of `Ref::cast`, which made a second handle and left the
  first to be dropped: a count up and a count down less at each of the 764 calls per row.
- Upstream statement preserved: `IStyleHost.StylingParent => (IStyleHost?)InheritanceParent`.
- Why behaviour is identical: the type test is the same (`is::<StyledElement>()`), so the result is
  the same object or nothing as before. The member stays virtual and the walks still call it at
  every step: the class hierarchy is not closed (`TopLevel` returns the host of the global styles,
  `PopupRoot` its logical parent, test classes their own), so the walk cannot read a field instead.

### The start of a reevaluation of all effective values

- File: `src/FerroUI.Base/property_store/value_store.rs`, `ValueStore::reevaluate_effective_values`.
- Machinery: the loop that announces the reevaluation visits the effective values in place under a
  shared borrow of the list, instead of copying the list into a `Vec` first: an allocation and a
  handle per value less for each reevaluation of a store that holds values.
- Upstream statements preserved: `ReevaluateEffectiveValues`: `BeginReevaluation` of every existing
  value by index, then the frames from the last to the first, then the removal of the values still
  unset. The second and third part are untouched, the third keeps its copy of the list.
- Why behaviour is identical: `EffectiveValue::begin_reevaluation`, the only implementation, resets
  the two priorities the value records and calls nothing, so the list cannot change during the
  visit and no borrow can collide; the values are visited in the same order. The copy also kept
  every value alive until the pass ended. Nothing reads a value through it, and the list of the
  third part holds the values that are removed there, so the only difference is when a value is
  freed that a handler removed from the store in the middle of the pass: at that point instead of at
  the end of the pass. An effective value has no destructor of its own.
- The third part is left as it is: its handlers can add and remove values, and upstream indexes the
  live list there (and stops when the list became shorter than the index) where the port walks a
  copy. Making it index the list would change what a reentrant handler sees, which is logic.

## What is not implemented, and why

- **Step 2, the implicit theme cache per host.** Upstream has no such cache, so it is new logic and
  not a missing port. It also cannot be shown to give the answer of the walk: the walk follows the
  styling parent (the inheritance parent, or what `TopLevel` and `PopupRoot` return), while the
  notification that would clear the cache follows the logical children (and the overrides of
  `TemplatedControl` and of the visual layer manager). The two trees differ for an element whose
  inheritance parent is not its logical parent; an element attached to a parent that is not rooted
  is deliberately not notified at all (`set_parent`); and a lookup has an effect of its own, since
  it builds a deferred resource. This is a deviation that needs a decision and is not proposed here.
- **Step 3, the selector prefilter.** New logic as well, its [H] is dropped (upstream has no such
  cache), and the figures leave it nothing to gain in the recycling case: no style is evaluated per
  recycled row.
- **Skipping hosts by a flag.** Upstream's walk does not consult `HasResources`; a host without a
  resource dictionary and without styles already costs two field reads.
- **A direct field read instead of the virtual styling parent.** The override set is not closed
  (see above).

What remains of the cost of this design is upstream's own logic: 17 implicit theme lookups, 43
resource walks of about 11 hosts and 8 providers each, and 18 walks of about 14 style hosts per
recycled row, with the dispatch of each step (design 01) and the notifications they raise (design
02).

### Left for a profile

Not changed, because nothing measured points at them yet; each would be machinery only.

- The probes of the theme dictionaries (`ResourceDictionary::theme_dictionary`) hash a
  `ThemeVariant` with the default hasher through `FerroDictionary`, twice per themed lookup of a
  dictionary that has theme dictionaries.
- `ResourceDictionary::try_get_resource` makes a handle of the default theme variant
  (`ThemeVariant::default()`) to compare with and to look up, once or twice per such dictionary.

## Expected gain

Small, and to be measured. The 5 to 10 % **[E]** of the design came from steps 2 and 3, which are
not implemented. **[E]** What the three changes take away per recycled row: the hashing of at most
343 table probes (fewer: an empty table is not hashed into), 764 count pairs of a handle, and one
list with a handle per value for each reevaluation of all values of a store that holds any (at
least the ends of the 36 styling passes of the 18 elements attached and the 18 detached that hold
values).

## Risks

Low for what is implemented: no value is cached, so nothing can go stale. The hasher is new code;
its test checks 603 keys of every kind and the texts that a word-at-a-time hasher gets wrong.

## Verification

- The styling and resource suites of `ferroui-base`, the theme tests of both themes
  (`control_templates.rs`, `documents.rs`), the catalog tests.
- New tests (they hold before the changes as well; they pin the walk for any later change of the
  lookup machinery):
  - `controls/resource_tests.rs`: `resources_with_many_keys_of_every_kind_are_found_by_an_equal_key`,
    `resource_key_hasher_hashes_equal_keys_equally_and_all_of_a_text`,
    `find_resource_finds_a_resource_added_later_to_a_host_on_the_way_up` (a host that had no
    resources, and the styles of the element), `find_resource_follows_the_element_to_another_parent`.
  - `styling/style_tests.rs`:
    `implicit_theme_replaced_in_the_resources_is_applied_when_reattached_to_the_same_logical_tree`,
    `implicit_theme_added_to_the_resources_is_applied_when_reattached`,
    `implicit_theme_removed_from_the_resources_is_detached_when_reattached`,
    `implicit_theme_is_reevaluated_when_moved_between_hosts_of_the_same_logical_tree`,
    `styling_parent_is_the_inheritance_parent_when_it_is_a_styled_element`.
  - Already there, ported from upstream: a style added to or removed from a host after attachment
    (`Adding_Style_Should_Attach_To_Control`, `Removing_Style_Should_Detach_From_Control` and the
    nested forms), the implicit theme across two logical trees
    (`Implicit_Theme_Is_Reevaluated_When_Removed_And_Added_To_Different_Logical_Tree`), the
    resource change notifications. All tests of `StyledElementTests_Theming.cs` are ported.
- `scroll-profile.mjs` and `first-frame.mjs`.

## Counters: before, expected after

Per recycled row, 20 pixels per step. "Before" is the table of design 09. No counter of this design
is expected to move: the changes alter what a step costs, not how many steps there are. A counter
that moves is a change of logic and a defect of this work.

| Counter | Before | Expected after | Measured after |
|---|---:|---|---|
| elements attached to a logical tree | 18.00 | 18.00 | to be measured |
| elements detached from a logical tree | 18.00 | 18.00 | to be measured |
| implicit theme lookups | 17.00 | 17.00 | to be measured |
| style hosts walked | 261.00 | 261.00 | to be measured |
| styles evaluated | 0.00 | 0.00 | to be measured |
| styles matched | 0.00 | 0.00 | to be measured |
| control themes evaluated | 4.00 | 4.00 | to be measured |
| control themes matched | 2.00 | 2.00 | to be measured |
| style instances attached | 2.00 | 2.00 | to be measured |
| style instances created | 2.00 | 2.00 | to be measured |
| resource lookups | 43.00 | 43.00 | to be measured |
| resource hosts probed | 467.00 | 467.00 | to be measured |
| content presenter children replaced | 10.00 | 10.00 | to be measured |
| virtual calls of `StyledElement::styling_parent` | 764.00 | 764.00 | to be measured |
| virtual calls of `ResourceProvider::try_get_resource` | 343.00 | 343.00 | to be measured |
| property changes raised | 459.24 | 459.24 | to be measured |
| allocations | 1224.0 | lower, by at most one per reevaluation of all effective values **[E]** | to be measured |
| layout microseconds (`release`) | 411.3 | lower | to be measured |

### Commands

The suites that hold the changes to upstream's behaviour:

```sh
cargo test -p ferroui-base --lib
cargo test -p ferroui-base --features perf-counters --lib perf_counters
cargo test -p ferroui-controls --lib
cargo test -p ferroui-themes-fluent
cargo test -p ferroui-themes-simple
cargo test -p control-catalog --lib
```

The figures, with the commands of design 09 (the counters, then the allocations, then the times
without a feature):

```sh
cargo test -p control-catalog --release --features perf-counters --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --profile dist --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

Each change is one commit, so each can be measured against its parent and reverted alone.

## Doubts

Most likely first.

1. Nothing was built or run. The new tests were written against the signatures in the source, and
   the hasher was checked by a simulation of its arithmetic outside the build (the distinct hashes
   its test asserts, the spread of buckets and tags); a test that does not compile or an assertion
   that is off by an assumption about a helper is the most likely thing to find.
2. The gain may be within the noise of the benchmark (10 % between runs on a shared machine): the
   changes are small by design.
3. The hasher on names that differ only in bits a fold does not separate: two words that differ in
   the same bits of their upper and lower half can still meet. Such keys collide in the table and
   are told apart by equality, so this costs time and no correctness; it was not seen for numbered
   names, addresses or small numbers.
4. The time at which an effective value is freed when a handler removes it from the store in the
   middle of a reevaluation (see the third change).
