//! Native row binding/event adapter. Layout remains in XAML and mutations remain in the row model.
//! The source installs direct callbacks, which survive removal while a caller retains the control.
//! Ordinary template bindings alone lose their data context on removal at the selected framework pin.
use crate::language_bindings::LanguageRowBindings;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
    input::InputElementImpl,
    interactivity::{InteractiveImpl, RoutedEventArgs},
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ButtonImpl, CheckBox, ContentControlImpl, ControlImpl,
    primitives::{TemplatedControlImpl, ToggleButtonImpl, ToggleButtonImplExt},
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{cell::RefCell, rc::Rc};
struct CapturedRow {
    row: Rc<LanguageRowBindings>,
    token: u64,
}
impl Drop for CapturedRow {
    fn drop(&mut self) {
        self.row.property_changed().remove(self.token);
    }
}
#[repr(C)]
pub struct LanguageCheckBox {
    base: CheckBox,
    captured: RefCell<Option<CapturedRow>>,
}
ferroui_base::ferro_class!(LanguageCheckBox:CheckBox);
ferroui_base::ferro_class_info!(LanguageCheckBox {
    new: LanguageCheckBox::new
});
ferroui_base::ferro_impl_classes!(LanguageCheckBox:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl StyledElementImpl for LanguageCheckBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        CheckBox::TYPE
    }
}
impl ToggleButtonImpl for LanguageCheckBox {
    fn on_is_checked_changed(this: &Self, e: &RoutedEventArgs) {
        let row = this.captured.borrow().as_ref().map(|c| c.row.clone());
        if let Some(row) = row {
            row.set_is_checked(this.is_checked());
        }
        Self::parent_on_is_checked_changed(this, e);
    }
}
ferroui_base::ferro_properties! {impl LanguageCheckBox {
    pub fn row_property() -> StyledProperty<Option<Rc<LanguageRowBindings>>> {
        FerroProperty::register::<Self,_>("Row",None)
    }
}}
impl LanguageCheckBox {
    fn static_constructor() {
        Self::row_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.capture_row())
            .detach();
    }
    pub fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self {
            base: CheckBox::construct(),
            captured: RefCell::new(None),
        })
    }
    pub fn row(&self) -> Option<Rc<LanguageRowBindings>> {
        self.get_value(Self::row_property())
    }
    pub fn set_row(&self, row: Option<Rc<LanguageRowBindings>>) {
        self.set_value(Self::row_property(), row);
    }
    fn capture_row(&self) {
        let Some(row) = self.row() else { return }; // Template teardown must not remove source callbacks.
        if let Some(existing) = self.captured.borrow().as_ref() {
            assert!(
                Rc::ptr_eq(&existing.row, &row),
                "source language controls are fresh per Populate; native template recycling changed the captured row"
            );
            return;
        }
        self.set_content(ferroui_base::metadata::into_markup_value(row.name()));
        self.set_is_checked(row.is_checked()); // Source initializes before installing its callbacks.
        let weak = self.to_ref().downgrade();
        let observed = Rc::downgrade(&row);
        let token = row.property_changed().add(Rc::new(move |name| {
            if name == "IsChecked"
                && let (Some(control), Some(row)) = (weak.upgrade(), observed.upgrade())
            {
                control.set_is_checked(row.is_checked());
            }
        }));
        *self.captured.borrow_mut() = Some(CapturedRow { row, token });
    }
}
