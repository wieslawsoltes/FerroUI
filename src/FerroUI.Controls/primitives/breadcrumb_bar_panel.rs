//! WinUI's `BreadcrumbLayout` (`controls/dev/Breadcrumb/BreadcrumbLayout.cpp` and `.h` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md): a structural port of `MeasureOverride` and
//! `ArrangeOverride` as the measure and the arrange of a panel.
//!
//! WinUI's layout is a `NonVirtualizingLayout` of the `ItemsRepeater` `PART_ItemsRepeater` of the
//! template of `BreadcrumbBar`; `ferroui-controls` cannot depend on the repeater crate, so the part
//! is this panel, whose children the breadcrumb bar realizes itself (docs/porting/winui-controls.md,
//! section 10.11).

use crate::{BreadcrumbBar, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size,
    StyledElementImpl, VisualImpl,
};
use std::cell::Cell;

/// Lays out the items of a [`BreadcrumbBar`] on one line (WinUI's `BreadcrumbLayout`). The first
/// child is the ellipsis item, the others the items of the bar in their order. All of them are
/// measured; the desired size is the width of the items without the ellipsis and the height of the
/// tallest of them. When the items are wider than the available width, the arrange shows the
/// ellipsis item followed by as many of the last items as fit (at least the last one), and gives
/// the others an empty rectangle; otherwise the ellipsis item gets the empty rectangle. Every shown
/// child is as tall as the tallest shown child.
#[repr(C)]
pub struct BreadcrumbBarPanel {
    base: Panel,
    available_size: Cell<Size>,
    ellipsis_is_rendered: Cell<bool>,
    first_rendered_item_index_after_ellipsis: Cell<usize>,
    visible_items_count: Cell<usize>,
}

ferro_class!(BreadcrumbBarPanel: Panel);
ferro_class_info!(BreadcrumbBarPanel {
    new: BreadcrumbBarPanel::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(
    BreadcrumbBarPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl LayoutableImpl for BreadcrumbBarPanel {
    // "Measuring is performed in a single step, every element is measured, including the ellipsis
    // item, but the total amount of space needed is only composed of the non-ellipsis breadcrumbs"
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.available_size.set(available_size);
        let mut accumulated_crumbs_size = Size::new(0.0, 0.0);
        for (i, child) in this.children().snapshot().iter().enumerate() {
            child.measure(available_size);
            if i != 0 {
                let desired = child.desired_size();
                accumulated_crumbs_size.width += desired.width;
                accumulated_crumbs_size.height = accumulated_crumbs_size.height.max(desired.height);
            }
        }
        this.ellipsis_is_rendered.set(accumulated_crumbs_size.width > available_size.width);
        accumulated_crumbs_size
    }

    // "Arranging is performed in a single step, as many elements are tried to be drawn going from
    // the last element towards the first one, if there's not enough space, then the ellipsis button
    // is drawn"
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        let item_count = children.len();
        let mut first_element_to_render = 0;
        this.first_rendered_item_index_after_ellipsis.set(item_count.saturating_sub(1));
        this.visible_items_count.set(0);

        // "If the ellipsis must be drawn, then we find the index (x) of the first element to be
        // rendered, any element with a lower index than x will be hidden (except for the ellipsis
        // button) and every element after x (including x) will be drawn. At the very least, the
        // ellipis and the last item will be rendered"
        let ellipsis_is_rendered = this.ellipsis_is_rendered.get() && item_count > 0;
        if ellipsis_is_rendered {
            first_element_to_render = this.first_breadcrumb_bar_item_to_arrange(&children);
            this.first_rendered_item_index_after_ellipsis.set(first_element_to_render);
        }

        // The height of the shown items (GetBreadcrumbBarItemsHeight).
        let mut max_element_height = if ellipsis_is_rendered { children[0].desired_size().height } else { 0.0 };
        for child in children.iter().skip(first_element_to_render) {
            max_element_height = max_element_height.max(child.desired_size().height);
        }

        let mut accumulated_widths = 0.0;
        let mut arrange_item = |child: &Ref<crate::Control>| {
            let width = child.desired_size().width;
            child.arrange(Rect::new(accumulated_widths, 0.0, width, max_element_height));
            accumulated_widths += width;
        };
        let hide_item = |child: &Ref<crate::Control>| child.arrange(Rect::new(0.0, 0.0, 0.0, 0.0));

        // "If there is at least one element, we may render the ellipsis item"
        if let Some(ellipsis) = children.first() {
            if ellipsis_is_rendered {
                arrange_item(ellipsis);
            } else {
                hide_item(ellipsis);
            }
        }

        // "For each item, if the item has an equal or larger index to the first element to render,
        // then render it, otherwise, hide it and add it to the list of hidden items"
        let mut visible_items_count = 0;
        for (i, child) in children.iter().enumerate().skip(1) {
            if i < first_element_to_render {
                hide_item(child);
            } else {
                arrange_item(child);
                visible_items_count += 1;
            }
        }
        this.visible_items_count.set(visible_items_count);

        if let Some(breadcrumb) = this.templated_parent().and_then(|parent| parent.cast::<BreadcrumbBar>()) {
            breadcrumb.re_index_visible_elements_for_accessibility();
        }
        final_size
    }
}

impl BreadcrumbBarPanel {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            available_size: Cell::new(Size::new(0.0, 0.0)),
            ellipsis_is_rendered: Cell::new(false),
            first_rendered_item_index_after_ellipsis: Cell::new(0),
            visible_items_count: Cell::new(0),
        }
    }

    /// Creates a breadcrumb bar panel.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the last arrange showed the ellipsis item (`EllipsisIsRendered`).
    pub fn ellipsis_is_rendered(&self) -> bool {
        self.ellipsis_is_rendered.get()
    }

    /// The index of the child shown right after the ellipsis item in the last arrange
    /// (`FirstRenderedItemIndexAfterEllipsis`); the last child while the ellipsis is not shown.
    pub fn first_rendered_item_index_after_ellipsis(&self) -> usize {
        self.first_rendered_item_index_after_ellipsis.get()
    }

    /// The number of items, the ellipsis item left out, the last arrange showed
    /// (`GetVisibleItemsCount`).
    pub fn visible_items_count(&self) -> usize {
        self.visible_items_count.get()
    }

    /// The index of the first item shown after the ellipsis: the last item always, and the items
    /// before it as long as they fit with the ellipsis (`GetFirstBreadcrumbBarItemToArrange`).
    fn first_breadcrumb_bar_item_to_arrange(&self, children: &[Ref<crate::Control>]) -> usize {
        let item_count = children.len();
        let available_width = self.available_size.get().width;
        let mut accum_length = children[item_count - 1].desired_size().width + children[0].desired_size().width;
        for i in (0..item_count.saturating_sub(1)).rev() {
            let new_accum_length = accum_length + children[i].desired_size().width;
            if new_accum_length > available_width {
                return i + 1;
            }
            accum_length = new_accum_length;
        }
        0
    }
}
