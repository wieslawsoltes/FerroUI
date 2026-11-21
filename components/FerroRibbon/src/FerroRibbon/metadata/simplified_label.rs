use crate::controls::RibbonSimplifiedLabel;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{MarkupEnumMember, MarkupType, MarkupTypeKind, MarkupTyped};
use std::rc::Rc;

#[doc(hidden)]
pub struct RibbonSimplifiedLabelMetadata;
#[allow(non_upper_case_globals)]
impl RibbonSimplifiedLabelMetadata {
    pub const Auto: RibbonSimplifiedLabel = RibbonSimplifiedLabel::Auto;
    pub const Show: RibbonSimplifiedLabel = RibbonSimplifiedLabel::Show;
    pub const Hide: RibbonSimplifiedLabel = RibbonSimplifiedLabel::Hide;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> RibbonSimplifiedLabel {
        RibbonSimplifiedLabel::from_raw(value)
    }
}
impl MarkupTyped for RibbonSimplifiedLabelMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonSimplifiedLabel",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon.Controls");
            m.handles = &[ValueType::of::<RibbonSimplifiedLabel>];
            m.nullable = Some(ValueType::of::<Option<RibbonSimplifiedLabel>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Auto",
                    value: 0,
                    rust_variant: Some("Auto"),
                },
                MarkupEnumMember {
                    name: "Show",
                    value: 1,
                    rust_variant: Some("Show"),
                },
                MarkupEnumMember {
                    name: "Hide",
                    value: 2,
                    rust_variant: Some("Hide"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(RibbonSimplifiedLabel::from_raw(v)) as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<RibbonSimplifiedLabel>()
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
    MarkupType::register_all(&[RibbonSimplifiedLabelMetadata::MARKUP]);
    ValueTypes::register_nullable::<RibbonSimplifiedLabel>();
}
