//! WinUI's `ItemsStackPanel` (`dxaml/xcp/dxaml/lib/ItemsStackPanel_Partial.cpp` and the metadata
//! of `Microsoft.UI.Xaml.Controls.cs` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), on FerroUI's `VirtualizingStackPanel` (docs/porting/winui-calculator-gaps.md, LV-1).

use super::{GroupStyle, GroupedItemsView, ListViewBase, ListViewHeaderItem};
use crate::items_source::ItemsChangedEventArgs;
use crate::virtualizing_stack_panel::{IItemSpacing, ItemInsets};
use crate::{
    Control, ControlImpl, ItemsControl, ItemsSource, ItemsSourceView, PanelImpl, ScrollViewer, VirtualizingPanelImpl,
    VirtualizingPanelImplExt,
};
use crate::VirtualizingStackPanel;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt, Orientation};
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_property, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, Thickness, Vector, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::rc::Rc;

/// What stays in view when items are added or removed (WinUI's `ItemsUpdatingScrollMode`, as
/// `ModernCollectionBasePanel_ViewportManagement_Partial.cpp` tracks it).
///
/// - `KeepItemsInView`: the first visible item stays where it is in the viewport, unless the list
///   is scrolled to its start.
/// - `KeepScrollOffset`: the offset stays; the items move when items before them change.
/// - `KeepLastItemInView`: a list scrolled to its end stays at its end, so that an added last
///   item comes into view; scrolled elsewhere, the first visible item stays where it is.
///
/// The panel keeps the item in place after the layout pass of the change, with the residual
/// distance the scroll anchoring of the scroll viewer left: virtualization recycles the anchor
/// when items are added before the realized range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ItemsUpdatingScrollMode {
    #[default]
    KeepItemsInView = 0,
    KeepScrollOffset = 1,
    KeepLastItemInView = 2,
}

ferro_markup_enum!(
    ItemsUpdatingScrollMode { KeepItemsInView, KeepScrollOffset, KeepLastItemInView },
    { namespace: "FerroUI.Controls" }
);

/// Where the headers of the groups are (WinUI's `GroupHeaderPlacement`): before the items of the
/// group, along the stack, or beside them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GroupHeaderPlacement {
    #[default]
    Top = 0,
    Left = 1,
}

ferro_markup_enum!(GroupHeaderPlacement { Top, Left }, { namespace: "FerroUI.Controls.Primitives" });

/// The virtualizing items panel of a list view (WinUI's `ItemsStackPanel`): FerroUI's
/// [`VirtualizingStackPanel`] (orientation, cache length, recycling) with WinUI's visible and
/// cache indexes, and the headers of a grouped source ([`GroupedItemsView`]) (#1258).
///
/// An index stays the index of an item. The panel keeps the space of a header as an inset of the
/// first item of its group, so the virtualization, the size estimates and the scrolling of the
/// base apply unchanged, and realizes only the headers of the realized groups, and the pinned one.
#[repr(C)]
pub struct ItemsStackPanel {
    base: VirtualizingStackPanel,
    groups: RefCell<Option<Rc<PanelGroups>>>,
    viewport_changed: RefCell<Option<Subscription>>,
    /// The item kept in place across a change of the items, if any (WinUI's `m_viewportBehavior`).
    tracked: Cell<Option<Tracked>>,
    /// The layout pass after a change of the items, while an item is tracked.
    tracking: RefCell<Option<Subscription>>,
}

/// What a change of the items keeps in view, for the layout passes left in which an estimated
/// position is corrected again.
#[derive(Clone, Copy)]
struct Tracked {
    target: TrackedTarget,
    passes: u8,
}

#[derive(Clone, Copy)]
enum TrackedTarget {
    /// The first visible item: its index after the change and the distance of its start from the
    /// start of the viewport.
    Item { index: i32, offset: f64 },
    /// The end of the list (WinUI's `isTrackingExtentEnd`).
    End,
}

/// The headers of the groups of a panel.
struct PanelGroups {
    source: ItemsSource,
    headers: RefCell<BTreeMap<usize, Ref<ListViewHeaderItem>>>,
    pool: RefCell<Vec<Ref<ListViewHeaderItem>>>,
    /// The widest header beside its items (`GroupHeaderPlacement::Left`), which only grows, so
    /// that the items do not move sideways while the list scrolls.
    beside_size: Cell<f64>,
}

impl PanelGroups {
    fn view(&self) -> &GroupedItemsView {
        self.source.downcast_ref::<GroupedItemsView>().expect("the source of the groups is a grouped view")
    }

    fn style(&self, panel: &ItemsStackPanel) -> Option<Ref<GroupStyle>> {
        let list = panel.items_control()?.cast::<ListViewBase>()?;
        let styles = list.group_style();
        (styles.count() > 0).then(|| styles.get(0))
    }

    /// Whether the group at `index` shows a header.
    fn shows(&self, panel: &ItemsStackPanel, index: usize) -> bool {
        self.view().group_len(index) > 0 || !self.style(panel).is_some_and(|style| style.hides_if_empty())
    }

    /// The groups whose headers come before the item that starts the group at `group`: the empty
    /// groups right before it, and itself.
    fn run_of(&self, group: usize) -> RangeInclusive<usize> {
        let view = self.view();
        let mut first = group;
        while first > 0 && view.group_len(first - 1) == 0 {
            first -= 1;
        }
        first..=group
    }

    /// The empty groups at the end of the list, when the group at `group` is the last with items:
    /// their headers follow its last item. Any other empty group is in the run of the next group.
    fn trailing_of(&self, group: usize) -> std::ops::Range<usize> {
        let view = self.view();
        let mut end = group + 1;
        while end < view.group_count() && view.group_len(end) == 0 {
            end += 1;
        }
        if end == view.group_count() { group + 1..end } else { group + 1..group + 1 }
    }

    fn header(&self, panel: &ItemsStackPanel, group: usize) -> Ref<ListViewHeaderItem> {
        if let Some(header) = self.headers.borrow().get(&group) {
            return header.clone();
        }
        let pooled = self.pool.borrow_mut().pop();
        let header = pooled.unwrap_or_else(|| {
            let header = ListViewHeaderItem::new();
            header.set_z_index(1);
            let control: Ref<Control> = header.clone().upcast();
            panel.add_internal_child(&control);
            header
        });
        header.set_is_visible(true);
        let style = self.style(panel);
        header.set_content_template(style.as_ref().and_then(|style| style.header_template()));
        if let Some(theme) = style.as_ref().and_then(|style| style.header_container_style()) {
            header.set_theme(theme);
        }
        header.set_content(self.view().group_at(group).and_then(|group| group.group()));
        self.headers.borrow_mut().insert(group, header.clone());
        header
    }

    /// The size of the header of `group` along the stack, or across it when it is beside.
    fn header_size(&self, panel: &ItemsStackPanel, group: usize, available: Size) -> f64 {
        if !self.shows(panel, group) {
            return 0.0;
        }
        let header = self.header(panel, group);
        let horizontal = panel.orientation() == Orientation::Horizontal;
        let beside = panel.group_header_placement() == GroupHeaderPlacement::Left;
        let constraint = match horizontal {
            true => Size::new(f64::INFINITY, available.height),
            false => Size::new(available.width, f64::INFINITY),
        };
        header.measure(constraint);
        let desired = header.desired_size();
        match (horizontal, beside) {
            (false, false) | (true, true) => desired.height,
            (true, false) | (false, true) => desired.width,
        }
    }

    /// The padding of a group: before and after it along the stack, and across it.
    fn padding(panel: &ItemsStackPanel) -> (f64, f64, f64, f64) {
        let padding = panel.group_padding();
        match panel.orientation() {
            Orientation::Horizontal => (padding.left, padding.right, padding.top, padding.bottom),
            _ => (padding.top, padding.bottom, padding.left, padding.right),
        }
    }

    fn insets(&self, panel: &ItemsStackPanel, index: i32, available: Size) -> ItemInsets {
        let Ok(index) = usize::try_from(index) else { return ItemInsets::default() };
        let view = self.view();
        let Some((group, position)) = view.group_of(index) else { return ItemInsets::default() };
        let (before, after, cross_start, cross_end) = Self::padding(panel);
        let mut insets = ItemInsets { leading: 0.0, trailing: 0.0, cross_start, cross_end };
        let beside = panel.group_header_placement() == GroupHeaderPlacement::Left;
        if position == 0 {
            insets.leading = before;
            for shown in self.run_of(group) {
                let size = self.header_size(panel, shown, available);
                if beside {
                    self.beside_size.set(self.beside_size.get().max(size));
                } else {
                    insets.leading += size;
                }
            }
        }
        if position + 1 == view.group_len(group) {
            insets.trailing = after;
            if !beside {
                for shown in self.trailing_of(group) {
                    insets.trailing += self.header_size(panel, shown, available);
                }
            }
        }
        if beside {
            insets.cross_start += self.beside_size.get();
        }
        insets
    }

    /// Keeps the headers of the realized groups and of the pinned one, and pools the others.
    fn update_headers(&self, panel: &ItemsStackPanel, available: Size) {
        let view = self.view();
        let mut needed: Vec<usize> = Vec::new();
        let (first, last) = (panel.first_realized_index(), panel.last_realized_index());
        if first >= 0 {
            for index in first..=last {
                let Some((group, position)) = view.group_of(index as usize) else { continue };
                if position == 0 {
                    needed.extend(self.run_of(group));
                }
                if position + 1 == view.group_len(group) {
                    needed.extend(self.trailing_of(group));
                }
            }
            if panel.are_sticky_group_headers_enabled() {
                if let Some((group, _)) = view.group_of(first.max(panel.first_visible_index()) as usize) {
                    needed.push(group);
                }
            }
        }
        needed.retain(|group| self.shows(panel, *group));
        let stale: Vec<usize> =
            self.headers.borrow().keys().copied().filter(|group| !needed.contains(group)).collect();
        for group in stale {
            if let Some(header) = self.headers.borrow_mut().remove(&group) {
                header.set_is_visible(false);
                header.set_content(None);
                self.pool.borrow_mut().push(header);
            }
        }
        for group in needed {
            self.header_size(panel, group, available);
        }
    }

    /// Places the headers: each before the first item of its group; the header of the group at
    /// the start of the viewport stays there until the next header pushes it away.
    fn arrange_headers(&self, panel: &ItemsStackPanel, final_size: Size) {
        let view = self.view();
        let horizontal = panel.orientation() == Orientation::Horizontal;
        let beside = panel.group_header_placement() == GroupHeaderPlacement::Left;
        let (_, _, cross_start, cross_end) = Self::padding(panel);
        let cross = if horizontal { final_size.height } else { final_size.width };
        let size_of = |header: &Ref<ListViewHeaderItem>| {
            let desired = header.desired_size();
            if horizontal { desired.width } else { desired.height }
        };
        let mut natural: BTreeMap<usize, f64> = BTreeMap::new();
        let (first, last) = (panel.first_realized_index(), panel.last_realized_index());
        for index in first.max(0)..=last {
            let Some((group, position)) = view.group_of(index as usize) else { continue };
            let Some(container) = panel.container_from_index(index) else { continue };
            let bounds = container.bounds();
            let (start, end) = if horizontal { (bounds.x, bounds.right()) } else { (bounds.y, bounds.bottom()) };
            if position == 0 {
                let mut u = start;
                if beside {
                    natural.insert(group, u);
                } else {
                    for shown in self.run_of(group).rev() {
                        let Some(header) = self.headers.borrow().get(&shown).cloned() else { continue };
                        u -= size_of(&header);
                        natural.insert(shown, u);
                    }
                }
            }
            if position + 1 == view.group_len(group) && !beside {
                let mut u = end;
                for shown in self.trailing_of(group) {
                    let Some(header) = self.headers.borrow().get(&shown).cloned() else { continue };
                    natural.insert(shown, u);
                    u += size_of(&header);
                }
            }
        }
        let viewport = panel.view_port();
        let viewport_start = if horizontal { viewport.x } else { viewport.y };
        let pinned = panel
            .are_sticky_group_headers_enabled()
            .then(|| panel.first_visible_index())
            .filter(|index| *index >= 0)
            .and_then(|index| view.group_of(index as usize))
            .map(|(group, _)| group);
        for (group, header) in self.headers.borrow().iter() {
            let size = size_of(header);
            let mut u = natural.get(group).copied();
            if Some(*group) == pinned {
                let mut position = u.map_or(viewport_start, |u| u.max(viewport_start));
                if let Some((_, next)) = natural.range(group + 1..).next() {
                    position = position.min(next - size);
                }
                u = Some(position);
            }
            let Some(u) = u else {
                header.arrange(Rect::default());
                continue;
            };
            let across = match beside {
                true => header_across(header, horizontal),
                false => (cross - cross_start - cross_end).max(0.0),
            };
            header.arrange(match horizontal {
                true => Rect::new(u, cross_start, size, across),
                false => Rect::new(cross_start, u, across, size),
            });
        }
    }
}

/// The index of the item at `index` after the change `e`: moved by the items added or removed
/// before it; a removed item gives its place to the item after the removed ones. `None` after a
/// reset, or when no item is left at that place.
fn index_after_change(index: i32, e: &ItemsChangedEventArgs<'_>) -> Option<i32> {
    let removed = |index: i32| {
        let (start, count) = (e.old_starting_index, e.old_items.len() as i32);
        match () {
            _ if start < 0 => index,
            _ if start + count <= index => index - count,
            _ if start <= index => start,
            _ => index,
        }
    };
    let added = |index: i32| match e.new_starting_index {
        start if start >= 0 && start <= index => index + e.new_items.len() as i32,
        _ => index,
    };
    match e.action {
        NotifyCollectionChangedAction::Add => Some(added(index)),
        NotifyCollectionChangedAction::Remove => Some(removed(index)),
        NotifyCollectionChangedAction::Replace => Some(index),
        NotifyCollectionChangedAction::Move => Some(added(removed(index))),
        NotifyCollectionChangedAction::Reset => None,
    }
}

/// The size of a header beside its items, across the stack.
fn header_across(header: &Ref<ListViewHeaderItem>, horizontal: bool) -> f64 {
    let desired = header.desired_size();
    if horizontal { desired.height } else { desired.width }
}

/// The insets of the items of a panel with groups.
struct GroupSpacing(WeakRef<ItemsStackPanel>);

impl IItemSpacing for GroupSpacing {
    fn insets(&self, index: i32, available: Size) -> ItemInsets {
        let Some(panel) = self.0.upgrade() else { return ItemInsets::default() };
        let groups = panel.groups.borrow().clone();
        groups.map_or_else(ItemInsets::default, |groups| groups.insets(&panel, index, available))
    }
}

ferro_class!(ItemsStackPanel: VirtualizingStackPanel);
ferroui_base::ferro_class_info!(ItemsStackPanel {
    new: ItemsStackPanel::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(
    ItemsStackPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl FerroObjectImpl for ItemsStackPanel {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::items_updating_scroll_mode_property().as_property() {
            let mode = change.get_new_value::<ItemsUpdatingScrollMode>();
            this.set_anchors_items(mode != ItemsUpdatingScrollMode::KeepScrollOffset);
        }
    }
}

impl LayoutableImpl for ItemsStackPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let size = Self::parent_measure_override(this, available_size);
        let groups = this.groups.borrow().clone();
        if let Some(groups) = groups {
            groups.update_headers(this, available_size);
        }
        size
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let size = Self::parent_arrange_override(this, final_size);
        let groups = this.groups.borrow().clone();
        if let Some(groups) = groups {
            groups.arrange_headers(this, final_size);
        }
        size
    }
}

impl VirtualizingPanelImpl for ItemsStackPanel {
    fn on_items_changed(this: &Self, items: &Rc<ItemsSourceView>, e: &ItemsChangedEventArgs<'_>) {
        let keeps_end = this.keeps_end_in_view();
        let first_visible = if keeps_end { None } else { this.first_visible_position() };
        this.update_groups();
        Self::parent_on_items_changed(this, items, e);
        if keeps_end {
            this.scroll_to_end_of_items();
        } else if let Some((index, offset)) = first_visible {
            if let Some(index) = index_after_change(index, e) {
                this.track(TrackedTarget::Item { index, offset });
            }
        }
    }

    fn on_items_control_changed(this: &Self, old_value: Option<&Ref<ItemsControl>>) {
        Self::parent_on_items_control_changed(this, old_value);
        this.update_groups();
        if this.keeps_end_in_view() {
            this.scroll_to_end_of_items();
        }
    }
}

ferroui_base::ferro_properties! { impl ItemsStackPanel {
    ferro_property!(
        /// Defines the `ItemsUpdatingScrollMode` property.
        pub fn items_updating_scroll_mode_property() -> StyledProperty<ItemsUpdatingScrollMode> {
            FerroProperty::register::<ItemsStackPanel, _>(
                "ItemsUpdatingScrollMode",
                ItemsUpdatingScrollMode::KeepItemsInView,
            )
        }
    );

    ferro_property!(
        /// Defines the `GroupPadding` property: the space around each group.
        pub fn group_padding_property() -> StyledProperty<Thickness> {
            FerroProperty::register::<ItemsStackPanel, _>("GroupPadding", Thickness::default())
        }
    );

    ferro_property!(
        /// Defines the `GroupHeaderPlacement` property.
        pub fn group_header_placement_property() -> StyledProperty<GroupHeaderPlacement> {
            FerroProperty::register::<ItemsStackPanel, _>("GroupHeaderPlacement", GroupHeaderPlacement::Top)
        }
    );

    ferro_property!(
        /// Defines the `AreStickyGroupHeadersEnabled` property: the header of the group at the
        /// start of the viewport stays there.
        pub fn are_sticky_group_headers_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ItemsStackPanel, _>("AreStickyGroupHeadersEnabled", true)
        }
    );
} }

impl ItemsStackPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<ItemsStackPanel>(&[
            Self::group_padding_property().as_property(),
            Self::group_header_placement_property().as_property(),
            Self::are_sticky_group_headers_enabled_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: VirtualizingStackPanel::construct(),
            groups: RefCell::new(None),
            viewport_changed: RefCell::new(None),
            tracked: Cell::new(None),
            tracking: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Follows the items source of the list: a grouped view gets its headers.
    fn update_groups(&self) {
        let source = self
            .items_control()
            .and_then(|control| control.items_source())
            .filter(|source| source.downcast_ref::<GroupedItemsView>().is_some());
        let current = self.groups.borrow().as_ref().map(|groups| groups.source.clone());
        if current.as_ref().map(ItemsSource::identity) == source.as_ref().map(ItemsSource::identity) {
            return;
        }
        if let Some(old) = self.groups.borrow_mut().take() {
            for header in old.headers.borrow().values().chain(old.pool.borrow().iter()) {
                let control: Ref<Control> = header.clone().upcast();
                self.remove_internal_child(&control);
            }
        }
        let Some(source) = source else {
            self.set_item_spacing(None);
            return;
        };
        let weak = self.to_ref().downgrade();
        *self.groups.borrow_mut() = Some(Rc::new(PanelGroups {
            source,
            headers: RefCell::new(BTreeMap::new()),
            pool: RefCell::new(Vec::new()),
            beside_size: Cell::new(0.0),
        }));
        self.set_item_spacing(Some(Rc::new(GroupSpacing(weak.clone()))));
        // A pinned header moves with every scroll, also inside the realized range.
        if self.viewport_changed.borrow().is_none() {
            let subscription = self.effective_viewport_changed(move |_| {
                if let Some(panel) = weak.upgrade() {
                    if panel.groups.borrow().is_some() && panel.are_sticky_group_headers_enabled() {
                        panel.invalidate_arrange();
                    }
                }
            });
            *self.viewport_changed.borrow_mut() = Some(subscription);
        }
    }

    /// Whether a change of the items has to bring the end of the list into view: with
    /// `KeepLastItemInView`, when the viewport reaches the end of the panel, or nothing is laid out
    /// yet (WinUI's `isTrackingExtentEnd`, and its tracking of the last element after a reset).
    fn keeps_end_in_view(&self) -> bool {
        if self.items_updating_scroll_mode() != ItemsUpdatingScrollMode::KeepLastItemInView {
            return false;
        }
        if self.last_realized_index() < 0 {
            return true;
        }
        let (viewport, size) = (self.view_port(), self.bounds().size());
        let (end, extent) = match self.orientation() {
            Orientation::Horizontal => (viewport.right(), size.width),
            Orientation::Vertical => (viewport.bottom(), size.height),
        };
        extent <= end + 0.5
    }

    /// The first visible item and the distance of its start from the start of the viewport, when a
    /// change of the items has to keep it in place: not with `KeepScrollOffset`, and with
    /// `KeepItemsInView` not at the start of the list (WinUI's `BeginTrackingFirstVisibleElement`).
    fn first_visible_position(&self) -> Option<(i32, f64)> {
        let mode = self.items_updating_scroll_mode();
        if mode == ItemsUpdatingScrollMode::KeepScrollOffset {
            return None;
        }
        let viewport_start = self.start_of(self.view_port());
        if mode == ItemsUpdatingScrollMode::KeepItemsInView && viewport_start <= 0.0 {
            return None;
        }
        let index = self.first_visible_index();
        let container = self.container_from_index(index)?;
        Some((index, self.start_of(container.bounds()) - viewport_start))
    }

    /// The start of `rect` along the stack.
    fn start_of(&self, rect: Rect) -> f64 {
        match self.orientation() {
            Orientation::Horizontal => rect.x,
            Orientation::Vertical => rect.y,
        }
    }

    /// Keeps `target` in place after the next layout passes.
    fn track(&self, target: TrackedTarget) {
        self.tracked.set(Some(Tracked { target, passes: 3 }));
        if self.tracking.borrow().is_some() {
            return;
        }
        let weak = self.to_ref().downgrade();
        let subscription = self.layout_updated(move || {
            if let Some(panel) = weak.upgrade() {
                panel.correct_tracked();
            }
        });
        *self.tracking.borrow_mut() = Some(subscription);
    }

    /// Scrolls the tracked item back to its distance from the start of the viewport, or the
    /// viewport to the end of the list. A position that is only estimated, because the item is not
    /// realized yet or the extent changes with the realized sizes, is corrected again after the
    /// next layout passes.
    fn correct_tracked(&self) {
        let Some(mut tracked) = self.tracked.take() else {
            drop(self.tracking.borrow_mut().take());
            return;
        };
        let viewer = self.find_ancestor_of_type::<ScrollViewer>(false);
        let horizontal = self.orientation() == Orientation::Horizontal;
        let along = |vector: Vector| if horizontal { vector.x } else { vector.y };
        let (delta, settled) = match (tracked.target, viewer.as_ref()) {
            (_, None) => (0.0, true),
            (TrackedTarget::Item { index, offset }, Some(_)) => {
                let (start, realized) = match self.container_from_index(index) {
                    Some(container) => (self.start_of(container.bounds()), true),
                    None => (self.get_or_estimate_element_u(index), false),
                };
                let delta = start - self.start_of(self.view_port()) - offset;
                (delta, realized && delta.abs() <= 0.5)
            }
            (TrackedTarget::End, Some(viewer)) => {
                let (extent, viewport) = (viewer.extent(), viewer.viewport());
                let end = if horizontal {
                    extent.width - viewport.width
                } else {
                    extent.height - viewport.height
                };
                let delta = end.max(0.0) - along(viewer.offset());
                (delta, delta.abs() <= 0.5)
            }
        };
        if let Some(viewer) = viewer.filter(|_| delta.abs() > 0.5) {
            let offset = viewer.offset();
            let corrected = match horizontal {
                true => Vector::new(offset.x + delta, offset.y),
                false => Vector::new(offset.x, offset.y + delta),
            };
            viewer.set_current_value(ScrollViewer::offset_property(), corrected);
        }
        tracked.passes = tracked.passes.saturating_sub(1);
        if settled || tracked.passes == 0 {
            drop(self.tracking.borrow_mut().take());
        } else {
            self.tracked.set(Some(tracked));
        }
    }

    /// Scrolls the scroll viewer of the panel to its end along the stack. The offset is coerced
    /// again when the extent grows in the next layout pass, so the end is the new one.
    fn scroll_to_end_of_items(&self) {
        let Some(viewer) = self.find_ancestor_of_type::<ScrollViewer>(false) else {
            return;
        };
        let offset = viewer.offset();
        let end = match self.orientation() {
            Orientation::Horizontal => Vector::new(f64::INFINITY, offset.y),
            Orientation::Vertical => Vector::new(offset.x, f64::INFINITY),
        };
        viewer.set_current_value(ScrollViewer::offset_property(), end);
        self.track(TrackedTarget::End);
    }

    pub fn items_updating_scroll_mode(&self) -> ItemsUpdatingScrollMode {
        self.get_value(Self::items_updating_scroll_mode_property())
    }

    pub fn set_items_updating_scroll_mode(&self, value: ItemsUpdatingScrollMode) {
        self.set_value(Self::items_updating_scroll_mode_property(), value)
    }

    pub fn group_padding(&self) -> Thickness {
        self.get_value(Self::group_padding_property())
    }

    pub fn set_group_padding(&self, value: Thickness) {
        self.set_value(Self::group_padding_property(), value)
    }

    pub fn group_header_placement(&self) -> GroupHeaderPlacement {
        self.get_value(Self::group_header_placement_property())
    }

    pub fn set_group_header_placement(&self, value: GroupHeaderPlacement) {
        self.set_value(Self::group_header_placement_property(), value)
    }

    pub fn are_sticky_group_headers_enabled(&self) -> bool {
        self.get_value(Self::are_sticky_group_headers_enabled_property())
    }

    pub fn set_are_sticky_group_headers_enabled(&self, value: bool) {
        self.set_value(Self::are_sticky_group_headers_enabled_property(), value)
    }

    /// The realized header of the group `group`, if the panel shows the groups of a grouped source.
    pub(crate) fn group_header(&self, group: usize) -> Option<Ref<ListViewHeaderItem>> {
        let groups = self.groups.borrow();
        let header = groups.as_ref()?.headers.borrow().get(&group).cloned();
        header
    }

    /// The first realized container, including the cache (WinUI's `FirstCacheIndex`).
    pub fn first_cache_index(&self) -> i32 {
        self.first_realized_index()
    }

    /// The last realized container, including the cache (WinUI's `LastCacheIndex`).
    pub fn last_cache_index(&self) -> i32 {
        self.last_realized_index()
    }

    /// The realized indexes whose containers intersect the viewport.
    fn visible_indexes(&self) -> impl Iterator<Item = i32> + '_ {
        let viewport = self.view_port();
        let horizontal = self.orientation() == Orientation::Horizontal;
        let (first, last) = (self.first_realized_index(), self.last_realized_index());
        (first.max(0)..=last).filter(move |index| {
            if *index < 0 {
                return false;
            }
            self.container_from_index(*index).is_some_and(|container| {
                let bounds = container.bounds();
                if horizontal {
                    bounds.right() > viewport.x && bounds.x < viewport.right()
                } else {
                    bounds.bottom() > viewport.y && bounds.y < viewport.bottom()
                }
            })
        })
    }

    /// The first item at least partly in the viewport (WinUI's `FirstVisibleIndex`), -1 without.
    pub fn first_visible_index(&self) -> i32 {
        self.visible_indexes().next().unwrap_or(-1)
    }

    /// The last item at least partly in the viewport (WinUI's `LastVisibleIndex`), -1 without.
    pub fn last_visible_index(&self) -> i32 {
        self.visible_indexes().last().unwrap_or(-1)
    }
}
