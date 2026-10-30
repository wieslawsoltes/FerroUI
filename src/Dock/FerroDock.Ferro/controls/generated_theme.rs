//! Native type/value projection shared by the two original generated content hosts.
use crate::model_projection;
use ferrodock_controls_recycling::from_recycling_value;
use ferrodock_model::core::IDockItemContainerMetadata;
use ferroui_base::{
    controls::ResourceKey,
    data::core::{ValueType, ValueTypes},
    styling::{ControlTheme, ThemeVariant},
    BoxedValue, TypeInfo,
};
use ferroui_controls::primitives::TemplatedControl;
use std::rc::Rc;
pub(super) fn apply(control: &TemplatedControl, context: Option<&BoxedValue>) {
    let theme = context
        .and_then(theme_metadata)
        .map(|theme| from_recycling_value(&theme));
    let Some(theme) = theme else {
        control.clear_value(ferroui_base::StyledElement::theme_property());
        return;
    };
    if let Some(theme) =
        ValueTypes::as_object(&*theme).and_then(|object| object.cast::<ControlTheme>())
    {
        control.set_theme(Some(theme));
        return;
    }
    if let Some(key) = resource_key(&theme) {
        if let Some(Some(value)) = control.try_find_resource(&key, None) {
            if let Some(theme) =
                ValueTypes::as_object(&*value).and_then(|object| object.cast::<ControlTheme>())
            {
                control.set_theme(Some(theme));
                return;
            }
        }
    }
    control.clear_value(ferroui_base::StyledElement::theme_property());
}
fn theme_metadata(value: &BoxedValue) -> Option<ferrodock_model::BoxedValue> {
    if let Some(metadata) = value.downcast_ref::<Rc<dyn IDockItemContainerMetadata>>() {
        return metadata.item_container_theme();
    }
    if let Some(model) = model_projection::dockable(Some(value)) {
        if let Some(metadata) = model.as_dock_item_container_metadata() {
            return metadata.item_container_theme();
        }
    }
    ValueTypes::try_cast(value, ValueType::of::<Rc<dyn IDockItemContainerMetadata>>())?
        .downcast_ref::<Rc<dyn IDockItemContainerMetadata>>()?
        .item_container_theme()
}
fn resource_key(value: &BoxedValue) -> Option<ResourceKey> {
    if let Some(key) = value.downcast_ref::<ResourceKey>() {
        return Some(key.clone());
    }
    if let Some(text) = value.downcast_ref::<String>() {
        return Some(ResourceKey::from(text.as_str()));
    }
    if let Some(text) = value.downcast_ref::<&'static str>() {
        return Some(ResourceKey::from(*text));
    }
    if let Some(kind) = value.downcast_ref::<&'static TypeInfo>() {
        return Some(ResourceKey::from(*kind));
    }
    if let Some(kind) = value.downcast_ref::<ValueType>() {
        return TypeInfo::find_by_handle(kind.id()).map(|(kind, _)| ResourceKey::from(kind));
    }
    if let Some(variant) = value.downcast_ref::<ThemeVariant>() {
        return Some(variant.key().clone());
    }
    macro_rules! scalar_key {($($kind:ty),+ $(,)?) => {$(if let Some(value)=value.downcast_ref::<$kind>() {return Some(ResourceKey::object(*value));})+};}
    scalar_key!(bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
    None
}
