//! `ExtractedConverters` (`ReactiveUI.Binding/Bindings/Converters/ExtractedConverters.cs`).

use crate::{IBindingFallbackConverter, IBindingTypeConverter, ISetMethodBindingConverter};
use std::fmt;
use std::rc::Rc;

/// The converters read from a Splat resolver by
/// [`ConverterMigrationHelper::extract_converters`](crate::ConverterMigrationHelper::extract_converters)
/// (`ExtractedConverters`).
#[derive(Clone, Default)]
pub struct ExtractedConverters {
    /// The typed converters (`TypedConverters`).
    pub typed_converters: Vec<Rc<dyn IBindingTypeConverter>>,
    /// The fallback converters (`FallbackConverters`).
    pub fallback_converters: Vec<Rc<dyn IBindingFallbackConverter>>,
    /// The set-method converters (`SetMethodConverters`).
    pub set_method_converters: Vec<Rc<dyn ISetMethodBindingConverter>>,
}

impl fmt::Debug for ExtractedConverters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExtractedConverters: {} typed, {} fallback, {} set-method",
            self.typed_converters.len(),
            self.fallback_converters.len(),
            self.set_method_converters.len()
        )
    }
}
