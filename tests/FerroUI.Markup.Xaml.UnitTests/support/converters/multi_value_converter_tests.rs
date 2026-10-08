//! The test types declared by the upstream test file
//! `Converters/MultiValueConverterTests.cs`.

use ferroui_base::utilities::CultureInfo;
use std::rc::{Rc, Weak};

use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};

use crate::support::TypeModule;

// --- TestMultiValueConverter -------------------------------------------------

/// A converter that returns the special values of bindings: text for a
/// positive product of its two numbers, the unset marker for zero and the
/// do-nothing marker for a negative product.
pub struct TestMultiValueConverter {
    this: Weak<TestMultiValueConverter>,
}

crate::test_identity_eq!(TestMultiValueConverter);

impl TestMultiValueConverter {
    pub fn new() -> Rc<TestMultiValueConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The shared instance.
    pub fn instance() -> Rc<TestMultiValueConverter> {
        thread_local! {
            static INSTANCE: Rc<TestMultiValueConverter> = TestMultiValueConverter::new();
        }
        INSTANCE.with(Rc::clone)
    }

    /// The converter as the multi-value converter contract.
    pub fn as_multi_value_converter(&self) -> Rc<dyn IMultiValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IMultiValueConverter for TestMultiValueConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let int = |index: usize| values[index].as_ref().and_then(|value| value.downcast_ref::<i32>()).copied();

        if let (Some(i), Some(j)) = (int(0), int(1)) {
            let p = i * j;

            if p > 0 {
                return Ok(Some(Rc::new("foo".to_string())));
            }

            if p == 0 {
                return Ok(Some(FerroProperty::unset_value()));
            }

            return Ok(Some(BindingOperations::do_nothing()));
        }

        Ok(Some(Rc::new("(default)".to_string())))
    }
}

ferro_markup_type!(class TestMultiValueConverter {
    this: Rc<TestMultiValueConverter>,
    handles: [TestMultiValueConverter, Rc<TestMultiValueConverter>, Option<Rc<TestMultiValueConverter>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => TestMultiValueConverter::new],
    fields: [Instance: Rc<TestMultiValueConverter> => TestMultiValueConverter::instance],
});

// --- TupleOfInt32 ------------------------------------------------------------

/// The stand-in of the two-number tuple the test uses as data context
/// (`Tuple.Create(2, 2)`): a shared object with the read-only properties
/// `Item1` and `Item2`.
pub struct TupleOfInt32 {
    item1: i32,
    item2: i32,
}

crate::test_identity_eq!(TupleOfInt32);

impl TupleOfInt32 {
    pub fn create(item1: i32, item2: i32) -> Rc<Self> {
        Rc::new(Self { item1, item2 })
    }

    pub fn item1(&self) -> i32 {
        self.item1
    }

    pub fn item2(&self) -> i32 {
        self.item2
    }
}

ferro_markup_type!(class TupleOfInt32 {
    this: Rc<TupleOfInt32>,
    handles: [TupleOfInt32, Rc<TupleOfInt32>, Option<Rc<TupleOfInt32>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
    properties: [
        Item1: i32 { get: |this: &Rc<TupleOfInt32>| this.item1() },
        Item2: i32 { get: |this: &Rc<TupleOfInt32>| this.item2() },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<TestMultiValueConverter as MarkupTyped>::MARKUP, <TupleOfInt32 as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<TestMultiValueConverter>();
        ValueTypes::register_reference::<TupleOfInt32>();
        ValueTypes::register_cast::<TestMultiValueConverter, Rc<dyn IMultiValueConverter>>(
            TestMultiValueConverter::as_multi_value_converter,
        );
    },
};
