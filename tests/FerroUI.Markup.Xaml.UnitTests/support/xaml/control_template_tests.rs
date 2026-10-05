//! The test types declared by the upstream test file `Xaml/ControlTemplateTests.cs`.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::DashStyle;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::templates::{IControlTemplate, ITemplateWithParam, TemplateResult};
use ferroui_controls::{Border, ContentControl, ContentControlImpl, Control, ControlImpl, Panel, PanelImpl};
use ferroui_markup_xaml::templates::TemplateContent;

use crate::support::TypeModule;

// --- ListBoxHierarchyLine ----------------------------------------------------

/// A panel with a styled property whose value is a dash style.
#[repr(C)]
pub struct ListBoxHierarchyLine {
    base: Panel,
}

ferro_class!(ListBoxHierarchyLine: Panel);
ferro_impl_classes!(
    ListBoxHierarchyLine: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);
ferro_class_info!(ListBoxHierarchyLine {
    new: ListBoxHierarchyLine::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

ferro_properties! {
    impl ListBoxHierarchyLine {
        pub fn line_dash_style_property() -> StyledProperty<Option<Ref<DashStyle>>> {
            FerroProperty::register::<ListBoxHierarchyLine, _>("LineDashStyle", None)
        }
    }
}

impl ListBoxHierarchyLine {
    pub fn construct() -> Self {
        Self { base: Panel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn line_dash_style(&self) -> Option<Ref<DashStyle>> {
        self.get_value(Self::line_dash_style_property())
    }

    pub fn set_line_dash_style(&self, value: Option<Ref<DashStyle>>) {
        self.set_value(Self::line_dash_style_property(), value)
    }
}

// --- CustomControlWithParts --------------------------------------------------

/// A content control that declares the parts its template must have.
#[repr(C)]
pub struct CustomControlWithParts {
    base: ContentControl,
}

ferro_class!(CustomControlWithParts: ContentControl);
ferro_impl_classes!(
    CustomControlWithParts: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(CustomControlWithParts {
    new: CustomControlWithParts::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
        attributes: [
            TemplatePart("PART_MainContentBorder", type(Ref<Border>)),
            TemplatePart("PART_ContentPresenter", type(Ref<ContentPresenter>)),
        ],
    },
});

impl CustomControlWithParts {
    pub fn construct() -> Self {
        Self { base: ContentControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

// --- CustomButtonWithParts ---------------------------------------------------

/// A control that inherits the declared parts of its base class.
#[repr(C)]
pub struct CustomButtonWithParts {
    base: CustomControlWithParts,
}

ferro_class!(CustomButtonWithParts: CustomControlWithParts);
ferro_impl_classes!(
    CustomButtonWithParts: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(CustomButtonWithParts {
    new: CustomButtonWithParts::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

impl CustomButtonWithParts {
    pub fn construct() -> Self {
        Self { base: CustomControlWithParts::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

// --- CustomControlTemplate ---------------------------------------------------

/// A control template that is not the one of the markup library: a plain
/// class that implements the control template contract.
pub struct CustomControlTemplate {
    this: Weak<CustomControlTemplate>,
    content: RefCell<Option<BoxedValue>>,
    target_type: Cell<Option<&'static TypeInfo>>,
}

crate::test_identity_eq!(CustomControlTemplate);

impl CustomControlTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), content: RefCell::new(None), target_type: Cell::new(None) })
    }

    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    pub fn target_type(&self) -> Option<&'static TypeInfo> {
        self.target_type.get()
    }

    pub fn set_target_type(&self, value: Option<&'static TypeInfo>) {
        self.target_type.set(value);
    }

    pub fn build(&self, _control: &Ref<TemplatedControl>) -> Option<TemplateResult<Ref<Control>>> {
        TemplateContent::load(self.content().as_ref())
    }

    fn as_control_template(&self) -> Rc<dyn IControlTemplate> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl ITemplateWithParam<Ref<TemplatedControl>, Option<TemplateResult<Ref<Control>>>> for CustomControlTemplate {
    fn build(&self, param: &Ref<TemplatedControl>) -> Option<TemplateResult<Ref<Control>>> {
        CustomControlTemplate::build(self, param)
    }
}

impl IControlTemplate for CustomControlTemplate {}

ferro_markup_type!(class CustomControlTemplate {
    this: Rc<CustomControlTemplate>,
    handles: [CustomControlTemplate, Rc<CustomControlTemplate>, Option<Rc<CustomControlTemplate>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    interfaces: [Rc<dyn IControlTemplate>],
    constructors: [() => CustomControlTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: |this: &Rc<CustomControlTemplate>| this.content(),
            set: |this: &Rc<CustomControlTemplate>, value: Option<BoxedValue>| this.set_content(value)
        } [TemplateContent],
        TargetType: Option<&'static TypeInfo> {
            get: |this: &Rc<CustomControlTemplate>| this.target_type(),
            set: |this: &Rc<CustomControlTemplate>, value: Option<&'static TypeInfo>| this.set_target_type(value)
        },
    ],
    methods: [
        fn Build(Ref<TemplatedControl>) -> Option<Ref<Control>> =>
            |this: &Rc<CustomControlTemplate>, control: Ref<TemplatedControl>| {
                this.build(&control).map(|result| result.result().clone())
            },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[ListBoxHierarchyLine::TYPE, CustomControlWithParts::TYPE, CustomButtonWithParts::TYPE],
    markup_types: &[<CustomControlTemplate as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<CustomControlTemplate>();
        ValueTypes::register_cast::<CustomControlTemplate, Rc<dyn IControlTemplate>>(
            CustomControlTemplate::as_control_template,
        );
    },
};
