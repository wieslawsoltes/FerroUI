use super::TemplatedControl;
use ferroui_base::controls::NameScopeRef;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::RoutedEventArgs;

/// Holds the details of the `TemplateApplied` event.
#[derive(Clone)]
pub struct TemplateAppliedEventArgs {
    base: RoutedEventArgs,
    name_scope: NameScopeRef,
}

ferro_routed_event_args!(TemplateAppliedEventArgs: RoutedEventArgs);

impl TemplateAppliedEventArgs {
    /// Creates the args for the name scope of the applied template.
    pub fn new(name_scope: NameScopeRef) -> Self {
        Self { base: RoutedEventArgs::with_event(TemplatedControl::template_applied_event()), name_scope }
    }

    /// The name scope of the applied template.
    pub fn name_scope(&self) -> &NameScopeRef {
        &self.name_scope
    }
}
