use super::WindowDrawnDecorationsContent;
use crate::templates::TemplateResult;
use ferroui_base::styling::ITemplate;
use ferroui_base::Ref;

/// Interface for a template that produces [`WindowDrawnDecorationsContent`].
///
/// Extends [`ITemplate`] so that a setter assigns the template object
/// itself to a property of this type instead of building it.
pub trait IWindowDrawnDecorationsTemplate: ITemplate {
    /// Builds the template and returns the content with its name scope.
    fn build_typed(&self) -> TemplateResult<Ref<WindowDrawnDecorationsContent>>;
}

impl PartialEq for dyn IWindowDrawnDecorationsTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
