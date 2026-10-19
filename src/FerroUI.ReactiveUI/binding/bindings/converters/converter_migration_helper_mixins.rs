//! `ConverterMigrationHelperMixins`
//! (`ReactiveUI.Binding/Bindings/Converters/ConverterMigrationHelperMixins.cs`).

use crate::{ConverterMigrationHelper, ConverterService};
use ferroui_splat::IReadonlyDependencyResolver;

/// Imports the converters of a Splat resolver into a [`ConverterService`]
/// (`ConverterMigrationHelperMixins`).
pub trait ConverterMigrationHelperMixins {
    /// Registers every converter that `resolver` holds in the matching registry of this service
    /// (`ImportFrom`).
    fn import_from(&self, resolver: &dyn IReadonlyDependencyResolver);
}

impl ConverterMigrationHelperMixins for ConverterService {
    fn import_from(&self, resolver: &dyn IReadonlyDependencyResolver) {
        let extracted = ConverterMigrationHelper::extract_converters(resolver);
        for converter in extracted.typed_converters {
            self.typed_converters().register(converter);
        }
        for converter in extracted.fallback_converters {
            self.fallback_converters().register(converter);
        }
        for converter in extracted.set_method_converters {
            self.set_method_converters().register(converter);
        }
    }
}
