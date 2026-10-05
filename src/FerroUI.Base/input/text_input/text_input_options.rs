use super::{TextInputContentType, TextInputReturnKeyType};
use crate::{ferro_property, AttachedProperty, FerroProperty, StyledElement, StyledPropertyOptions};
use std::rc::Rc;

/// Describes how a text input wants the input method of the platform to
/// behave: the properties are attached to a text editing control (they are
/// inherited) and gathered into a `TextInputOptions` value for the input
/// method.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextInputOptions {
    /// The content type (mostly for determining the shape of the virtual keyboard).
    pub content_type: TextInputContentType,

    /// Determines what the Return key says and how it behaves.
    pub return_key_type: TextInputReturnKeyType,

    /// Text is multiline.
    pub multiline: bool,

    /// Text is in lower case.
    pub lowercase: bool,

    /// Text is in upper case.
    pub uppercase: bool,

    /// Automatically capitalize letters at the start of the sentence.
    pub auto_capitalization: bool,

    /// Text contains sensitive data like card numbers and should not be stored.
    pub is_sensitive: bool,

    /// Determines whether text suggestions are shown; `None` leaves the decision to the platform.
    pub show_suggestions: Option<bool>,

    /// Hints about the locales (BCP 47 language tags) the user is expected to type in.
    pub locale_hints: Option<Rc<[String]>>,
}

crate::ferro_static_type!(TextInputOptions);

crate::ferro_properties! {
    impl TextInputOptions, also [
        TextInputOptions::content_type_property,
        TextInputOptions::return_key_type_property,
        TextInputOptions::multiline_property,
        TextInputOptions::lowercase_property,
        TextInputOptions::uppercase_property,
        TextInputOptions::auto_capitalization_property,
        TextInputOptions::is_sensitive_property,
        TextInputOptions::show_suggestions_property,
        TextInputOptions::locale_hints_property,
    ] {}
}

impl TextInputOptions {
    /// Creates the options from the values of the attached properties of a
    /// control.
    pub fn from_styled_element(ferro_object: &StyledElement) -> TextInputOptions {
        TextInputOptions {
            content_type: Self::get_content_type(ferro_object),
            return_key_type: Self::get_return_key_type(ferro_object),
            multiline: Self::get_multiline(ferro_object),
            auto_capitalization: Self::get_auto_capitalization(ferro_object),
            is_sensitive: Self::get_is_sensitive(ferro_object),
            lowercase: Self::get_lowercase(ferro_object),
            uppercase: Self::get_uppercase(ferro_object),
            show_suggestions: Self::get_show_suggestions(ferro_object),
            locale_hints: Self::get_locale_hints(ferro_object),
        }
    }

    /// The default options.
    pub fn default_options() -> TextInputOptions {
        TextInputOptions::default()
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `ContentType` attached property.
        pub fn content_type_property() -> AttachedProperty<TextInputContentType> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "ContentType",
                StyledPropertyOptions::new(TextInputContentType::Normal).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `ContentType` property on a control.
    pub fn set_content_type(ferro_object: &StyledElement, value: TextInputContentType) {
        ferro_object.set_value(Self::content_type_property(), value)
    }

    /// Gets the value of the attached `ContentType` property of a control.
    pub fn get_content_type(ferro_object: &StyledElement) -> TextInputContentType {
        ferro_object.get_value(Self::content_type_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `ReturnKeyType` attached property.
        pub fn return_key_type_property() -> AttachedProperty<TextInputReturnKeyType> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "ReturnKeyType",
                StyledPropertyOptions::new(TextInputReturnKeyType::Default).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `ReturnKeyType` property on a control.
    pub fn set_return_key_type(ferro_object: &StyledElement, value: TextInputReturnKeyType) {
        ferro_object.set_value(Self::return_key_type_property(), value)
    }

    /// Gets the value of the attached `ReturnKeyType` property of a control.
    pub fn get_return_key_type(ferro_object: &StyledElement) -> TextInputReturnKeyType {
        ferro_object.get_value(Self::return_key_type_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `Multiline` attached property.
        pub fn multiline_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "Multiline",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `Multiline` property on a control.
    pub fn set_multiline(ferro_object: &StyledElement, value: bool) {
        ferro_object.set_value(Self::multiline_property(), value)
    }

    /// Gets the value of the attached `Multiline` property of a control.
    pub fn get_multiline(ferro_object: &StyledElement) -> bool {
        ferro_object.get_value(Self::multiline_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `Lowercase` attached property.
        pub fn lowercase_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "Lowercase",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `Lowercase` property on a control.
    pub fn set_lowercase(ferro_object: &StyledElement, value: bool) {
        ferro_object.set_value(Self::lowercase_property(), value)
    }

    /// Gets the value of the attached `Lowercase` property of a control.
    pub fn get_lowercase(ferro_object: &StyledElement) -> bool {
        ferro_object.get_value(Self::lowercase_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `Uppercase` attached property.
        pub fn uppercase_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "Uppercase",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `Uppercase` property on a control.
    pub fn set_uppercase(ferro_object: &StyledElement, value: bool) {
        ferro_object.set_value(Self::uppercase_property(), value)
    }

    /// Gets the value of the attached `Uppercase` property of a control.
    pub fn get_uppercase(ferro_object: &StyledElement) -> bool {
        ferro_object.get_value(Self::uppercase_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `AutoCapitalization` attached property.
        pub fn auto_capitalization_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "AutoCapitalization",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `AutoCapitalization` property on a control.
    pub fn set_auto_capitalization(ferro_object: &StyledElement, value: bool) {
        ferro_object.set_value(Self::auto_capitalization_property(), value)
    }

    /// Gets the value of the attached `AutoCapitalization` property of a control.
    pub fn get_auto_capitalization(ferro_object: &StyledElement) -> bool {
        ferro_object.get_value(Self::auto_capitalization_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `IsSensitive` attached property.
        pub fn is_sensitive_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "IsSensitive",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `IsSensitive` property on a control.
    pub fn set_is_sensitive(ferro_object: &StyledElement, value: bool) {
        ferro_object.set_value(Self::is_sensitive_property(), value)
    }

    /// Gets the value of the attached `IsSensitive` property of a control.
    pub fn get_is_sensitive(ferro_object: &StyledElement) -> bool {
        ferro_object.get_value(Self::is_sensitive_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `ShowSuggestions` attached property.
        pub fn show_suggestions_property() -> AttachedProperty<Option<bool>> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "ShowSuggestions",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `ShowSuggestions` property on a control.
    pub fn set_show_suggestions(ferro_object: &StyledElement, value: Option<bool>) {
        ferro_object.set_value(Self::show_suggestions_property(), value)
    }

    /// Gets the value of the attached `ShowSuggestions` property of a control.
    pub fn get_show_suggestions(ferro_object: &StyledElement) -> Option<bool> {
        ferro_object.get_value(Self::show_suggestions_property())
    }

    ferro_property!(for TextInputOptions;
        /// Defines the `LocaleHints` attached property.
        pub fn locale_hints_property() -> AttachedProperty<Option<Rc<[String]>>> {
            FerroProperty::register_attached_with::<TextInputOptions, StyledElement, _>(
                "LocaleHints",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    /// Sets the value of the attached `LocaleHints` property on a control.
    pub fn set_locale_hints(ferro_object: &StyledElement, value: Option<Rc<[String]>>) {
        ferro_object.set_value(Self::locale_hints_property(), value)
    }

    /// Gets the value of the attached `LocaleHints` property of a control.
    pub fn get_locale_hints(ferro_object: &StyledElement) -> Option<Rc<[String]>> {
        ferro_object.get_value(Self::locale_hints_property())
    }
}
