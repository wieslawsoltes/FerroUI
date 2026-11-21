//! Native metadata fixture: explicit integer-backed enumeration and nullable use.
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{
    MarkupAssembly, MarkupEnumMember, MarkupType, MarkupTypeKind, MarkupTyped, XmlnsDefinition,
};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Choice(pub i32);
pub struct ChoiceMetadata;
#[allow(non_upper_case_globals)]
impl ChoiceMetadata {
    pub const None: Choice = Choice(0);
    pub const Primary: Choice = Choice(1);
    pub fn from_i32(value: i32) -> Choice {
        Choice(value)
    }
}
impl MarkupTyped for ChoiceMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("Choice", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("OpenEnums");
            m.handles = &[ValueType::of::<Choice>];
            m.nullable = Some(ValueType::of::<Option<Choice>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "None",
                    value: 0,
                    get: || Rc::new(Choice(0)),
                    rust_variant: Some("None"),
                },
                MarkupEnumMember {
                    name: "Primary",
                    value: 1,
                    get: || Rc::new(Choice(1)),
                    rust_variant: Some("Primary"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v)
                    .ok()
                    .map(|v| Rc::new(Choice(v)) as ferroui_base::BoxedValue)
            });
            m.enum_to_value = Some(|v| v.downcast_ref::<Choice>().map(|v| i64::from(v.0)));
            m.enum_i32_factory = ferroui_base::__ferro_compiler_metadata!(@value Some("from_i32"));
            m
        };
        &META
    };
}
#[derive(Default)]
pub struct EnumBag {
    pub value: Cell<Choice>,
    pub optional: Cell<Option<Choice>>,
    pub closed: Cell<Closed>,
    pub bits: Cell<Bits>,
}
impl PartialEq for EnumBag {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for EnumBag {}
impl EnumBag {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
pub struct EnumBagMetadata;
ferroui_base::ferro_markup_type!(struct EnumBagMetadata as "EnumBag" {
    namespace:"OpenEnums", this:Rc<EnumBag>, handles:[Rc<EnumBag>],
    constructors:[()=>EnumBag::new],
    properties:[Value:Choice {get:|b:&Rc<EnumBag>|b.value.get(),set:|b:&Rc<EnumBag>,v|b.value.set(v)},
      Optional:Option<Choice> {get:|b:&Rc<EnumBag>|b.optional.get(),set:|b:&Rc<EnumBag>,v|b.optional.set(v)},
      Closed:Closed {get:|b:&Rc<EnumBag>|b.closed.get(),set:|b:&Rc<EnumBag>,v|b.closed.set(v)},
      Bits:Bits {get:|b:&Rc<EnumBag>|b.bits.get(),set:|b:&Rc<EnumBag>,v|b.bits.set(v)}]
});
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "OpenEnumValues",
    crate_name: "open_enum_values",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: "urn:open-enums",
        namespace: "OpenEnums",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
pub fn register_types() {
    MarkupAssembly::register(&ASSEMBLY);
    MarkupType::register_all(&[
        ChoiceMetadata::MARKUP,
        EnumBagMetadata::MARKUP,
        Closed::MARKUP,
        BitsMetadata::MARKUP,
    ]);
    ValueTypes::register_nullable::<Choice>();
    ferroui_base::metadata::register_type_rust_paths(&[(
        || std::any::TypeId::of::<Choice>(),
        "::open_enum_values::Choice",
    )]);
    MarkupType::register_rust_paths(&[
        (
            ChoiceMetadata::MARKUP,
            "::open_enum_values::ChoiceMetadata",
            false,
        ),
        (
            EnumBagMetadata::MARKUP,
            "::open_enum_values::EnumBagMetadata",
            false,
        ),
        (Closed::MARKUP, "::open_enum_values::Closed", false),
        (
            BitsMetadata::MARKUP,
            "::open_enum_values::BitsMetadata",
            false,
        ),
    ]);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum Closed {
    #[default]
    None = 0,
    First = 1,
}
ferroui_base::ferro_markup_enum!(Closed {None,First},{namespace:"OpenEnums"});
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bits(pub i32);
pub struct BitsMetadata;
impl BitsMetadata {
    pub fn __markup_flags(value: i64) -> Bits {
        assert!(value & !3 == 0);
        Bits(value as i32)
    }
}
impl MarkupTyped for BitsMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("Bits", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("OpenEnums");
            m.handles = &[ValueType::of::<Bits>];
            m.is_flags = true;
            m.enum_members = &[
                MarkupEnumMember {
                    name: "One",
                    value: 1,
                    get: || Rc::new(Bits(1)),
                    rust_variant: None,
                },
                MarkupEnumMember {
                    name: "Two",
                    value: 2,
                    get: || Rc::new(Bits(2)),
                    rust_variant: None,
                },
            ];
            m.enum_from_value = Some(|v| {
                (v & !3 == 0).then(|| Rc::new(Bits(v as i32)) as ferroui_base::BoxedValue)
            });
            m.enum_to_value = Some(|v| v.downcast_ref::<Bits>().map(|v| i64::from(v.0)));
            // Flags must keep their composition path, even if this unrelated slot is set.
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("must_not_be_called"));
            m
        };
        &META
    };
}
