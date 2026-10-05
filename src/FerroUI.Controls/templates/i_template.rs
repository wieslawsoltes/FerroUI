use ferroui_base::styling::ITemplate;
use std::rc::Rc;

/// Creates a control (the single type argument form of the template
/// contract).
///
/// `TControl` is the exact value produced, usually `Ref<Control>` or
/// `Option<Ref<Control>>`.
pub trait ITemplateOf<TControl>: ITemplate {
    /// Creates the control.
    fn build_typed(&self) -> TControl;
}

impl<TControl: 'static> PartialEq for dyn ITemplateOf<TControl> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Creates a control based on a parameter (the two type argument form of
/// the template contract).
pub trait ITemplateWithParam<TParam, TControl> {
    /// Creates the control.
    fn build(&self, param: &TParam) -> TControl;
}

/// A shared handle to a template producing `TControl`.
pub type TemplateRef<TControl> = Rc<dyn ITemplateOf<TControl>>;
