use crate::primitives::RibbonItemContentPart;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{MarkupEnumMember, MarkupType, MarkupTypeKind, MarkupTyped};
use std::rc::Rc;

#[doc(hidden)]
pub struct RibbonItemContentPartMetadata;
#[allow(non_upper_case_globals)]
impl RibbonItemContentPartMetadata {
    pub const All: RibbonItemContentPart = RibbonItemContentPart::All;
    pub const IconOnly: RibbonItemContentPart = RibbonItemContentPart::IconOnly;
    pub const LabelAndChevron: RibbonItemContentPart = RibbonItemContentPart::LabelAndChevron;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> RibbonItemContentPart {
        RibbonItemContentPart::from_raw(value)
    }
}
impl MarkupTyped for RibbonItemContentPartMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonItemContentPart",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon.Controls.Primitives");
            m.handles = &[ValueType::of::<RibbonItemContentPart>];
            m.nullable = Some(ValueType::of::<Option<RibbonItemContentPart>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "All",
                    value: 0,
                    rust_variant: Some("All"),
                },
                MarkupEnumMember {
                    name: "IconOnly",
                    value: 1,
                    rust_variant: Some("IconOnly"),
                },
                MarkupEnumMember {
                    name: "LabelAndChevron",
                    value: 2,
                    rust_variant: Some("LabelAndChevron"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(RibbonItemContentPart::from_raw(v)) as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<RibbonItemContentPart>()
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
    MarkupType::register_all(&[RibbonItemContentPartMetadata::MARKUP]);
    ValueTypes::register_nullable::<RibbonItemContentPart>();
    ValueTypes::register_display::<RibbonItemContentPart>();
}
