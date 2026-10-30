# Native API shapes and signature decisions

The complete declaration list is in [API.md](API.md). Its pinned source is
[ItemsRepeater.idl](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/ItemsRepeater.idl)
and the four private/automation/test-hook IDL files recorded in that map.
This page explains the signature changes required to use the native FerroUI
type system. An identifier match in the generated map remains a navigation
check; the constructor, default, error and lifecycle contracts need the scenario
evidence described in [VALIDATION.md](VALIDATION.md).

## Objects, values and nullability

| WinUI contract | Native Rust form | Meaning |
|---|---|---|
| Layout/control/dependency object | `Ref<T>` and `WeakRef<T>` through `ferro_class!` | Native inheritance, virtual dispatch, properties and tree ownership; weak callbacks do not keep detached hosts alive |
| Nonvisual shared service or identity-bearing event data | `Rc<T>` or a cloneable handle containing `Rc<T>` | Shared state and explicit identity without introducing a second COM object model |
| `Object` item data | `Option<BoxedValue>` | Arbitrary native values remain nullable; a null data item is distinct from an absent realized control |
| `Object` collection source | `Option<ItemsSource>` | Native list/notification/key-mapping contracts preserve collection identity; an explicitly supplied `ItemsSourceView` is preserved |
| Nullable `UIElement`, layout or provider | `Option<Ref<Control>>`, `Option<Ref<Layout>>`, or `Option<Ref<ItemCollectionTransitionProvider>>` | Absence is explicit in the type; required element arguments use `Ref<Control>` |
| `IReference<Boolean>` selection state | `Option<bool>` | `None` represents an indeterminate/partly selected branch; it does not collapse into `false` |
| `Double[]` metadata input | `&[f64]` | The callback supplies borrowed input and the arguments copy the accepted interval into owned request storage |
| Transition `IVector` | `Vec<Rc<ItemCollectionTransition>>` | The override receives an owned batch containing shared transition identities |
| Read-only selected-item/index views | `SelectedItems<T>` plus native metadata view adapters | Lazy views capture their count and hold weak selection nodes; the model publishes replacement views through property notifications |
| `IStringable` on `IndexPath` | `Display`/`Debug` | The source's `R.0.1` formatting remains available through normal Rust formatting |

These mappings are implemented in the native
[source contracts](../../../src/FerroUI.Controls/items_source.rs),
[layout contexts](../../../src/FerroUI.Controls.Repeater/layouts/layout_context.rs),
[selection service](../../../src/FerroUI.Controls.Repeater/selection/selection_model.rs),
[selected views](../../../src/FerroUI.Controls.Repeater/selection/selected_items.rs),
[metadata request arguments](../../../src/FerroUI.Controls.Repeater/layouts/lined_flow_layout_items_info_requested_event_args.rs)
and [transition provider](../../../src/FerroUI.Controls.Repeater/item_collection_transition_provider.rs).

`ItemsSourceView::count()` uses the native `IItemsList::count` contract to read
the current size. It does not expose WinRT's `GetSizeCore` override or maintain
that implementation's count cache. Collection notifications, source identity,
stable keys and indexed data access still use the existing native source
interfaces. The resulting difference in size-query invocation counts is
explicitly recorded in [WR-22](DEVIATIONS.md); it is not treated as an equivalent
assertion in the source factory scenario.

`SelectedItems<T>` follows the pinned
[`SelectedItems.h`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/SelectedItems.h)
and [`SelectionModel.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/SelectionModel.cpp):
the count is captured when the view is constructed, while indexed access reads
the current selected ranges of its referenced nodes. Access rejects a discarded
node with `SelectionError::SelectionChanged`. A still-live node can expose a
changed selection through an older view; there is no selection-version check or
immutable item snapshot. The indexed-list adapters themselves do not publish
collection notifications. Native bindings observe the model's `SelectedItems`
and `SelectedIndices` property notifications and obtain refreshed views.

## Overload spelling

Rust methods have distinct names. The generator records each overload using its
full IDL signature, so removing a grouped/owner/options variant cannot be hidden
by the presence of the shorter method with the same upstream name.

| Source overload | Native entry point |
|---|---|
| `VirtualizingLayoutContext.GetOrCreateElementAt(index)` | `get_or_create_element_at(index)` |
| `VirtualizingLayoutContext.GetOrCreateElementAt(index, options)` | `get_or_create_element_at_with_options(index, options)` |
| `RecyclePool.PutElement(element, key)` | `put_element(element, key)` |
| `RecyclePool.PutElement(element, key, owner)` | `put_element_with_owner(element, key, owner)` |
| `RecyclePool.TryGetElement(key)` | `try_get_element(key)` |
| `RecyclePool.TryGetElement(key, owner)` | `try_get_element_with_owner(key, owner)` |
| `IndexPath.CreateFrom(index)` | `create_from(index)` |
| `IndexPath.CreateFrom(group, item)` | `create_from_group_and_item_index(group, item)` |
| `SelectionModel.SetAnchorIndex(index)` | `set_anchor_index_flat(index)` |
| `SelectionModel.SetAnchorIndex(group, item)` | `set_anchor_index_with_group(group, item)` |
| `SelectionModel.Select(index)` / `Deselect(index)` | `select(index)` / `deselect(index)` |
| `SelectionModel.Select(group, item)` / `Deselect(group, item)` | `select_with_group(group, item)` / `deselect_with_group(group, item)` |
| `SelectionModel.IsSelected(index)` | `is_selected(index)` |
| `SelectionModel.IsSelected(group, item)` | `is_selected_with_group(group, item)` |
| `SelectionModel.SelectRangeFromAnchor(index)` / `DeselectRangeFromAnchor(index)` | `select_range_from_anchor(index)` / `deselect_range_from_anchor(index)` |
| `SelectionModel.SelectRangeFromAnchor(group, item)` / `DeselectRangeFromAnchor(group, item)` | `select_range_from_anchor_with_group(group, item)` / `deselect_range_from_anchor_with_group(group, item)` |

The `AnchorIndex` property setter is separately named
`set_anchor_index(Option<IndexPath>)`; it must not be mistaken for the flat-index
method above. Arbitrary-depth `SelectAt`, `DeselectAt` and range operations borrow
an `IndexPath`. `IndexPath::create_from_indices` accepts an iterator of `i32`
values rather than requiring a WinRT vector.

Source method names and overload arities remain in the native markup metadata
where reflective invocation needs them. Rust callers use the explicit names.
See [pool APIs](../../../src/FerroUI.Controls.Repeater/recycle_pool.rs),
[selection APIs](../../../src/FerroUI.Controls.Repeater/selection/selection_model.rs)
and [selection markup](../../../src/FerroUI.Controls.Repeater/selection/markup.rs).

## Errors, callbacks and extension points

Recoverable pool/template, metadata-range and selection errors use
`ElementFactoryError`, `LinedFlowLayoutError` and `SelectionError` with `Result`.
For example, pool lookup returns `Result<Option<Ref<Control>>, ElementFactoryError>`:
an empty pool is successful absence, while an invalid owner is an error.
An expired selection callback returns `SelectionError::EventArgsExpired` when
its source or source path is accessed. Array-length and requested-range failures
remain distinguishable in lined-flow metadata callbacks.

Invalid layout reentrancy and an unimplemented required base virtual remain
programmer errors. Native base classes reject those calls; a panic in that path
represents the source precondition/abstract-core failure, not a missing normal
implementation. Such behavior must stay paired with the deriving-override and
invalid-call tests in [the layout contract suite](../../../src/FerroUI.Controls.Repeater/layout_contract_tests.rs).

`ferro_class!` traits preserve overridable layout/factory/provider operations.
`FlowLayout::get_extent` groups each realized boundary's element, source index
and bounds into `FlowLayoutRealizedElement`; the source's three related
arguments remain one coherent Rust value. The callback preserves separate
first/last boundary values. The algorithm delegates through the layout virtuals
instead of bypassing an application override.

Native event subscription methods return `Rc<dyn IDisposable>`. Explicit
`dispose()` detaches the handler; simply dropping the handle is not the
framework's unsubscribe convention. Rust callbacks receive the typed arguments;
native markup event adapters supply the sender and arguments required by the
markup delegate contract. Reentrant event delivery takes a handler snapshot.

## Native scrolling and bring targets

Repeater registers candidates with the nearest native `IScrollAnchorProvider`
ancestor, following pinned
[`ViewportManager::EnsureScroller`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/ViewportManager.cpp#L943).
A native `ScrollContentPresenter` therefore owns the candidates even when an
`ItemsRepeaterScrollHost` wraps its `ScrollViewer`. The compatibility host still
supplies aligned and animated scroll requests. Native viewer anchoring remains
observable through `ScrollViewer::current_anchor()`.

FerroUI's default bring rectangle is the target's full arranged `Bounds`.
[`Control::try_execute_bring_into_view`](../../../src/FerroUI.Controls/control_extensions.rs)
waits until measure and arrange are valid before computing that rectangle. The
[compatibility host](../../../src/FerroUI.Controls.Repeater/items_repeater_scroll_host.rs)
also waits for valid target and ancestor layout before capturing its transformed
destination. This prevents a recycled control's parked bounds from determining
the request and keeps a repeated aligned request stable.

The pinned WinUI
[`UIElement::StartBringIntoViewWithOptionsImpl`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/dxaml/xcp/dxaml/lib/UIElement_Partial.cpp#L1694)
instead reads `RenderSize` immediately when `TargetRect` is omitted.
[`ItemsRepeater::GetOrCreateElementImpl`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/ItemsRepeater.cpp#L312)
measures a new target without arranging it. Its first disconnected request can
therefore capture a zero-height rectangle. This is a source transport
difference, recorded under [WR-27](DEVIATIONS.md), rather than a change to the
flow algorithm's realization boundaries.

The [aligned-bring scenario](../../../src/FerroUI.Controls.Repeater/viewport_tests.rs)
retains the source's 400 × 600 viewport, wrapped bound text, 94-DIP border and
5-DIP margin. A row has a 104-DIP desired height; the arranged target is
390 × 94 DIPs. Centering item 100's full rectangle gives
`100 × 104 + 5 − (600 − 94) / 2 = 10152`. The comparison with the pinned
[source scenario](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/APITests/EffectiveViewportScrollViewerTests.cs#L370)
is explicit in the test:

| Target and vertical alignment | Pinned source realized indices | Native realized indices | Native offset |
|---|---|---|---|
| Disconnected 100, center | 96–103 | 96–104 | 10152 |
| Already realized 103, center with animation | 99–107 | 99–107 | 10464 |
| Disconnected 0, center, clamped to the start | 0–6 | 0–6 | 0 |
| Disconnected 20, top | 19–26 | 19–26 | 2085 |

Layout-visible rectangles use the layout's coordinate origin, while native
scroll offsets and arranged child bounds use physical content coordinates.
Pinned [`StackLayout::GetExtent`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/StackLayout.cpp#L191)
can retain a negative extent origin after making a distant anchor. For example,
bringing row 500 of a 20-DIP stack establishes origin −10000. Scrolling back
to native offset zero can correctly report layout-visible Y = −10000 while
row 0 is arranged at Y = 0. The native regression verifies the zero scroll
offset, the visible rectangle relative to the extent origin, row 0's bounds
and recycling of the distant window.

## Defaults and sentinel values

The IDL's attributes and the source constructors jointly define defaults. A
public zero default must not erase an internal uninitialized measurement
sentinel. The following distinctions are significant for behavior:

| Contract | Native default / distinction | Source and implementation |
|---|---|---|
| Repeater cache lengths | `2.0`; values must be finite and nonnegative | [Repeater properties](../../../src/FerroUI.Controls.Repeater/items_repeater.rs) |
| Stack layout | Vertical orientation, zero spacing, virtualization enabled | [Stack layout](../../../src/FerroUI.Controls.Repeater/layouts/stack_layout.rs) |
| Uniform grid | Horizontal orientation, zero public minimum dimensions/spacings, start justification, no stretch, maximum `-1` | [Uniform grid](../../../src/FerroUI.Controls.Repeater/layouts/uniform_grid_layout.rs) |
| Untouched uniform-grid item dimensions | Internal `NaN` fields cause the first item to supply its natural size until a dimension property change establishes a numeric value; the internal sentinel is separate from the public zero default | [Uniform grid initialization](../../../src/FerroUI.Controls.Repeater/layouts/uniform_grid_layout.rs) |
| Flow layout | Horizontal orientation, zero line/item spacing, start line alignment | [Flow layout](../../../src/FerroUI.Controls.Repeater/layouts/flow_layout.rs) |
| Lined flow | Start justification, no stretch, zero spacing, `LineHeight = NaN`, `ActualLineHeight = 0` before measurement | [Pinned constants](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/LinedFlowLayout.h), [native layout](../../../src/FerroUI.Controls.Repeater/layouts/lined_flow_layout.rs) |
| Metadata global minimum/maximum width | `-1` denotes unspecified; negative assignments normalize to that sentinel | [Request arguments](../../../src/FerroUI.Controls.Repeater/layouts/lined_flow_layout_items_info_requested_event_args.rs) |

Default-value assertions, invalid argument tests and actual host behavior remain
separate review gates. The signature inventory records all declared members;
it does not infer these behaviors from a matching identifier.
