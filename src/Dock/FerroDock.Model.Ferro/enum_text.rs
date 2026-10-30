// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Model integration enum type-converter metadata used by native compiled bindings.
use ferroui_base::metadata::{MarkupInvokeError, MarkupType, MarkupValue};

// The native 0.3.0 binding engine already handles numbers and nullable values.
// Supply the source enum type-converter's text leg through its standard parse
// metadata, over the same enum members and original domain value handles.
// Undeclared numeric values remain the native plain-enum representation gap.
pub(crate) fn parse(
    metadata: &MarkupType,
    args: &[MarkupValue],
) -> Result<MarkupValue, MarkupInvokeError> {
    if args.len() != 1 {
        return Err(MarkupInvokeError::ArgumentCount {
            expected: 1,
            actual: args.len(),
        });
    }
    let text = args[0]
        .as_ref()
        .and_then(|value| value.downcast_ref::<String>())
        .ok_or_else(|| MarkupInvokeError::Argument {
            index: 0,
            expected: "String",
            actual: args[0]
                .as_ref()
                .map_or_else(|| "null".into(), |value| value.type_name().into()),
        })?;
    let candidate = text.trim();
    let number = candidate.parse::<i32>().ok().map(i64::from).or_else(|| {
        let mut number = 0_i64;
        for part in candidate.split(',') {
            let part = part.trim();
            number |= metadata
                .enum_members
                .iter()
                .find(|member| member.name.eq_ignore_ascii_case(part))?
                .value;
        }
        Some(number)
    });
    number
        .and_then(|number| (metadata.enum_from_value?)(number))
        .map(Some)
        .ok_or_else(|| {
            MarkupInvokeError::Failed(format!("Invalid {} value '{text}'.", metadata.name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{
        data::core::{ValueType, ValueTypes},
        metadata::MarkupTyped,
        BoxedValue,
    };
    use std::rc::Rc;
    #[test]
    fn native_type_converter_metadata_uses_original_members_and_nullable_values() {
        crate::register_types();
        for metadata in [
            crate::SplitViewDisplayModeMetadata::MARKUP,
            crate::SplitViewPanePlacementMetadata::MARKUP,
            crate::OrientationMetadata::MARKUP,
            crate::GridResizeDirectionMetadata::MARKUP,
            crate::DockModeMetadata::MARKUP,
            crate::DocumentTabLayoutMetadata::MARKUP,
            crate::AlignmentMetadata::MARKUP,
            crate::GripModeMetadata::MARKUP,
            crate::PinnedDockDisplayModeMetadata::MARKUP,
        ] {
            let target = (metadata.handles[0])();
            for member in metadata.enum_members {
                for text in [
                    member.name.to_owned(),
                    member.name.to_lowercase(),
                    format!(" {} ", member.name),
                    member.value.to_string(),
                ] {
                    let input: BoxedValue = Rc::new(text.clone());
                    let converted = ValueTypes::try_convert(Some(&input), target)
                        .unwrap()
                        .unwrap();
                    assert_eq!(
                        (metadata.enum_to_value.unwrap())(&*converted),
                        Some(member.value),
                        "{} {text}",
                        metadata.name
                    );
                    assert!(
                        ValueTypes::try_convert(Some(&input), (metadata.nullable.unwrap())())
                            .is_some()
                    );
                }
            }
            assert!(parse(metadata, &[]).is_err());
            assert!(parse(metadata, &[None]).is_err());
            assert!(parse(metadata, &[Some(Rc::new(12_i32))]).is_err());
            for invalid in ["invalid", "", "  ", "2147483648", "1,0", "Top,invalid"] {
                assert!(
                    parse(metadata, &[Some(Rc::new(invalid.to_owned()))]).is_err(),
                    "{} {invalid}",
                    metadata.name
                );
            }
        }
        let input: BoxedValue = Rc::new("1".to_owned());
        assert_eq!(
            ValueTypes::try_convert(
                Some(&input),
                ValueType::of::<ferrodock_model::core::GridResizeDirection>()
            )
            .unwrap()
            .unwrap()
            .downcast_ref::<ferrodock_model::core::GridResizeDirection>(),
            Some(&ferrodock_model::core::GridResizeDirection::Rows)
        );
    }
}
