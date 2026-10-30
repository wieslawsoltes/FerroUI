// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IMultiValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;
/// Original first non-null value converter.
pub struct EitherNotNullConverter;
impl PartialEq for EitherNotNullConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl EitherNotNullConverter {
    /// Creates the source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared UI-thread converter.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<EitherNotNullConverter>=EitherNotNullConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IMultiValueConverter for EitherNotNullConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        for value in values {
            if value.is_some() {
                return Ok(value.clone());
            }
        }
        // The native interface borrows its inputs. Keep the source list-valued
        // fallback; retaining the caller's original list identity remains open.
        Ok(Some(Rc::new(values.to_vec())))
    }
}
ferroui_base::ferro_markup_type!(class EitherNotNullConverter{namespace:"FerroDock.Ferro.Converters",this:Rc<EitherNotNullConverter>,handles:[Rc<EitherNotNullConverter>,Option<Rc<EitherNotNullConverter>>],interfaces:[Rc<dyn IMultiValueConverter>],constructors:[()=>EitherNotNullConverter::new],fields:[Instance:Rc<EitherNotNullConverter> =>EitherNotNullConverter::instance],});
