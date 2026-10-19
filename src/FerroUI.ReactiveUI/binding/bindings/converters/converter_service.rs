//! `ConverterService` (`ReactiveUI.Binding/Bindings/Converters/ConverterService.cs`).

use crate::{
    BindingFallbackConverterRegistry, BindingTypeConverterRegistry, ConverterType, IBindingFallbackConverter,
    IBindingTypeConverter, ISetMethodBindingConverter, SetMethodBindingConverterRegistry,
};
use std::fmt;
use std::rc::Rc;

/// What [`ConverterService::resolve_converter`] found: a typed converter of the exact pair of
/// types, or a fallback converter (upstream's `object?` result).
#[derive(Clone)]
pub enum ResolvedConverter {
    /// A typed converter ([`IBindingTypeConverter`]).
    Typed(Rc<dyn IBindingTypeConverter>),
    /// A fallback converter ([`IBindingFallbackConverter`]).
    Fallback(Rc<dyn IBindingFallbackConverter>),
}

impl ResolvedConverter {
    /// The typed converter, if this is one.
    pub fn as_typed(&self) -> Option<&Rc<dyn IBindingTypeConverter>> {
        match self {
            ResolvedConverter::Typed(converter) => Some(converter),
            ResolvedConverter::Fallback(_) => None,
        }
    }

    /// The fallback converter, if this is one.
    pub fn as_fallback(&self) -> Option<&Rc<dyn IBindingFallbackConverter>> {
        match self {
            ResolvedConverter::Typed(_) => None,
            ResolvedConverter::Fallback(converter) => Some(converter),
        }
    }
}

impl fmt::Debug for ResolvedConverter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolvedConverter::Typed(converter) => {
                write!(f, "Typed({} -> {})", converter.from_type(), converter.to_type())
            }
            ResolvedConverter::Fallback(_) => f.write_str("Fallback"),
        }
    }
}

/// The converters of the bindings: the typed, fallback and set-method registries
/// (`ConverterService`). [`BindingConverters::current`](crate::BindingConverters::current) is the
/// one the bindings use; [`register_default_converters`](crate::register_default_converters) fills a
/// service with the built-in converters.
#[derive(Default)]
pub struct ConverterService {
    typed_converters: BindingTypeConverterRegistry,
    fallback_converters: BindingFallbackConverterRegistry,
    set_method_converters: SetMethodBindingConverterRegistry,
}

impl ConverterService {
    /// A service with empty registries.
    pub fn new() -> Self {
        Self::default()
    }

    /// The typed converters (`TypedConverters`).
    pub fn typed_converters(&self) -> &BindingTypeConverterRegistry {
        &self.typed_converters
    }

    /// The fallback converters (`FallbackConverters`).
    pub fn fallback_converters(&self) -> &BindingFallbackConverterRegistry {
        &self.fallback_converters
    }

    /// The set-method converters (`SetMethodConverters`).
    pub fn set_method_converters(&self) -> &SetMethodBindingConverterRegistry {
        &self.set_method_converters
    }

    /// The converter from `from_type` to `to_type`: the typed converter of the exact pair with the
    /// highest affinity, or else the fallback converter with the highest affinity, or `None`
    /// (`ResolveConverter`). A typed converter wins over a fallback converter whatever their
    /// affinities.
    pub fn resolve_converter(&self, from_type: ConverterType, to_type: ConverterType) -> Option<ResolvedConverter> {
        if let Some(typed) = self.typed_converters.try_get_converter(from_type, to_type) {
            return Some(ResolvedConverter::Typed(typed));
        }
        self.fallback_converters.try_get_converter(from_type, to_type).map(ResolvedConverter::Fallback)
    }

    /// The set-method converter for `from_type` and `to_type` (`ResolveSetMethodConverter`).
    pub fn resolve_set_method_converter(
        &self,
        from_type: Option<ConverterType>,
        to_type: Option<ConverterType>,
    ) -> Option<Rc<dyn ISetMethodBindingConverter>> {
        self.set_method_converters.try_get_converter(from_type, to_type)
    }
}

impl fmt::Debug for ConverterService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ConverterService: Typed: {:?}; fallback: {:?}; set-method: {:?}",
            self.typed_converters, self.fallback_converters, self.set_method_converters
        )
    }
}
