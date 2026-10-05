//! The type converter contract: the counterpart of the component model's
//! `TypeConverter` and `ITypeDescriptorContext`, which the converters of
//! the markup runtime derive from and receive.

use crate::XamlLoadException;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// The context a type converter converts in. For markup it is the service
/// provider of the place the value is converted for: base URI, parent
/// stack, type resolver.
pub trait ITypeDescriptorContext: IServiceProvider {
    /// The object the conversion is performed for, if the context knows it.
    fn instance(&self) -> Option<BoxedValue> {
        None
    }
}

/// Contexts compare by identity, so that they can be held in untyped
/// values.
impl PartialEq for dyn ITypeDescriptorContext {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// A service provider as a type descriptor context.
pub struct ServiceProviderTypeDescriptorContext(Rc<dyn IServiceProvider>);

impl ServiceProviderTypeDescriptorContext {
    pub fn new(service_provider: Rc<dyn IServiceProvider>) -> Rc<dyn ITypeDescriptorContext> {
        Rc::new(Self(service_provider))
    }
}

impl IServiceProvider for ServiceProviderTypeDescriptorContext {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        self.0.get_service(service_type)
    }
}

impl ITypeDescriptorContext for ServiceProviderTypeDescriptorContext {}

/// The context as the service provider it is.
pub(crate) fn service_provider_of(context: Option<&Rc<dyn ITypeDescriptorContext>>) -> Option<Rc<dyn IServiceProvider>> {
    let context = context?.clone();
    let service_provider: Rc<dyn IServiceProvider> = context;
    Some(service_provider)
}

/// The text a converter was given, or the error of the cast to text.
pub(crate) fn text_of(value: Option<&BoxedValue>) -> Result<&str, XamlLoadException> {
    let text = value.and_then(|v| v.downcast_ref::<String>().map(String::as_str).or_else(|| v.downcast_ref::<&'static str>().copied()));
    text.ok_or_else(|| {
        XamlLoadException::with_message(format!(
            "Unable to cast object of type '{}' to type 'String'.",
            value.map_or("null", |v| (**v).type_name())
        ))
    })
}

/// Converts values of other types (text, above all) to the type the
/// converter is for, and back.
///
/// The defaults are the ones of the base class of the managed original: a
/// converter converts from nothing and to text.
pub trait TypeConverter {
    /// Whether the converter can convert a value of `source_type` to its
    /// type.
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, _source_type: ValueType) -> bool {
        false
    }

    /// Converts `value` to the type of the converter.
    fn convert_from(
        &self,
        _context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        Err(XamlLoadException::with_message(format!(
            "TypeConverter cannot convert from {}.",
            value.map_or("(null)", |v| (**v).type_name())
        )))
    }

    /// Whether the converter can convert a value of its type to
    /// `destination_type`.
    fn can_convert_to(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, destination_type: ValueType) -> bool {
        destination_type.is_string()
    }

    /// Converts `value` to `destination_type`.
    fn convert_to(
        &self,
        _context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
        destination_type: ValueType,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        if destination_type.is_string() {
            let text = match value {
                Some(value) => ValueTypes::try_to_string(&**value).unwrap_or_else(|| (**value).type_name().to_string()),
                None => String::new(),
            };
            return Ok(Some(Rc::new(text)));
        }
        Err(XamlLoadException::with_message(format!(
            "'TypeConverter' is unable to convert '{}' to '{}'.",
            value.map_or("(null)", |v| (**v).type_name()),
            destination_type
        )))
    }
}

/// Converter handles compare by identity, so that they can be held in
/// untyped values.
impl PartialEq for dyn TypeConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

ferro_markup_type!(interface dyn ITypeDescriptorContext as "ITypeDescriptorContext" {
    this: Rc<dyn ITypeDescriptorContext>,
    handles: [Rc<dyn ITypeDescriptorContext>, Option<Rc<dyn ITypeDescriptorContext>>],
    namespace: "System.ComponentModel",
    interfaces: [Rc<dyn IServiceProvider>],
});

ferro_markup_type!(class dyn TypeConverter as "TypeConverter" {
    this: Rc<dyn TypeConverter>,
    handles: [Rc<dyn TypeConverter>, Option<Rc<dyn TypeConverter>>],
    namespace: "System.ComponentModel",
    methods: [
        fn CanConvertFrom(Option<Rc<dyn ITypeDescriptorContext>>, ValueType) -> bool =>
            |this: &Rc<dyn TypeConverter>, context: Option<Rc<dyn ITypeDescriptorContext>>, source_type: ValueType| {
                this.can_convert_from(context.as_ref(), source_type)
            },
        try fn ConvertFrom(Option<Rc<dyn ITypeDescriptorContext>>, Option<CultureInfo>, Option<BoxedValue>) -> Option<BoxedValue> =>
            |this: &Rc<dyn TypeConverter>,
             context: Option<Rc<dyn ITypeDescriptorContext>>,
             culture: Option<CultureInfo>,
             value: Option<BoxedValue>| {
                this.convert_from(context.as_ref(), culture.as_ref(), value.as_ref())
            },
    ],
});

/// Declares the markup metadata of a converter class: its parameterless
/// constructor, its base (the type converter contract) and the two members
/// the XAML compiler calls.
macro_rules! type_converter_markup {
    ($converter:ident) => {
        ::ferroui_base::ferro_markup_type!(class $converter {
            this: ::std::rc::Rc<$converter>,
            handles: [$converter, ::std::rc::Rc<$converter>, ::std::option::Option<::std::rc::Rc<$converter>>],
            base: ::std::rc::Rc<dyn $crate::converters::TypeConverter>,
            constructors: [() => $converter::new],
            methods: [
                fn CanConvertFrom(
                    ::std::option::Option<::std::rc::Rc<dyn $crate::converters::ITypeDescriptorContext>>,
                    ::ferroui_base::data::core::ValueType
                ) -> bool =>
                    |this: &::std::rc::Rc<$converter>,
                     context: ::std::option::Option<::std::rc::Rc<dyn $crate::converters::ITypeDescriptorContext>>,
                     source_type: ::ferroui_base::data::core::ValueType| {
                        $crate::converters::TypeConverter::can_convert_from(&**this, context.as_ref(), source_type)
                    },
                try fn ConvertFrom(
                    ::std::option::Option<::std::rc::Rc<dyn $crate::converters::ITypeDescriptorContext>>,
                    ::std::option::Option<::ferroui_base::utilities::CultureInfo>,
                    ::std::option::Option<::ferroui_base::BoxedValue>
                ) -> ::std::option::Option<::ferroui_base::BoxedValue> =>
                    |this: &::std::rc::Rc<$converter>,
                     context: ::std::option::Option<::std::rc::Rc<dyn $crate::converters::ITypeDescriptorContext>>,
                     culture: ::std::option::Option<::ferroui_base::utilities::CultureInfo>,
                     value: ::std::option::Option<::ferroui_base::BoxedValue>| {
                        $crate::converters::TypeConverter::convert_from(
                            &**this,
                            context.as_ref(),
                            culture.as_ref(),
                            value.as_ref(),
                        )
                    },
            ],
        });

        impl $converter {
            pub fn new() -> ::std::rc::Rc<Self> {
                ::std::rc::Rc::new(Self)
            }

            /// The converter as the type converter contract.
            pub fn as_type_converter(self: &::std::rc::Rc<Self>) -> ::std::rc::Rc<dyn $crate::converters::TypeConverter> {
                self.clone()
            }
        }

        $crate::identity_eq!($converter);
    };
}
pub(crate) use type_converter_markup;
