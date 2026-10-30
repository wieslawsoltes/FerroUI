// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDockWindow;
use ferroui_base::{
    data::{
        converters::IValueConverter,
        core::{ValueType, ValueTypes},
        BindingError,
    },
    utilities::CultureInfo,
    BoxedValue, FerroProperty,
};
use std::rc::Rc;
/// Displays the original model title, falling back to its concrete runtime class name.
pub struct TitleOrTypeNameConverter;
impl PartialEq for TitleOrTypeNameConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl TitleOrTypeNameConverter {
    /// Creates the original stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the original shared converter instance in the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<TitleOrTypeNameConverter>=TitleOrTypeNameConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for TitleOrTypeNameConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let value = value.and_then(|value| ValueTypes::normalize(value.clone()));
        let Some(value) = value else {
            return Ok(Some(FerroProperty::unset_value()));
        };
        if let Some(model) = crate::model_projection::dockable(Some(&value)) {
            let title = model.title();
            return Ok(Some(Rc::new(if title.is_empty() {
                model.runtime_type_name().to_owned()
            } else {
                title
            })));
        }
        if let Some(window) = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDockWindow>>())
            .and_then(|value| value.downcast_ref::<Rc<dyn IDockWindow>>().cloned())
        {
            let title = window.title();
            return Ok(Some(Rc::new(if title.is_empty() {
                window.runtime_type_name().to_owned()
            } else {
                title
            })));
        }
        let name = ValueTypes::as_object(&*value)
            .map(|object| object.get_type().name().to_owned())
            .unwrap_or_else(|| {
                let full = ValueTypes::type_full_name(ValueType::of_value(&*value));
                full.rsplit(['.', '+', ':'])
                    .next()
                    .unwrap_or(&full)
                    .to_owned()
            });
        Ok(Some(Rc::new(name)))
    }
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message(
            "The method or operation is not implemented.",
        ))
    }
}
ferroui_base::ferro_markup_type!(class TitleOrTypeNameConverter {
 namespace:"FerroDock.Ferro.Converters",this:Rc<TitleOrTypeNameConverter>,handles:[Rc<TitleOrTypeNameConverter>,Option<Rc<TitleOrTypeNameConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>TitleOrTypeNameConverter::new],fields:[Instance:Rc<TitleOrTypeNameConverter> =>TitleOrTypeNameConverter::instance],
});
