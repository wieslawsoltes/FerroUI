use crate::assigned_binding::AssignedBinding;
use crate::utils::BindingEvaluator;
use crate::{ContentControl, Control};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_property, AnyValue, AttachedProperty, BoxedValue, FerroObject, FerroProperty, Ref, StyledPropertyOptions,
};

/// Allows to customize text searching in selecting items controls.
pub struct TextSearch;

ferroui_base::ferro_static_type!(TextSearch);

ferroui_base::ferro_properties! { impl TextSearch {
    ferro_property!(
        /// Defines the `Text` attached property.
        /// This text will be considered during text search in selecting
        /// items controls (such as a combo box). This property is usually
        /// applied to an item container directly.
        pub fn text_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<TextSearch, FerroObject, _>("Text", None)
        }
    );

    ferro_property!(
        /// Defines the `TextBinding` attached property.
        /// The binding will be applied to each item during text search in
        /// selecting items controls (such as a combo box).
        pub fn text_binding_property() -> AttachedProperty<Option<AssignedBinding>> {
            FerroProperty::register_attached_with::<TextSearch, FerroObject, _>(
                "TextBinding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }
    );
} }

impl TextSearch {
    /// Sets the value of the `Text` attached property to a given element.
    pub fn set_text(element: &FerroObject, text: Option<String>) {
        element.set_value(Self::text_property(), text)
    }

    /// Gets the value of the `Text` attached property from a given element.
    pub fn get_text(element: &FerroObject) -> Option<String> {
        element.get_value(Self::text_property())
    }

    /// Sets the value of the `TextBinding` attached property to a given
    /// element.
    pub fn set_text_binding(element: &FerroObject, value: Option<AssignedBinding>) {
        element.set_value(Self::text_binding_property(), value)
    }

    /// Gets the value of the `TextBinding` attached property from a given
    /// element.
    pub fn get_text_binding(element: &FerroObject) -> Option<AssignedBinding> {
        element.get_value(Self::text_binding_property())
    }

    /// The text used for an item during text search.
    ///
    /// The effective text is, in order: the `Text` attached property of an
    /// item that is an object, the text binding evaluated against the item,
    /// the text form of the content of an item that is a content control,
    /// the text form of the item.
    pub(crate) fn get_effective_text(
        item: &Option<BoxedValue>,
        text_binding_evaluator: Option<&Ref<BindingEvaluator>>,
    ) -> String {
        let Some(value) = item else { return String::new() };

        if let Some(obj) = ValueTypes::as_object(&**value) {
            let text = obj.get_value(Self::text_property());
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                return text;
            }
        }

        if let Some(evaluator) = text_binding_evaluator {
            let text = evaluator.evaluate_string(item);
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                return text;
            }
        }

        if let Some(content_control) = Control::from_boxed(value).and_then(|c| c.cast::<ContentControl>()) {
            return match content_control.content() {
                Some(content) => Self::to_text(&*content),
                None => String::new(),
            };
        }

        Self::to_text(&**value)
    }

    /// The text form of a value.
    fn to_text(value: &dyn AnyValue) -> String {
        ValueTypes::try_to_string(value).unwrap_or_else(|| value.any_value_type_name().to_string())
    }
}
