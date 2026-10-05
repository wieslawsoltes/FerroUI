use ferroui_base::controls::INameScope;
use ferroui_base::{FerroObject, ObjectType, Ref, StyledElement, Upcast};

/// Registers a control built by a function template in the template's name
/// scope.
pub trait FuncTemplateNameScopeExtensions: Sized {
    /// Registers the control in `scope` under its name and returns it.
    ///
    /// Panics if the control has no name.
    fn register_in_name_scope(self, scope: &dyn INameScope) -> Self;
}

impl<T: ObjectType + Upcast<StyledElement>> FuncTemplateNameScopeExtensions for Ref<T> {
    fn register_in_name_scope(self, scope: &dyn INameScope) -> Self {
        let element: &StyledElement = (*self).upcast();
        let Some(name) = element.name() else {
            panic!("RegisterInNameScope must be called on a control with non-null name.");
        };

        scope.register(&name, self.clone().upcast::<FerroObject>());
        self
    }
}
