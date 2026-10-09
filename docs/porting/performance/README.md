# Run-time performance: findings and improvement designs

This folder records what a scrolling trace of the ControlCatalog site showed, and holds one design
document per improvement that follows from it. Nothing here is implemented yet, except where a design
says so. Each design can be taken up on its own.

Status: the native part of design 09 is written (the benchmark of recycling of the table view, with
allocations per recycled row, and the counters behind the feature `perf-counters`), with its first
figures. Designs 03 and 04 are verified against those figures and upstream: what the rule below
allows of them is implemented (the pool of the lists of old and new inherited values; the hasher of
the resource dictionary, the handle of the styling parent and the start of a reevaluation in
place), their caches and their reordered walk are not, and their gains are still to be measured.
Designs 05 and 06 are verified in the same way: of 05 two small changes of machinery are
implemented (a box the data context nodes made for nothing, and the order in which a conversion
asks whom to log against), its typed path for template bindings is not (upstream has none); 06 has
nothing the rule allows (the cache of the formatter is upstream's and hits when upstream's does)
and nothing is implemented for it.
Designs 01 and 02 are verified in the same way too: of design 02 two of the three hypotheses are
contradicted by the code (the list of handlers is not copied for a notification, the arguments
borrow the values) and its fast path is not allowed; what it leaves is the dispatch of the
overrides, which is design 01, and a copy of the value in the effective value. Design 01 is
implemented where a class states what it overrides (the table takes the slot of the base class
for the other members), for every implementation that overrides nothing and for the root class
members on the path of a property change; their gains are to be measured. The other designs are
not started.

Markers, as in the other performance documents: **[M]** measured, **[E]** estimated with the reasoning
next to it, **[H]** a hypothesis that the design must verify before any code changes.

## The rule every design follows

The port follows the upstream project statement by statement, and that does not change: no design
here alters what the framework does or the order in which it does it. A recycled row is still
cleared, removed, prepared and added; a property change still raises the same notifications to the
same listeners in the same order. The designs change how the port's own machinery carries that work
out (dispatch, storage, caching of values that upstream also computes, build settings), and each one
names the upstream behaviour it must preserve and the tests that hold it to that. A change that
needs different logic belongs in `docs/porting/DEVIATIONS.md` and needs a decision first; none is
proposed here.

## What was measured

Source: a DevTools trace of the published site (2026-10-07) while the TableView page was scrolled by
its scroll bar, and a reproduction with function names (the module linked with `--profiling-funcs`,
60 wheel events of 120 pixels, 30 down and 30 up, headless Chrome 154 with WebGL on SwiftShader,
Apple M3 Pro, the machine shared with other work). The script of the reproduction is
`scripts/browser/scroll-profile.mjs` (pull request 47).

- **[M]** In the trace, one task per pointer event takes 20 to 34 ms while the scroll bar is
  dragged, all of it inside the WebAssembly module. A jump of the offset by a viewport re-realises
  every visible row, and such a task is one layout pass over about 22 rows of five cells. A frame at
  60 Hz has 16.7 ms.
- **[M]** The reproduction spends 574 ms of processor time on the 60 wheel events (about 9.6 ms per
  event, each of which recycles a few rows).
- **[M]** The profile is flat: 1,091 distinct functions have samples and none has more than 3 % of
  the time.
- **[M]** WebAssembly tiering is not the cause: with the baseline compiler disabled
  (`--js-flags=--no-liftoff`) the time is the same (559 and 565 ms against 547 and 555 ms).

Where the 574 ms go, by the outermost function of each kind (inclusive times; the rows overlap, since
for example a binding publishes through a property change):

| Work | ms | Share | Design |
|---|---:|---:|---|
| Measure pass of the virtualizing panel | 259 | 45 % | |
| ... re-attaching recycled rows (logical and visual tree) | 68 | 12 % | 03, 04 |
| ... clearing the cells of recycled rows | 48 | 8 % | 02, 05 |
| ... preparing rows (cells rebuilt, data context) | 30 | 5 % | 02, 05 |
| ... measuring rows: new text blocks attached, styled, shaped | 78 | 14 % | 04, 06 |
| ... removing recycled rows | 26 | 5 % | 03 |
| Compositor render and Skia | 145 | 25 % | 07 |
| Arrange | 32 | 6 % | |
| Compositor commit | 32 | 6 % | 07 |

The same time by mechanism, across all of the above:

| Mechanism | ms | Share | Design |
|---|---:|---:|---|
| Property change notifications (`raise_property_changed` and below) | 118 | 21 % | 02 |
| Bindings (template bindings 59 ms, dynamic resources 21 ms, the rest value bindings) | 96 | 17 % | 05 |
| Content presenter replacing its child | 76 | 13 % | 04, 06 |
| Text layout (shaping 16 ms) | 55 | 10 % | 06 |
| Inherited values after a parent change | 54 | 10 % | 03 |
| Attaching to the logical tree | 53 | 9 % | 04 |
| Resource lookups (the implicit theme 14 ms) | 42 | 7 % | 04 |
| Applying styles | 34 | 6 % | 04 |
| Forwarding closures of the virtual tables (self time) | 46 | 8 % | 01 |
| Value store and effective values (self time) | 56 | 10 % | 02, 03 |
| Allocator (self time) | 11 | 2 % | 08 |
| Diagnostic overlay (frames per second) | 13 | 2 % | 07 |

What the build settings give was measured as well (pull request 47; design 08):

| `browser` profile | Module MB gzip | Scrolling ms | Change |
|---|---:|---:|---:|
| `opt-level = "z"`, level 3 for the markup crates | 10.10 | 612 | |
| `ferroui-base` and `ferroui-controls` at "s" | 10.86 | 492 | -20 % |
| every crate at 3 | 12.62 | 412 | -26 % |

## Conclusions

1. The work per recycled row is the upstream design, and it is expensive by design: a row leaves
   both trees and enters them again, and each of its cells gets a new text block. In the managed
   original each step is cheap; in the port each step goes through machinery that costs more per
   call than its managed counterpart.
2. No single function is slow. The cost is the number of small calls per row, so the gains are in
   the machinery that every one of those calls goes through: virtual dispatch (01), property change
   notification (02), inheritance (03), tree attachment and styling (04), bindings (05).
3. Build settings are worth about a quarter at most (08), and a fifth of that quarter is already
   taken.
4. A quarter of the time is drawing, not layout (07); part of it is the diagnostic overlay the
   browser host of the sample turns on, as upstream does.

## The designs

Ordered by expected gain for the scrolling case, largest first. The gains are estimates until the
first step of each design (always a measurement) has been done.

| # | Design | Touches | Expected gain | Risk | Size |
|---|---|---|---|---|---|
| [01](designs/01-virtual-dispatch.md) | Virtual tables without forwarding closures (written for the implementations that state what they override: all the empty ones and eleven with overrides; inlining dropped: it cannot remove a call through a table) | `type_system.rs`, the class macro, `ferro_object.rs` | to be measured (was 5 to 8 % **[E]** with every chain gone) | Low to medium | Medium |
| [02](designs/02-property-change-notification.md) | Cheaper property change notifications (the four steps dropped: the lists and the arguments already cost nothing, the fast path is different logic; written: the dispatch of the overrides by design 01, one copy of the value less per set) | `ferro_object.rs`, the property store | small, to be measured (was 5 to 10 % **[E]** with the four steps) | Medium | Small |
| [03](designs/03-inheritance-parent-change.md) | Inherited values on a parent change (the pool of lists written; the single pass dropped: it reorders notifications) | the property store | small, to be measured (was 4 to 7 % **[E]** with the single pass) | Low | Small |
| [04](designs/04-tree-attachment-and-styling.md) | Tree attachment: styles, implicit themes, resources (the hasher and two handle and list changes written; the caches dropped: upstream has none) | `styled_element.rs`, resources, the property store | small, to be measured (was 5 to 10 % **[E]** with the caches) | Low | Small |
| [05](designs/05-bindings-on-recycle.md) | Bindings while a container is recycled (a box and an order corrected; the typed path for template bindings dropped: upstream has none) | data bindings | small, to be measured (was 4 to 8 % **[E]** with the typed path) | Low | Small |
| [06](designs/06-text-layout.md) | Text layout: shaped runs and line metrics (verified; nothing implemented: the caches of the design are not upstream's) | none | none (was 3 to 6 % **[E]** with the caches) | None | None |
| [07](designs/07-compositor-frame-cost.md) | Cost of a composed frame | composition, Skia backend | 3 to 8 % **[E]** | Medium | Medium |
| [08](designs/08-build-settings.md) | Build settings of the browser module | `Cargo.toml`, the link | up to 6 % more **[M]** | Low | Small |
| [09](designs/09-measurement.md) | Benchmarks and a regression gate (items 1 and 2 written) | scripts, CI, `diagnostics/perf_counters.rs`, the catalog tests | none by itself | Low | Medium |

Design 09 comes first in time: every other design starts with a measurement it provides. The
estimates do not add up; several designs shorten the same call chains.

## Related documents

- `docs/porting/desktop-performance.md`: size and start-up of the desktop applications.
- `docs/porting/browser-platform.md`, sections 18 and 19: size and start-up of the browser module.
- `docs/porting/DEVIATIONS.md`: where the port departs from upstream.
