use crate::compat::ContentDialogButton;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{MarkupEnumMember, MarkupType, MarkupTypeKind, MarkupTyped};
use std::rc::Rc;

#[doc(hidden)]
pub struct ContentDialogButtonMetadata;
#[allow(non_upper_case_globals)]
impl ContentDialogButtonMetadata {
    pub const None: ContentDialogButton = ContentDialogButton::None;
    pub const Primary: ContentDialogButton = ContentDialogButton::Primary;
    pub const Secondary: ContentDialogButton = ContentDialogButton::Secondary;
    pub const Close: ContentDialogButton = ContentDialogButton::Close;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ContentDialogButton {
        ContentDialogButton::from_raw(value)
    }
}
impl MarkupTyped for ContentDialogButtonMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m =
                MarkupType::new("ContentDialogButton", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon.Controls");
            m.handles = &[ValueType::of::<ContentDialogButton>];
            m.nullable = Some(ValueType::of::<Option<ContentDialogButton>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "None",
                    value: 0,
                    rust_variant: Some("None"),
                },
                MarkupEnumMember {
                    name: "Primary",
                    value: 1,
                    rust_variant: Some("Primary"),
                },
                MarkupEnumMember {
                    name: "Secondary",
                    value: 2,
                    rust_variant: Some("Secondary"),
                },
                MarkupEnumMember {
                    name: "Close",
                    value: 3,
                    rust_variant: Some("Close"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v)
                    .ok()
                    .map(|v| Rc::new(ContentDialogButton::from_raw(v)) as ferroui_base::BoxedValue)
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ContentDialogButton>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
pub(crate) fn register() {
    MarkupType::register_all(&[ContentDialogButtonMetadata::MARKUP]);
    ValueTypes::register_nullable::<ContentDialogButton>();
}
