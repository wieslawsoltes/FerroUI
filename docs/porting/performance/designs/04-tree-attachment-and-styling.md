# 04. Tree attachment: styles, implicit themes, resources

Status: design, not implemented.

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

## Expected gain

5 to 10 % **[E]**: steps 2 and 3 remove most of the 42 ms of lookups and a large part of the 34 ms
of style application for elements that are recycled or created in bulk, and they help start-up,
where every element of the first view is attached once.

## Risks

Medium. A stale cache shows the wrong theme or style, so invalidation is the whole design. The
caches go in one at a time, each with tests that mutate resources and styles after elements are
attached.

## Verification

- The styling and resource suites of `ferroui-base`, the theme tests of both themes
  (`control_templates.rs`, `documents.rs`), the catalog tests.
- New tests: replace a resource, add a style, switch the theme variant and change a class after
  attachment; each must restyle as before.
- `scroll-profile.mjs` and `first-frame.mjs`.
