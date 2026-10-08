use super::{ValueType, ValueTypes};
use crate::{BoxedValue, UnsetValueType};

/// Converts the values produced by a binding to the type of its target (and
/// target values back to the type of the source).
///
/// The managed implementation has a default converter for compiled bindings
/// (instance checks, `ToString`, numeric conversions, declared type
/// converters) and a reflection based one. Both are answered here by the
/// [`ValueTypes`] table, which holds the conversions the application
/// declared.
///
/// The reflection converter (string-path bindings) also turns the delegate
/// of a method into a command for a command-typed target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TargetTypeConverter {
    reflection: bool,
}

impl TargetTypeConverter {
    /// The converter used by compiled bindings.
    pub fn get_default_converter() -> Self {
        TargetTypeConverter { reflection: false }
    }

    /// The converter used by bindings with a string path.
    pub fn get_reflection_converter() -> Self {
        TargetTypeConverter { reflection: true }
    }

    /// Converts `value` to exactly `type_`. Returns `None` if the value
    /// cannot be converted. The unset marker passes through unchanged.
    pub fn try_convert(&self, value: Option<&BoxedValue>, type_: ValueType) -> Option<Option<BoxedValue>> {
        if value.is_some_and(|v| v.is::<UnsetValueType>()) {
            return Some(value.cloned());
        }
        if self.reflection {
            if let Some(command) =
                value.and_then(|value| crate::data::converters::method_to_command(value, type_))
            {
                return Some(Some(command));
            }
        }
        // Includes the conversion of text with the `parse:` of the markup
        // metadata of the target type (the type converter of the type in the
        // managed original).
        ValueTypes::try_convert(value, type_)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::converters::MethodToCommandConverter;
    use crate::input::ICommand;
    use crate::metadata::{MarkupDelegate, MarkupType, MarkupTyped};
    use crate::data::converters::IValueConverter as _;
    use crate::{ferro_markup_type, FerroObject, FerroProperty, ObjectType as _, Ref};
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct ConvertedLength(i32);

    impl ConvertedLength {
        fn parse(text: &str) -> Result<Self, String> {
            text.strip_suffix("px")
                .and_then(|number| number.parse().ok())
                .map(ConvertedLength)
                .ok_or_else(|| format!("'{text}' is not a length"))
        }
    }

    ferro_markup_type!(struct ConvertedLength { handles: [ConvertedLength], parse: ConvertedLength::parse });

    pub struct ConvertedVm {
        runs: Cell<i32>,
    }

    impl PartialEq for ConvertedVm {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    ferro_markup_type!(class ConvertedVm {
        this: Rc<ConvertedVm>,
        handles: [ConvertedVm, Rc<ConvertedVm>],
        methods: [
            fn Run() => |vm: &Rc<ConvertedVm>| vm.runs.set(vm.runs.get() + 1),
            fn Two(i32, i32) => |_: &Rc<ConvertedVm>, _: i32, _: i32| {},
        ],
    });

    fn text(value: &str) -> BoxedValue {
        Rc::new(value.to_string())
    }

    #[test]
    fn text_is_converted_with_the_parse_of_the_metadata_of_the_target_type() {
        MarkupType::register(<ConvertedLength as MarkupTyped>::MARKUP);
        ValueTypes::register_nullable::<ConvertedLength>();
        for converter in [TargetTypeConverter::get_default_converter(), TargetTypeConverter::get_reflection_converter()] {
            let converted = converter.try_convert(Some(&text("12px")), ValueType::of::<ConvertedLength>());
            let converted = converted.flatten().expect("a length");
            assert_eq!(converted.downcast_ref::<ConvertedLength>(), Some(&ConvertedLength(12)));

            let nullable = converter.try_convert(Some(&text("3px")), ValueType::of::<Option<ConvertedLength>>());
            let nullable = nullable.flatten().expect("a nullable length");
            assert_eq!(nullable.downcast_ref::<Option<ConvertedLength>>(), Some(&Some(ConvertedLength(3))));

            // Text that does not parse, and values that are not text, do not convert.
            assert!(converter.try_convert(Some(&text("wide")), ValueType::of::<ConvertedLength>()).is_none());
            let number: BoxedValue = Rc::new(1.5f64);
            assert!(converter.try_convert(Some(&number), ValueType::of::<ConvertedLength>()).is_none());
            // The unset marker passes through.
            let unset = FerroProperty::unset_value();
            assert!(converter.try_convert(Some(&unset), ValueType::of::<ConvertedLength>()).flatten().is_some());
        }
    }

    crate::tests::test_class!(ConvertedShape: FerroObject);
    crate::tests::test_class!(ConvertedDerivedShape: ConvertedShape);
    crate::tests::test_class!(ConvertedPlainObject: FerroObject);

    thread_local! {
        static PARSED_SHAPES: Cell<i32> = const { Cell::new(0) };
    }

    impl ConvertedShape {
        fn parse(text: &str) -> Result<Ref<ConvertedShape>, String> {
            if !text.starts_with('M') {
                return Err(format!("'{text}' is not path data"));
            }
            PARSED_SHAPES.with(|count| count.set(count.get() + 1));
            Ok(ConvertedShape::new())
        }
    }

    crate::ferro_class_info!(ConvertedShape { markup: { parse: ConvertedShape::parse } });
    crate::ferro_class_info!(ConvertedDerivedShape { markup: {} });

    #[test]
    fn text_is_converted_to_a_class_with_the_parse_of_its_metadata() {
        ValueTypes::register_object::<ConvertedShape>();
        ValueTypes::register_object::<ConvertedDerivedShape>();
        ValueTypes::register_object::<ConvertedPlainObject>();
        ValueTypes::register_upcast::<ConvertedDerivedShape, ConvertedShape>();
        let data = text("M0,0l1,0");
        for converter in [TargetTypeConverter::get_default_converter(), TargetTypeConverter::get_reflection_converter()] {
            // The handle and its nullable form (the type of a property that holds a shape).
            let converted = converter.try_convert(Some(&data), ValueType::of::<Ref<ConvertedShape>>());
            assert!(converted.flatten().expect("a shape").is::<Ref<ConvertedShape>>());
            let converted = converter.try_convert(Some(&data), ValueType::of::<Option<Ref<ConvertedShape>>>());
            let converted = converted.flatten().expect("a nullable shape");
            assert!(converted.downcast_ref::<Option<Ref<ConvertedShape>>>().is_some_and(Option::is_some));

            // Text that does not parse is not converted.
            let none = converter.try_convert(Some(&text("x")), ValueType::of::<Option<Ref<ConvertedShape>>>());
            assert!(none.is_none());
            // The conversion of the base class applies to a derived class, but its result
            // is not a value of the derived class.
            let derived = converter.try_convert(Some(&data), ValueType::of::<Ref<ConvertedDerivedShape>>());
            assert!(derived.is_none());
            // A class that states no conversion.
            let plain = converter.try_convert(Some(&data), ValueType::of::<Option<Ref<ConvertedPlainObject>>>());
            assert!(plain.is_none());
        }
        assert!(PARSED_SHAPES.with(Cell::get) >= 4);

        // Every conversion of a value to a type consults it: the table itself and the
        // default value converter of bindings.
        let converted = ValueTypes::try_convert(Some(&data), ValueType::of::<Option<Ref<ConvertedShape>>>());
        assert!(converted.flatten().is_some());
        assert_eq!(ValueTypes::class_of(ValueType::of::<Ref<ConvertedShape>>()), Some(ConvertedShape::TYPE));
        assert_eq!(ValueTypes::class_of(ValueType::of::<Option<Ref<ConvertedShape>>>()), Some(ConvertedShape::TYPE));
        assert_eq!(ValueTypes::class_of(ValueType::of::<String>()), None);
        let default = crate::data::converters::DefaultValueConverter::instance();
        let converted = default.convert(Some(&data), ValueType::of::<Option<Ref<ConvertedShape>>>(), None, &crate::utilities::CultureInfo::invariant_culture()).unwrap();
        assert!(converted.expect("a value").downcast_ref::<Option<Ref<ConvertedShape>>>().is_some_and(Option::is_some));
        MarkupType::register(<ConvertedLength as MarkupTyped>::MARKUP);
        let length = default.convert_back(Some(&text("7px")), ValueType::of::<ConvertedLength>(), None, &crate::utilities::CultureInfo::invariant_culture()).unwrap();
        assert_eq!(length.expect("a value").downcast_ref::<ConvertedLength>(), Some(&ConvertedLength(7)));

        // The reverse direction is the registered display form (here: of a number).
        let number: BoxedValue = Rc::new(1.5f64);
        let shown = ValueTypes::try_convert(Some(&number), ValueType::of::<String>()).flatten().expect("text");
        assert_eq!(shown.downcast_ref::<String>().map(String::as_str), Some("1.5"));
    }

    #[test]
    fn only_the_reflection_converter_turns_a_method_into_a_command() {
        ValueTypes::register_reference::<ConvertedVm>();
        let markup = <ConvertedVm as MarkupTyped>::MARKUP;
        let vm = Rc::new(ConvertedVm { runs: Cell::new(0) });
        let delegate = |name: &str| -> BoxedValue {
            let method = markup.find_methods(name).next().expect("the method");
            Rc::new(MarkupDelegate::for_method(Some(vm.clone()), markup, method))
        };
        let reflection = TargetTypeConverter::get_reflection_converter();
        let default = TargetTypeConverter::get_default_converter();
        assert_ne!(reflection, default);

        for target in [ValueType::of::<Rc<dyn ICommand>>(), ValueType::of::<Option<Rc<dyn ICommand>>>()] {
            let command = reflection.try_convert(Some(&delegate("Run")), target).flatten().expect("a command");
            assert_eq!(ValueType::of_value(&*command), target);
            assert!(default.try_convert(Some(&delegate("Run")), target).is_none());
            // A method with more than one parameter is no command.
            assert!(reflection.try_convert(Some(&delegate("Two")), target).is_none());
            // Neither is a callback that is not the delegate of a method.
            let callback: BoxedValue = Rc::new(MarkupDelegate::new(|_| None));
            assert!(reflection.try_convert(Some(&callback), target).is_none());
        }
        let command = reflection.try_convert(Some(&delegate("Run")), ValueType::of::<Rc<dyn ICommand>>()).flatten();
        command.unwrap().downcast_ref::<Rc<dyn ICommand>>().unwrap().execute(None);
        assert_eq!(vm.runs.get(), 1);

        // A callback as a command calls the callback with the parameter.
        let seen = Rc::new(Cell::new(0));
        let callback = MarkupDelegate::new({
            let seen = seen.clone();
            move |arguments| {
                seen.set(arguments.len());
                None
            }
        });
        let command = MethodToCommandConverter::new(&callback);
        assert!(command.can_execute(None));
        command.execute(None);
        assert_eq!(seen.get(), 1);
    }
}
