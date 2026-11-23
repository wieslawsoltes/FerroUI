use super::{CompiledTemplateParts, TemplatedControl};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{ObjectType, Ref};
use std::rc::Rc;

/// Holds the details of the `TemplateApplied` event.
#[derive(Clone)]
pub struct TemplateAppliedEventArgs {
    base: RoutedEventArgs,
    name_scope: NameScopeRef,
    compiled_parts: Option<Rc<CompiledTemplateParts>>,
}

ferro_routed_event_args!(TemplateAppliedEventArgs: RoutedEventArgs);

impl TemplateAppliedEventArgs {
    /// Creates the args for the name scope of the applied template.
    pub fn new(name_scope: NameScopeRef) -> Self {
        Self::with_compiled_parts(name_scope, None)
    }

    /// The args of a template whose build function handed over its parts.
    pub(super) fn with_compiled_parts(
        name_scope: NameScopeRef,
        compiled_parts: Option<Rc<CompiledTemplateParts>>,
    ) -> Self {
        Self {
            base: RoutedEventArgs::with_event(TemplatedControl::template_applied_event()),
            name_scope,
            compiled_parts,
        }
    }

    /// The part `name`, declared at `index` of the control's template parts: from the parts the
    /// compiler resolved, or by its name for any other template. The same result either way.
    pub fn part<T: ObjectType>(&self, index: usize, name: &str) -> Option<Ref<T>> {
        if let Some(part) = self.compiled_parts.as_ref().and_then(|parts| parts.slot(index, name)) {
            ferroui_base::perf_count!(TemplatePartsCompiled);
            return part.cast::<T>();
        }
        ferroui_base::perf_count!(TemplatePartsByName);
        self.name_scope.find_as::<T>(name)
    }

    /// [`part`](Self::part) of a required part: panics as `get_as` does when it is missing.
    pub fn required_part<T: ObjectType>(&self, index: usize, name: &str) -> Ref<T> {
        self.part::<T>(index, name).unwrap_or_else(|| self.name_scope.get_as::<T>(name))
    }

    /// Whether the compiler resolved the parts of this template.
    #[doc(hidden)]
    pub fn has_compiled_parts(&self) -> bool {
        self.compiled_parts.is_some()
    }

    /// The name scope of the applied template.
    pub fn name_scope(&self) -> &NameScopeRef {
        &self.name_scope
    }
}
