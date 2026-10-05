use ferroui_base::TypeInfo;

/// Declares that a templated control expects a named part of a given type
/// in its control template.
///
/// Classes expose their parts through a `template_parts()` function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemplatePartAttribute {
    /// The pre-defined name of the part.
    pub name: &'static str,
    /// The type of the named part: the part is of this class or a class
    /// derived from it.
    pub type_: &'static TypeInfo,
    /// Whether the part is required to be present in the template. A
    /// required part not found in the template is an error.
    pub is_required: bool,
}

impl TemplatePartAttribute {
    /// Declares an optional part.
    pub const fn new(name: &'static str, type_: &'static TypeInfo) -> Self {
        Self { name, type_, is_required: false }
    }

    /// Declares a required part.
    pub const fn required(name: &'static str, type_: &'static TypeInfo) -> Self {
        Self { name, type_, is_required: true }
    }
}
