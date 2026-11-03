//! Native attached input overrides, from Input/ScreenKeyboardInput.cs (Apache-2.0).
use ferroboard_core::editor::{CapitalizationMode, EnterAction, InputAttributes, InputKind};
use ferroui_base::{
    AttachedProperty, BoxedValue, FerroObject, FerroProperty, Visual, ferro_properties,
    ferro_static_type,
};

pub struct ScreenKeyboardInput;
ferro_static_type!(ScreenKeyboardInput);
ferro_properties! {impl ScreenKeyboardInput {
    pub fn input_kind_property()->AttachedProperty<Option<BoxedValue>> {FerroProperty::register_attached::<ScreenKeyboardInput,FerroObject,_>("InputKind",None)}
    pub fn enter_action_property()->AttachedProperty<Option<BoxedValue>> {FerroProperty::register_attached::<ScreenKeyboardInput,FerroObject,_>("EnterAction",None)}
    pub fn capitalization_property()->AttachedProperty<Option<BoxedValue>> {FerroProperty::register_attached::<ScreenKeyboardInput,FerroObject,_>("Capitalization",None)}
    pub fn suggestions_enabled_property()->AttachedProperty<bool> {FerroProperty::register_attached::<ScreenKeyboardInput,FerroObject,_>("SuggestionsEnabled",true)}
    pub fn is_private_property()->AttachedProperty<bool> {FerroProperty::register_attached::<ScreenKeyboardInput,FerroObject,_>("IsPrivate",false)}
    pub fn is_enabled_property()->AttachedProperty<bool> {FerroProperty::register_attached::<ScreenKeyboardInput,FerroObject,_>("IsEnabled",true)}
}}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputOverrideError {
    UnrepresentedEnumValue { type_name: &'static str, value: i32 },
    NullReference,
}
impl std::fmt::Display for InputOverrideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for InputOverrideError {}
impl ScreenKeyboardInput {
    pub fn get_input_kind(element: &FerroObject) -> Option<BoxedValue> {
        element.get_value(Self::input_kind_property())
    }
    pub fn set_input_kind(element: &FerroObject, value: Option<BoxedValue>) {
        element.set_value(Self::input_kind_property(), value);
    }
    pub fn get_enter_action(element: &FerroObject) -> Option<BoxedValue> {
        element.get_value(Self::enter_action_property())
    }
    pub fn set_enter_action(element: &FerroObject, value: Option<BoxedValue>) {
        element.set_value(Self::enter_action_property(), value);
    }
    pub fn get_capitalization(element: &FerroObject) -> Option<BoxedValue> {
        element.get_value(Self::capitalization_property())
    }
    pub fn set_capitalization(element: &FerroObject, value: Option<BoxedValue>) {
        element.set_value(Self::capitalization_property(), value);
    }
    pub fn get_suggestions_enabled(element: &FerroObject) -> bool {
        element.get_value(Self::suggestions_enabled_property())
    }
    pub fn set_suggestions_enabled(element: &FerroObject, value: bool) {
        element.set_value(Self::suggestions_enabled_property(), value);
    }
    pub fn get_is_private(element: &FerroObject) -> bool {
        element.get_value(Self::is_private_property())
    }
    pub fn set_is_private(element: &FerroObject, value: bool) {
        element.set_value(Self::is_private_property(), value);
    }
    pub fn get_is_enabled(element: &FerroObject) -> bool {
        element.get_value(Self::is_enabled_property())
    }
    pub fn set_is_enabled(element: &FerroObject, value: bool) {
        element.set_value(Self::is_enabled_property(), value);
    }
    // Explicit runtime-null owner bridges for the original static Get/Set methods.
    pub fn get_input_kind_nullable(
        element: Option<&FerroObject>,
    ) -> Result<Option<BoxedValue>, InputOverrideError> {
        Ok(Self::get_input_kind(
            element.ok_or(InputOverrideError::NullReference)?,
        ))
    }
    pub fn set_input_kind_nullable(
        element: Option<&FerroObject>,
        value: Option<BoxedValue>,
    ) -> Result<(), InputOverrideError> {
        Self::set_input_kind(element.ok_or(InputOverrideError::NullReference)?, value);
        Ok(())
    }
    pub fn get_enter_action_nullable(
        element: Option<&FerroObject>,
    ) -> Result<Option<BoxedValue>, InputOverrideError> {
        Ok(Self::get_enter_action(
            element.ok_or(InputOverrideError::NullReference)?,
        ))
    }
    pub fn set_enter_action_nullable(
        element: Option<&FerroObject>,
        value: Option<BoxedValue>,
    ) -> Result<(), InputOverrideError> {
        Self::set_enter_action(element.ok_or(InputOverrideError::NullReference)?, value);
        Ok(())
    }
    pub fn get_capitalization_nullable(
        element: Option<&FerroObject>,
    ) -> Result<Option<BoxedValue>, InputOverrideError> {
        Ok(Self::get_capitalization(
            element.ok_or(InputOverrideError::NullReference)?,
        ))
    }
    pub fn set_capitalization_nullable(
        element: Option<&FerroObject>,
        value: Option<BoxedValue>,
    ) -> Result<(), InputOverrideError> {
        Self::set_capitalization(element.ok_or(InputOverrideError::NullReference)?, value);
        Ok(())
    }
    pub fn get_suggestions_enabled_nullable(
        element: Option<&FerroObject>,
    ) -> Result<bool, InputOverrideError> {
        Ok(Self::get_suggestions_enabled(
            element.ok_or(InputOverrideError::NullReference)?,
        ))
    }
    pub fn set_suggestions_enabled_nullable(
        element: Option<&FerroObject>,
        value: bool,
    ) -> Result<(), InputOverrideError> {
        Self::set_suggestions_enabled(element.ok_or(InputOverrideError::NullReference)?, value);
        Ok(())
    }
    pub fn get_is_private_nullable(
        element: Option<&FerroObject>,
    ) -> Result<bool, InputOverrideError> {
        Ok(Self::get_is_private(
            element.ok_or(InputOverrideError::NullReference)?,
        ))
    }
    pub fn set_is_private_nullable(
        element: Option<&FerroObject>,
        value: bool,
    ) -> Result<(), InputOverrideError> {
        Self::set_is_private(element.ok_or(InputOverrideError::NullReference)?, value);
        Ok(())
    }
    pub fn get_is_enabled_nullable(
        element: Option<&FerroObject>,
    ) -> Result<bool, InputOverrideError> {
        Ok(Self::get_is_enabled(
            element.ok_or(InputOverrideError::NullReference)?,
        ))
    }
    pub fn set_is_enabled_nullable(
        element: Option<&FerroObject>,
        value: bool,
    ) -> Result<(), InputOverrideError> {
        Self::set_is_enabled(element.ok_or(InputOverrideError::NullReference)?, value);
        Ok(())
    }
    pub fn apply(
        element: &Visual,
        attributes: impl Into<std::sync::Arc<InputAttributes>>,
    ) -> Result<std::sync::Arc<InputAttributes>, InputOverrideError> {
        Self::apply_nullable(Some(element), Some(attributes.into()))?
            .ok_or(InputOverrideError::NullReference)
    }
    /// Explicit CLR null bridge. Null is retained until the source first dereferences it.
    pub fn apply_nullable(
        element: Option<&Visual>,
        mut attributes: Option<std::sync::Arc<InputAttributes>>,
    ) -> Result<Option<std::sync::Arc<InputAttributes>>, InputOverrideError> {
        let mut current = element.map(Visual::to_ref);
        for _ in 0..6 {
            let Some(value) = current else {
                break;
            };
            if let Some(kind) = parse_enum(Self::get_input_kind(&value), INPUT_KINDS, InputKind)? {
                attributes = Some(
                    attributes
                        .as_ref()
                        .ok_or(InputOverrideError::NullReference)?
                        .with_values(|v| v.kind = kind),
                );
                break;
            }
            current = value.visual_parent();
        }
        let element = element.ok_or(InputOverrideError::NullReference)?;
        if let Some(action) =
            parse_enum(Self::get_enter_action(element), ENTER_ACTIONS, EnterAction)?
        {
            attributes = Some(
                attributes
                    .as_ref()
                    .ok_or(InputOverrideError::NullReference)?
                    .with_values(|v| v.enter_action = action),
            );
        }
        if let Some(cap) = parse_enum(
            Self::get_capitalization(element),
            CAPITALIZATIONS,
            CapitalizationMode,
        )? {
            attributes = Some(
                attributes
                    .as_ref()
                    .ok_or(InputOverrideError::NullReference)?
                    .with_values(|v| v.capitalization = cap),
            );
        }
        if !Self::get_suggestions_enabled(element) {
            attributes = Some(
                attributes
                    .as_ref()
                    .ok_or(InputOverrideError::NullReference)?
                    .with_values(|v| {
                        v.allow_suggestions = false;
                        v.allow_auto_correct = false;
                    }),
            );
        }
        if Self::get_is_private(element) {
            attributes = Some(
                attributes
                    .as_ref()
                    .ok_or(InputOverrideError::NullReference)?
                    .with_values(|v| v.is_private = true),
            );
        }
        Ok(attributes)
    }
    pub fn is_enabled_for(element: &Visual) -> bool {
        Self::is_enabled_for_nullable(Some(element))
    }
    pub fn is_enabled_for_nullable(element: Option<&Visual>) -> bool {
        let mut current = element.map(Visual::to_ref);
        for _ in 0..8 {
            let Some(value) = current else {
                break;
            };
            if !Self::get_is_enabled(&value) {
                return false;
            }
            current = value.visual_parent();
        }
        true
    }
}
const INPUT_KINDS: &[(&str, InputKind)] = &[
    ("Text", InputKind::Text),
    ("Number", InputKind::Number),
    ("Decimal", InputKind::Decimal),
    ("Phone", InputKind::Phone),
    ("DateTime", InputKind::DateTime),
    ("Email", InputKind::Email),
    ("Uri", InputKind::Uri),
    ("Password", InputKind::Password),
    ("NumericPassword", InputKind::NumericPassword),
    ("Search", InputKind::Search),
    ("Chat", InputKind::Chat),
];
const ENTER_ACTIONS: &[(&str, EnterAction)] = &[
    ("Default", EnterAction::Default),
    ("NewLine", EnterAction::NewLine),
    ("Go", EnterAction::Go),
    ("Search", EnterAction::Search),
    ("Send", EnterAction::Send),
    ("Next", EnterAction::Next),
    ("Previous", EnterAction::Previous),
    ("Done", EnterAction::Done),
];
const CAPITALIZATIONS: &[(&str, CapitalizationMode)] = &[
    ("None", CapitalizationMode::None),
    ("Characters", CapitalizationMode::Characters),
    ("Words", CapitalizationMode::Words),
    ("Sentences", CapitalizationMode::Sentences),
];
fn parse_enum<T: Copy + 'static>(
    value: Option<BoxedValue>,
    variants: &[(&str, T)],
    from_raw: fn(i32) -> T,
) -> Result<Option<T>, InputOverrideError> {
    let Some(value) = value else { return Ok(None) };
    if let Some(value) = value.downcast_ref::<T>() {
        return Ok(Some(*value));
    }
    // Utf16Text is the explicit native bridge for CLR strings, including unpaired units.
    let text = if let Some(text) = value.downcast_ref::<ferroboard_core::text::Utf16Text>() {
        text.clone()
    } else if let Some(text) = value.downcast_ref::<String>() {
        text.as_str().into()
    } else {
        return Ok(None);
    };
    let names: Vec<_> = variants.iter().map(|(name, _)| *name).collect();
    Ok(crate::enum_bridge::parse_enum(Some(&text), &names).map(from_raw))
}
