//! `ConverterMigrationHelper` (`ReactiveUI.Binding/Bindings/Converters/ConverterMigrationHelper.cs`).

use crate::{ExtractedConverters, IBindingFallbackConverter, IBindingTypeConverter, ISetMethodBindingConverter};
use ferroui_splat::{IReadonlyDependencyResolver, ReadonlyDependencyResolverExt};

/// Moves the converters that an application registered in a Splat resolver, as earlier versions of
/// ReactiveUI did, into a [`ConverterService`](crate::ConverterService)
/// (`ConverterMigrationHelper`).
pub struct ConverterMigrationHelper;

impl ConverterMigrationHelper {
    /// The converters registered in `resolver` as `dyn IBindingTypeConverter`,
    /// `dyn IBindingFallbackConverter` and `dyn ISetMethodBindingConverter`; a registration of no
    /// value is skipped (`ExtractConverters`).
    pub fn extract_converters(resolver: &dyn IReadonlyDependencyResolver) -> ExtractedConverters {
        ExtractedConverters {
            typed_converters: resolver.get_services::<dyn IBindingTypeConverter>(),
            fallback_converters: resolver.get_services::<dyn IBindingFallbackConverter>(),
            set_method_converters: resolver.get_services::<dyn ISetMethodBindingConverter>(),
        }
    }
}
