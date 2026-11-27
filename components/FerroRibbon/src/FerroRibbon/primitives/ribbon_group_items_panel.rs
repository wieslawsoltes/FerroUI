//! Native group-item layout from RibbonGroupItemsPanel.cs. Native item/container
//! branches are present.
use super::{RibbonMetrics, ribbon_hidden_focus};
use crate::controls::IRibbonItem;
use ferroribbon_core::{
    RibbonGroupItemsLayout, RibbonItemSize,
    collections::RibbonReadOnlyList,
    layout::{RibbonArrangeItem, RibbonArrangeResult, RibbonGroupItemsArranger},
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    utilities::math_utilities::min,
};
use ferroui_controls::{Control, ControlImpl, Panel, PanelImpl};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[repr(C)]
pub struct RibbonGroupItemsPanel {
    base: Panel,
    columns: RefCell<Option<RibbonArrangeResult>>,
    visible: RefCell<Vec<Ref<Control>>>,
    visible_version: Cell<i32>,
    items_layout: Cell<RibbonGroupItemsLayout>,
    row_count: Cell<i32>,
    is_simplified: Cell<bool>,
    metrics: RefCell<Option<Arc<RibbonMetrics>>>,
    in_line_mask: RefCell<Option<RibbonReadOnlyList<bool>>>,
    simplified_spacing: Cell<f64>,
}
ferro_class!(RibbonGroupItemsPanel:Panel);
ferro_impl_classes!(RibbonGroupItemsPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(RibbonGroupItemsPanel {
    new:RibbonGroupItemsPanel::new,
    markup:{namespace:"FerroRibbon.Controls.Primitives",content:Children,
        properties:[
            ItemsLayout:RibbonGroupItemsLayout{get:RibbonGroupItemsPanel::items_layout,set:RibbonGroupItemsPanel::set_items_layout},
            RowCount:i32{get:RibbonGroupItemsPanel::row_count,set:RibbonGroupItemsPanel::set_row_count},
            IsSimplified:bool{get:RibbonGroupItemsPanel::is_simplified,set:RibbonGroupItemsPanel::set_is_simplified},
            Metrics:Option<Arc<RibbonMetrics>>{get:RibbonGroupItemsPanel::metrics,set:RibbonGroupItemsPanel::set_metrics},
            InLineMask:Option<RibbonReadOnlyList<bool>>{get:RibbonGroupItemsPanel::in_line_mask,set:RibbonGroupItemsPanel::set_in_line_mask},
            SimplifiedSpacing:f64{get:RibbonGroupItemsPanel::simplified_spacing,set:RibbonGroupItemsPanel::set_simplified_spacing},
        ],
        methods:[fn IsFullHeight(Ref<Control>)->bool=>|this:&RibbonGroupItemsPanel,child:Ref<Control>|this.is_full_height(&child)],
    },
});
impl RibbonGroupItemsPanel {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            columns: RefCell::new(None),
            visible: RefCell::new(Vec::new()),
            visible_version: Cell::new(0),
            items_layout: Cell::new(RibbonGroupItemsLayout::Columns),
            row_count: Cell::new(3),
            is_simplified: Cell::new(false),
            metrics: RefCell::new(Some(RibbonMetrics::comfortable())),
            in_line_mask: RefCell::new(None),
            simplified_spacing: Cell::new(1.),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items_layout(&self) -> RibbonGroupItemsLayout {
        self.items_layout.get()
    }
    pub fn set_items_layout(&self, value: RibbonGroupItemsLayout) {
        self.items_layout.set(value)
    }
    pub fn row_count(&self) -> i32 {
        self.row_count.get()
    }
    pub fn set_row_count(&self, value: i32) {
        self.row_count.set(value)
    }
    pub fn is_simplified(&self) -> bool {
        self.is_simplified.get()
    }
    pub fn set_is_simplified(&self, value: bool) {
        self.is_simplified.set(value)
    }
    pub fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        self.metrics.borrow().clone()
    }
    pub fn set_metrics(&self, value: Option<Arc<RibbonMetrics>>) {
        self.metrics.replace(value);
    }
    pub fn in_line_mask(&self) -> Option<RibbonReadOnlyList<bool>> {
        self.in_line_mask.borrow().clone()
    }
    pub fn set_in_line_mask(&self, value: Option<RibbonReadOnlyList<bool>>) {
        self.in_line_mask.replace(value);
    }
    pub fn simplified_spacing(&self) -> f64 {
        self.simplified_spacing.get()
    }
    pub fn set_simplified_spacing(&self, value: f64) {
        self.simplified_spacing.set(value)
    }
    pub(crate) fn is_in_line(&self, index: i32) -> bool {
        // Re-read the property after user Count/index access, as the original
        // short-circuit expression does for a custom live IReadOnlyList.
        if self.in_line_mask().is_none()
            || index >= self.in_line_mask().expect("InLineMask became null").count()
        {
            true
        } else {
            self.in_line_mask()
                .expect("InLineMask became null")
                .get(index)
        }
    }
    pub fn is_full_height(&self, child: &Control) -> bool {
        if let Some(separator) = child.downcast_ref::<crate::controls::RibbonSeparator>() {
            return separator.orientation() == ferroui_base::layout::Orientation::Vertical;
        }
        if child
            .downcast_ref::<crate::controls::RibbonItemsContainer>()
            .is_some()
        {
            return child.desired_size().height
                > self
                    .metrics()
                    .expect("RibbonGroupItemsPanel.Metrics is null")
                    .row_height()
                    * 1.6;
        }
        let boxed: BoxedValue = Rc::new(child.to_ref());
        if let Some(value) =
            ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItem>>()).flatten()
            && value
                .downcast_ref::<Rc<dyn IRibbonItem>>()
                .expect("registered item conversion")
                .current_size()
                == RibbonItemSize::Large
        {
            return true;
        }
        child.desired_size().height
            > self
                .metrics()
                .expect("RibbonGroupItemsPanel.Metrics is null")
                .row_height()
                * 1.6
    }
    fn changed_visible(&self) {
        self.visible_version
            .set(self.visible_version.get().wrapping_add(1))
    }
    fn clear_visible(&self) {
        // List.Clear retains its backing capacity. Drop old child references
        // outside the borrow, then reuse the allocation unless destruction
        // reentered measure and installed a newer visible list.
        let mut previous = std::mem::take(&mut *self.visible.borrow_mut());
        self.changed_visible();
        let cleared_at = self.visible_version.get();
        previous.clear();
        if self.visible_version.get() == cleared_at {
            let replaced = self.visible.replace(previous);
            debug_assert!(replaced.is_empty());
        }
    }
    fn each_visible(&self, mut action: impl FnMut(&Ref<Control>)) {
        let version = self.visible_version.get();
        let mut index = 0;
        loop {
            assert_eq!(
                version,
                self.visible_version.get(),
                "RibbonGroupItemsPanel visible items changed during enumeration"
            );
            let child = self.visible.borrow().get(index).cloned();
            let Some(child) = child else { break };
            action(&child);
            index += 1;
        }
    }
}
impl LayoutableImpl for RibbonGroupItemsPanel {
    fn measure_override(this: &Self, _available: Size) -> Size {
        let metrics = this.metrics();
        this.clear_visible();
        let children = this.children();
        let mut i = 0;
        while i < children.count() {
            let child = children.get(i);
            i += 1;
            if !child.is_visible() {
                continue;
            }
            child.measure(Size::INFINITY);
            this.visible.borrow_mut().push(child);
            this.changed_visible();
        }
        if this.is_simplified() {
            let mut x = 0.;
            let mut count = 0_i32;
            let mut i = 0;
            while i < children.count() {
                let child = children.get(i);
                let index = i32::try_from(i).expect("source Int32 child index");
                i += 1;
                if !child.is_visible() || !this.is_in_line(index) {
                    continue;
                }
                x += child.desired_size().width;
                count = count.wrapping_add(1);
            }
            return Size::new(
                x + 0.max(count.wrapping_sub(1)) as f64 * this.simplified_spacing(),
                metrics
                    .expect("RibbonGroupItemsPanel.Metrics is null")
                    .simplified_item_height(),
            );
        }
        let content_height = metrics
            .expect("RibbonGroupItemsPanel.Metrics is null")
            .group_content_height();
        if this.items_layout() == RibbonGroupItemsLayout::Rows {
            let mut width: Option<f64> = None;
            this.each_visible(|child| {
                let next = child.desired_size().width;
                width = Some(match width {
                    Some(old) if !old.is_nan() && next <= old => old,
                    Some(old) if next.is_nan() => old,
                    _ => next,
                })
            });
            return Size::new(width.unwrap_or(0.), content_height);
        }
        let mut items = Vec::with_capacity(this.visible.borrow().len());
        this.each_visible(|child| {
            items.push(RibbonArrangeItem::new(
                child.desired_size().width,
                child.desired_size().height,
                this.is_full_height(child),
            ))
        });
        let columns = RibbonGroupItemsArranger::arrange_with_options(
            &items,
            content_height,
            this.row_count(),
            2.,
            false,
        );
        let width = columns.width;
        this.columns.replace(Some(columns));
        Size::new(width, content_height)
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let hidden = Rect::new(0., 0., 0., 0.);
        if this.is_simplified() {
            let mut x = 0.;
            let children = this.children();
            let mut i = 0;
            while i < children.count() {
                let child = children.get(i);
                let index = i32::try_from(i).expect("source Int32 child index");
                i += 1;
                if !child.is_visible() {
                    continue;
                }
                if !this.is_in_line(index) {
                    child.arrange(hidden);
                    ribbon_hidden_focus::set(&child.upcast(), true);
                    continue;
                }
                let h = child.desired_size().height;
                child.arrange(Rect::new(
                    x,
                    (final_size.height - h) / 2.,
                    child.desired_size().width,
                    h,
                ));
                ribbon_hidden_focus::set(&child.clone().upcast(), false);
                x += child.desired_size().width + this.simplified_spacing();
            }
            return final_size;
        }
        this.each_visible(|child| ribbon_hidden_focus::set(&child.clone().upcast(), false));
        if this.items_layout() == RibbonGroupItemsLayout::Rows {
            let rows = 1.max(
                i32::try_from(this.visible.borrow().len()).expect("source Int32 visible Count"),
            );
            let slot = final_size.height
                / rows.max(if this.row_count() == 3 && rows < 3 {
                    rows
                } else {
                    this.row_count()
                }) as f64;
            let total = slot * rows as f64;
            let top = (final_size.height - total) / 2.;
            let mut i = 0;
            while i < this.visible.borrow().len() {
                let child = this.visible.borrow()[i].clone();
                let h = min(child.desired_size().height, slot);
                child.arrange(Rect::new(
                    0.,
                    top + i as f64 * slot + (slot - h) / 2.,
                    child.desired_size().width,
                    h,
                ));
                i += 1;
            }
            return final_size;
        }
        let has_columns = this.columns.borrow().is_some();
        if has_columns {
            let mut i = 0;
            while i < this.visible.borrow().len() {
                let placement = {
                    let columns = this.columns.borrow();
                    let columns = columns.as_ref().expect("cached columns became null");
                    let Some(placement) = columns.items.get(i) else {
                        break;
                    };
                    *placement
                };
                let child = this.visible.borrow()[i].clone();
                child.arrange(Rect::new(
                    placement.x,
                    placement.y,
                    placement.width,
                    placement.height,
                ));
                i += 1;
            }
        }
        final_size
    }
}
