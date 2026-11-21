//! RibbonInputs.cs spinner state machine, native input/template/capture services.
use super::{
    RibbonControlBaseImpl, RibbonInputBase, RibbonInputBaseImpl, RibbonItemHelper,
    RibbonKeyTipResult,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Point, Ref, StyledElementImpl,
    StyledProperty, TypeInfo, VisualImpl,
    data::converters::composite_format::{FormatArg, format_arg_with},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, NavigationMethod,
        PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutingStrategies},
    layout::LayoutableImpl,
    metadata::MarkupDelegate,
    reactive::IDisposable,
    utilities::{
        CultureInfo,
        math_utilities::{max, min},
    },
};
use ferroui_controls::{
    Button, Control, ControlImpl, RepeatButton, TextBox, TopLevel,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
pub struct RibbonSpinner {
    base: RibbonInputBase,
    text_box: RefCell<Option<Ref<TextBox>>>,
    up: RefCell<Option<Ref<RepeatButton>>>,
    down: RefCell<Option<Ref<RepeatButton>>>,
    scrub_handle: RefCell<Option<Ref<Control>>>,
    scrub_start: Cell<Option<Point>>,
    scrub_value: Cell<f64>,
    scrub_old_value: Cell<f64>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    value_changed: RefCell<Option<MarkupDelegate>>,
    value_committed: RefCell<Option<MarkupDelegate>>,
}
ferro_class!(RibbonSpinner:RibbonInputBase);
ferro_class_info!(RibbonSpinner{new:RibbonSpinner::new,markup:{namespace:"FerroRibbon.Controls",events:[ValueChanged(Option<BoxedValue>,Rc<RibbonValueChangedEventArgs>){add:RibbonSpinner::add_value_changed,remove:RibbonSpinner::remove_value_changed},ValueCommitted(Option<BoxedValue>,Rc<RibbonValueChangedEventArgs>){add:RibbonSpinner::add_value_committed,remove:RibbonSpinner::remove_value_committed}],methods:[fn FormatValue(f64)->String=>RibbonSpinner::format_value,fn CommitText()=>RibbonSpinner::commit_text]}});
ferro_impl_classes!(RibbonSpinner:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,RibbonInputBaseImpl);
ferro_properties! {impl RibbonSpinner{
 pub fn value_property()->StyledProperty<f64>{FerroProperty::register_with::<Self,_>("Value",ferroui_base::StyledPropertyOptions::new(0.).binding_writeback_uses_change_value(true))}
 pub fn minimum_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Minimum",f64::MIN)}
 pub fn maximum_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Maximum",f64::MAX)}
 pub fn increment_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Increment",1.)}
 pub fn format_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Format",Some("0.##".into()))}
 pub fn unit_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Unit",None)}
 pub fn is_scrub_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsScrubEnabled",true)}
}}
impl RibbonSpinner {
    fn static_constructor() {
        Self::value_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                let (old, new) = e.get_old_and_new_value::<f64>();
                c.on_value_changed(old, new);
            })
            .detach();
        Self::minimum_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.coerce_value())
            .detach();
        Self::maximum_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.coerce_value())
            .detach();
        Self::format_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_text())
            .detach();
        Self::unit_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_text())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonInputBase::construct(),
            text_box: RefCell::new(None),
            up: RefCell::new(None),
            down: RefCell::new(None),
            scrub_handle: RefCell::new(None),
            scrub_start: Cell::new(None),
            scrub_value: Cell::new(0.),
            scrub_old_value: Cell::new(0.),
            subscriptions: RefCell::new(vec![]),
            value_changed: RefCell::new(None),
            value_committed: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_numeric_value(&self, value: f64) {
        self.set_value(Self::value_property(), self.coerce(value));
    }
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }
    pub fn set_minimum(&self, v: f64) {
        self.set_value(Self::minimum_property(), v)
    }
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }
    pub fn set_maximum(&self, v: f64) {
        self.set_value(Self::maximum_property(), v)
    }
    pub fn increment(&self) -> f64 {
        self.get_value(Self::increment_property())
    }
    pub fn set_increment(&self, v: f64) {
        self.set_value(Self::increment_property(), v)
    }
    pub fn format(&self) -> Option<String> {
        self.get_value(Self::format_property())
    }
    pub fn set_format(&self, v: Option<String>) {
        self.set_value(Self::format_property(), v)
    }
    pub fn unit(&self) -> Option<String> {
        self.get_value(Self::unit_property())
    }
    pub fn set_unit(&self, v: Option<String>) {
        self.set_value(Self::unit_property(), v)
    }
    pub fn is_scrub_enabled(&self) -> bool {
        self.get_value(Self::is_scrub_enabled_property())
    }
    pub fn set_is_scrub_enabled(&self, v: bool) {
        self.set_value(Self::is_scrub_enabled_property(), v)
    }
    pub fn add_value_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.value_changed.borrow().as_ref(), h.as_ref());
        self.value_changed.replace(next);
    }
    pub fn remove_value_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.value_changed.borrow().as_ref(), h.as_ref());
        self.value_changed.replace(next);
    }
    pub fn add_value_committed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.value_committed.borrow().as_ref(), h.as_ref());
        self.value_committed.replace(next);
    }
    pub fn remove_value_committed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.value_committed.borrow().as_ref(), h.as_ref());
        self.value_committed.replace(next);
    }
    fn raise(&self, slot: &RefCell<Option<MarkupDelegate>>, args: Rc<RibbonValueChangedEventArgs>) {
        let h = slot.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(Rc::new(args))])
                .expect("Spinner event handler");
        }
    }
    pub fn format_value(&self, value: f64) -> String {
        let mut text = format_arg_with(
            &FormatArg::F64(value),
            self.format().as_deref().unwrap_or(""),
            &CultureInfo::current_formatting(),
        )
        .expect("RibbonSpinner.FormatValue numeric format");
        if let Some(unit) = self.unit().filter(|u| !u.is_empty()) {
            if unit.encode_utf16().count() > 1 {
                text.push(' ');
            }
            text.push_str(&unit);
        }
        text
    }
    fn coerce(&self, value: f64) -> f64 {
        let minimum = self.minimum();
        let maximum = max(minimum, self.maximum());
        if value.is_nan() {
            minimum
        } else {
            min(maximum, max(minimum, value))
        }
    }
    fn equals(a: f64, b: f64) -> bool {
        a == b || (a.is_nan() && b.is_nan())
    }
    fn coerce_value(&self) {
        let v = self.coerce(self.value());
        if !Self::equals(v, self.value()) {
            self.set_numeric_value(v);
        }
    }
    fn on_value_changed(&self, old: f64, new: f64) {
        let v = self.coerce(new);
        if !Self::equals(v, new) {
            self.set_value(Self::value_property(), v);
            return;
        }
        self.update_text();
        self.raise(
            &self.value_changed,
            RibbonValueChangedEventArgs::new(old, new),
        );
    }
    fn update_text(&self) {
        let p = self.text_box.borrow().clone();
        if let Some(p) = p {
            let root =
                TopLevel::get_top_level(Some(self)).or_else(|| TopLevel::get_top_level(Some(&*p)));
            if root.is_none_or(|r| {
                !r.focus_manager()
                    .get_focused_element()
                    .is_some_and(|f| f.ptr_eq(&p))
            }) {
                p.set_text(Some(&self.format_value(self.value())));
            }
        }
    }
    fn step(&self, delta: f64) {
        let old = self.value();
        self.set_numeric_value(old + delta);
        let p = self.text_box.borrow().clone();
        if let Some(p) = p {
            p.set_text(Some(&self.format_value(self.value())));
        }
        self.commit(old);
    }
    fn commit(&self, old: f64) {
        self.execute_command(Some(Rc::new(self.value())))
            .expect("Spinner.ExecuteCommand");
        self.raise(
            &self.value_committed,
            RibbonValueChangedEventArgs::new(old, self.value()),
        );
    }
    pub fn commit_text(&self) {
        let Some(p) = self.text_box.borrow().clone() else {
            return;
        };
        use ferroribbon_core::text::{RibbonOrdinalIgnoreCaseKey, trim};
        let text = p
            .text()
            .unwrap_or_default()
            .encode_utf16()
            .collect::<Vec<_>>();
        let mut text = trim(&text);
        if let Some(unit) = self.unit().filter(|u| !u.is_empty()) {
            let unit = unit.encode_utf16().collect::<Vec<_>>();
            if text.len() >= unit.len()
                && RibbonOrdinalIgnoreCaseKey(text[text.len() - unit.len()..].into())
                    == RibbonOrdinalIgnoreCaseKey(unit.as_slice().into())
            {
                text = trim(&text[..text.len() - unit.len()]);
            }
        }
        let text =
            String::from_utf16(text).expect("Native spinner input cannot represent invalid UTF16");
        let mut value = 0.;
        if RibbonInputBase::try_parse_number(Some(&text), &mut value) {
            let old = self.value();
            let changed = (self.coerce(value) - old).abs() > f64::from_bits(1);
            self.set_numeric_value(value);
            if changed {
                self.commit(old);
            }
        }
        let current = self
            .text_box
            .borrow()
            .clone()
            .expect("Spinner current text part was cleared during CommitText");
        current.set_text(Some(&self.format_value(self.value())));
    }
    fn on_text_key_down(&self, e: &KeyEventArgs) {
        match e.key {
            Key::Up => self.step(self.increment()),
            Key::Down => self.step(-self.increment()),
            Key::PageUp => self.step(self.increment() * 10.),
            Key::PageDown => self.step(-self.increment() * 10.),
            Key::Enter => self.commit_text(),
            Key::Escape => {
                let p = self.text_box.borrow().clone();
                if let Some(p) = p {
                    p.set_text(Some(&self.format_value(self.value())));
                }
                return;
            }
            _ => return,
        }
        e.set_handled(true);
    }
    fn root_point(&self, e: &PointerEventArgs) -> Point {
        let root = RibbonItemHelper::find_owner(Some(self))
            .and_then(|owner| owner.owner_ribbon())
            .and_then(|ribbon| TopLevel::get_top_level(Some(&*ribbon)))
            .or_else(|| TopLevel::get_top_level(Some(self)));
        e.try_get_position_precise(root.as_deref().map(|r| &**r as &ferroui_base::Visual))
            .expect("Spinner requires continuous owning-root pointer coordinates")
    }
    fn on_scrub_pressed(&self, e: &PointerPressedEventArgs) {
        if !self.is_scrub_enabled() || self.scrub_handle.borrow().is_none() {
            return;
        }
        self.scrub_start.set(Some(self.root_point(e)));
        self.scrub_value.set(self.value());
        self.scrub_old_value.set(self.value());
        let current = self
            .scrub_handle
            .borrow()
            .clone()
            .expect("Spinner current scrub part was cleared during press");
        current.capture_pointer(e.pointer());
        e.set_handled(true);
    }
    fn on_scrub_moved(&self, e: &PointerEventArgs) {
        if let Some(start) = self.scrub_start.get() {
            let dx = self.root_point(e).x - start.x;
            self.set_numeric_value(
                self.scrub_value.get() + (dx / 4.).round_ties_even() * self.increment(),
            );
        }
    }
    fn on_scrub_released(&self, e: &PointerReleasedEventArgs) {
        if self.scrub_start.get().is_none() {
            return;
        }
        self.scrub_start.set(None);
        let h = self.scrub_handle.borrow().clone();
        if let Some(h) = h
            && let Some(captured) = e.pointer().captured()
            && captured.ptr_eq(&h)
        {
            e.pointer().capture(None);
        }
        self.commit(self.scrub_old_value.get());
    }
}
impl StyledElementImpl for RibbonSpinner {
    fn style_key_override(_c: &Self) -> &'static TypeInfo {
        Self::TYPE
    }
}
impl FerroObjectImpl for RibbonSpinner {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        c.set_input_width(64.);
    }
}
impl TemplatedControlImpl for RibbonSpinner {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        for s in std::mem::take(&mut *c.subscriptions.borrow_mut()) {
            s.dispose();
        }
        Self::parent_on_apply_template(c, e);
        let text = e.name_scope().find_as::<TextBox>("PART_TextBox");
        c.text_box.replace(text.clone());
        let up = e.name_scope().find_as::<RepeatButton>("PART_UpButton");
        let down = e.name_scope().find_as::<RepeatButton>("PART_DownButton");
        c.up.replace(up.clone());
        c.down.replace(down.clone());
        let handle = e.name_scope().find_as::<Control>("PART_Label");
        c.scrub_handle.replace(handle.clone());
        if let Some(p) = text {
            let weak = c.to_ref().downgrade();
            let key = p.add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_, e| {
                    if let Some(c) = weak.upgrade() {
                        c.on_text_key_down(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let weak = c.to_ref().downgrade();
            let lost = p.add_disposable_handler(InputElement::lost_focus_event(), move |_, _| {
                if let Some(c) = weak.upgrade() {
                    c.commit_text();
                }
            });
            let weak = c.to_ref().downgrade();
            let wheel = p.add_disposable_handler_with(
                InputElement::pointer_wheel_changed_event(),
                move |_, e| {
                    if let Some(c) = weak.upgrade() {
                        c.step(if e.delta().y > 0. {
                            c.increment()
                        } else {
                            -c.increment()
                        });
                        e.set_handled(true);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            c.subscriptions
                .borrow_mut()
                .extend([key.detach(), lost.detach(), wheel.detach()]);
            ferroui_controls::automation::AutomationProperties::set_name(
                &p,
                Some(&c.label().unwrap_or_default()),
            );
        }
        for (part, forward) in [(up, true), (down, false)] {
            if let Some(p) = part {
                let weak = c.to_ref().downgrade();
                let sub = p.add_disposable_handler(Button::click_event(), move |_, _| {
                    if let Some(c) = weak.upgrade() {
                        c.step(if forward {
                            c.increment()
                        } else {
                            -c.increment()
                        });
                    }
                });
                c.subscriptions.borrow_mut().push(sub.detach());
            }
        }
        if let Some(h) = handle {
            let weak = c.to_ref().downgrade();
            let pressed =
                h.add_disposable_handler(InputElement::pointer_pressed_event(), move |_, e| {
                    if let Some(c) = weak.upgrade() {
                        c.on_scrub_pressed(e);
                    }
                });
            let weak = c.to_ref().downgrade();
            let moved =
                h.add_disposable_handler(InputElement::pointer_moved_event(), move |_, e| {
                    if let Some(c) = weak.upgrade() {
                        c.on_scrub_moved(e);
                    }
                });
            let weak = c.to_ref().downgrade();
            let released =
                h.add_disposable_handler(InputElement::pointer_released_event(), move |_, e| {
                    if let Some(c) = weak.upgrade() {
                        c.on_scrub_released(e);
                    }
                });
            c.subscriptions.borrow_mut().extend([
                pressed.detach(),
                moved.detach(),
                released.detach(),
            ]);
        }
        c.update_text();
    }
}
impl RibbonControlBaseImpl for RibbonSpinner {
    fn create_linked_copy_core(c: &Self) -> Option<Ref<Control>> {
        let copy = Self::new();
        RibbonItemHelper::link_common(c, &copy);
        for p in [
            Self::minimum_property().as_property(),
            Self::maximum_property().as_property(),
            Self::increment_property().as_property(),
            Self::format_property().as_property(),
            Self::unit_property().as_property(),
            RibbonInputBase::input_width_property().as_property(),
        ] {
            RibbonItemHelper::link(c, &copy, p, false);
        }
        RibbonItemHelper::link(c, &copy, Self::value_property().as_property(), true);
        let source = c.to_ref();
        copy.add_value_committed(Some(MarkupDelegate::new(move |args| {
            let e = args[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonValueChangedEventArgs>>()
                .unwrap()
                .clone();
            source
                .execute_command(Some(Rc::new(source.value())))
                .expect("Linked spinner.ExecuteCommand");
            source.raise(&source.value_committed, e);
            None
        })));
        Some(copy.upcast())
    }
    fn on_key_tip_core(c: &Self) -> Rc<RibbonKeyTipResult> {
        let p = c.text_box.borrow().clone();
        if let Some(p) = p {
            p.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        }
        let p = c.text_box.borrow().clone();
        if let Some(p) = p {
            p.select_all();
        }
        RibbonKeyTipResult::close()
    }
    fn invoke_core(c: &Self) -> bool {
        c.on_key_tip_core();
        true
    }
}

pub struct RibbonValueChangedEventArgs {
    old_value: f64,
    new_value: f64,
}
impl PartialEq for RibbonValueChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl RibbonValueChangedEventArgs {
    pub fn new(old_value: f64, new_value: f64) -> Rc<Self> {
        Rc::new(Self {
            old_value,
            new_value,
        })
    }
    pub fn old_value(&self) -> f64 {
        self.old_value
    }
    pub fn new_value(&self) -> f64 {
        self.new_value
    }
}
ferroui_base::ferro_markup_type!(class RibbonValueChangedEventArgs{namespace:"FerroRibbon.Controls",this:Rc<RibbonValueChangedEventArgs>,handles:[RibbonValueChangedEventArgs,Rc<RibbonValueChangedEventArgs>,Option<Rc<RibbonValueChangedEventArgs>>],constructors:[(f64,f64)=>RibbonValueChangedEventArgs::new],properties:[OldValue:f64{get:|v:&Rc<RibbonValueChangedEventArgs>|v.old_value()},NewValue:f64{get:|v:&Rc<RibbonValueChangedEventArgs>|v.new_value()}]});

#[cfg(test)]
#[path = "../spinner_tests.rs"]
mod tests;
