//! RibbonTitleBar.cs: RibbonZoomControl. Copyright Wiesław Šoltés, MIT.
//! Native properties, template parts and commands; Rust callbacks use HandlerList.
use super::{RibbonControlBase, RibbonControlBaseImpl};
use crate::compat::RibbonInputSlider;
use ferroribbon_core::localization::RibbonStrings;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, TypeInfo, VisualImpl,
    data::converters::composite_format::{FormatArg, format_arg_with},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElementImpl, KeyboardNavigation},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::MarkupDelegate,
    reactive::{Disposable, IDisposable},
    utilities::{CultureInfo, EventArgs, HandlerList, math_utilities::max},
};
use ferroui_controls::{
    Button, Control, ControlImpl, Slider, ToolTip,
    automation::AutomationProperties,
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
type ValueHandler = dyn Fn(&RibbonZoomControl, f64);
type DialogHandler = dyn Fn(&RibbonZoomControl);

#[repr(C)]
pub struct RibbonZoomControl {
    base: RibbonControlBase,
    slider: RefCell<Option<Ref<Slider>>>,
    syncing_slider: Cell<bool>,
    user_change: Cell<bool>,
    parts: RefCell<Vec<Rc<dyn IDisposable>>>,
    value_handlers: HandlerList<ValueHandler>,
    dialog_handlers: HandlerList<DialogHandler>,
    markup_value_handlers: RefCell<Vec<(MarkupDelegate, u64)>>,
    markup_dialog_handlers: RefCell<Vec<(MarkupDelegate, u64)>>,
}
ferro_class!(RibbonZoomControl:RibbonControlBase);
ferro_class_info!(RibbonZoomControl {
    new:RibbonZoomControl::new,
    markup:{namespace:"FerroRibbon.Controls",
        properties:[ValueText:String{get:RibbonZoomControl::value_text}],
        events:[
            ValueChanged(Option<BoxedValue>,f64){add:RibbonZoomControl::add_value_changed,remove:RibbonZoomControl::remove_value_changed},
            ZoomDialogRequested(Option<BoxedValue>,EventArgs){add:RibbonZoomControl::add_zoom_dialog_requested,remove:RibbonZoomControl::remove_zoom_dialog_requested}
        ]
    }
});
ferro_impl_classes!(RibbonZoomControl:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_properties! {impl RibbonZoomControl {
    pub fn value_property()->StyledProperty<f64>{FerroProperty::register_with::<Self,_>("Value",ferroui_base::StyledPropertyOptions::new(100.).binding_writeback_uses_change_value(true))}
    pub fn minimum_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Minimum",10.)}
    pub fn maximum_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Maximum",500.)}
    pub fn step_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Step",10.)}
    pub fn value_text_property()->StyledProperty<String>{FerroProperty::register::<Self,_>("ValueText","100%".into())}
}}
impl RibbonZoomControl {
    fn static_constructor() {
        Self::value_property()
            .changed()
            .add_class_handler::<Self>(|c, e| c.on_value_changed(e.get_new_value::<f64>()))
            .detach();
        for p in [Self::minimum_property(), Self::maximum_property()] {
            p.changed()
                .add_class_handler::<Self>(|c, _| c.on_range_changed())
                .detach();
        }
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
            slider: RefCell::new(None),
            syncing_slider: Cell::new(false),
            user_change: Cell::new(false),
            parts: RefCell::new(vec![]),
            value_handlers: HandlerList::new(),
            dialog_handlers: HandlerList::new(),
            markup_value_handlers: RefCell::new(vec![]),
            markup_dialog_handlers: RefCell::new(vec![]),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    /// Named to avoid shadowing FerroObject's property-system setter.
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
    pub fn step(&self) -> f64 {
        self.get_value(Self::step_property())
    }
    pub fn set_step(&self, v: f64) {
        self.set_value(Self::step_property(), v);
    }
    pub fn value_text(&self) -> String {
        self.get_value(Self::value_text_property())
    }

    /// Retains subscription order and snapshot invocation during reentrancy.
    /// Dispose the native token to detach; ignoring it leaves the handler installed.
    pub fn value_changed(&self, handler: impl Fn(&Self, f64) + 'static) -> Rc<dyn IDisposable> {
        let token = self.value_handlers.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.value_handlers.remove(token);
            }
        })
    }
    pub fn zoom_dialog_requested(&self, handler: impl Fn(&Self) + 'static) -> Rc<dyn IDisposable> {
        let token = self.dialog_handlers.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.dialog_handlers.remove(token);
            }
        })
    }
    // These adapters are only the native compiled/runtime XAML event boundary.
    // Rust callers and the control's state machine share the same HandlerLists.
    pub fn add_value_changed(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let token = self.value_handlers.add(Rc::new(move |c, value| {
            callback
                .try_invoke(&[Some(Control::boxed(c.to_ref())), Some(Rc::new(value))])
                .expect("Zoom ValueChanged XAML handler");
        }));
        self.markup_value_handlers
            .borrow_mut()
            .push((handler, token));
    }
    pub fn remove_value_changed(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut handlers = self.markup_value_handlers.borrow_mut();
            handlers
                .iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|i| handlers.remove(i))
        };
        if let Some((_, token)) = removed {
            self.value_handlers.remove(token);
        }
    }
    pub fn add_zoom_dialog_requested(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let token = self.dialog_handlers.add(Rc::new(move |c| {
            callback
                .try_invoke(&[
                    Some(Control::boxed(c.to_ref())),
                    Some(Rc::new(EventArgs::EMPTY)),
                ])
                .expect("Zoom dialog XAML handler");
        }));
        self.markup_dialog_handlers
            .borrow_mut()
            .push((handler, token));
    }
    pub fn remove_zoom_dialog_requested(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut handlers = self.markup_dialog_handlers.borrow_mut();
            handlers
                .iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|i| handlers.remove(i))
        };
        if let Some((_, token)) = removed {
            self.dialog_handlers.remove(token);
        }
    }
    fn request_dialog(&self) {
        for (_, handler) in self.dialog_handlers.snapshot().iter() {
            handler(self);
        }
    }
    fn set_user_value(&self, value: f64) {
        self.user_change.set(true);
        let _reset = Reset(&self.user_change);
        self.set_numeric_value(value);
    }
    fn clamp(&self, value: f64) -> f64 {
        let minimum = self.minimum();
        let maximum = self.maximum();
        if maximum >= minimum {
            value.clamp(minimum, maximum)
        } else {
            minimum
        }
    }
    fn on_range_changed(&self) {
        self.sync_slider();
        let clamped = self.clamp(self.value());
        if clamped != self.value() {
            self.set_numeric_value(clamped);
        }
    }
    fn on_value_changed(&self, value: f64) {
        let clamped = self.clamp(value);
        if clamped != value {
            self.set_numeric_value(clamped);
            return;
        }
        let mut text = format_arg_with(
            &FormatArg::F64(value),
            "0",
            &CultureInfo::current_formatting(),
        )
        .expect("Zoom percentage format");
        text.push('%');
        self.set_value(Self::value_text_property(), text);
        self.sync_slider();
        for (_, handler) in self.value_handlers.snapshot().iter() {
            handler(self, value);
        }
        if self.user_change.get() {
            self.execute_command(Some(Rc::new(value)))
                .expect("Zoom command routing");
        }
    }
    fn slider_part(&self) -> Ref<Slider> {
        self.slider
            .borrow()
            .clone()
            .expect("Zoom slider removed during synchronization")
    }
    fn sync_slider(&self) {
        if self.slider.borrow().is_none() {
            return;
        }
        self.syncing_slider.set(true);
        let _reset = Reset(&self.syncing_slider);
        let minimum = self.minimum();
        RibbonInputSlider::ensure_finite(minimum);
        self.slider_part().set_minimum(minimum);
        let maximum = max(self.minimum(), self.maximum());
        RibbonInputSlider::ensure_finite(maximum);
        self.slider_part().set_maximum(maximum);
        let value = self.value();
        RibbonInputSlider::ensure_finite(value);
        self.slider_part().set_range_value(value);
    }
    fn part_label(direction: i32) -> Option<String> {
        let strings = RibbonStrings::current();
        let label = match direction {
            -1 => strings.zoom_out(),
            1 => strings.zoom_in(),
            _ => strings.zoom(),
        };
        label.map(|v| {
            String::from_utf16(v.as_utf16()).expect("Zoom localization must be native text")
        })
    }
    fn clear_part_handlers(&self) {
        for token in std::mem::take(&mut *self.parts.borrow_mut()) {
            token.dispose();
        }
    }
}
impl Drop for RibbonZoomControl {
    fn drop(&mut self) {
        self.clear_part_handlers();
    }
}
impl FerroObjectImpl for RibbonZoomControl {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        c.set_can_add_to_quick_access(false);
        KeyboardNavigation::set_is_tab_stop(c, false);
    }
}
impl StyledElementImpl for RibbonZoomControl {
    fn style_key_override(_: &Self) -> &'static TypeInfo {
        Self::TYPE
    }
}
impl TemplatedControlImpl for RibbonZoomControl {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        c.clear_part_handlers();
        Self::parent_on_apply_template(c, e);
        c.slider
            .replace(e.name_scope().find_as::<Slider>("PART_Slider"));
        for (name, direction) in [
            ("PART_ZoomOut", -1),
            ("PART_ZoomIn", 1),
            ("PART_Percent", 0),
        ] {
            if let Some(button) = e.name_scope().find_as::<Button>(name) {
                let weak = c.to_ref().downgrade();
                let token = button.add_disposable_handler(Button::click_event(), move |_, _| {
                    if let Some(c) = weak.upgrade() {
                        match direction {
                            -1 => c.set_user_value(
                                ((c.value() - c.step()) / c.step()).ceil() * c.step(),
                            ),
                            1 => c.set_user_value(
                                ((c.value() + c.step()) / c.step()).floor() * c.step(),
                            ),
                            _ => c.request_dialog(),
                        }
                    }
                });
                c.parts.borrow_mut().push(token.detach());
                AutomationProperties::set_name(&button, Self::part_label(direction).as_deref());
                if direction != 0 {
                    ToolTip::set_tip(
                        &button,
                        Self::part_label(direction).map(|v| Rc::new(v) as BoxedValue),
                    );
                }
            }
        }
        let slider = c.slider.borrow().clone();
        if let Some(slider) = slider {
            let label = Self::part_label(0);
            AutomationProperties::set_name(&slider, label.as_deref());
            c.sync_slider();
            let weak = c.to_ref().downgrade();
            let token =
                slider.add_disposable_handler(RangeBase::value_changed_event(), move |_, e| {
                    if let Some(c) = weak.upgrade()
                        && !c.syncing_slider.get()
                    {
                        c.set_user_value(e.new_value());
                    }
                });
            c.parts.borrow_mut().push(token.detach());
        }
    }
}
impl RibbonControlBaseImpl for RibbonZoomControl {
    fn invoke_core(c: &Self) -> bool {
        c.request_dialog();
        true
    }
}

#[cfg(test)]
#[path = "../zoom_control_tests.rs"]
mod tests;
