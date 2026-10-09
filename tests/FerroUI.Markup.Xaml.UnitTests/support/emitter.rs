//! Not a port of an upstream file: the classes the corpus of the differential
//! harness of the Rust emitter (`crate::emitter`) needs beyond the framework.
//! Generated code names them by the public Rust paths registered here.

use std::rc::{Rc, Weak};

use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{IServiceProvider, MarkupType, MarkupTyped};
use ferroui_base::styling::DuplicateSetterError;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty,
    InitializationError, ObjectType, Ref, StyledElementImpl, StyledElementImplExt, StyledProperty, TypeInfo, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_markup_xaml::converters::{ITypeDescriptorContext, TypeConverter};
use ferroui_markup_xaml::{ServiceProviderExtensions, XamlLoadException};

use super::xaml::event_tests::{MyButton, MyHost, MyPanel};
use super::TypeModule;

/// A control whose `EndInit` fails, after ending the initialisation, with the
/// error a style with two setters for its `Tag` gives: the failure of a member
/// generated code calls.
#[repr(C)]
pub struct FailingEndInit {
    base: Control,
}

ferro_class!(FailingEndInit: Control);
ferro_impl_classes!(FailingEndInit: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(FailingEndInit {
    new: FailingEndInit::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests" },
});

impl FerroObjectImpl for FailingEndInit {}

impl StyledElementImpl for FailingEndInit {
    fn try_end_init(this: &Self) -> Result<(), InitializationError> {
        Self::parent_try_end_init(this)?;
        Err(InitializationError::DuplicateSetter(DuplicateSetterError {
            property: "Tag".to_string(),
            style: "FailingEndInit".to_string(),
        }))
    }
}

impl FailingEndInit {
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

// --- The type converter of the corpus ---------------------------------------

/// What the type converter of the corpus converts a text to.
pub struct Caption {
    text: String,
}

crate::test_identity_eq!(Caption);

impl Caption {
    pub fn new(text: &str) -> Rc<Self> {
        Rc::new(Self { text: text.to_string() })
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }
}

ferro_markup_type!(class Caption {
    this: Rc<Caption>,
    handles: [Rc<Caption>, Option<Rc<Caption>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    properties: [
        Text: String { get: |this: &Rc<Caption>| this.text() },
    ],
});

/// Converts a text to a caption when the document is loaded, with what the context it is
/// given states: `/name` is the name after the base URI of the document, `@name` the name
/// after the class of the nearest parent that is a control. `none` is no caption (null),
/// `number` a value that is no caption, and a text that starts with `!` is not converted.
pub struct CaptionConverter {
    this: Weak<CaptionConverter>,
}

crate::test_identity_eq!(CaptionConverter);

impl CaptionConverter {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    fn as_type_converter(&self) -> Rc<dyn TypeConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl TypeConverter for CaptionConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let text = value
            .and_then(|value| value.downcast_ref::<String>())
            .ok_or_else(|| XamlLoadException::with_message("CaptionConverter converts a text."))?;
        let service_provider = context.map(|context| {
            let service_provider: Rc<dyn IServiceProvider> = context.clone();
            service_provider
        });
        let caption = match text.as_str() {
            "none" => return Ok(None),
            "number" => return Ok(Some(Rc::new(1i32))),
            failing if failing.starts_with('!') => return Err(XamlLoadException::with_message(format!("No caption is made of '{failing}'."))),
            rooted if rooted.starts_with('/') => {
                let base_uri = service_provider.as_ref().and_then(|service_provider| service_provider.get_context_base_uri());
                format!("{}{rooted}", base_uri.map_or("(no base URI)".to_string(), |uri| uri.to_string()))
            }
            relative if relative.starts_with('@') => {
                let parent = service_provider.as_ref().and_then(|service_provider| service_provider.get_first_parent::<Ref<Control>>());
                format!("{}{relative}", parent.map_or("(no parent)".to_string(), |parent| parent.get_type().name().to_string()))
            }
            plain => plain.to_string(),
        };
        Ok(Some(Rc::new(Caption::new(&caption))))
    }
}

ferro_markup_type!(class CaptionConverter {
    this: Rc<CaptionConverter>,
    handles: [CaptionConverter, Rc<CaptionConverter>, Option<Rc<CaptionConverter>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    base: Rc<dyn TypeConverter>,
    constructors: [() => CaptionConverter::new],
    methods: [
        fn CanConvertFrom(Option<Rc<dyn ITypeDescriptorContext>>, ValueType) -> bool =>
            |this: &Rc<CaptionConverter>, context: Option<Rc<dyn ITypeDescriptorContext>>, source_type: ValueType| {
                this.can_convert_from(context.as_ref(), source_type)
            },
        try fn ConvertFrom(Option<Rc<dyn ITypeDescriptorContext>>, Option<CultureInfo>, Option<BoxedValue>) -> Option<BoxedValue> =>
            |this: &Rc<CaptionConverter>,
             context: Option<Rc<dyn ITypeDescriptorContext>>,
             culture: Option<CultureInfo>,
             value: Option<BoxedValue>| {
                this.convert_from(context.as_ref(), culture.as_ref(), value.as_ref())
            },
    ],
});

/// A control whose registered property states its type converter with an attribute on
/// the property: a text assigned to it in markup is converted when the document is loaded.
#[repr(C)]
pub struct Captioned {
    base: Control,
}

ferro_class!(Captioned: Control);
ferro_impl_classes!(Captioned: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(Captioned {
    new: Captioned::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests",
        property_attributes: [Caption: [TypeConverter(type(Rc<CaptionConverter>))]],
    },
});

ferro_properties! {
    impl Captioned {
        pub fn caption_property() -> StyledProperty<Option<Rc<Caption>>> {
            FerroProperty::register::<Captioned, _>("Caption", None)
        }
    }
}

impl Captioned {
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn caption(&self) -> Option<Rc<Caption>> {
        self.get_value(Self::caption_property())
    }
}

// --- The typed list of the corpus --------------------------------------------

/// An item of the list of the corpus.
pub struct Row {
    name: String,
}

crate::test_identity_eq!(Row);

impl Row {
    pub fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: name.to_string() })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// A value of a Rust type no metadata declares.
    pub fn stamp(&self) -> Stamp {
        Stamp(self.name.len() as u32)
    }

    /// A value of the nullable form of a number.
    pub fn length(&self) -> Option<i32> {
        Some(self.name.len() as i32)
    }
}

/// A Rust type no metadata declares: the type of a property a compiled binding reads,
/// which generated code cannot name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp(pub u32);

ferro_markup_type!(class Row {
    this: Rc<Row>,
    handles: [Rc<Row>, Option<Rc<Row>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    properties: [
        Name: String { get: |this: &Rc<Row>| this.name() },
        Stamp: Stamp { get: |this: &Rc<Row>| this.stamp() },
        Length: Option<i32> { get: |this: &Rc<Row>| this.length() },
    ],
});

// The list of rows as a type of markup, declared with the macro the controls crate exports:
// the element type is what the data type of an item template is inferred from when the
// items of the control are bound to the list.
ferroui_controls::ferro_markup_list!(pub RowList: Rc<Row>);

/// The data of the document with the list: what its items control is bound to.
pub struct Table {
    rows: FerroList<Rc<Row>>,
}

crate::test_identity_eq!(Table);

impl Table {
    pub fn new() -> Rc<Self> {
        let rows = FerroList::new();
        rows.add(Row::new("first"));
        rows.add(Row::new("second"));
        Rc::new(Self { rows })
    }

    pub fn rows(&self) -> FerroList<Rc<Row>> {
        self.rows.clone()
    }
}

ferro_markup_type!(class Table {
    this: Rc<Table>,
    handles: [Rc<Table>, Option<Rc<Table>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    constructors: [() => Table::new],
    properties: [
        Rows: FerroList<Rc<Row>> { get: |this: &Rc<Table>| this.rows() },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[FailingEndInit::TYPE, Captioned::TYPE],
    markup_types: &[
        <Caption as MarkupTyped>::MARKUP,
        <CaptionConverter as MarkupTyped>::MARKUP,
        <Row as MarkupTyped>::MARKUP,
        <Table as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        RowList::register();
        ValueTypes::register_nullable::<Rc<Row>>();
        ValueTypes::register_nullable::<Rc<Table>>();
        ValueTypes::register_nullable::<Rc<Caption>>();
        ValueTypes::register_reference::<CaptionConverter>();
        ValueTypes::register_cast::<CaptionConverter, Rc<dyn TypeConverter>>(CaptionConverter::as_type_converter);
    },
};

/// The public Rust paths of the classes of this module, for generated code.
pub(crate) const RUST_PATHS: &[(&TypeInfo, &str)] = &[
    (FailingEndInit::TYPE, "ferroui_markup_xaml_tests::support::emitter::FailingEndInit"),
    (Captioned::TYPE, "ferroui_markup_xaml_tests::support::emitter::Captioned"),
    // The classes of the class documents of the corpus.
    (MyButton::TYPE, "ferroui_markup_xaml_tests::support::xaml::event_tests::MyButton"),
    (MyHost::TYPE, "ferroui_markup_xaml_tests::support::xaml::event_tests::MyHost"),
    (MyPanel::TYPE, "ferroui_markup_xaml_tests::support::xaml::event_tests::MyPanel"),
];

/// The public Rust path of the type whose accessor of a registered property generated
/// code calls (`Captioned::caption_property()`), by the type itself.
pub(crate) const TYPE_RUST_PATHS: &[(fn() -> std::any::TypeId, &str)] =
    &[(|| std::any::TypeId::of::<Captioned>(), "ferroui_markup_xaml_tests::support::emitter::Captioned")];

/// The public Rust paths of the types with markup metadata of this module, for generated code.
pub(crate) const MARKUP_RUST_PATHS: &[(&MarkupType, &str, bool)] = &[
    (<Caption as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::emitter::Caption", false),
    (<CaptionConverter as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::emitter::CaptionConverter", false),
    (<Row as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::emitter::Row", false),
    (<RowList as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::emitter::RowList", false),
    (<Table as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::emitter::Table", false),
];
