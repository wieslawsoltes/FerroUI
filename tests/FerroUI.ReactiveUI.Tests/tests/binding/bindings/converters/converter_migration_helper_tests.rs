//! `ConverterMigrationHelperTests` (`ReactiveUI.Binding.Tests/Bindings/Converters/ConverterMigrationHelperTests.cs`).
//!
//! Upstream's `TestDependencyResolver` is a list that answers `GetServices<T>` with the services of
//! type `T`; the port registers the converters in Splat's `ModernDependencyResolver` under the
//! traits they are read as, and a `null` registration as a constant of no value.

use crate::tests::binding::bindings::type_converters::same_object;
use ferroui_reactiveui::*;
use ferroui_splat::{IMutableDependencyResolver, ModernDependencyResolver, MutableDependencyResolverExt, ServiceType};
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

const BELOW_DEFAULT_AFFINITY: i32 = 3;
const DEFAULT_AFFINITY: i32 = 5;
const EXPECTED_TWO_CONVERTERS: i32 = 2;

fn int_type() -> ConverterType {
    ConverterType::of::<i32>()
}

fn string_type() -> ConverterType {
    ConverterType::of::<String>()
}

/// Whether `list` holds `expected`.
fn contains<T: ?Sized, U: ?Sized>(list: &[Rc<T>], expected: &Rc<U>) -> bool {
    list.iter().any(|item| same_object(item, expected))
}

/// Registers a typed converter of no value (upstream's `RegisterService<IBindingTypeConverter>(null!)`).
fn register_null_typed_converter(resolver: &ModernDependencyResolver) {
    resolver.register_constant_erased(None, None, ServiceType::of::<dyn IBindingTypeConverter>(), None);
}

// Not ported: ExtractConverters_ShouldThrowArgumentNullException_WhenResolverIsNull - argument validation

#[test]
fn extract_converters_should_return_empty_lists_when_no_converters_registered() {
    let resolver = ModernDependencyResolver::new();

    let ExtractedConverters { typed_converters, fallback_converters, set_method_converters } =
        ConverterMigrationHelper::extract_converters(&resolver);

    assert!(typed_converters.is_empty());
    assert!(fallback_converters.is_empty());
    assert!(set_method_converters.is_empty());
}

#[test]
fn extract_converters_should_extract_typed_converters() {
    let converter1 = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let converter2 = TestTypedConverter::<f64, bool>::new(BELOW_DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingTypeConverter>(converter1.clone());
    resolver.register_constant::<dyn IBindingTypeConverter>(converter2.clone());

    let extracted = ConverterMigrationHelper::extract_converters(&resolver);

    assert_eq!(extracted.typed_converters.len(), EXPECTED_TWO_CONVERTERS as usize);
    assert!(contains(&extracted.typed_converters, &converter1));
    assert!(contains(&extracted.typed_converters, &converter2));
    assert!(extracted.fallback_converters.is_empty());
    assert!(extracted.set_method_converters.is_empty());
}

#[test]
fn extract_converters_should_extract_fallback_converters() {
    let converter1 = TestFallbackConverter::new(DEFAULT_AFFINITY);
    let converter2 = TestFallbackConverter::new(BELOW_DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingFallbackConverter>(converter1.clone());
    resolver.register_constant::<dyn IBindingFallbackConverter>(converter2.clone());

    let extracted = ConverterMigrationHelper::extract_converters(&resolver);

    assert!(extracted.typed_converters.is_empty());
    assert_eq!(extracted.fallback_converters.len(), EXPECTED_TWO_CONVERTERS as usize);
    assert!(contains(&extracted.fallback_converters, &converter1));
    assert!(contains(&extracted.fallback_converters, &converter2));
    assert!(extracted.set_method_converters.is_empty());
}

#[test]
fn extract_converters_should_extract_set_method_converters() {
    let converter1 = TestSetMethodConverter::new(DEFAULT_AFFINITY);
    let converter2 = TestSetMethodConverter::new(BELOW_DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn ISetMethodBindingConverter>(converter1.clone());
    resolver.register_constant::<dyn ISetMethodBindingConverter>(converter2.clone());

    let extracted = ConverterMigrationHelper::extract_converters(&resolver);

    assert!(extracted.typed_converters.is_empty());
    assert!(extracted.fallback_converters.is_empty());
    assert_eq!(extracted.set_method_converters.len(), EXPECTED_TWO_CONVERTERS as usize);
    assert!(contains(&extracted.set_method_converters, &converter1));
    assert!(contains(&extracted.set_method_converters, &converter2));
}

#[test]
fn extract_converters_should_extract_all_converter_types() {
    let typed_converter = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let fallback_converter = TestFallbackConverter::new(BELOW_DEFAULT_AFFINITY);
    let set_method_converter = TestSetMethodConverter::new(EXPECTED_TWO_CONVERTERS);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingTypeConverter>(typed_converter.clone());
    resolver.register_constant::<dyn IBindingFallbackConverter>(fallback_converter.clone());
    resolver.register_constant::<dyn ISetMethodBindingConverter>(set_method_converter.clone());

    let extracted = ConverterMigrationHelper::extract_converters(&resolver);

    assert_eq!(extracted.typed_converters.len(), 1);
    assert!(contains(&extracted.typed_converters, &typed_converter));
    assert_eq!(extracted.fallback_converters.len(), 1);
    assert!(contains(&extracted.fallback_converters, &fallback_converter));
    assert_eq!(extracted.set_method_converters.len(), 1);
    assert!(contains(&extracted.set_method_converters, &set_method_converter));
}

#[test]
fn extract_converters_should_filter_out_null_converters() {
    let converter = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingTypeConverter>(converter.clone());
    register_null_typed_converter(&resolver);

    let extracted = ConverterMigrationHelper::extract_converters(&resolver);

    assert_eq!(extracted.typed_converters.len(), 1);
    assert!(contains(&extracted.typed_converters, &converter));
}

// Not ported: ImportFrom_ShouldThrowArgumentNullException_WhenConverterServiceIsNull - argument validation
// Not ported: ImportFrom_ShouldThrowArgumentNullException_WhenResolverIsNull - argument validation

#[test]
fn import_from_should_import_typed_converters() {
    let converter1 = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let converter2 = TestTypedConverter::<f64, bool>::new(BELOW_DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingTypeConverter>(converter1.clone());
    resolver.register_constant::<dyn IBindingTypeConverter>(converter2.clone());
    let service = ConverterService::new();

    service.import_from(&resolver);

    let result1 = service.resolve_converter(int_type(), string_type());
    let result2 = service.resolve_converter(ConverterType::of::<f64>(), ConverterType::of::<bool>());
    assert!(result1.as_ref().and_then(ResolvedConverter::as_typed).is_some_and(|typed| same_object(typed, &converter1)));
    assert!(result2.as_ref().and_then(ResolvedConverter::as_typed).is_some_and(|typed| same_object(typed, &converter2)));
}

#[test]
fn import_from_should_import_fallback_converters() {
    let converter = TestFallbackConverter::new(DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingFallbackConverter>(converter.clone());
    let service = ConverterService::new();

    service.import_from(&resolver);

    // The fallback converter answers when no typed converter matches.
    let result = service.resolve_converter(int_type(), string_type());
    assert!(result.as_ref().and_then(ResolvedConverter::as_fallback).is_some_and(|fallback| same_object(fallback, &converter)));
}

#[test]
fn import_from_should_import_set_method_converters() {
    let converter = TestSetMethodConverter::new(DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn ISetMethodBindingConverter>(converter.clone());
    let service = ConverterService::new();

    service.import_from(&resolver);

    let result = service.resolve_set_method_converter(Some(int_type()), Some(string_type()));
    assert!(result.is_some_and(|result| same_object(&result, &converter)));
}

#[test]
fn import_from_should_import_all_converter_types() {
    let typed_converter = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let fallback_converter = TestFallbackConverter::new(BELOW_DEFAULT_AFFINITY);
    let set_method_converter = TestSetMethodConverter::new(EXPECTED_TWO_CONVERTERS);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingTypeConverter>(typed_converter.clone());
    resolver.register_constant::<dyn IBindingFallbackConverter>(fallback_converter);
    resolver.register_constant::<dyn ISetMethodBindingConverter>(set_method_converter.clone());
    let service = ConverterService::new();

    service.import_from(&resolver);

    let typed_result = service.resolve_converter(int_type(), string_type());
    let set_method_result = service.resolve_set_method_converter(Some(int_type()), Some(string_type()));
    assert!(typed_result.as_ref().and_then(ResolvedConverter::as_typed).is_some_and(|typed| same_object(typed, &typed_converter)));
    assert!(set_method_result.is_some_and(|result| same_object(&result, &set_method_converter)));
}

#[test]
fn import_from_should_not_import_null_converters() {
    let converter = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant::<dyn IBindingTypeConverter>(converter.clone());
    register_null_typed_converter(&resolver);
    let service = ConverterService::new();

    service.import_from(&resolver);

    let result = service.resolve_converter(int_type(), string_type());
    assert!(result.as_ref().and_then(ResolvedConverter::as_typed).is_some_and(|typed| same_object(typed, &converter)));
    assert_eq!(service.typed_converters().get_all_converters().len(), 1);
}

/// A typed converter of the given affinity that never converts (`TestTypedConverter<TFrom, TTo>`).
struct TestTypedConverter<F, T> {
    affinity: i32,
    types: PhantomData<fn(F) -> T>,
}

impl<F: 'static, T: 'static> TestTypedConverter<F, T> {
    fn new(affinity: i32) -> Rc<Self> {
        Rc::new(Self { affinity, types: PhantomData })
    }
}

impl<F: 'static, T: 'static> BindingTypeConverter for TestTypedConverter<F, T> {
    type From = F;
    type To = T;

    fn affinity(&self) -> i32 {
        self.affinity
    }

    fn convert(&self, _from: Option<&F>, _conversion_hint: Option<&dyn Any>) -> Option<T> {
        None
    }
}

/// A fallback converter of the given affinity that never converts (`TestFallbackConverter`).
struct TestFallbackConverter {
    affinity: i32,
}

impl TestFallbackConverter {
    fn new(affinity: i32) -> Rc<Self> {
        Rc::new(Self { affinity })
    }
}

impl IBindingFallbackConverter for TestFallbackConverter {
    fn get_affinity_for_objects(&self, _from_type: ConverterType, _to_type: ConverterType) -> i32 {
        self.affinity
    }

    fn try_convert(
        &self,
        _from_type: ConverterType,
        _from: &dyn Any,
        _to_type: ConverterType,
        _conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        None
    }
}

/// A set-method converter of the given affinity whose set returns nothing
/// (`TestSetMethodConverter`).
struct TestSetMethodConverter {
    affinity: i32,
}

impl TestSetMethodConverter {
    fn new(affinity: i32) -> Rc<Self> {
        Rc::new(Self { affinity })
    }
}

impl ISetMethodBindingConverter for TestSetMethodConverter {
    fn get_affinity_for_objects(&self, _from_type: Option<ConverterType>, _to_type: Option<ConverterType>) -> i32 {
        self.affinity
    }

    fn perform_set(
        &self,
        _to_target: Option<Rc<dyn Any>>,
        _new_value: Option<Rc<dyn Any>>,
        _arguments: Option<&[Option<Rc<dyn Any>>]>,
    ) -> Option<Rc<dyn Any>> {
        None
    }
}
