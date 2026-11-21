use crate::compat::Orientation;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{MarkupEnumMember, MarkupType, MarkupTypeKind, MarkupTyped};
use std::rc::Rc;

#[doc(hidden)]
pub struct OrientationMetadata;
#[allow(non_upper_case_globals)]
impl OrientationMetadata {
    pub const Vertical: Orientation = Orientation::Vertical;
    pub const Horizontal: Orientation = Orientation::Horizontal;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> Orientation {
        Orientation::from_raw(value)
    }
}
impl MarkupTyped for OrientationMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("Orientation", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon.Compatibility");
            m.handles = &[ValueType::of::<Orientation>];
            m.nullable = Some(ValueType::of::<Option<Orientation>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Vertical",
                    value: 0,
                    rust_variant: Some("Vertical"),
                },
                MarkupEnumMember {
                    name: "Horizontal",
                    value: 1,
                    rust_variant: Some("Horizontal"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v)
                    .ok()
                    .map(|v| Rc::new(Orientation::from_raw(v)) as ferroui_base::BoxedValue)
            });
            m.enum_to_value = Some(|v| v.downcast_ref::<Orientation>().map(|v| i64::from(v.raw())));
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
pub(crate) fn register() {
    MarkupType::register_all(&[OrientationMetadata::MARKUP]);
    ValueTypes::register_nullable::<Orientation>();
    ValueTypes::register_cast::<ferroui_base::layout::Orientation, Orientation>(|v| (*v).into());
}
