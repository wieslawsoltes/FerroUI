//! RibbonInputs.cs: native RibbonSlider push/pull and command state machine.
use super::{RibbonControlBaseImpl, RibbonInputBase, RibbonInputBaseImpl, RibbonItemHelper};
use crate::compat::RibbonInputSlider;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, TypeInfo, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::MarkupDelegate,
    utilities::math_utilities::{max, min},
};
use ferroui_controls::{
    Control, ControlImpl, Slider,
    primitives::{
        RangeBase, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Reset<'a>(&'a Cell<bool>);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
#[repr(C)]
pub struct RibbonSlider {
    base: RibbonInputBase,
    slider: RefCell<Option<Ref<Slider>>>,
    pushing: Cell<bool>,
    pulling: Cell<bool>,
    subscription: RefCell<Option<ferroui_base::reactive::Subscription>>,
    changed: RefCell<Option<MarkupDelegate>>,
    committed: RefCell<Option<MarkupDelegate>>,
}
ferro_class!(RibbonSlider:RibbonInputBase);
ferro_class_info!(RibbonSlider{new:RibbonSlider::new,markup:{namespace:"FerroRibbon.Controls",events:[ValueChanged(Option<BoxedValue>,f64){add:RibbonSlider::add_value_changed,remove:RibbonSlider::remove_value_changed},ValueCommitted(Option<BoxedValue>,f64){add:RibbonSlider::add_value_committed,remove:RibbonSlider::remove_value_committed}]}});
ferro_impl_classes!(RibbonSlider:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,RibbonInputBaseImpl);
ferro_properties! {impl RibbonSlider{
 pub fn value_property()->StyledProperty<f64>{FerroProperty::register_with::<Self,_>("Value",ferroui_base::StyledPropertyOptions::new(0.).binding_writeback_uses_change_value(true))}
 pub fn minimum_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Minimum",0.)}
 pub fn maximum_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Maximum",100.)}
 pub fn step_frequency_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("StepFrequency",1.)}
}}
impl RibbonSlider {
    pub fn construct() -> Self {
        Self {
            base: RibbonInputBase::construct(),
            slider: RefCell::new(None),
            pushing: Cell::new(false),
            pulling: Cell::new(false),
            subscription: RefCell::new(None),
            changed: RefCell::new(None),
            committed: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn static_constructor() {
        Self::value_property()
            .changed()
            .add_class_handler::<Self>(|c, e| c.on_value_changed(e.get_new_value::<f64>()))
            .detach();
        for p in [
            Self::minimum_property(),
            Self::maximum_property(),
            Self::step_frequency_property(),
        ] {
            p.changed()
                .add_class_handler::<Self>(|c, _| c.sync_slider())
                .detach();
        }
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_numeric_value(&self, v: f64) {
        self.set_value(Self::value_property(), v);
    }
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }
    pub fn set_minimum(&self, v: f64) {
        self.set_value(Self::minimum_property(), v);
    }
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }
    pub fn set_maximum(&self, v: f64) {
        self.set_value(Self::maximum_property(), v);
    }
    pub fn step_frequency(&self) -> f64 {
        self.get_value(Self::step_frequency_property())
    }
    pub fn set_step_frequency(&self, v: f64) {
        self.set_value(Self::step_frequency_property(), v);
    }
    pub fn add_value_changed(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::combine(self.changed.borrow().as_ref(), h.as_ref());
        self.changed.replace(v);
    }
    pub fn remove_value_changed(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::remove(self.changed.borrow().as_ref(), h.as_ref());
        self.changed.replace(v);
    }
    pub fn add_value_committed(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::combine(self.committed.borrow().as_ref(), h.as_ref());
        self.committed.replace(v);
    }
    pub fn remove_value_committed(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::remove(self.committed.borrow().as_ref(), h.as_ref());
        self.committed.replace(v);
    }
    fn raise(&self, slot: &RefCell<Option<MarkupDelegate>>, v: f64) {
        let h = slot.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(Rc::new(v))])
                .expect("RibbonSlider event handler");
        }
    }
    fn current_part(&self) -> Ref<Slider> {
        self.slider
            .borrow()
            .clone()
            .expect("Current slider part was removed during synchronization")
    }
    fn on_value_changed(&self, v: f64) {
        if !self.pulling.get() {
            self.sync_slider();
        }
        self.raise(&self.changed, v);
    }
    fn sync_slider(&self) {
        if self.slider.borrow().is_none() || self.pushing.get() {
            return;
        }
        let minimum = self.minimum();
        let maximum = max(minimum, self.maximum());
        let value = if self.value().is_nan() {
            minimum
        } else {
            min(maximum, max(minimum, self.value()))
        };
        self.pushing.set(true);
        {
            let _reset = Reset(&self.pushing);
            RibbonInputSlider::ensure_finite(minimum);
            self.current_part().set_minimum(minimum);
            RibbonInputSlider::ensure_finite(maximum);
            self.current_part().set_maximum(maximum);
            let p = self.current_part();
            if let Some(p) = p.cast::<RibbonInputSlider>() {
                p.set_step_frequency(self.step_frequency());
            } else {
                p.set_tick_frequency(self.step_frequency());
            }
            RibbonInputSlider::ensure_finite(value);
            self.current_part().set_range_value(value);
        }
        let current = self.value();
        if !(value == current || (value.is_nan() && current.is_nan())) {
            self.pull(value);
        }
    }
    fn pull(&self, v: f64) {
        self.pulling.set(true);
        let _reset = Reset(&self.pulling);
        self.set_numeric_value(v);
    }
    fn on_slider_changed(&self, v: f64) {
        if self.pushing.get() {
            return;
        }
        self.pull(v);
        self.execute_command(Some(Rc::new(v)))
            .expect("RibbonSlider.ExecuteCommand");
        self.raise(&self.committed, v);
    }
}
impl StyledElementImpl for RibbonSlider {
    fn style_key_override(_: &Self) -> &'static TypeInfo {
        Self::TYPE
    }
}
impl FerroObjectImpl for RibbonSlider {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        c.set_input_width(120.);
    }
}
impl TemplatedControlImpl for RibbonSlider {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        let previous = c.subscription.borrow_mut().take();
        if let Some(s) = previous {
            s.dispose();
        }
        Self::parent_on_apply_template(c, e);
        c.slider
            .replace(e.name_scope().find_as::<Slider>("PART_Slider"));
        if c.slider.borrow().is_some() {
            c.sync_slider();
            let p = c.current_part();
            let weak = c.to_ref().downgrade();
            let s = p.add_disposable_handler(RangeBase::value_changed_event(), move |_, e| {
                if let Some(c) = weak.upgrade() {
                    c.on_slider_changed(e.new_value());
                }
            });
            c.subscription.replace(Some(s));
            ferroui_controls::automation::AutomationProperties::set_name(
                &c.current_part(),
                Some(&c.label().unwrap_or_default()),
            );
        }
    }
}
impl RibbonControlBaseImpl for RibbonSlider {
    fn create_linked_copy_core(c: &Self) -> Option<Ref<Control>> {
        let copy = Self::new();
        RibbonItemHelper::link_common(c, &copy);
        for p in [
            Self::minimum_property().as_property(),
            Self::maximum_property().as_property(),
            Self::step_frequency_property().as_property(),
            RibbonInputBase::input_width_property().as_property(),
        ] {
            RibbonItemHelper::link(c, &copy, p, false);
        }
        RibbonItemHelper::link(c, &copy, Self::value_property().as_property(), true);
        let source = c.to_ref();
        copy.add_value_committed(Some(MarkupDelegate::new(move |args| {
            let v = *args[1].as_ref().unwrap().downcast_ref::<f64>().unwrap();
            source
                .execute_command(Some(Rc::new(v)))
                .expect("Linked slider.ExecuteCommand");
            source.raise(&source.committed, v);
            None
        })));
        Some(copy.upcast())
    }
}

#[cfg(test)]
#[path = "../slider_tests.rs"]
mod tests;
