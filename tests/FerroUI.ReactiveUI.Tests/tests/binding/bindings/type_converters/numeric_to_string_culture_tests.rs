//! `NumericToStringCultureTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NumericToStringCultureTests.cs`).
//!
//! The port formats numbers with the invariant culture, which writes `42.5` and `42.50` as upstream's
//! en-US culture does. Upstream's matrix has a flavour axis (`ReactiveUI.Binding` and
//! `ReactiveUI.Binding.Reactive`); the port has one flavour, and the `decimal` rows wait for
//! `Decimal`.

use super::converted;
use ferroui_reactiveui::*;
use std::any::Any;
use std::rc::Rc;

// Not ported: TestCulture_WithoutOverride_UsesEnglishCulture - culture (the port has no current culture; it formats with the invariant culture)
// Not ported: TryConvert_FrenchCulture_UsesCurrentCulture - culture (the port formats with the invariant culture)

/// The hints of the matrix: none, two decimals, the format `F2`.
fn hints() -> Vec<Option<Rc<dyn Any>>> {
    let two_decimals: Rc<dyn Any> = Rc::new(2i32);
    let format: Rc<dyn Any> = Rc::new("F2");
    vec![None, Some(two_decimals), Some(format)]
}

#[test]
fn try_convert_english_culture_uses_current_culture() {
    const SINGLE_VALUE: f32 = 42.5;
    const DOUBLE_VALUE: f64 = 42.5;

    for hint in hints() {
        let expected = if hint.is_none() { "42.5" } else { "42.50" };
        let hint = hint.as_deref();
        for nullable in [false, true] {
            let mut cases: Vec<(Rc<dyn IBindingTypeConverter>, Rc<dyn Any>)> = Vec::new();
            if nullable {
                cases.push((Rc::new(NullableSingleToStringTypeConverter), Rc::new(Some(SINGLE_VALUE))));
                cases.push((Rc::new(NullableDoubleToStringTypeConverter), Rc::new(Some(DOUBLE_VALUE))));
            } else {
                cases.push((Rc::new(SingleToStringTypeConverter), Rc::new(SINGLE_VALUE)));
                cases.push((Rc::new(DoubleToStringTypeConverter), Rc::new(DOUBLE_VALUE)));
            }
            for (converter, value) in cases {
                let output = converted::<String>(converter.try_convert_typed(Some(&*value), hint));
                assert_eq!(output.as_deref(), Some(expected), "nullable: {nullable}, from: {}", converter.from_type());
            }
        }
    }
    // Not ported: the `Decimal` rows of the matrix - needs Decimal in ferroui-system (phase 1b)
}
