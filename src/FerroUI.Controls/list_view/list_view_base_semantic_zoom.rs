//! `ListViewBase` as a view of a [`SemanticZoom`] (WinUI's
//! `dxaml/xcp/dxaml/lib/ListViewBase_Partial_SemanticZoomInformation.cpp` and the semantic zoom
//! parts of `OnItemPrimaryInteractionGesture` and `OnGroupHeaderInteraction` of
//! `ListViewBase_Partial_Interaction.cpp`, microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! Left out: the scroll indicators WinUI blocks during a change and the scrolling placeholders it
//! turns off (FerroUI's lists have neither), and the nested group items of the old grouping
//! panels (`GroupItem`).

use super::{
    CollectionViewGroup, GroupedItemsView, ItemsStackPanel, ItemsWrapGrid, ListViewBase, ListViewBaseHeaderItem,
};
use crate::semantic_zoom::{ISemanticZoomInformation, SemanticZoom, SemanticZoomLocation, ViewChangeInput};
use crate::Control;
use ferroui_base::input::KeyModifiers;
use ferroui_base::{BoxedValue, Point, Rect, Ref, Vector, WeakRef};
use std::rc::Rc;

/// The state of a list as a view of a semantic zoom, allocated when a semantic zoom takes it.
pub(super) struct SemanticZoomViewState {
    owner: Option<WeakRef<SemanticZoom>>,
    is_active_view: bool,
    is_zoomed_in_view: bool,
    /// The group whose header asked for the change (WinUI's `m_tpSZRequestingItem`).
    requesting_group: Option<usize>,
    /// The item the zoomed-out view jumps from (WinUI's `SetFocusedIndex` before the toggle).
    focused_index: i32,
    /// How the destination item is focused at the end of the change (WinUI's
    /// `m_semanticZoomCompletedFocusState`).
    completed_input: ViewChangeInput,
}

impl Default for SemanticZoomViewState {
    fn default() -> Self {
        Self {
            owner: None,
            is_active_view: false,
            is_zoomed_in_view: true,
            requesting_group: None,
            focused_index: -1,
            completed_input: ViewChangeInput::Programmatic,
        }
    }
}

impl ListViewBase {
    fn zoom_state<R>(&self, read: impl FnOnce(&SemanticZoomViewState) -> R) -> R {
        match self.semantic_zoom.borrow().as_deref() {
            Some(state) => read(state),
            None => read(&SemanticZoomViewState::default()),
        }
    }

    fn zoom_state_mut(&self, write: impl FnOnce(&mut SemanticZoomViewState)) {
        let mut state = self.semantic_zoom.borrow_mut();
        write(state.get_or_insert_with(Default::default));
    }

    fn with_grouped_view<R>(&self, read: impl FnOnce(&GroupedItemsView) -> R) -> Option<R> {
        let source = self.items_source()?;
        let view = source.downcast_ref::<GroupedItemsView>()?;
        Some(read(view))
    }

    /// The group `item` stands for: the group itself, or the object its header shows (WinUI's
    /// `GetGroupItemIndex`).
    fn group_index_of(&self, item: &BoxedValue) -> Option<usize> {
        let as_group = item.downcast_ref::<Rc<CollectionViewGroup>>().cloned();
        self.with_grouped_view(|view| {
            (0..view.group_count()).find(|&index| {
                view.group_at(index).is_some_and(|group| {
                    as_group.as_ref().is_some_and(|candidate| Rc::ptr_eq(candidate, &group))
                        || group.group().is_some_and(|key| key.value_eq(&**item))
                })
            })
        })
        .flatten()
    }

    /// The realized header of `group` (WinUI's `HeaderFromIndex`).
    fn group_header_container(&self, group: usize) -> Option<Ref<Control>> {
        let panel = self.items_panel_root()?;
        if let Some(grid) = panel.cast::<ItemsWrapGrid>() {
            return grid.group_headers().get(group).map(|header| header.clone().upcast());
        }
        panel.cast::<ItemsStackPanel>()?.group_header(group).map(|header| header.upcast())
    }

    /// The group whose header is `header`.
    fn group_index_of_header(&self, header: &ListViewBaseHeaderItem) -> Option<usize> {
        let content = header.content()?;
        self.group_index_of(&content)
    }

    /// WinUI's `FindFirstItem`: the first item of the first group at or after `target_group` that
    /// has items, and its index.
    fn find_first_item(&self, target_group: usize) -> (i32, Option<BoxedValue>) {
        let index = self
            .with_grouped_view(|view| {
                (target_group..view.group_count())
                    .find(|&group| view.group_len(group) > 0)
                    .and_then(|group| view.group_start(group))
            })
            .flatten();
        match index {
            Some(index) => (index as i32, self.items_view().get_at(index)),
            None => (-1, None),
        }
    }

    /// The index of the realized container that holds the keyboard focus (WinUI's
    /// `GetFocusedIndex`).
    fn focused_index(&self) -> i32 {
        let Some(panel) = self.items_panel_root() else { return -1 };
        panel
            .children()
            .snapshot()
            .iter()
            .find(|child| child.is_keyboard_focus_within())
            .map_or(-1, |child| self.index_from_container(child))
    }

    /// WinUI's `GetLastFocusedIndex`: the focused item, else the item of the last jump, else the
    /// selected item, else the first.
    fn last_focused_index(&self) -> i32 {
        let focused = self.focused_index();
        if focused >= 0 {
            return focused;
        }
        let jumped = self.zoom_state(|state| state.focused_index);
        if jumped >= 0 && (jumped as usize) < self.items_view().count() {
            return jumped;
        }
        self.selected_index().max(0)
    }

    /// Focuses the item at `index`, scrolled into view (WinUI's `SetFocusedItem`).
    fn focus_item_at(&self, index: i32, input: ViewChangeInput) -> bool {
        if index < 0 || index as usize >= self.items_view().count() {
            return false;
        }
        self.scroll_into_view(index);
        self.update_layout();
        self.container_from_index(index)
            .is_some_and(|container| container.focus_with(input.navigation_method(), KeyModifiers::NONE))
    }

    /// Whether the semantic zoom groups the zoomed-in view (WinUI's `get_CanSemanticZoomGroup`).
    fn can_semantic_zoom_group(&self) -> bool {
        if self.is_zoomed_in_view() {
            return self.is_grouping();
        }
        self.semantic_zoom_owner()
            .and_then(|owner| owner.zoomed_in_view())
            .and_then(|view| view.cast::<ListViewBase>())
            .is_some_and(|view| view.is_grouping())
    }

    /// The child of the items panel closest to `point` (in the coordinates of the list), as an item
    /// index or a group (WinUI's `GetClosestElementInfo` of the panel).
    fn closest_element(&self, point: Point) -> Option<ClosestElement> {
        let panel = self.items_panel_root()?;
        let mut best: Option<(f64, ClosestElement)> = None;
        for child in panel.children().snapshot().iter() {
            if !child.is_visible() {
                continue;
            }
            let element = if let Some(header) = child.cast::<ListViewBaseHeaderItem>() {
                match self.group_index_of_header(&header) {
                    Some(group) => ClosestElement::Group(group, child.clone()),
                    None => continue,
                }
            } else {
                match self.index_from_container(&child) {
                    -1 => continue,
                    index => ClosestElement::Item(index, child.clone()),
                }
            };
            let Some(origin) = child.translate_point(Point::new(0.0, 0.0), self) else { continue };
            let size = child.bounds().size();
            let dx = (origin.x - point.x).max(point.x - (origin.x + size.width)).max(0.0);
            let dy = (origin.y - point.y).max(point.y - (origin.y + size.height)).max(0.0);
            let distance = dx * dx + dy * dy;
            if best.as_ref().is_none_or(|(closest, _)| distance < *closest) {
                best = Some((distance, element));
            }
        }
        best.map(|(_, element)| element)
    }

    /// Moves `element` to the start of the viewport (WinUI's `ScrollIntoViewAlignment::Leading`).
    fn bring_to_leading(&self, element: &Ref<Control>) {
        let (Some(scroll_viewer), Some(panel)) = (self.scroll_viewer(), self.items_panel_root()) else { return };
        let Some(position) = element.translate_point(Point::new(0.0, 0.0), &panel) else { return };
        let offset = scroll_viewer.offset();
        let (extent, viewport) = (scroll_viewer.extent(), scroll_viewer.viewport());
        let target = if extent.height > viewport.height {
            Vector::new(offset.x, position.y)
        } else {
            Vector::new(position.x, offset.y)
        };
        scroll_viewer.set_offset(target);
    }

    /// WinUI's `OnItemPrimaryInteractionGesture` of the zoomed-out view: the item becomes the
    /// target of the jump and the semantic zoom shows the zoomed-in view.
    pub(crate) fn zoomed_out_item_invoked(&self, container: &Ref<Control>, is_keyboard_input: bool) -> bool {
        let Some(owner) = self.semantic_zoom_owner() else { return false };
        if self.is_zoomed_in_view() {
            return false;
        }
        let input = if is_keyboard_input { ViewChangeInput::Keyboard } else { ViewChangeInput::Pointer };
        container.focus_with(input.navigation_method(), KeyModifiers::NONE);
        let index = self.index_from_container(container);
        self.zoom_state_mut(|state| state.focused_index = index);
        owner.toggle_active_view_with_input(input);
        true
    }

    /// WinUI's `OnGroupHeaderInteraction`: a tap on a group header of the zoomed-in view shows the
    /// zoomed-out view, which goes to the group of the header.
    pub(crate) fn group_header_interaction(&self, header: &ListViewBaseHeaderItem) -> bool {
        let Some(owner) = self.semantic_zoom_owner() else { return false };
        if !owner.can_change_views() || !self.is_zoomed_in_view() {
            return false;
        }
        let group = self.group_index_of_header(header);
        self.zoom_state_mut(|state| state.requesting_group = group);
        owner.toggle_active_view_with_input(ViewChangeInput::Pointer);
        true
    }
}

enum ClosestElement {
    Item(i32, Ref<Control>),
    Group(usize, Ref<Control>),
}

impl ISemanticZoomInformation for ListViewBase {
    fn initialize_view_change(&self) {
        // The focus state of the destination is known now: the semantic zoom forgets its input.
        let input = self.semantic_zoom_owner().map_or(ViewChangeInput::Programmatic, |owner| owner.processing_input());
        self.zoom_state_mut(|state| state.completed_input = input);
    }

    fn complete_view_change(&self) {}

    fn make_visible(&self, location: &SemanticZoomLocation) {
        let jump_list = self.semantic_zoom_owner().is_some();
        let is_zoomed_in = self.is_zoomed_in_view();
        let Some(item) = location.item() else { return };
        let Some(scroll_viewer) = self.scroll_viewer() else { return };
        let container = match self.group_index_of(&item) {
            Some(group) => {
                // A group: its first item into view, and leaving the zoomed-out view its header at
                // the start of the viewport.
                if let Some(first) = self
                    .with_grouped_view(|view| (view.group_len(group) > 0).then(|| view.group_start(group)))
                    .flatten()
                    .flatten()
                {
                    self.scroll_into_view(first as i32);
                }
                self.update_layout();
                let header = self.group_header_container(group);
                if let (Some(header), true) = (&header, jump_list && is_zoomed_in) {
                    self.bring_to_leading(header);
                    self.update_layout();
                }
                header
            }
            None => {
                let index = self.items_view().index_of(&Some(item));
                if index < 0 {
                    return;
                }
                self.scroll_into_view(index);
                self.update_layout();
                self.container_from_index(index)
            }
        };
        let Some(container) = container else { return };
        let Some(current) = container.translate_point(Point::new(0.0, 0.0), self) else { return };
        let size = container.bounds().size();
        // The zoomed-out view of a jump list puts the container at its top (or its bottom when a
        // group header asked); otherwise the container goes where the source item was.
        let target = if jump_list && !is_zoomed_in {
            let y = if location.is_bottom_alignment() { self.bounds().height - size.height } else { 0.0 };
            Point::new(current.x, y)
        } else {
            let bounds = location.bounds();
            Point::new(bounds.x, bounds.y)
        };
        let (extent, viewport, offset) = (scroll_viewer.extent(), scroll_viewer.viewport(), scroll_viewer.offset());
        let mut new_offset = offset;
        let mut remainder = Rect::new(0.0, 0.0, 0.0, 0.0);
        for horizontal in [true, false] {
            let (scrollable, current, target, length, view, old) = if horizontal {
                (extent.width - viewport.width, current.x, target.x, size.width, viewport.width, offset.x)
            } else {
                (extent.height - viewport.height, current.y, target.y, size.height, viewport.height, offset.y)
            };
            if scrollable <= 0.0 {
                continue;
            }
            // Not past the start or the end of the viewport; the rest is the remainder.
            let wanted = target - current;
            let amount = wanted.max(-current).min(view - current - length);
            let new = (old - amount).clamp(0.0, scrollable);
            let left = wanted - (old - new);
            if horizontal {
                new_offset = Vector::new(new, new_offset.y);
                remainder = Rect::new(left, remainder.y, 0.0, 0.0);
            } else {
                new_offset = Vector::new(new_offset.x, new);
                remainder = Rect::new(remainder.x, left, 0.0, 0.0);
            }
        }
        if new_offset != offset {
            scroll_viewer.set_offset(new_offset);
        }
        location.set_remainder(remainder);
    }

    fn start_view_change_from(&self, source: &SemanticZoomLocation, destination: &SemanticZoomLocation) {
        let mut item = source.item();
        let mut source_index = -1;
        let mut container: Option<Ref<Control>> = None;
        let mut jump_list = false;
        if item.is_none() {
            let zoom_point = source.zoom_point();
            if zoom_point.x != 0.0 || zoom_point.y != 0.0 {
                // A zoom centred on a point: the element under it.
                match self.closest_element(zoom_point) {
                    Some(ClosestElement::Item(index, child)) => {
                        source_index = index;
                        container = Some(child);
                    }
                    Some(ClosestElement::Group(group, child)) => {
                        item = self
                            .with_grouped_view(|view| view.group_at(group))
                            .flatten()
                            .map(|group| Rc::new(group) as BoxedValue);
                        container = Some(child);
                    }
                    None => {}
                }
            } else {
                let owner = self.semantic_zoom_owner();
                jump_list = owner.is_some();
                let is_zoomed_in = self.is_zoomed_in_view();
                if owner.as_ref().is_some_and(|owner| !is_zoomed_in && owner.is_cancelling_jump_list()) {
                    // Cancelled: no destination.
                    self.zoom_state_mut(|state| state.requesting_group = None);
                    return;
                }
                if jump_list && is_zoomed_in {
                    // The first item of the group whose header asked, else of the first group.
                    let requesting = self.zoom_state(|state| state.requesting_group);
                    let (index, first) = self.find_first_item(requesting.unwrap_or(0));
                    source_index = index;
                    item = first;
                    destination.set_is_bottom_alignment(requesting.is_some());
                } else {
                    // The item the user chose.
                    source_index = self.last_focused_index();
                }
            }
            if item.is_none() && source_index >= 0 && (source_index as usize) < self.items_view().count() {
                item = self.items_view().get_at(source_index as usize);
            }
            if item.is_some() {
                source.set_item(item.clone());
            }
            if container.is_none() && source_index >= 0 {
                container = self.container_from_index(source_index);
            }
            let bounds = match (&container, jump_list) {
                (Some(container), false) => container
                    .translate_point(Point::new(0.0, 0.0), self)
                    .map(|origin| Rect::new(origin.x, origin.y, container.bounds().width, container.bounds().height)),
                _ => None,
            };
            source.set_bounds(bounds.unwrap_or(Rect::new(zoom_point.x, zoom_point.y, 0.0, 0.0)));
        }

        // A grouped source gives the destination by itself: the group of the item zooming out, the
        // object of the group's header zooming in.
        if self.can_semantic_zoom_group() && destination.item().is_none() {
            if let Some(item) = &item {
                let group = item.downcast_ref::<Rc<CollectionViewGroup>>().cloned();
                let target = if self.is_zoomed_in_view() {
                    group
                        .or_else(|| {
                            (source_index >= 0)
                                .then(|| self.with_grouped_view(|view| view.group_of(source_index as usize)))
                                .flatten()
                                .flatten()
                                .and_then(|(group, _)| self.with_grouped_view(|view| view.group_at(group)).flatten())
                        })
                        .map(|group| Rc::new(group) as BoxedValue)
                } else {
                    group.and_then(|group| group.group())
                };
                if target.is_some() {
                    destination.set_item(target);
                }
            }
        }
        self.zoom_state_mut(|state| state.requesting_group = None);
    }

    fn start_view_change_to(&self, _source: &SemanticZoomLocation, _destination: &SemanticZoomLocation) {}

    fn complete_view_change_from(&self, _source: &SemanticZoomLocation, _destination: &SemanticZoomLocation) {}

    fn complete_view_change_to(&self, _source: &SemanticZoomLocation, destination: &SemanticZoomLocation) {
        // Only while the semantic zoom has the focus does the destination take it.
        let Some(owner) = self.semantic_zoom_owner() else { return };
        if !owner.is_keyboard_focus_within() {
            return;
        }
        let input = self.zoom_state(|state| state.completed_input);
        if let Some(item) = destination.item() {
            let index = self.items_view().index_of(&Some(item.clone()));
            if index >= 0 {
                self.focus_item_at(index, input);
            } else if let Some(group) = self.is_grouping().then(|| self.group_index_of(&item)).flatten() {
                // A group: the first item of it, or of the nearest group after or before it.
                let first = self
                    .with_grouped_view(|view| {
                        let count = view.group_count();
                        (group..count)
                            .chain((0..group).rev())
                            .find(|&candidate| view.group_len(candidate) > 0)
                            .and_then(|candidate| view.group_start(candidate))
                    })
                    .flatten();
                if let Some(first) = first {
                    self.focus_item_at(first as i32, input);
                }
            }
        }
        if !self.is_keyboard_focus_within()
            && !self.focus_item_at(self.last_focused_index(), input)
            && !self.is_keyboard_focus_within()
        {
            // Nothing that makes sense: the view itself, so the focus leaves the source view.
            self.focus_with(input.navigation_method(), KeyModifiers::NONE);
        }
    }

    fn is_active_view(&self) -> bool {
        self.zoom_state(|state| state.is_active_view)
    }

    fn set_is_active_view(&self, value: bool) {
        self.zoom_state_mut(|state| state.is_active_view = value);
    }

    fn is_zoomed_in_view(&self) -> bool {
        self.zoom_state(|state| state.is_zoomed_in_view)
    }

    fn set_is_zoomed_in_view(&self, value: bool) {
        self.zoom_state_mut(|state| state.is_zoomed_in_view = value);
    }

    fn semantic_zoom_owner(&self) -> Option<Ref<SemanticZoom>> {
        self.semantic_zoom.borrow().as_ref()?.owner.as_ref()?.upgrade()
    }

    fn set_semantic_zoom_owner(&self, value: Option<&Ref<SemanticZoom>>) {
        self.zoom_state_mut(|state| state.owner = value.map(Ref::downgrade));
    }
}
