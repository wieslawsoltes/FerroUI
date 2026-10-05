use super::{FuncTemplateWithParam, IControlTemplate, ITemplateWithParam, TemplateResult};
use crate::primitives::TemplatedControl;
use crate::Control;
use ferroui_base::controls::NameScopeRef;
use ferroui_base::{ObjectType, Ref, Upcast};
use std::rc::Rc;

/// A template for a [`TemplatedControl`].
pub struct FuncControlTemplate {
    base: FuncTemplateWithParam<Ref<TemplatedControl>, Ref<Control>>,
}

impl FuncControlTemplate {
    /// Creates a control template from the function used to create the
    /// control tree for a templated control.
    pub fn new(build: impl Fn(&Ref<TemplatedControl>, &NameScopeRef) -> Ref<Control> + 'static) -> Rc<Self> {
        Rc::new(Self { base: FuncTemplateWithParam::new(build) })
    }

    /// Creates a control template for templated controls of class `T`.
    ///
    /// Panics when applied to a control of another class.
    pub fn for_type<T: ObjectType + Upcast<TemplatedControl>>(
        build: impl Fn(&Ref<T>, &NameScopeRef) -> Ref<Control> + 'static,
    ) -> Rc<Self> {
        Self::new(move |x, s| match x.cast::<T>() {
            Some(x) => build(&x, s),
            None => panic!(
                "Unable to cast object of type '{}' to type '{}'.",
                x.get_type().name(),
                T::TYPE.name()
            ),
        })
    }
}

impl ITemplateWithParam<Ref<TemplatedControl>, Option<TemplateResult<Ref<Control>>>> for FuncControlTemplate {
    fn build(&self, param: &Ref<TemplatedControl>) -> Option<TemplateResult<Ref<Control>>> {
        let (control, scope) = self.base.build_with_name_scope(param);
        Some(TemplateResult::new(control, scope))
    }
}

impl IControlTemplate for FuncControlTemplate {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}
