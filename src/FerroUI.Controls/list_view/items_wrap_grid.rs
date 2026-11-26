//! WinUI's `ItemsWrapGrid` (`dxaml/xcp/dxaml/lib/ItemsWrapGrid_Partial.cpp`, the group layout of
//! `dxaml/xcp/components/moco/lib/WrappingLayoutStrategyImpl.cpp` and the metadata of
//! `Microsoft.UI.Xaml.Controls.cs` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), on FerroUI's `WrapPanel` (docs/porting/winui-calculator-gaps.md, LV-2, decision D5).

use super::{
    CollectionViewGroup, GridViewHeaderItem, GroupHeaderPlacement, GroupStyle, GroupedItemsView, ListViewBase,
};
use crate::{Control, ControlImpl, ItemsControl, ItemsSource, PanelImpl, WrapPanel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, Thickness, Visual, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The items panel of a grid view (WinUI's `ItemsWrapGrid`): the items in cells of one size, in
/// rows (`Orientation` horizontal) or columns, wrapping at the edge of the panel or after
/// [`maximum_rows_or_columns`](Self::maximum_rows_or_columns) items. The cell is `ItemWidth` by
/// `ItemHeight`, or the size of the first item where they are not set, as in WinUI.
///
/// With a grouped source ([`GroupedItemsView`]) each group starts a new line, with its
/// [`GridViewHeaderItem`] above the group ([`GroupHeaderPlacement::Top`]) or beside it
/// ([`GroupHeaderPlacement::Left`]) and [`group_padding`](Self::group_padding) around it
/// (WinUI's `WrappingLayoutStrategyImpl`: the inline and the parallel header strategies). The
/// headers are children of the panel after the items, and logical children of the list (#1424).
///
/// Recorded deviations (decision D5): it derives from FerroUI's [`WrapPanel`] and does not
/// virtualize; WinUI's panel is a modern collection panel that realizes the visible items only.
/// So every group has its header. A header beside its items is as wide as the widest of them,
/// where WinUI takes the size of the first. `AreStickyGroupHeadersEnabled` is not ported.
#[repr(C)]
pub struct ItemsWrapGrid {
    base: WrapPanel,
    groups: RefCell<Option<WrapGroups>>,
}

/// The headers of the groups of a panel, one for each group, in the order of the groups.
struct WrapGroups {
    source: ItemsSource,
    headers: Vec<(Ref<GridViewHeaderItem>, Option<Rc<CollectionViewGroup>>)>,
}

/// The layout of a panel along the two directions of WinUI's wrapping layout: `v` is the
/// direction the lines advance in (the virtualizing direction, down for rows), `s` the direction
/// the items of a line stack in.
#[derive(Clone, Copy)]
struct Axes {
    horizontal: bool,
}

impl Axes {
    fn s(self, size: Size) -> f64 {
        if self.horizontal {
            size.width
        } else {
            size.height
        }
    }

    fn v(self, size: Size) -> f64 {
        if self.horizontal {
            size.height
        } else {
            size.width
        }
    }

    fn size(self, s: f64, v: f64) -> Size {
        if self.horizontal {
            Size::new(s, v)
        } else {
            Size::new(v, s)
        }
    }

    fn rect(self, s: f64, v: f64, size: Size) -> Rect {
        if self.horizontal {
            Rect::new(s, v, size.width, size.height)
        } else {
            Rect::new(v, s, size.width, size.height)
        }
    }

    /// The padding of a group: before and after it along `v`, before and after it along `s`.
    fn padding(self, padding: Thickness) -> (f64, f64, f64, f64) {
        match self.horizontal {
            true => (padding.top, padding.bottom, padding.left, padding.right),
            false => (padding.left, padding.right, padding.top, padding.bottom),
        }
    }
}

ferro_class!(ItemsWrapGrid: WrapPanel);
ferroui_base::ferro_class_info!(ItemsWrapGrid {
    new: ItemsWrapGrid::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(
    ItemsWrapGrid: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    FerroObjectImpl
);

ferroui_base::ferro_properties! { impl ItemsWrapGrid {
    ferro_property!(
        /// Defines the `MaximumRowsOrColumns` property: the items of a row (or a column) at most;
        /// -1 (WinUI's default) wraps at the edge only.
        pub fn maximum_rows_or_columns_property() -> StyledProperty<i32> {
            FerroProperty::register::<ItemsWrapGrid, _>("MaximumRowsOrColumns", -1)
        }
    );

    ferro_property!(
        /// Defines the `GroupPadding` property: the space around each group.
        pub fn group_padding_property() -> StyledProperty<Thickness> {
            FerroProperty::register::<ItemsWrapGrid, _>("GroupPadding", Thickness::default())
        }
    );

    ferro_property!(
        /// Defines the `GroupHeaderPlacement` property.
        pub fn group_header_placement_property() -> StyledProperty<GroupHeaderPlacement> {
            FerroProperty::register::<ItemsWrapGrid, _>("GroupHeaderPlacement", GroupHeaderPlacement::Top)
        }
    );
} }

impl ItemsWrapGrid {
    fn static_constructor() {
        Layoutable::affects_measure::<ItemsWrapGrid>(&[
            Self::maximum_rows_or_columns_property().as_property(),
            Self::group_padding_property().as_property(),
            Self::group_header_placement_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: WrapPanel::construct(), groups: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn maximum_rows_or_columns(&self) -> i32 {
        self.get_value(Self::maximum_rows_or_columns_property())
    }

    pub fn set_maximum_rows_or_columns(&self, value: i32) {
        self.set_value(Self::maximum_rows_or_columns_property(), value)
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

    /// The headers of the groups, in the order of the groups; empty without a grouped source.
    pub(crate) fn group_headers(&self) -> Vec<Ref<GridViewHeaderItem>> {
        let groups = self.groups.borrow();
        groups
            .as_ref()
            .map(|groups| groups.headers.iter().map(|(header, _)| header.clone()).collect())
            .unwrap_or_default()
    }

    /// The list whose items the panel hosts, if it hosts the items of one.
    fn owner(&self) -> Option<Ref<ItemsControl>> {
        if !self.is_items_host() {
            return None;
        }
        self.templated_parent().and_then(|parent| parent.cast::<ItemsControl>())
    }

    /// The first item and the item count of each group, with a grouped source.
    fn group_runs(&self) -> Option<Vec<(usize, usize)>> {
        let groups = self.groups.borrow();
        let view = groups.as_ref()?.source.downcast_ref::<GroupedItemsView>()?;
        Some(
            (0..view.group_count())
                .map(|index| (view.group_start(index).unwrap_or(0), view.group_len(index)))
                .collect(),
        )
    }

    /// Follows the items source of the list: a grouped view gets a header for each group, the
    /// other sources none. Run before each measure: the children change with the items.
    fn update_groups(&self) {
        let owner = self.owner();
        let source = owner
            .as_ref()
            .and_then(|owner| owner.items_source())
            .filter(|source| source.downcast_ref::<GroupedItemsView>().is_some());
        let current = self.groups.borrow().as_ref().map(|groups| groups.source.identity());
        if current != source.as_ref().map(ItemsSource::identity) {
            if let Some(old) = self.groups.borrow_mut().take() {
                for (header, _) in old.headers {
                    self.remove_header(owner.as_ref(), &header);
                }
            }
            if let Some(source) = source.clone() {
                *self.groups.borrow_mut() = Some(WrapGroups { source, headers: Vec::new() });
            }
        }
        let (Some(owner), Some(source)) = (owner, source) else {
            return;
        };
        let Some(view) = source.downcast_ref::<GroupedItemsView>() else {
            return;
        };
        let style = owner
            .cast::<ListViewBase>()
            .map(|list| list.group_style())
            .filter(|styles| styles.count() > 0)
            .map(|styles| styles.get(0));
        let mut groups = self.groups.borrow_mut();
        let Some(groups) = groups.as_mut() else {
            return;
        };
        let headers = &mut groups.headers;
        while headers.len() > view.group_count() {
            let (header, _) = headers.pop().expect("a header");
            self.remove_header(Some(&owner), &header);
        }
        for index in 0..view.group_count() {
            let group = view.group_at(index);
            if index == headers.len() {
                let header = GridViewHeaderItem::new();
                Self::style_header(&header, style.as_ref());
                owner.add_logical_child(&header.clone().upcast());
                self.visual_children().add(header.clone().upcast::<Visual>());
                headers.push((header, None));
            }
            let (header, shown) = &mut headers[index];
            let same = match (shown.as_ref(), group.as_ref()) {
                (Some(shown), Some(group)) => Rc::ptr_eq(shown, group),
                (None, None) => true,
                _ => false,
            };
            if !same {
                header.set_content(group.as_ref().and_then(|group| group.group()));
                *shown = group;
            }
            let visible = view.group_len(index) > 0 || !style.as_ref().is_some_and(|style| style.hides_if_empty());
            header.set_is_visible(visible);
        }
    }

    fn style_header(header: &GridViewHeaderItem, style: Option<&Ref<GroupStyle>>) {
        header.set_content_template(style.and_then(|style| style.header_template()));
        if let Some(theme) = style.and_then(|style| style.header_container_style()) {
            header.set_theme(theme);
        }
    }

    fn remove_header(&self, owner: Option<&Ref<ItemsControl>>, header: &Ref<GridViewHeaderItem>) {
        let control: Ref<Control> = header.clone().upcast();
        if let Some(owner) = owner {
            owner.remove_logical_child(&control);
        }
        self.visual_children().remove(&control.upcast::<Visual>());
    }

    /// Lays the groups out: the headers and the items of each group from a new line, as WinUI's
    /// `WrappingLayoutStrategyImpl` (`GetHeaderBounds`, `GetContainerBounds`). Measures, or with
    /// `arrange` also arranges, and returns the extent of the groups.
    fn layout_groups(&self, size: Size, runs: &[(usize, usize)], arrange: bool) -> Size {
        let axes = Axes { horizontal: self.orientation() == Orientation::Horizontal };
        let (before_v, after_v, before_s, after_s) = axes.padding(self.group_padding());
        // Top in rows and Left in columns are WinUI's inline headers: before the items along `v`;
        // the others are parallel: beside the items, which move along `s` by the header.
        let inline = (self.group_header_placement() == GroupHeaderPlacement::Top) == axes.horizontal;
        let headers = self.group_headers();
        let header_constraint = axes.size(axes.s(size), f64::INFINITY);
        let mut parallel_s: f64 = 0.0;
        for header in headers.iter().filter(|header| header.is_visible()) {
            if !arrange {
                header.measure(header_constraint);
            }
            if !inline {
                parallel_s = parallel_s.max(axes.s(header.desired_size()));
            }
        }
        let children = self.children().snapshot();
        let cell = self.cell_size(size);
        if !arrange {
            for child in children.iter() {
                child.measure(cell);
            }
        }
        let stacking = (axes.s(size) - before_s - after_s - parallel_s).max(0.0);
        let per_line = self.per_line(stacking, axes.s(cell));
        let (mut v, mut extent_s): (f64, f64) = (0.0, 0.0);
        for (index, header) in headers.iter().enumerate() {
            if !header.is_visible() {
                if arrange {
                    header.arrange(Rect::default());
                }
                continue;
            }
            let (start, len) = runs.get(index).copied().unwrap_or((0, 0));
            let items: Vec<&Ref<Control>> =
                children.iter().skip(start).take(len).filter(|child| child.is_visible()).collect();
            v += before_v;
            let desired = header.desired_size();
            if arrange {
                header.arrange(axes.rect(before_s, v, desired));
            }
            let items_v_start = if inline { v + axes.v(desired) } else { v };
            let items_s_start = before_s + parallel_s;
            for (position, child) in items.iter().enumerate() {
                let (line, slot) = (position / per_line, position % per_line);
                if arrange {
                    let rect = axes.rect(
                        items_s_start + slot as f64 * axes.s(cell),
                        items_v_start + line as f64 * axes.v(cell),
                        cell,
                    );
                    child.arrange(rect);
                }
            }
            let lines = items.len().div_ceil(per_line);
            let items_v = lines as f64 * axes.v(cell);
            let in_line = per_line.min(items.len());
            v += if inline { axes.v(desired) + items_v } else { items_v.max(axes.v(desired)) };
            v += after_v;
            let header_s = if inline { axes.s(desired) } else { 0.0 };
            extent_s = extent_s.max(before_s + header_s.max(parallel_s + in_line as f64 * axes.s(cell)) + after_s);
        }
        axes.size(extent_s, v)
    }

    /// The size of a cell: `ItemWidth` by `ItemHeight`, the size of the first item where unset.
    fn cell_size(&self, available: Size) -> Size {
        let children = self.children().snapshot();
        let first = children.iter().find(|child| child.is_visible());
        let mut width = self.item_width();
        let mut height = self.item_height();
        if width.is_nan() || height.is_nan() {
            if let Some(first) = first {
                first.measure(available);
                let desired = first.desired_size();
                if width.is_nan() {
                    width = desired.width;
                }
                if height.is_nan() {
                    height = desired.height;
                }
            }
        }
        Size::new(if width.is_nan() { 0.0 } else { width }, if height.is_nan() { 0.0 } else { height })
    }

    /// The cells of a line: as many as fit, at most `MaximumRowsOrColumns`, at least one.
    fn per_line(&self, extent: f64, cell: f64) -> usize {
        let fit = if cell > 0.0 && extent.is_finite() { (extent / cell).floor() as usize } else { usize::MAX };
        let maximum = self.maximum_rows_or_columns();
        let limited = if maximum > 0 { fit.min(maximum as usize) } else { fit };
        limited.max(1)
    }
}

impl LayoutableImpl for ItemsWrapGrid {
    /// The holds of the wrap panel follow its own lines; the cells of this panel hold nothing.
    fn measure_hold_override(_this: &Self, _constraint: Size) -> Option<ferroui_base::layout::MeasureHold> {
        None
    }

    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.update_groups();
        if let Some(runs) = this.group_runs() {
            return this.layout_groups(available_size, &runs, false);
        }
        let cell = this.cell_size(available_size);
        let children = this.children().snapshot();
        let count = children.iter().filter(|child| child.is_visible()).count();
        for child in children.iter() {
            child.measure(cell);
        }
        if count == 0 {
            return Size::default();
        }
        let horizontal = this.orientation() == Orientation::Horizontal;
        let per_line = if horizontal {
            this.per_line(available_size.width, cell.width)
        } else {
            this.per_line(available_size.height, cell.height)
        };
        let in_line = per_line.min(count);
        let lines = count.div_ceil(per_line);
        if horizontal {
            Size::new(in_line as f64 * cell.width, lines as f64 * cell.height)
        } else {
            Size::new(lines as f64 * cell.width, in_line as f64 * cell.height)
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(runs) = this.group_runs() {
            this.layout_groups(final_size, &runs, true);
            return final_size;
        }
        let cell = this.cell_size(final_size);
        let horizontal = this.orientation() == Orientation::Horizontal;
        let per_line = if horizontal {
            this.per_line(final_size.width, cell.width)
        } else {
            this.per_line(final_size.height, cell.height)
        };
        let mut position = 0;
        for child in this.children().snapshot().iter() {
            if !child.is_visible() {
                continue;
            }
            let (line, slot) = (position / per_line, position % per_line);
            let rect = if horizontal {
                Rect::new(slot as f64 * cell.width, line as f64 * cell.height, cell.width, cell.height)
            } else {
                Rect::new(line as f64 * cell.width, slot as f64 * cell.height, cell.width, cell.height)
            };
            child.arrange(rect);
            position += 1;
        }
        final_size
    }
}
