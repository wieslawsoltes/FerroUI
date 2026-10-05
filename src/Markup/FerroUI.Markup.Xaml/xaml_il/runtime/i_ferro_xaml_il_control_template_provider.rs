//! Port of `XamlIl/Runtime/IFerroXamlIlControlTemplateProvider.cs`.

use ferroui_base::ferro_markup_type;
use std::rc::Rc;

/// A marker service: present in the service provider while the content of
/// a control template is being built.
pub trait IFerroXamlIlControlTemplateProvider {}

impl PartialEq for dyn IFerroXamlIlControlTemplateProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

ferro_markup_type!(interface dyn IFerroXamlIlControlTemplateProvider as "IFerroXamlIlControlTemplateProvider" {
    this: Rc<dyn IFerroXamlIlControlTemplateProvider>,
    handles: [Rc<dyn IFerroXamlIlControlTemplateProvider>, Option<Rc<dyn IFerroXamlIlControlTemplateProvider>>],
});
