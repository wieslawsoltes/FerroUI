//! Port of `ViewModels/DataValidationViewModel.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::BindingError;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ErrorConverter;
use mini_mvvm::ViewModelBase;
use std::cell::RefCell;
use std::rc::Rc;

/// The view model of the data validation page.
pub struct DataValidationViewModel {
    base: ViewModelBase,
    data_annotations_sample: RefCell<Option<String>>,
    converter: ErrorConverter,
    exception_inside_setter_sample: RefCell<Option<String>>,
    exception_converter: ErrorConverter,
}

impl PartialEq for DataValidationViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for DataValidationViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl DataValidationViewModel {
    pub fn new() -> Rc<DataValidationViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            data_annotations_sample: RefCell::new(None),
            converter: ErrorConverter::new(|o: &BoxedValue| {
                Some(Rc::new(format!("Error: {}", ValueTypes::to_display_string(Some(o)))) as BoxedValue)
            }),
            exception_inside_setter_sample: RefCell::new(None),
            exception_converter: ErrorConverter::new(|o: &BoxedValue| {
                let text = match o.downcast_ref::<BindingError>() {
                    Some(ex) => format!("Huh, there was an Exception: {ex}"),
                    None => "Something went really wrong!".to_string(),
                };
                Some(Rc::new(text) as BoxedValue)
            }),
        })
    }

    pub fn data_annotations_sample(&self) -> Option<String> {
        self.data_annotations_sample.borrow().clone()
    }

    pub fn set_data_annotations_sample(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.data_annotations_sample, value, "DataAnnotationsSample");
    }

    pub fn converter(&self) -> ErrorConverter {
        self.converter.clone()
    }

    pub fn exception_inside_setter_sample(&self) -> Option<String> {
        self.exception_inside_setter_sample.borrow().clone()
    }

    /// The setter fails for a text shorter than five characters (the
    /// `ArgumentOutOfRangeException` of the managed original).
    pub fn set_exception_inside_setter_sample(&self, value: Option<String>) -> Result<(), String> {
        match &value {
            Some(value) if value.encode_utf16().count() >= 5 => {}
            _ => return Err("Give me 5 or more letter please :-) (Parameter 'value')".to_string()),
        }

        self.base.raise_and_set_if_changed(&self.exception_inside_setter_sample, value, "ExceptionInsideSetterSample");
        Ok(())
    }

    pub fn exception_converter(&self) -> ErrorConverter {
        self.exception_converter.clone()
    }
}

ferro_markup_type!(class DataValidationViewModel {
    this: Rc<DataValidationViewModel>,
    handles: [DataValidationViewModel, Rc<DataValidationViewModel>, Option<Rc<DataValidationViewModel>>],
    constructors: [() => DataValidationViewModel::new],
    properties: [
        DataAnnotationsSample: Option<String> {
            get: |this: &Rc<DataValidationViewModel>| this.data_annotations_sample(),
            set: |this: &Rc<DataValidationViewModel>, value: Option<String>| this.set_data_annotations_sample(value)
        },
        Converter: ErrorConverter { get: |this: &Rc<DataValidationViewModel>| this.converter() },
        ExceptionInsideSetterSample: Option<String> {
            get: |this: &Rc<DataValidationViewModel>| this.exception_inside_setter_sample(),
            try_set: |this: &Rc<DataValidationViewModel>, value: Option<String>| {
                this.set_exception_inside_setter_sample(value)
            }
        },
        ExceptionConverter: ErrorConverter { get: |this: &Rc<DataValidationViewModel>| this.exception_converter() },
    ],
    notify_property_changed: DataValidationViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_setter_rejects_a_text_shorter_than_five_characters() {
        let view_model = DataValidationViewModel::new();
        assert!(view_model.set_exception_inside_setter_sample(None).is_err());
        assert_eq!(
            Err("Give me 5 or more letter please :-) (Parameter 'value')".to_string()),
            view_model.set_exception_inside_setter_sample(Some("abcd".to_string()))
        );
        assert_eq!(None, view_model.exception_inside_setter_sample());
        assert_eq!(Ok(()), view_model.set_exception_inside_setter_sample(Some("abcde".to_string())));
        assert_eq!(Some("abcde".to_string()), view_model.exception_inside_setter_sample());
    }

    #[test]
    fn the_converters_describe_the_error() {
        let view_model = DataValidationViewModel::new();
        let text: BoxedValue = Rc::new("required".to_string());
        let converted = view_model.converter().convert(&text).expect("a value");
        assert_eq!(Some(&"Error: required".to_string()), converted.downcast_ref::<String>());

        let converted = view_model.exception_converter().convert(&text).expect("a value");
        assert_eq!(Some(&"Something went really wrong!".to_string()), converted.downcast_ref::<String>());

        let exception: BoxedValue = Rc::new(BindingError::message("too short"));
        let converted = view_model.exception_converter().convert(&exception).expect("a value");
        assert_eq!(Some(&"Huh, there was an Exception: too short".to_string()), converted.downcast_ref::<String>());
    }
}
