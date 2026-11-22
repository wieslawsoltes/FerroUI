use ferroui_base::data::core::{ValueTypeRegistration, ValueTypes};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_edit_core::highlighting::{
    HighlightingDefinition, HighlightingDefinitionResolver, HighlightingError, HighlightingManager,
};
use std::{fmt, rc::Rc};

ferroui_base::ferro_thread_local! {
    static BUILTIN_HIGHLIGHTING: HighlightingManager = HighlightingManager::new();
}

/// A compiled XSHD definition that can be assigned to an editor property.
#[derive(Clone, Debug)]
pub struct SyntaxHighlighting(pub Rc<HighlightingDefinition>);

impl SyntaxHighlighting {
    /// Looks up a name in the application resolver, then in the bundled catalog.
    pub fn from_name(name: &str) -> Result<Option<Self>, HighlightingError> {
        if let Some(resolver) =
            FerroLocator::current().get_service::<dyn HighlightingDefinitionResolver>()
        {
            if let Some(definition) = resolver.resolve(name)? {
                return Ok(Some(Self(definition)));
            }
        }
        BUILTIN_HIGHLIGHTING
            .with(|manager| manager.get_definition(name).map(|value| value.map(Self)))
    }
}

impl PartialEq for SyntaxHighlighting {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SyntaxHighlighting {}

impl fmt::Display for SyntaxHighlighting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.name)
    }
}

impl ValueTypeRegistration for SyntaxHighlighting {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<SyntaxHighlighting>();
        ValueTypes::register_display::<SyntaxHighlighting>();
    }
}

// Markup parsing returns the nullable property value, including for unknown names.
ferroui_base::ferro_markup_type!(class SyntaxHighlighting {
    namespace: "FerroUI.Edit",
    handles: [Option<SyntaxHighlighting>, SyntaxHighlighting],
    parse: SyntaxHighlighting::from_name,
});
