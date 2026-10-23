//! Native values of the text pattern. The attribute identifiers and reserved
//! values follow WinUI's `EnumDefs.g.h` and `CUIATextRangeProviderWrapper` at
//! `fe244ca6ac4d482712b234a9de3cf771cc6481de` (Microsoft Corporation, MIT).
//! Only owned, portable values cross the dispatcher boundary.

use super::automation_node::{ELEMENT_NOT_AVAILABLE, ELEMENT_NOT_ENABLED, NOT_SUPPORTED};
use super::interop::{TextPatternRangeEndpoint, TextUnit, UiaCoreTypesApi};
use super::marshalling::{ComVariant, VariantValue};
use ferroui_base::media::Color;
use ferroui_controls::automation::provider::{self as portable, TextAttribute, TextAttributeValue};
use ferroui_controls::automation::ElementNotEnabledException;
use ferroui_microcom::{ComPtr, HResult, IUnknown, COR_E_INVALIDOPERATION, E_FAIL, E_INVALIDARG};

pub(crate) fn provider_error(error: portable::ProviderError) -> HResult {
    if error.downcast_ref::<ElementNotEnabledException>().is_some() {
        return ELEMENT_NOT_ENABLED;
    }
    match error.downcast_ref::<portable::TextProviderError>() {
        Some(portable::TextProviderError::ElementNotAvailable) => ELEMENT_NOT_AVAILABLE,
        Some(
            portable::TextProviderError::IncompatibleRange
            | portable::TextProviderError::InvalidArgument,
        ) => HResult(E_INVALIDARG),
        Some(portable::TextProviderError::UnsupportedAttribute) => NOT_SUPPORTED,
        Some(
            portable::TextProviderError::SelectionNotSupported
            | portable::TextProviderError::MultipleSelectionNotSupported,
        ) => HResult(COR_E_INVALIDOPERATION),
        Some(portable::TextProviderError::GeometryUnavailable) | None => HResult(E_FAIL),
    }
}

pub(crate) fn endpoint(
    value: TextPatternRangeEndpoint,
) -> Result<portable::TextPatternRangeEndpoint, HResult> {
    match value {
        TextPatternRangeEndpoint::Start => Ok(portable::TextPatternRangeEndpoint::Start),
        TextPatternRangeEndpoint::End => Ok(portable::TextPatternRangeEndpoint::End),
        _ => Err(HResult(E_INVALIDARG)),
    }
}

pub(crate) fn unit(value: TextUnit) -> Result<portable::TextUnit, HResult> {
    match value {
        TextUnit::Character => Ok(portable::TextUnit::Character),
        TextUnit::Format => Ok(portable::TextUnit::Format),
        TextUnit::Word => Ok(portable::TextUnit::Word),
        TextUnit::Line => Ok(portable::TextUnit::Line),
        TextUnit::Paragraph => Ok(portable::TextUnit::Paragraph),
        TextUnit::Page => Ok(portable::TextUnit::Page),
        TextUnit::Document => Ok(portable::TextUnit::Document),
        _ => Err(HResult(E_INVALIDARG)),
    }
}

/// The values of `AutomationTextAttributesEnum` in the pinned reference.
pub(crate) fn attribute(id: i32) -> TextAttribute {
    match id {
        40001 => TextAttribute::BackgroundColor,
        40005 => TextAttribute::FontName,
        40006 => TextAttribute::FontSize,
        40007 => TextAttribute::FontWeight,
        40008 => TextAttribute::ForegroundColor,
        40009 => TextAttribute::HorizontalTextAlignment,
        40010 => TextAttribute::IndentationFirstLine,
        40011 => TextAttribute::IndentationLeading,
        40012 => TextAttribute::IndentationTrailing,
        40013 => TextAttribute::IsHidden,
        40014 => TextAttribute::IsItalic,
        40015 => TextAttribute::IsReadOnly,
        40016 => TextAttribute::IsSubscript,
        40017 => TextAttribute::IsSuperscript,
        40018 => TextAttribute::MarginBottom,
        40019 => TextAttribute::MarginLeading,
        40020 => TextAttribute::MarginTop,
        40021 => TextAttribute::MarginTrailing,
        40023 => TextAttribute::OverlineColor,
        40024 => TextAttribute::OverlineStyle,
        40025 => TextAttribute::StrikethroughColor,
        40026 => TextAttribute::StrikethroughStyle,
        40028 => TextAttribute::TextFlowDirections,
        40029 => TextAttribute::UnderlineColor,
        40030 => TextAttribute::UnderlineStyle,
        _ => TextAttribute::Unknown(id),
    }
}

pub(crate) fn attribute_to_variant(value: TextAttributeValue) -> Result<ComVariant, HResult> {
    let value = match value {
        TextAttributeValue::Bool(value) => VariantValue::Bool(value),
        TextAttributeValue::Integer(value) => VariantValue::I4(value),
        TextAttributeValue::Number(value) => VariantValue::R8(value),
        TextAttributeValue::String(value) => VariantValue::Bstr(value),
        TextAttributeValue::Color(value) => VariantValue::I4(
            i32::from(value.r) | i32::from(value.g) << 8 | i32::from(value.b) << 16,
        ),
        TextAttributeValue::Mixed => {
            VariantValue::Unknown(UiaCoreTypesApi::reserved_mixed_attribute_value()?)
        }
        TextAttributeValue::NotSupported => {
            VariantValue::Unknown(UiaCoreTypesApi::reserved_not_supported_value()?)
        }
    };
    ComVariant::create(Some(&value)).map_err(|_| HResult(E_FAIL))
}

/// Reads a borrowed native variant. The returned value owns its text, and no
/// native interface pointer is carried to another thread.
///
/// # Safety
/// `value` contains a valid borrowed COM variant for the duration of this call.
pub(crate) unsafe fn attribute_from_variant(
    attribute: TextAttribute,
    value: &ComVariant,
) -> Result<TextAttributeValue, HResult> {
    // SAFETY: the caller supplies a borrowed, live native variant. `as_object`
    // copies strings and takes its own reference to any native unknown.
    let value = unsafe { value.as_object() }.map_err(|_| HResult(E_INVALIDARG))?;
    if let Some(VariantValue::Unknown(value)) = &value {
        let identity = value.cast::<IUnknown>()?;
        if identity == UiaCoreTypesApi::reserved_mixed_attribute_value()?.cast::<IUnknown>()? {
            return Ok(TextAttributeValue::Mixed);
        }
        if identity == UiaCoreTypesApi::reserved_not_supported_value()?.cast::<IUnknown>()? {
            return Ok(TextAttributeValue::NotSupported);
        }
        return Err(HResult(E_INVALIDARG));
    }
    let value = match (attribute, value) {
        (
            TextAttribute::ForegroundColor
            | TextAttribute::BackgroundColor
            | TextAttribute::UnderlineColor
            | TextAttribute::OverlineColor
            | TextAttribute::StrikethroughColor,
            Some(VariantValue::I4(value)),
        ) => TextAttributeValue::Color(color_from_colorref(value as u32)),
        (
            TextAttribute::ForegroundColor
            | TextAttribute::BackgroundColor
            | TextAttribute::UnderlineColor
            | TextAttribute::OverlineColor
            | TextAttribute::StrikethroughColor,
            Some(VariantValue::UI4(value)),
        ) => TextAttributeValue::Color(color_from_colorref(value)),
        (_, Some(VariantValue::Bool(value))) => TextAttributeValue::Bool(value),
        (_, Some(VariantValue::I4(value))) => TextAttributeValue::Integer(value),
        (_, Some(VariantValue::R8(value))) if value.is_finite() => {
            TextAttributeValue::Number(value)
        }
        (_, Some(VariantValue::Bstr(value))) => TextAttributeValue::String(value),
        _ => return Err(HResult(E_INVALIDARG)),
    };
    Ok(value)
}

fn color_from_colorref(value: u32) -> Color {
    Color::from_rgb(value as u8, (value >> 8) as u8, (value >> 16) as u8)
}

/// Obtains canonical COM identity without assuming the implementation behind an
/// incoming pointer. Keeping this owned reference alive also keeps the incoming
/// target alive while a synchronous dispatcher call looks it up.
pub(crate) fn identity_of<I: ferroui_microcom::Interface>(
    value: &I,
) -> Result<ComPtr<IUnknown>, HResult> {
    ComPtr::from_ref(value).cast::<IUnknown>()
}
