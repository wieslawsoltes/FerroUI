use super::{ITemplateOf, ITemplateWithParam};
use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::styling::ITemplate;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Creates a control from a function.
///
/// `TControl` is the exact value produced. When the template is used as a
/// setter value it must be the value type of the setter's property (for
/// example `Option<Ref<Control>>`).
pub struct FuncTemplate<TControl> {
    func: Box<dyn Fn() -> TControl>,
}

impl<TControl: PartialEq + 'static> FuncTemplate<TControl> {
    /// Creates a template from the function used to create the control.
    pub fn new(func: impl Fn() -> TControl + 'static) -> Rc<Self> {
        Rc::new(Self { func: Box::new(func) })
    }
}

impl<TControl: PartialEq + 'static> ITemplateOf<TControl> for FuncTemplate<TControl> {
    fn build_typed(&self) -> TControl {
        (self.func)()
    }
}

impl<TControl: PartialEq + 'static> ITemplate for FuncTemplate<TControl> {
    fn build(&self) -> BoxedValue {
        Rc::new(self.build_typed())
    }
}

/// Creates a control from a function taking a parameter and the name scope
/// to register the named parts in.
pub struct FuncTemplateWithParam<TParam, TControl> {
    func: Box<dyn Fn(&TParam, &NameScopeRef) -> TControl>,
}

impl<TParam, TControl> FuncTemplateWithParam<TParam, TControl> {
    /// Creates a template from the function used to create the control.
    pub fn new(func: impl Fn(&TParam, &NameScopeRef) -> TControl + 'static) -> Self {
        Self { func: Box::new(func) }
    }

    /// Creates the control together with the name scope passed to the
    /// function.
    pub fn build_with_name_scope(&self, param: &TParam) -> (TControl, NameScopeRef) {
        let scope = NameScopeRef::new(NameScope::new());
        let rv = (self.func)(param, &scope);
        (rv, scope)
    }
}

impl<TParam, TControl> ITemplateWithParam<TParam, TControl> for FuncTemplateWithParam<TParam, TControl> {
    fn build(&self, param: &TParam) -> TControl {
        self.build_with_name_scope(param).0
    }
}
