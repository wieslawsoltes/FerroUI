//! Original text input; actual native TextBox template part, command and linked-copy state.
use super::{
    RibbonControlBaseImpl, RibbonInputBase, RibbonInputBaseImpl, RibbonItemHelper,
    RibbonKeyTipResult,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties,
    input::{InputElement, InputElementImpl, Key, KeyEventArgs, NavigationMethod},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::MarkupDelegate,
    reactive::IDisposable,
};
use ferroui_controls::{
    Control, ControlImpl, TextBox,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::{cell::RefCell, rc::Rc};
#[repr(C)]
pub struct RibbonTextBox {
    base: RibbonInputBase,
    text_box: RefCell<Option<Ref<TextBox>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    submitted: RefCell<Option<MarkupDelegate>>,
}
ferro_class!(RibbonTextBox:RibbonInputBase);
ferro_class_info!(RibbonTextBox{new:RibbonTextBox::new,markup:{namespace:"FerroRibbon.Controls",events:[Submitted(Option<BoxedValue>,Option<String>){add:RibbonTextBox::add_submitted,remove:RibbonTextBox::remove_submitted}]}});
ferro_impl_classes!(RibbonTextBox:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,RibbonInputBaseImpl);
ferro_properties! {impl RibbonTextBox{pub fn text_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Text",Some(String::new()))}}}
impl RibbonTextBox {
    fn static_constructor() {
        Self::text_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_text_changed())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonInputBase::construct(),
            text_box: RefCell::new(None),
            subscriptions: RefCell::new(vec![]),
            submitted: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, v: Option<String>) {
        self.set_value(Self::text_property(), v)
    }
    pub fn add_submitted(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.submitted.borrow().as_ref(), h.as_ref());
        self.submitted.replace(next);
    }
    pub fn remove_submitted(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.submitted.borrow().as_ref(), h.as_ref());
        self.submitted.replace(next);
    }
    fn raise_submitted(&self, text: Option<String>) {
        let callback = self.submitted.borrow().clone();
        if let Some(f) = callback {
            f.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                text.map(|v| Rc::new(v) as BoxedValue),
            ])
            .expect("Submitted handler");
        }
    }
    fn on_text_changed(&self) {
        let text_box = self.text_box.borrow().clone();
        if let Some(t) = text_box
            && t.text() != self.text()
        {
            t.set_text(self.text().as_deref());
        }
    }
    fn on_inner_text_changed(&self) {
        let text = self
            .text_box
            .borrow()
            .as_ref()
            .and_then(|t| t.text())
            .unwrap_or_default();
        self.set_text(Some(text));
    }
    fn on_inner_key_down(&self, e: &KeyEventArgs) {
        if e.key == Key::Enter {
            self.raise_submitted(self.text());
            self.execute_command(self.text().map(|v| Rc::new(v) as BoxedValue))
                .expect("RibbonTextBox.ExecuteCommand");
            e.set_handled(true);
        }
    }
}
impl FerroObjectImpl for RibbonTextBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_input_width(140.);
    }
}
impl TemplatedControlImpl for RibbonTextBox {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let old = std::mem::take(&mut *this.subscriptions.borrow_mut());
        for s in old {
            s.dispose();
        }
        Self::parent_on_apply_template(this, e);
        let part = e.name_scope().find_as::<TextBox>("PART_TextBox");
        this.text_box.replace(part.clone());
        if let Some(part) = part {
            part.set_text(this.text().as_deref());
            let weak = this.to_ref().downgrade();
            let key = part.add_disposable_handler(InputElement::key_down_event(), move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_inner_key_down(e);
                }
            });
            let weak = this.to_ref().downgrade();
            let text = part.add_disposable_handler(TextBox::text_changed_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_inner_text_changed();
                }
            });
            this.subscriptions
                .borrow_mut()
                .extend([key.detach(), text.detach()]);
            ferroui_controls::automation::AutomationProperties::set_name(
                &part,
                Some(
                    &this
                        .label()
                        .or_else(|| this.placeholder_text())
                        .unwrap_or_default(),
                ),
            );
        }
    }
}
impl RibbonControlBaseImpl for RibbonTextBox {
    fn create_linked_copy_core(this: &Self) -> Option<Ref<Control>> {
        let copy = Self::new();
        RibbonItemHelper::link_common(this, &copy);
        RibbonItemHelper::link(
            this,
            &copy,
            RibbonInputBase::input_width_property().as_property(),
            false,
        );
        RibbonItemHelper::link(
            this,
            &copy,
            RibbonInputBase::placeholder_text_property().as_property(),
            false,
        );
        RibbonItemHelper::link(this, &copy, Self::text_property().as_property(), true);
        // Copy -> source is acyclic: source holds no copy. Source callback survives unlink,
        // exactly as upstream; automatic callback cleanup would change command behavior.
        let source = this.to_ref();
        copy.add_submitted(Some(MarkupDelegate::new(move |args| {
            let text = args.get(1).and_then(Option::as_ref).map(|v| {
                v.downcast_ref::<String>()
                    .expect("Submitted String")
                    .clone()
            });
            source.raise_submitted(text.clone());
            source
                .execute_command(text.map(|v| Rc::new(v) as BoxedValue))
                .expect("RibbonTextBox linked ExecuteCommand");
            None
        })));
        Some(copy.upcast())
    }
    fn on_key_tip_core(this: &Self) -> Rc<RibbonKeyTipResult> {
        let text = this.text_box.borrow().clone();
        if let Some(t) = text {
            t.focus_with(
                NavigationMethod::Tab,
                ferroui_base::input::KeyModifiers::NONE,
            );
        }
        RibbonKeyTipResult::close()
    }
    fn invoke_core(this: &Self) -> bool {
        let text = this.text_box.borrow().clone();
        if let Some(t) = text {
            t.focus_with(
                NavigationMethod::Tab,
                ferroui_base::input::KeyModifiers::NONE,
            );
        }
        true
    }
}
#[cfg(test)]
#[path = "../text_input_tests.rs"]
mod tests;
