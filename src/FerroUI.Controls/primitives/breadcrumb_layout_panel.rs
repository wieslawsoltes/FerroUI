//! The panel of WinUI's `BreadcrumbBar`: a structural port of `BreadcrumbLayout`
//! (`controls/dev/Breadcrumb/BreadcrumbLayout.cpp` and `.h` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::{ControlImpl, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The panel of the items of a [`BreadcrumbBar`](crate::BreadcrumbBar) (WinUI's `BreadcrumbLayout`,
/// the layout of its `ItemsRepeater`): its first child is the ellipsis item, the others the items
/// of the bar. It lays out as many items as fit, from the last one towards the first; when they do
/// not all fit, the ellipsis item comes first and the items before the first one that fits are
/// hidden (arranged at an empty rectangle). At the very least the ellipsis and the last item are
/// shown.
#[repr(C)]
pub struct BreadcrumbLayoutPanel {
    base: Panel,
    available_size: Cell<Size>,
    ellipsis_is_rendered: Cell<bool>,
    first_rendered_item_index_after_ellipsis: Cell<usize>,
    visible_items_count: Cell<usize>,
    arranged: RefCell<Option<Rc<dyn Fn()>>>,
}

ferro_class!(BreadcrumbLayoutPanel: Panel);
ferroui_base::ferro_class_info!(BreadcrumbLayoutPanel {
    new: BreadcrumbLayoutPanel::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(BreadcrumbLayoutPanel: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);
ferroui_base::ferro_impl_classes!(BreadcrumbLayoutPanel: FerroObjectImpl);

impl LayoutableImpl for BreadcrumbLayoutPanel {
    // "Measuring is performed in a single step, every element is measured, including the ellipsis
    // item, but the total amount of space needed is only composed of the non-ellipsis breadcrumbs"
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.available_size.set(available_size);
        let children = this.children().snapshot();
        let mut accumulated = Size::new(0.0, 0.0);
        for (index, child) in children.iter().enumerate() {
            child.measure(available_size);
            if index != 0 {
                let desired = child.desired_size();
                accumulated.width += desired.width;
                accumulated.height = accumulated.height.max(desired.height);
            }
        }
        this.ellipsis_is_rendered.set(accumulated.width > available_size.width);
        accumulated
    }

    // "Arranging is performed in a single step, as many elements are tried to be drawn going from
    // the last element towards the first one, if there's not enough space, then the ellipsis
    // button is drawn"
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        let item_count = children.len();
        let mut first_element_to_render = 0;
        this.first_rendered_item_index_after_ellipsis.set(item_count.saturating_sub(1));
        this.visible_items_count.set(0);
        let ellipsis_is_rendered = this.ellipsis_is_rendered.get();
        if ellipsis_is_rendered {
            first_element_to_render = this.get_first_breadcrumb_bar_item_to_arrange(&children);
            this.first_rendered_item_index_after_ellipsis.set(first_element_to_render);
        }

        let mut accumulated_widths = 0.0;
        // GetBreadcrumbBarItemsHeight
        let mut max_element_height: f64 = 0.0;
        if ellipsis_is_rendered {
            if let Some(ellipsis) = children.first() {
                max_element_height = ellipsis.desired_size().height;
            }
        }
        for child in children.iter().skip(first_element_to_render) {
            max_element_height = max_element_height.max(child.desired_size().height);
        }

        let arrange_item = |child: &crate::Control, accumulated_widths: &mut f64| {
            let size = child.desired_size();
            child.arrange(Rect::new(*accumulated_widths, 0.0, size.width, max_element_height));
            *accumulated_widths += size.width;
        };
        let hide_item = |child: &crate::Control| child.arrange(Rect::new(0.0, 0.0, 0.0, 0.0));

        if let Some(ellipsis) = children.first() {
            if ellipsis_is_rendered {
                arrange_item(ellipsis, &mut accumulated_widths);
            } else {
                hide_item(ellipsis);
            }
        }
        for (index, child) in children.iter().enumerate().skip(1) {
            if index < first_element_to_render {
                hide_item(child);
            } else {
                arrange_item(child, &mut accumulated_widths);
                this.visible_items_count.set(this.visible_items_count.get() + 1);
            }
        }

        let arranged = this.arranged.borrow().clone();
        if let Some(arranged) = arranged {
            arranged();
        }
        final_size
    }
}

impl BreadcrumbLayoutPanel {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            available_size: Cell::new(Size::new(0.0, 0.0)),
            ellipsis_is_rendered: Cell::new(false),
            first_rendered_item_index_after_ellipsis: Cell::new(0),
            visible_items_count: Cell::new(0),
            arranged: RefCell::new(None),
        }
    }

    /// Creates an empty panel.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the last arrangement shows the ellipsis item (WinUI's `EllipsisIsRendered`).
    pub fn ellipsis_is_rendered(&self) -> bool {
        self.ellipsis_is_rendered.get()
    }

    /// The index of the first child shown after the ellipsis item (WinUI's
    /// `FirstRenderedItemIndexAfterEllipsis`).
    pub fn first_rendered_item_index_after_ellipsis(&self) -> usize {
        self.first_rendered_item_index_after_ellipsis.get()
    }

    /// The number of items shown, without the ellipsis item (WinUI's `GetVisibleItemsCount`).
    pub fn visible_items_count(&self) -> usize {
        self.visible_items_count.get()
    }

    /// Sets what runs after each arrangement (the bar indexes its visible items for automation).
    pub(crate) fn set_arranged(&self, arranged: Option<Rc<dyn Fn()>>) {
        *self.arranged.borrow_mut() = arranged;
    }

    /// The index of the first item that fits after the ellipsis and the last item, going from the
    /// last towards the first (WinUI's `GetFirstBreadcrumbBarItemToArrange`).
    fn get_first_breadcrumb_bar_item_to_arrange(&self, children: &[Ref<crate::Control>]) -> usize {
        let item_count = children.len();
        if item_count < 2 {
            return 0;
        }
        let available_width = self.available_size.get().width;
        let mut accumulated = children[item_count - 1].desired_size().width + children[0].desired_size().width;
        for index in (0..item_count - 1).rev() {
            let next = accumulated + children[index].desired_size().width;
            if next > available_width {
                return index + 1;
            }
            accumulated = next;
        }
        0
    }
}
