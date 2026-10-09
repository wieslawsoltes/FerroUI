use crate::metadata::PseudoClassesAttribute;
use crate::primitives::TemplatedControlImpl;
use crate::templates::IDataTemplate;
use crate::{ContentControl, ContentControlImpl, Control, ControlImpl};
use ferroui_base::data::{
    AggregateError, AggregateException, BindingChainException, BindingError, DataValidationException,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, BoxedValue, DirectProperty,
    ElementRef, FerroObject, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElement, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Converts an error of a control into the object that is shown for it; an
/// error converted to `None` is dropped.
///
/// Two converters are equal when they are the same function object.
#[derive(Clone)]
pub struct ErrorConverter(Rc<dyn Fn(&BoxedValue) -> Option<BoxedValue>>);

impl ErrorConverter {
    pub fn new(convert: impl Fn(&BoxedValue) -> Option<BoxedValue> + 'static) -> Self {
        Self(Rc::new(convert))
    }

    /// Converts an error.
    pub fn convert(&self, error: &BoxedValue) -> Option<BoxedValue> {
        (self.0)(error)
    }
}

impl PartialEq for ErrorConverter {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for ErrorConverter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ErrorConverter")
    }
}

/// A control which displays an error notifier when there is a data
/// validation error. Provides attached properties to track errors on a
/// control.
///
/// You will probably only want to create instances inside of control
/// templates. Pseudo-classes (on the control with the errors): `:error`.
///
/// An error is an untyped value: the error data of a data validation
/// error, or the error itself boxed as a [`BindingError`].
#[repr(C)]
pub struct DataValidationErrors {
    base: ContentControl,
    owner: RefCell<Option<ElementRef<Control>>>,
}

ferro_class!(DataValidationErrors: ContentControl);
ferroui_base::ferro_class_info!(DataValidationErrors { new: DataValidationErrors::new });
ferro_impl_classes!(
    DataValidationErrors: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

ferroui_base::ferro_impl_classes!(DataValidationErrors: FerroObjectImpl);

impl DataValidationErrors {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":error"]);
}

ferroui_base::ferro_properties! { impl DataValidationErrors {
    ferro_property!(
        /// Defines the `DataValidationErrors.Errors` attached property.
        pub fn errors_property() -> AttachedProperty<Option<Vec<BoxedValue>>> {
            FerroProperty::register_attached::<DataValidationErrors, Control, _>("Errors", None)
        }
    );

    ferro_property!(
        /// Defines the `DataValidationErrors.HasErrors` attached property.
        pub fn has_errors_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<DataValidationErrors, Control, _>("HasErrors", false)
        }
    );

    ferro_property!(
        /// Defines the `DataValidationErrors.ErrorConverter` attached
        /// property.
        pub fn error_converter_property() -> AttachedProperty<Option<ErrorConverter>> {
            FerroProperty::register_attached::<DataValidationErrors, Control, _>("ErrorConverter", None)
        }
    );

    ferro_property!(
        /// Defines the `DataValidationErrors.ErrorTemplate` property.
        pub fn error_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<DataValidationErrors, _>("ErrorTemplate", None)
        }
    );

    ferro_property!(
        /// Stores the original, not converted errors passed by the control.
        fn original_errors_property() -> AttachedProperty<Option<Vec<BoxedValue>>> {
            FerroProperty::register_attached::<DataValidationErrors, Control, _>("OriginalErrors", None)
        }
    );

    ferro_property!(
        /// Prevents executing `errors_changed` after the errors are updated
        /// internally from `on_errors_or_converter_changed`.
        fn overriding_errors_internally_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<DataValidationErrors, Control, _>("OverridingErrorsInternally", false)
        }
    );

    ferro_property!(
        /// Defines the `Owner` property.
        ///
        /// Deviation (DEVIATIONS.md, Data validation): the owner is the
        /// control the errors are shown in, an ancestor of this control,
        /// which this control does not own, so the value is an element
        /// reference where the managed property holds the control. The themes make it the `DataContext` of a
        /// part of the template (`DataContext="{TemplateBinding Owner}"`),
        /// which holds the reference as it is.
        pub fn owner_property() -> DirectProperty<DataValidationErrors, Option<ElementRef<Control>>> {
            ferroui_base::data::core::ValueTypes::register_element_ref::<Control>();
            FerroProperty::register_direct::<DataValidationErrors, _>(
                "Owner",
                |o| o.owner.borrow().clone(),
                Some(|o, v| o.set_owner(ElementRef::resolve(&v))),
                None,
            )
        }
    );
} }

impl DataValidationErrors {
    /// Initializes the static members of the `DataValidationErrors` class.
    fn static_constructor() {
        Self::errors_property().changed().subscribe(Self::errors_changed);
        Self::has_errors_property().changed().subscribe(Self::has_errors_changed);
        StyledElement::templated_parent_property()
            .changed()
            .add_class_handler::<DataValidationErrors>(|x, e| x.on_templated_parent_change(e));
        Self::error_converter_property().changed().subscribe(Self::on_error_converter_changed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct(), owner: RefCell::new(None) }
    }

    /// Initializes a new instance of the `DataValidationErrors` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The control whose errors are displayed.
    pub fn owner(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.owner.borrow())
    }

    pub fn set_owner(&self, value: Option<Ref<Control>>) {
        self.set_and_raise(Self::owner_property(), &self.owner, ElementRef::from_nullable(value));
    }

    /// The control of a change of one of the attached properties.
    ///
    /// # Panics
    ///
    /// When the sender is not a control: the attached properties are
    /// defined for controls.
    fn sender_control(e: &FerroPropertyChangedEventArgs<'_>) -> Ref<Control> {
        match e.sender().downcast_ref::<Control>() {
            Some(control) => control.to_ref(),
            None => panic!(
                "Unable to cast object of type '{}' to type 'Control'.",
                e.sender().get_type().name()
            ),
        }
    }

    fn on_error_converter_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        Self::on_errors_or_converter_changed(&Self::sender_control(e));
    }

    fn on_templated_parent_change(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.owner().is_none() {
            let new_value = e.get_new_value::<Option<Ref<FerroObject>>>();
            self.set_owner(new_value.and_then(|value| value.cast::<Control>()));
        }
    }

    /// The data template of the errors.
    pub fn error_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::error_template_property())
    }

    pub fn set_error_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::error_template_property(), value)
    }

    fn errors_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let control = Self::sender_control(e);

        if control.get_value(Self::overriding_errors_internally_property()) {
            return;
        }

        let errors = e.get_new_value::<Option<Vec<BoxedValue>>>();

        // Update the original errors.
        control.set_value(Self::original_errors_property(), errors);

        Self::on_errors_or_converter_changed(&control);
    }

    fn has_errors_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let control = Self::sender_control(e);
        control.pseudo_classes().set(":error", e.get_new_value::<bool>());
    }

    /// Gets the errors of a control.
    pub fn get_errors(control: &Control) -> Option<Vec<BoxedValue>> {
        control.get_value(Self::errors_property())
    }

    /// Sets the errors of a control.
    pub fn set_errors(control: &Control, errors: Option<Vec<BoxedValue>>) {
        control.set_value(Self::errors_property(), errors);
    }

    /// Sets the errors of a control from the error of a binding.
    pub fn set_error(control: &Control, error: Option<&BindingError>) {
        Self::set_errors(
            control,
            Self::unpack_exception(error)
                .map(|errors| errors.iter().filter_map(Self::unpack_data_validation_exception).collect()),
        );
    }

    fn on_errors_or_converter_changed(control: &Control) {
        let converter = Self::get_error_converter(control);
        let original_errors = control.get_value(Self::original_errors_property());
        let new_errors: Option<Vec<BoxedValue>> = match converter {
            None => original_errors,
            Some(converter) => {
                original_errors.map(|errors| errors.iter().filter_map(|error| converter.convert(error)).collect())
            }
        };

        let has_errors = new_errors.as_ref().is_some_and(|errors| !errors.is_empty());

        control.set_current_value(Self::overriding_errors_internally_property(), true);

        {
            // Restores the flag when the scope ends, also by a panic of a handler.
            struct Restore<'a>(&'a Control);

            impl Drop for Restore<'_> {
                fn drop(&mut self) {
                    self.0.set_current_value(DataValidationErrors::overriding_errors_internally_property(), false);
                }
            }

            let _restore = Restore(control);

            control.set_current_value(Self::errors_property(), new_errors);
        }

        control.set_value(Self::has_errors_property(), has_errors);
    }

    /// Clears the errors of a control.
    pub fn clear_errors(control: &Control) {
        Self::set_errors(control, None);
    }

    /// Whether a control has errors.
    pub fn get_has_errors(control: &Control) -> bool {
        control.get_value(Self::has_errors_property())
    }

    /// Gets the error converter of a control.
    pub fn get_error_converter(control: &Control) -> Option<ErrorConverter> {
        control.get_value(Self::error_converter_property())
    }

    /// Sets the error converter of a control.
    pub fn set_error_converter(control: &Control, converter: Option<ErrorConverter>) {
        control.set_value(Self::error_converter_property(), converter);
    }

    /// The errors of an error: the inner errors of an aggregate or the error
    /// itself, without the errors of the binding chain.
    fn unpack_exception(exception: Option<&BindingError>) -> Option<Vec<BindingError>> {
        let exception = exception?;
        let inner = exception.inner();

        let exceptions: Vec<BindingError> = if let Some(aggregate) = inner.downcast_ref::<AggregateError>() {
            aggregate.inner_errors().to_vec()
        } else if let Some(aggregate) = inner.downcast_ref::<AggregateException>() {
            aggregate.inner_exceptions().iter().cloned().map(BindingError::new).collect()
        } else {
            vec![exception.clone()]
        };

        Some(exceptions.into_iter().filter(|x| !x.inner().is::<BindingChainException>()).collect())
    }

    /// The error data of a data validation error (`None` when it has none),
    /// any other error as it is.
    fn unpack_data_validation_exception(exception: &BindingError) -> Option<BoxedValue> {
        if let Some(data_validation_exception) = exception.inner().downcast_ref::<DataValidationException>() {
            return data_validation_exception.error_data().cloned();
        }

        let error: BoxedValue = Rc::new(exception.clone());
        Some(error)
    }
}
