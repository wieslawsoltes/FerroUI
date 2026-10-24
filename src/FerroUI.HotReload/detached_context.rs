//! Keep inherited view models available while a transaction detaches a tree.
//! Inherited DataContext disappearing can make bindings mutate shared model state
//! (for example a ListBox selection model's source) before rollback can restore it.
use ferroui_base::{
    data::BindingPriority, diagnostics::FerroObjectDiagnosticExtensions,
    metadata::from_markup_value, BoxedValue, Ref, StyledElement,
};

pub(crate) struct DetachedContext {
    root: Ref<StyledElement>,
    version: u64,
}
impl DetachedContext {
    pub(crate) fn freeze(value: &Option<BoxedValue>) -> Option<Self> {
        let root = from_markup_value::<Ref<StyledElement>>(value)?;
        let property = StyledElement::data_context_property();
        let diagnostic = root.get_diagnostic(property);
        if diagnostic.priority() != BindingPriority::Inherited
            || diagnostic.is_overridden_current_value()
            || root.has_local_value_binding(property)
            || root.data_context().is_none()
        {
            return None;
        }
        root.local_write_version(property);
        root.set_data_context(root.data_context());
        let version = root.local_write_version(property);
        Some(Self { root, version })
    }
    /// Only the tree reattached by rollback resumes inheriting its context.
    /// A retired tree keeps its context until it drops, so detachment does not
    /// publish empty binding values into a view model shared with the new tree.
    pub(crate) fn restore(self) {
        let property = StyledElement::data_context_property();
        if self.root.local_write_version(property) == self.version {
            self.root.clear_value(property);
        }
    }
}
