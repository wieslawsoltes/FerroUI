//! `BindingTypeConverterDispatchTests` (`ReactiveUI.Binding.Tests/Bindings/BindingTypeConverterDispatchTests.cs`).

use super::type_converters::converted;
use crate::tests::binding::test_models::{StubBindingTypeConverter, StubFallbackConverter};
use ferroui_reactiveui::*;
use std::any::Any;
use std::rc::Rc;

const FALLBACK_RESULT: &str = "fallback";
const CONVERTED_INTEGER: i32 = 42;
// Upstream's value, which happens to be close to a constant of `std::f64::consts`.
#[allow(clippy::approx_constant)]
const CONVERTED_DOUBLE: f64 = 3.14;

fn text(value: &str) -> String {
    value.to_string()
}

/// A typed stub of the given types that always produces `value`.
fn constant_stub(from_type: ConverterType, to_type: ConverterType, value: i32) -> StubBindingTypeConverter {
    StubBindingTypeConverter::new(from_type, to_type, move |_, _| Some(Rc::new(value) as Rc<dyn Any>))
}

#[test]
fn try_convert_exact_type_match_returns_true() {
    let converter = StubBindingTypeConverter::new(ConverterType::of::<String>(), ConverterType::of::<i32>(), |from, _| {
        let parsed: i32 = from?.downcast_ref::<String>()?.parse().ok()?;
        Some(Rc::new(parsed) as Rc<dyn Any>)
    });
    let result = BindingTypeConverterDispatch::try_convert(&converter, Some(&text("42")), ConverterType::of::<i32>(), None);
    assert_eq!(converted::<i32>(result), Some(CONVERTED_INTEGER));
}

#[test]
fn try_convert_source_type_mismatch_returns_false() {
    let converter = constant_stub(ConverterType::of::<String>(), ConverterType::of::<i32>(), CONVERTED_INTEGER);
    let result = BindingTypeConverterDispatch::try_convert(&converter, Some(&true), ConverterType::of::<i32>(), None);
    assert!(result.is_none());
}

#[test]
fn try_convert_target_type_mismatch_returns_false() {
    let converter = constant_stub(ConverterType::of::<String>(), ConverterType::of::<i32>(), CONVERTED_INTEGER);
    let result = BindingTypeConverterDispatch::try_convert(&converter, Some(&text("42")), ConverterType::of::<bool>(), None);
    assert!(result.is_none());
}

#[test]
fn try_convert_nullable_target_type_returns_true() {
    let converter = StubBindingTypeConverter::new(ConverterType::of::<f64>(), ConverterType::of::<Option<f64>>(), |from, _| {
        Some(Rc::new(from?.downcast_ref::<f64>().copied()) as Rc<dyn Any>)
    });
    let result = BindingTypeConverterDispatch::try_convert(
        &converter,
        Some(&CONVERTED_DOUBLE),
        ConverterType::of::<Option<f64>>(),
        None,
    );
    assert_eq!(converted::<Option<f64>>(result), Some(Some(CONVERTED_DOUBLE)));
}

// Not ported: TryConvert_BoxedNonNullableToNullableConverter_ReturnsTrue - reflection (`Nullable.GetUnderlyingType`; a Rust value keeps its `Option`, so the dispatch matches the source type exactly)
// Not ported: TryConvert_NullInputWithNonNullableFromType_ReturnsFalse - reflection (`Type.IsValueType`; the dispatch hands `None` to the converter, which decides)

#[test]
fn try_convert_passes_conversion_hint() {
    const HINT_VALUE: &str = "some-hint";
    let converter = StubBindingTypeConverter::new(ConverterType::of::<String>(), ConverterType::of::<String>(), |_, hint| {
        let hint = *hint?.downcast_ref::<&'static str>()?;
        Some(Rc::new(hint.to_string()) as Rc<dyn Any>)
    });
    let result = BindingTypeConverterDispatch::try_convert(
        &converter,
        Some(&text("input")),
        ConverterType::of::<String>(),
        Some(&HINT_VALUE),
    );
    assert_eq!(converted::<String>(result), Some(HINT_VALUE.to_string()));
}

#[test]
fn try_convert_fallback_success_returns_true() {
    let converter = StubFallbackConverter::new(|_, _, _, _| Some(Rc::new("fallback-result".to_string()) as Rc<dyn Any>));
    let result = BindingTypeConverterDispatch::try_convert_fallback(
        &converter,
        ConverterType::of::<i32>(),
        &CONVERTED_INTEGER,
        ConverterType::of::<String>(),
        None,
    );
    assert_eq!(converted::<String>(result), Some("fallback-result".to_string()));
}

// Not ported: TryConvertFallback_NullResult_ReturnsFalse - type system (a fallback converter returns `Option<Rc<dyn Any>>`, so it cannot report a success without a value)

#[test]
fn try_convert_fallback_converter_fails_returns_false() {
    let converter = StubFallbackConverter::new(|_, _, _, _| None);
    let result = BindingTypeConverterDispatch::try_convert_fallback(
        &converter,
        ConverterType::of::<i32>(),
        &CONVERTED_INTEGER,
        ConverterType::of::<String>(),
        None,
    );
    assert!(result.is_none());
}

#[test]
fn try_convert_any_routes_to_typed_converter() {
    let converter = ResolvedConverter::Typed(Rc::new(constant_stub(
        ConverterType::of::<String>(),
        ConverterType::of::<i32>(),
        CONVERTED_INTEGER,
    )));
    let result = BindingTypeConverterDispatch::try_convert_any(
        Some(&converter),
        ConverterType::of::<String>(),
        Some(&text("input")),
        ConverterType::of::<i32>(),
        None,
    );
    assert_eq!(converted::<i32>(result), Some(CONVERTED_INTEGER));
}

#[test]
fn try_convert_any_routes_to_fallback_converter() {
    let converter = ResolvedConverter::Fallback(Rc::new(StubFallbackConverter::new(|_, _, _, _| {
        Some(Rc::new(FALLBACK_RESULT.to_string()) as Rc<dyn Any>)
    })));
    let result = BindingTypeConverterDispatch::try_convert_any(
        Some(&converter),
        ConverterType::of::<i32>(),
        Some(&CONVERTED_INTEGER),
        ConverterType::of::<String>(),
        None,
    );
    assert_eq!(converted::<String>(result), Some(FALLBACK_RESULT.to_string()));
}

#[test]
fn try_convert_any_null_input_with_fallback_returns_false() {
    let converter = ResolvedConverter::Fallback(Rc::new(StubFallbackConverter::new(|_, _, _, _| {
        Some(Rc::new(FALLBACK_RESULT.to_string()) as Rc<dyn Any>)
    })));
    let result = BindingTypeConverterDispatch::try_convert_any(
        Some(&converter),
        ConverterType::of::<i32>(),
        None,
        ConverterType::of::<String>(),
        None,
    );
    assert!(result.is_none());
}

// Not ported: TryConvertAny_UnknownConverterType_ReturnsFalse - type system (`ResolvedConverter` holds a typed or a fallback converter, nothing else)

#[test]
fn try_convert_any_null_converter_returns_false() {
    let result = BindingTypeConverterDispatch::try_convert_any(
        None,
        ConverterType::of::<i32>(),
        Some(&CONVERTED_INTEGER),
        ConverterType::of::<String>(),
        None,
    );
    assert!(result.is_none());
}

#[test]
fn try_convert_null_input_with_nullable_from_type_proceeds_to_convert() {
    let converter = StubBindingTypeConverter::new(ConverterType::of::<Option<i32>>(), ConverterType::of::<String>(), |from, _| {
        let value = from.and_then(|from| from.downcast_ref::<Option<i32>>().copied().flatten());
        Some(Rc::new(value.map_or_else(|| "null".to_string(), |value| value.to_string())) as Rc<dyn Any>)
    });
    let result = BindingTypeConverterDispatch::try_convert(&converter, None, ConverterType::of::<String>(), None);
    assert_eq!(converted::<String>(result), Some("null".to_string()));
}

#[test]
fn try_convert_nullable_integer_to_string_null_input_succeeds() {
    let result =
        BindingTypeConverterDispatch::try_convert(&NullableIntegerToStringTypeConverter, None, ConverterType::of::<String>(), None);
    // Upstream converts null to a null string; the port converts it to an empty string.
    assert_eq!(converted::<String>(result), Some(String::new()));
}
