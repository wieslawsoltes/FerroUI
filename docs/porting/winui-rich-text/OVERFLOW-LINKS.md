# Linked rich-text overflow ownership

`RichTextBlockOverflow.ContentSource` exposes the primary `RichTextBlock`
resolved during layout setup. Its immediate predecessor remains a separate
relationship: the preceding overflow supplies the next page break and inherited
document formatting. Keeping those relationships separate preserves sequential
pagination without making a public getter perform layout.

This checkpoint ports the cached-master reset/setup contract and uses that same
cache for configured context-flyout fallback and selection eligibility. It does
not complete every overflow, geometry, printing, selection or drawing contract.

## Pinned source evidence

Every native reference below uses WinUI commit
[`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de).
These are bounded reviewed members; their containing files remain partial.

| Source | Reviewed contract |
|---|---|
| [`RichTextBlockOverflow_Partial.cpp`, `get_ContentSourceImpl`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/dxaml/lib/RichTextBlockOverflow_Partial.cpp#L71-L91) | Project the primary `RichTextBlock` obtained from the core getter. |
| [`CoreImports.cpp`, `CRichTextBlockOverflow_GetMaster`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/dll/CoreImports.cpp#L3067-L3084) | Read `m_pMaster` directly; do not traverse the chain or force layout. |
| [`RichTextBlockOverflow.h`, `GetMaster`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/inc/RichTextBlockOverflow.h#L86) | Expose the same cached primary internally. |
| [`RichTextBlockOverflow.cpp`, `PreviousLinkAttached` and `PreviousLinkDetached`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L419-L475) | Clear the cached master and page state through the continuation suffix when a predecessor changes. |
| [`RichTextBlockOverflow.cpp`, `SetupLinkedBlockLayout`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L1254-L1299) | Inspect only the immediate predecessor: use a primary directly or an overflow's already resolved master. An unresolved predecessor leaves the later cache empty. Assign the master before testing block-engine, block-collection or overflowing-content availability. |
| [`RichTextBlockOverflow.cpp`, measure and empty-content handling](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L824-L880) | Actual measure performs setup. Removing an empty page does not clear the resolved master. |
| [`RichTextBlockOverflow.cpp`, `PreviousBreakUpdated`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L408-L417) | A new preceding break invalidates the continuation's measure. |
| [`RichTextBlockOverflow.cpp`, destructor](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L60-L73) | Detach the successor when its predecessor is destroyed. |
| [`RichTextBlockOverflow.cpp`, `GetContextFlyout`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L1574-L1588) and [`TextCore.cpp`, selection eligibility](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/common/TextCore.cpp#L200-L232) | Read the same cached primary for master flyout fallback and the overflow's default selection gate. |
| [`RichTextBlockOverflow.cpp`, `PreChildrenPrintVirtual`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L1490-L1501) | Print preparation also performs setup. This call site remains part of the pending print port. |

## State and layout

The private
[`OverflowLinkState`](../../../src/FerroUI.Controls/rich_text_overflow_links.rs)
holds the installed successor, the resolved primary and generations for link
requests, inherited flow and master changes. Existing `flow_parent` remains the
weak immediate predecessor. Its renamed private accessor,
`previous_flow_parent`, is used for inherited highlighters and line bounds.
Normal document and page-break lookup retains that predecessor relationship.

| Operation | Cached primary |
|---|---|
| Construct or attach a continuation | Empty. |
| Read `ContentSource`, query its registered property, or inspect context fallback | Unchanged; these reads do not resolve links or measure. |
| Measure an overflow directly after a primary | Resolve that primary. |
| Measure an overflow after an already resolved overflow | Copy its resolved primary as a weak reference. |
| Measure an overflow after an unresolved overflow | Remain empty; do not walk farther back. |
| Resolve with empty blocks or no text remaining for this page | Retain the primary. |
| Change width, font or document text | Retain the primary while ordinary page layout is invalidated. |
| Change the predecessor or detach the chain | Clear this view and its installed continuation suffix before framework callbacks. |

Only actual overflow measure invokes `setup_linked_overflow_master`. A geometry
query may already have created a framework page cache, but that query cannot
resolve a predecessor's public master as a side effect. Resolving a new master
clears that earlier page cache. The next page publication then uses the existing
continuation invalidation path to schedule the successor for another measure.
This supports out-of-order measure without replacing one-predecessor setup with
an eager chain walk.

Configured context menus now use the same pure cache read. A local non-null
overflow flyout remains available before master setup; fallback to a primary
requires that primary to have been resolved. The intermediate overflow's own
flyout does not become the later view's master flyout. The remaining menu
selection, explicit-null, clear, cancellation and placement policies are in
[CONTEXT-MENUS.md](CONTEXT-MENUS.md).

## Framework ownership and notifications

Link reconciliation reads the current effective `OverflowContentTarget` and
the edge that was actually installed. It does not trust an old/new property
event pair that an earlier derived callback may already have superseded.
An installed forward edge is accepted only when the successor's backward weak
reference names that exact predecessor.

Detaching the old edge, attaching the new edge and clearing affected masters and
page caches happen before invalidation, coercion or property callbacks. Each
notification record retains the flow and master generations it owns. A nested
reparent, layout setup or link change advances those generations, so the older
notification wave cannot erase or publish values for the newer state.

`ContentSource` continues to use its existing read-only `DirectProperty`.
FerroUI's property registry supplies untyped access and name-based bindings;
there is no duplicate plain-property metadata declaration. Its direct property
notifications describe actual cache transitions. The inherited `Document`
notification also follows the affected suffix when a link changes.

Drop handling compares weak object identity without upgrading the dying
predecessor. It detaches only the successor still owned by that installed edge.
Before severing the edge, it clones the dying predecessor's effective document
directly. A successor can no longer upgrade that predecessor at this point;
asking the successor for its old document would incorrectly return its orphan
document. Capturing the predecessor's document preserves the old notification
value for every surviving descendant, including an unmeasured suffix.
Callbacks may then reparent a survivor; generation checks preserve the new
owner.

These notifications and callback rules are FerroUI integrations. Native
`ContentSource` notification ordering was not separately measured. Native
forward targets use strong ownership; this port retains FerroUI's weak
`ElementRef` target policy. Expired weak element references do not manufacture
a strong reference to a dying object merely to publish an old property value.

## Regression coverage

Twenty tests in
[`rich_text_overflow_master_tests.rs`](../../../src/FerroUI.Controls/rich_text_overflow_master_tests.rs)
cover the new state contract. Their executable filter is
`rich_overflow_master_`.

| Area | Assertions |
|---|---|
| Public and registered property reads | Getter, typed direct-property access and untyped name lookup all remain empty before setup and report the primary afterward; the property remains read-only. |
| Real binding | `ReflectionBinding("ContentSource")` targets `Control.Tag`. The target observable records exactly null, primary A, null, primary B through measure, unlink, reparent and setup. Measuring only the predecessor leaves a later overflow's bound value empty. |
| Layout ordering | Late continuation measure waits for its immediate predecessor; a new preceding page invalidates it for retry. Empty and non-overflowing content still resolve a primary. |
| Sequential flow | The second overflow reports the primary, while its content start remains the first overflow's content end. |
| Reset and reparent | The entire affected suffix is cleared before observers; relinking requires setup again; descendant documents follow their new owner. |
| Ordinary invalidation | Font, width and text changes preserve master identity without fabricated master-change events. |
| Lifetime | Weak primary ownership, intermediate destruction, old/new document notifications before and after setup, and surviving descendants are checked separately. |
| Reentry | Derived property handlers, invalidation, reset, setup and Drop callbacks preserve newer links, resolved masters and layout. |
| Invalid input and context lookup | A rejected raw property target cannot detach another owner; context fallback and `ContentSource` use the same lazy cache. |

These are source-derived contracts and framework ownership regressions, not new
native runtime measurements. Actual compiler and test results, source and log
hashes, and any unsuccessful attempts belong in [TESTS.md](TESTS.md) and the
source-bound continuation validation record; the presence of a test definition
does not establish a pass.

## Remaining boundaries

Printing still needs its native setup call site and the wider print pipeline.
Existing geometry APIs that format before measure remain mapped separately;
this change does not establish full native formatting or selection timing.
Native cursor propagation, page-engine ownership, strong target lifetime,
automatic default/selection flyouts and other unfinished overflow members keep
their partial or pending dispositions. No containing runtime file is promoted
to complete by this checkpoint.
