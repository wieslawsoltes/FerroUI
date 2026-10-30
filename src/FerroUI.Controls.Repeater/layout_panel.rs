// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use super::{ElementRealizationOptions, Layout, LayoutContext, LayoutContextHost, NonVirtualizingLayoutContext};
use ferroui_controls::utils::BorderRenderHelper;
use ferroui_controls::{Border, Control, ControlImpl, Decorator, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::{BackgroundSizing, BoxShadows, DrawingContext, IBrush};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    BoxedValue, CornerRadius, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    Thickness, Visual, VisualImpl, WeakRef};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// Hosts explicit children using a shareable Layout, including virtualizing
/// layouts adapted to a fixed collection. Border and padding inset the layout.
#[repr(C)]
pub struct LayoutPanel {
    base: Panel,
    context: OnceCell<Ref<NonVirtualizingLayoutContext>>,
    effective_layout: RefCell<Option<Ref<Layout>>>,
    layout_state: RefCell<Option<BoxedValue>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    border_renderer: RefCell<BorderRenderHelper>,
}
ferro_class!(LayoutPanel: Panel);
ferro_class_info!(LayoutPanel {
    new: LayoutPanel::new,
    markup: { namespace: "FerroUI.Controls", content: Children },
});
ferro_impl_classes!(LayoutPanel: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);

impl FerroObjectImpl for LayoutPanel {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::layout_property().as_property() { this.change_layout(); }
    }
}
impl VisualImpl for LayoutPanel {
    fn render(this: &Self, context: &mut DrawingContext) {
        this.border_renderer.borrow_mut().render(context, this.bounds().size(), this.border_thickness(),
            this.corner_radius(), BackgroundSizing::InnerBorderEdge, this.background().as_ref(),
            this.border_brush().as_ref(), &BoxShadows::default());
    }
    fn clip_to_bounds_radius(this: &Self) -> Option<CornerRadius> { Some(this.corner_radius()) }
}
impl LayoutableImpl for LayoutPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let inset = this.padding() + this.border_thickness();
        let inner = Size::new((available.width - inset.left - inset.right).max(0.0),
            (available.height - inset.top - inset.bottom).max(0.0));
        let desired = if let Some(layout) = this.layout() {
            layout.measure(&this.layout_context(), inner)
        } else {
            let mut desired = Size::default();
            for child in this.children().snapshot().iter() {
                child.measure(inner);
                desired.width = desired.width.max(child.desired_size().width);
                desired.height = desired.height.max(child.desired_size().height);
            }
            desired
        };
        Size::new(desired.width + inset.left + inset.right, desired.height + inset.top + inset.bottom)
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let inset = this.padding() + this.border_thickness();
        let inner = Size::new((final_size.width - inset.left - inset.right).max(0.0),
            (final_size.height - inset.top - inset.bottom).max(0.0));
        if let Some(layout) = this.layout() {
            let result = layout.arrange(&this.layout_context(), inner);
            if inset.left != 0.0 || inset.top != 0.0 {
                for child in this.children().snapshot().iter() {
                    if let Some(slot) = child.previous_arrange() {
                        child.arrange(Rect::new(slot.x + inset.left, slot.y + inset.top, slot.width, slot.height));
                    }
                }
            }
            Size::new(result.width + inset.left + inset.right, result.height + inset.top + inset.bottom)
        } else {
            for child in this.children().snapshot().iter() {
                child.arrange(Rect::new(inset.left, inset.top, inner.width, inner.height));
            }
            final_size
        }
    }
}
ferro_properties! { impl LayoutPanel {
    pub fn layout_property() -> StyledProperty<Option<Ref<Layout>>> {
        FerroProperty::register::<LayoutPanel, _>("Layout", None)
    }
    pub fn border_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        Border::border_brush_property().add_owner::<LayoutPanel>()
    }
    pub fn border_thickness_property() -> StyledProperty<Thickness> {
        Border::border_thickness_property().add_owner::<LayoutPanel>()
    }
    pub fn corner_radius_property() -> StyledProperty<CornerRadius> {
        Border::corner_radius_property().add_owner::<LayoutPanel>()
    }
    pub fn padding_property() -> StyledProperty<Thickness> {
        Decorator::padding_property().add_owner::<LayoutPanel>()
    }
} }
impl LayoutPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<LayoutPanel>(&[Self::border_thickness_property().as_property(), Self::padding_property().as_property()]);
        Visual::affects_render::<LayoutPanel>(&[Self::border_brush_property().as_property(),
            Self::border_thickness_property().as_property(), Self::corner_radius_property().as_property()]);
    }
    pub fn construct() -> Self {
        Self { base: Panel::construct(), context: OnceCell::new(), effective_layout: RefCell::new(None),
            layout_state: RefCell::new(None), subscriptions: RefCell::new(Vec::new()),
            border_renderer: RefCell::new(BorderRenderHelper::new()) }
    }
    pub fn new() -> Ref<Self> { instantiate(Self::construct()) }
    pub fn layout(&self) -> Option<Ref<Layout>> { self.get_value(Self::layout_property()) }
    pub fn set_layout(&self, value: Option<Ref<Layout>>) { self.set_value(Self::layout_property(), value); }
    pub fn border_brush(&self) -> Option<Rc<dyn IBrush>> { self.get_value(Self::border_brush_property()) }
    pub fn set_border_brush(&self, value: Option<Rc<dyn IBrush>>) { self.set_value(Self::border_brush_property(), value); }
    pub fn border_thickness(&self) -> Thickness { self.get_value(Self::border_thickness_property()) }
    pub fn set_border_thickness(&self, value: Thickness) { self.set_value(Self::border_thickness_property(), value); }
    pub fn corner_radius(&self) -> CornerRadius { self.get_value(Self::corner_radius_property()) }
    pub fn set_corner_radius(&self, value: CornerRadius) { self.set_value(Self::corner_radius_property(), value); }
    pub fn padding(&self) -> Thickness { self.get_value(Self::padding_property()) }
    pub fn set_padding(&self, value: Thickness) { self.set_value(Self::padding_property(), value); }
    pub fn layout_state(&self) -> Option<BoxedValue> { self.layout_state.borrow().clone() }
    pub fn set_layout_state(&self, value: Option<BoxedValue>) { *self.layout_state.borrow_mut() = value; }
    pub fn layout_context(&self) -> Ref<LayoutContext> {
        self.context.get_or_init(|| NonVirtualizingLayoutContext::with_host(Rc::new(
            LayoutPanelLayoutContext(self.to_ref().downgrade())))).clone().upcast()
    }
    fn change_layout(&self) {
        let context = self.layout_context();
        let old = self.effective_layout.borrow_mut().take();
        if let Some(old) = old {
            old.uninitialize_for_context(&context);
            // The outgoing layout can observe its state while uninitializing.
            // Ownership ends here even if a custom layout leaves the value in
            // the context; a replacement must initialize with its own state.
            self.set_layout_state(None);
        }
        for subscription in std::mem::take(&mut *self.subscriptions.borrow_mut()) { subscription.dispose(); }
        let layout = self.layout();
        *self.effective_layout.borrow_mut() = layout.clone();
        if let Some(layout) = layout {
            layout.initialize_for_context(&context);
            let weak = self.to_ref().downgrade();
            let measure = layout.measure_invalidated(move || { if let Some(owner) = weak.upgrade() { owner.invalidate_measure(); } });
            let weak = self.to_ref().downgrade();
            let arrange = layout.arrange_invalidated(move || { if let Some(owner) = weak.upgrade() { owner.invalidate_arrange(); } });
            self.subscriptions.borrow_mut().extend([measure, arrange]);
        }
        self.invalidate_measure();
    }
}
impl Drop for LayoutPanel {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) { subscription.dispose(); }
    }
}

struct LayoutPanelLayoutContext(WeakRef<LayoutPanel>);
impl LayoutPanelLayoutContext {
    fn owner(&self) -> Ref<LayoutPanel> { self.0.upgrade().expect("LayoutPanel was released.") }
}
impl LayoutContextHost for LayoutPanelLayoutContext {
    fn item_count(&self) -> i32 { self.owner().children().count() as i32 }
    fn get_item_at(&self, index: i32) -> Option<BoxedValue> {
        self.children().get(index as usize).cloned().map(Control::boxed)
    }
    fn get_or_create_element_at(&self, index: i32, _options: ElementRealizationOptions) -> Ref<Control> {
        self.children()[index as usize].clone()
    }
    fn recycle_element(&self, _element: &Ref<Control>) { /* Explicit children remain owned by the panel. */ }
    fn layout_state(&self) -> Option<BoxedValue> { self.owner().layout_state() }
    fn set_layout_state(&self, value: Option<BoxedValue>) { self.owner().set_layout_state(value); }
    fn realization_rect(&self) -> Rect { Rect::from_size(Size::INFINITY) }
    fn layout_origin(&self) -> Point { Point::default() }
    fn set_layout_origin(&self, value: Point) { assert_eq!(value, Point::default(), "A LayoutPanel has a fixed layout origin."); }
    fn children(&self) -> Vec<Ref<Control>> { self.owner().children().snapshot().to_vec() }
    fn is_virtualizing(&self) -> bool { false }
}
