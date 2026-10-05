use ferroui_base::controls::NameScopeRef;

/// The result of building a template: the built object and the name scope
/// its named parts were registered in.
#[derive(Clone)]
pub struct TemplateResult<T> {
    result: T,
    name_scope: NameScopeRef,
}

impl<T> TemplateResult<T> {
    pub fn new(result: T, name_scope: NameScopeRef) -> Self {
        Self { result, name_scope }
    }

    /// The built object.
    pub fn result(&self) -> &T {
        &self.result
    }

    /// The name scope of the built object.
    pub fn name_scope(&self) -> &NameScopeRef {
        &self.name_scope
    }

    /// Splits the result into the built object and its name scope.
    pub fn deconstruct(self) -> (T, NameScopeRef) {
        (self.result, self.name_scope)
    }
}
