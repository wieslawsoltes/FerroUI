//! Port of `GenericMarkupExtension.cs`.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::media::Color;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_markup_xaml::{MarkupExtension, XamlLoadException};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

/// A markup extension with a type argument, which markup states with `x:TypeArguments`.
///
/// The managed original is an open generic class the markup compiler instantiates for the
/// type arguments a document names. Markup knows the instantiations a crate declares
/// (docs/porting/DEVIATIONS.md, "Run-time type system of markup"): the one the documents of
/// the sample name is declared below ([`GenericMarkupExtensionOfColor`]).
pub struct GenericMarkupExtension<T> {
    value: RefCell<T>,
}

impl<T> PartialEq for GenericMarkupExtension<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<T: Clone + Default + Display + 'static> GenericMarkupExtension<T> {
    pub fn new() -> Rc<GenericMarkupExtension<T>> {
        Rc::new(Self { value: RefCell::new(T::default()) })
    }

    pub fn value(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: T) {
        *self.value.borrow_mut() = value;
    }
}

impl<T: Clone + Default + Display + 'static> MarkupExtension for GenericMarkupExtension<T> {
    fn provide_value(&self, _service_provider: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException> {
        // `Value?.GetType().Name`: the name of the type of the value, without its namespace.
        let full_name = ValueTypes::type_full_name(ValueType::of::<T>());
        let name = full_name.rsplit('.').next().unwrap_or(&full_name);
        Ok(Some(Rc::new(format!("{}: {}", name, self.value())) as BoxedValue))
    }
}

/// Declares an instantiation of [`GenericMarkupExtension`] to markup: `$carrier` carries the
/// markup metadata of `GenericMarkupExtension<$value>`.
macro_rules! generic_markup_extension {
    ($carrier:ident: $value:ty) => {
        /// Carries the markup metadata of an instantiation of the generic markup extension.
        pub struct $carrier;

        ferro_markup_type!(class $carrier as "GenericMarkupExtension`1" {
            this: Rc<GenericMarkupExtension<$value>>,
            handles: [
                GenericMarkupExtension<$value>,
                Rc<GenericMarkupExtension<$value>>,
                Option<Rc<GenericMarkupExtension<$value>>>
            ],
            generic: "GenericMarkupExtension`1" [$value],
            constructors: [() => GenericMarkupExtension::<$value>::new],
            properties: [
                Value: $value {
                    get: |this: &Rc<GenericMarkupExtension<$value>>| this.value(),
                    set: |this: &Rc<GenericMarkupExtension<$value>>, value: $value| this.set_value(value)
                },
            ],
            methods: [
                try fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
                    |this: &Rc<GenericMarkupExtension<$value>>, service_provider: Rc<dyn IServiceProvider>| {
                        this.provide_value(&service_provider)
                    },
            ],
        });
    };
}

// `{local:GenericMarkupExtension Value=Red, x:TypeArguments=Color}` of `MainWindow.xaml`.
generic_markup_extension!(GenericMarkupExtensionOfColor: Color);

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_base::media::Colors;
    use std::any::{Any, TypeId};

    struct NoServices;

    impl IServiceProvider for NoServices {
        fn get_service(&self, _service_type: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
    }

    #[test]
    fn the_extension_provides_the_name_of_the_type_and_the_value() {
        crate::register_types();
        let extension = GenericMarkupExtension::<Color>::new();
        extension.set_value(Colors::RED);
        let services: Rc<dyn IServiceProvider> = Rc::new(NoServices);
        let provided = extension.provide_value(&services).ok().flatten().expect("a value");
        assert_eq!(Some(&String::from("Color: Red")), provided.downcast_ref::<String>());
    }
}
