# Repeater design and native integration

## Framework boundaries

The source reference is the [pinned WinUI Repeater tree](https://github.com/microsoft/microsoft-ui-xaml/tree/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater).
The public names are retained for traceability. Object ownership, dependency
properties, metadata, rendering and input use FerroUI's existing mechanisms.

| Source mechanism | FerroUI mechanism | Implementation |
|---|---|---|
| `DependencyObject`, `FrameworkElement` and generated C++/WinRT classes | `FerroObject`, `Control`/`Panel`, `Ref<T>`, `WeakRef<T>` and `ferro_class!` | [ItemsRepeater](../../../src/FerroUI.Controls.Repeater/items_repeater.rs), [layout classes](../../../src/FerroUI.Controls.Repeater/layouts/layout.rs) |
| Dependency properties and property-change callbacks | `StyledProperty`, `AttachedProperty`, `ferro_properties!`, class initialization and normal invalidation | [Repeater properties](../../../src/FerroUI.Controls.Repeater/items_repeater.rs), [native registration](../../../src/FerroUI.Controls/register_types.rs) |
| `IInspectable` data and WinRT collection interfaces | `BoxedValue`, `ItemsSource`, `ItemsSourceView`, collection notifications and `IKeyIndexMapping` | [source view](../../../src/FerroUI.Controls/items_source_view.rs), [source](../../../src/FerroUI.Controls/items_source.rs), [key mapping](../../../src/FerroUI.Controls/i_key_index_mapping.rs) |
| `DataTemplate`, selector, `IElementFactory` | Existing `IDataTemplate`/recycling-template contracts and native factory traits/classes | [template adapter](../../../src/FerroUI.Controls.Repeater/item_template_wrapper.rs), [factory](../../../src/FerroUI.Controls.Repeater/element_factory.rs) |
| Effective viewport, scrolling surface and scroll anchor | `Visual`, `Layoutable`, native `ScrollViewer`, `IScrollAnchorProvider`, compatibility host | [viewport manager](../../../src/FerroUI.Controls.Repeater/viewport_manager.rs), [scroll host](../../../src/FerroUI.Controls.Repeater/items_repeater_scroll_host.rs) |
| WinRT events and event revokers | `HandlerList`, snapshots, explicit `IDisposable` subscriptions and weak back-references | [control events](../../../src/FerroUI.Controls.Repeater/items_repeater_event_args.rs), [transition provider](../../../src/FerroUI.Controls.Repeater/item_collection_transition_provider.rs) |
| Rendering callbacks and `QueryPerformanceCounter` | Native global animation clock, dispatcher render priority and dispatcher time | [build-tree scheduler](../../../src/FerroUI.Controls.Repeater/build_tree_scheduler.rs) |
| Composition animations | Native animation/transform/opacity services with cancellation and transient-value restoration | [animation manager](../../../src/FerroUI.Controls.Repeater/animation_manager.rs), [lined-flow transition provider](../../../src/FerroUI.Controls.Repeater/lined_flow_layout_item_collection_transition_provider.rs) |
| WinRT `ICustomPropertyProvider` and activation metadata | `ferro_class_info!`, `ferro_markup_type!`, `ValueTypes`, native property notifications | [layout markup](../../../src/FerroUI.Controls.Repeater/markup.rs), [service markup](../../../src/FerroUI.Controls.Repeater/services_markup.rs), [selection metadata](../../../src/FerroUI.Controls.Repeater/selection/markup.rs) |
| UI Automation peer and tab-focus ordering | Native automation peer and custom keyboard navigation | [automation peer](../../../src/FerroUI.Controls.Repeater/repeater_automation_peer.rs), [control](../../../src/FerroUI.Controls.Repeater/items_repeater.rs) |

Repeater does not introduce another renderer, data-binding engine, dependency
property store, focus manager or dispatcher. Native controls remain normal
FerroUI children with styles, bindings, logical/visual ancestry and automation.

## Element ownership

[`VirtualizationInfo`](../../../src/FerroUI.Controls.Repeater/virtualization_info.rs)
records source index, stable key, owner state, pin count, layout bounds and phased
binding state. [`ViewManager`](../../../src/FerroUI.Controls.Repeater/view_manager.rs)
coordinates ownership; [`UniqueIdElementPool`](../../../src/FerroUI.Controls.Repeater/unique_id_element_pool.rs)
temporarily retains elements during a keyed reset.

| Owner | Meaning | Exit condition |
|---|---|---|
| Element factory | The control can be supplied by its source/template pool | A layout requests it and preparation binds it to an item |
| Layout | The element participates in the current layout's realization | A later pass omits it, the layout explicitly recycles it, or the collection removes it |
| Pinned pool | The element is outside the layout window but focus or another pin retains it | It is requested again or its final pin is released |
| Unique-ID reset pool | A reset may reuse the element for the same stable key | A new source index claims the key; unmatched elements clear after the reset pass |
| Animator | A removal transition temporarily retains the control | Transition completion returns it to its factory without recycling a newly reused element twice |

`ForceCreate` can request a second element for an index. `SuppressAutoRecycle`
transfers responsibility to the layout until it explicitly recycles the
element. Index-change events must observe the new mapping, preparation must
expose the prepared parent/data/index, and clearing must preserve the lifecycle
ordering expected by the factory. The conformance and service tests exercise
these boundaries rather than only testing the final child count.

Native tree ownership uses strong child references and weak back-references.
Pools retain reusable controls but do not keep dead owners alive. Template pools
retain template identity while elements remain associated, and prune dead
template registrations. Explicit disposal remains necessary for event
subscriptions: the existing FerroUI convention does not interpret dropping an
`IDisposable` handle as automatic unsubscription.

## Native focus ordering

WinUI's `ChildrenInTabFocusOrderIterable` exposes realized children in source
order, which can differ from visual insertion order after recycling. The port
extends the existing
[`ICustomKeyboardNavigation`](../../../src/FerroUI.Base/input/i_custom_keyboard_navigation.rs)
with `get_children_in_tab_focus_order`. Its default returns `None`, preserving
normal visual-child enumeration for other controls. Repeater returns its
realized direct children, excluding pooled visuals, in source order.

The native [focus helper](../../../src/FerroUI.Base/input/focus_helpers.rs) uses
this projection for traversal when focus enters, leaves or wraps through a
container. Repeater also consults it while walking descendants, so nested
repeaters retain their own source order. This integrates the ordering contract
with FerroUI's focus manager and its `Once`/`Cycle` semantics rather than
maintaining a separate tab-focus engine. Routed focus events update each
repeater's own pin contribution; a nested focus target must not add multiple
pins to the same outer container.

Additional [native integration scenarios](../../../src/FerroUI.Controls.Repeater/view_manager_acceptance_tests.rs)
exercise `Once`, `Local`, `Cycle`, `Contained` and `None` navigation, exclude a
discarded active item while its visual remains pooled or owned by the animator,
and remember nested `Once` targets without duplicating ancestor pins. Direct
removal and detaching-factory scenarios verify that ancestor focus pins are
released before the visual parent disappears, while preserving an independent
manual pin and any new focus target selected by a recycling callback. These are
FerroUI service regressions beyond the pinned test-declaration inventory; their
presence is separate from the execution checkpoints in [VALIDATION.md](VALIDATION.md).

## Layout contexts and state

[`LayoutContext`](../../../src/FerroUI.Controls.Repeater/layouts/layout_context.rs)
is an extensible native object. `VirtualizingLayoutContext` exposes source access,
element creation/recycling, visible and realization rectangles, a recommended
anchor, layout origin and opaque per-host state. `NonVirtualizingLayoutContext`
exposes fixed children. Both adapter directions preserve the layout-state bridge.

`LayoutContextHost` lets the repeater and fixed-child host supply these services
without a parallel Windows object model. Implementations must not retain a
`RefCell` borrow across element creation, layout virtual calls or application
events. A factory, template, layout or callback can reenter native framework code.
The control guards invalid collection changes and recursive measure/arrange,
and restores those guards even when a programmer-error panic is caught by tests.

Stack, uniform-grid and flow layouts keep independent state in the context, so
a layout instance can be used by multiple hosts where the source contract allows
it. Lined-flow's stateful contract rejects simultaneous use by multiple hosts;
detaching and reusing it must discard old anchors, frozen-window state and queued
asynchronous invalidation.

## Layout algorithms

`FlowLayoutAlgorithm` and `ElementManager` provide shared contiguous realization,
anchor-based forward/backward generation, collection-change index tracking,
measure/arrange bounds and recycling. The callback trait supplies layout-specific
measurement, provisional arrangement, line breaks, anchoring and extent
estimation. Stack, flow and uniform-grid retain their distinct policies rather
than all becoming one fixed-size panel.

`LinedFlowLayout` separates source metadata, metric history, sized windows,
realization windows and row distribution. Full-source metadata can use the fast
path. Partial metadata requests retain covered ranges and request missing
intervals; a declined interval falls back to measured control dimensions for
the window. Sizing a row is distinct from keeping every control in that row
realized. Arrangement preserves the source's `GetFirstFullyRealizedLineIndex`
rule: skip an incompletely realized leading row, then arrange the retained
prefix of a trailing row without creating its missing controls.

FerroUI's `DesiredSize` already includes the control margin. Allocation limits
therefore need to use the same outer-size coordinate system. A maximum width
must not remove a horizontal margin already included by native measure, and
conflicting minimum/maximum constraints must follow FerroUI's native minimum
precedence. Metadata constraints are a separate source contract; their upstream
processing order is retained and covered explicitly.

Aspect-ratio history is bounded and weighted. Cache trimming uses absolute
source indices when selecting the blocks closest to a distant viewport. Public
item-to-line locks, frozen ranges and anchor retention must be invalidated at
the same logical transitions as the source; the deliberate pinned-source cache
correction is recorded in [DEVIATIONS.md](DEVIATIONS.md).

## Source changes and hierarchical selection

Repeater consumes the existing source view's insert/remove/replace/move/reset
notifications. It preserves unaffected elements, remaps realized and pinned
indices, and uses `IKeyIndexMapping` when the source offers stable keys. A reset
without stable identity must not accidentally reuse an old item binding by
numeric index.

The native [`ItemsSourceView`](../../../src/FerroUI.Controls/items_source_view.rs)
reads its current count through `IItemsList::count`. WinUI instead caches the
result of its overridable `GetSizeCore` and refreshes it on a collection change.
The port retains FerroUI's live count contract and introduces no count cache.
The source-derived `CanQueryElementFactory` scenario preserves data-read and
factory-callback counts, element identity and parent identity; its WinRT
size-override invocation counts do not apply to this native collection boundary.
This adaptation is recorded as [WR-22](DEVIATIONS.md).

The hierarchical selection model is in
[`repeater::selection`](../../../src/FerroUI.Controls.Repeater/selection/mod.rs).
Its source tree is separate from the realized visual tree. `IndexPath` identifies
arbitrary-depth nodes; child-source requests are scoped to a callback lifetime.
Selected index/item views are shared handles and materialize source data lazily.
They implement the native read-only indexed-list contract. Bindings observe the
model's `SelectedItems` and `SelectedIndices` property notifications and obtain
replacement views, so they can drive another FerroUI repeater without flattening
the full hierarchy on every selection change. The views themselves do not publish
collection notifications; their captured count and weak-node access semantics
are detailed in [API-SHAPES.md](API-SHAPES.md).

The existing [inclusive `IndexRange`](../../../src/FerroUI.Controls/selection/index_range.rs)
is reused. This avoids a second, subtly different interval implementation for
selection. The source's ignored single-item-range double-notification concern is
recorded explicitly in the scenario map; an opposite expected result is never
counted as an equivalent test.

## Scheduling, phasing and transitions

The [build-tree scheduler](../../../src/FerroUI.Controls.Repeater/build_tree_scheduler.rs)
uses native rendering pulses and dispatcher time. Work is ordered by phase and
registration identity, has a configurable budget, and yields remaining work to
another frame. The dispatcher render queue is the fallback for hosts without a
global rendering clock. No background thread or blocking wait is required.

[`Phaser`](../../../src/FerroUI.Controls.Repeater/phaser.rs) keeps phase-zero
binding synchronous, schedules later phases after preparation, prioritizes
visible controls and cancels work when a binding generation is recycled. A
callback can recycle its own element; its old phase must not be restored after
return. Repeated/nonincreasing phases are invalid input to the phasing contract.

The [transition provider](../../../src/FerroUI.Controls.Repeater/item_collection_transition_provider.rs)
queues independent batches for the next rendering frame, retains started
transitions for their keepalive interval and completes unstarted transitions.
The [transition manager](../../../src/FerroUI.Controls.Repeater/transition_manager.rs)
combines collection and layout triggers and filters completion to its own
elements. Motion preferences come from FerroUI `IPlatformSettings`; local opt-out
and the provider's policy remain separate from the native user preference.

Weak callbacks and generation checks protect detach/reuse. A queued callback
from an old host cannot invalidate or mutate a replacement context. Timer
shutdown is required whenever arrangement metadata makes delayed control
measurement unnecessary, including the regular metadata path.

`LinedFlowLayout` also stops its timer in `Drop`. The release regression queues
an immediately due callback, drops the last layout reference, drains pending
jobs and lays out a fresh host. This validates the native UI-thread lifetime;
FerroUI's non-`Send` `Ref`/`Rc` ownership has no equivalent of a COM finalizer
releasing a UI object from an arbitrary thread.

## Extending and reviewing the port

Add an implementation and its source mapping together. Keep `ferro_properties!`
and `ferro_class_info!` authoritative for registration. Expose properties through
native metadata rather than special-case XML parsing. Register concrete classes
in the crate's type table and use the current runtime/compiled XAML regression
fixtures to verify template dictionaries, interface handles and nullability.

Before calling a source family complete, review its full mapped C++ source,
including private branches; execute the corresponding Rust scenarios; inspect
the unresolved API/scenario map; and record platform-dependent work separately.
Do not use test-function counts or source-file correspondence as substitutes for
that review.
