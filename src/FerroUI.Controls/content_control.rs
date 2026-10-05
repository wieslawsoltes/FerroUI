use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::presenters::{register_content_presenter_host, ContentPresenter, IContentPresenterHost};
use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate};
use crate::{Control, ControlImpl};
use ferroui_base::collections::FerroList;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElement, StyledElementImpl, StyledProperty,
    VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

const PC_EMPTY: &str = ":empty";

/// Displays content according to an [`IDataTemplate`].
#[repr(C)]
pub struct ContentControl {
    base: TemplatedControl,
    presenter: RefCell<Option<Ref<ContentPresenter>>>,
}

ferro_class! {
    ContentControl: TemplatedControl, virtuals ContentControlImpl: TemplatedControlImpl {
        /// Called when a content presenter whose templated parent is this
        /// control is created. Returns true if the presenter was registered
        /// as a presenter of the control.
        fn register_content_presenter(this, presenter: &ContentPresenter) -> bool;
    }
}
ferroui_base::ferro_class_info!(ContentControl { new: ContentControl::new });

ferro_impl_classes!(
    ContentControl: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for ContentControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::content_property().as_property() {
            this.content_changed(change);
        }
    }
}

impl ContentControlImpl for ContentControl {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        if presenter.name().as_deref() == Some("PART_ContentPresenter") {
            *this.presenter.borrow_mut() = Some(presenter.to_ref());
            return true;
        }

        false
    }
}

impl IContentPresenterHost for ContentControl {
    fn logical_children(&self) -> &FerroList<Ref<StyledElement>> {
        StyledElement::logical_children(self)
    }

    fn register_content_presenter(&self, presenter: &ContentPresenter) -> bool {
        ContentControl::register_content_presenter(self, presenter)
    }
}

impl ContentControl {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_ContentPresenter", <ContentPresenter as StaticType>::TYPE)];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_EMPTY]);
}

impl crate::i_content_control::IContentControl for ContentControl {
    fn content(&self) -> Option<BoxedValue> {
        ContentControl::content(self)
    }

    fn set_content(&self, value: Option<BoxedValue>) {
        ContentControl::set_content(self, value)
    }

    fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        ContentControl::content_template(self)
    }

    fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        ContentControl::set_content_template(self, value)
    }

    fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        ContentControl::horizontal_content_alignment(self)
    }

    fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        ContentControl::set_horizontal_content_alignment(self, value)
    }

    fn vertical_content_alignment(&self) -> VerticalAlignment {
        ContentControl::vertical_content_alignment(self)
    }

    fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        ContentControl::set_vertical_content_alignment(self, value)
    }
}

ferroui_base::ferro_properties! { impl ContentControl {
    ferro_property!(
        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentControl, _>("Content", None)
        }
    );

    ferro_property!(
        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ContentControl, _>("ContentTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            FerroProperty::register::<ContentControl, _>("HorizontalContentAlignment", HorizontalAlignment::Stretch)
        }
    );

    ferro_property!(
        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            FerroProperty::register::<ContentControl, _>("VerticalContentAlignment", VerticalAlignment::Stretch)
        }
    );
} }

impl ContentControl {
    fn static_constructor() {
        register_content_presenter_host::<ContentControl>();
        crate::i_content_control::register_content_control::<ContentControl>();

        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, ns| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".to_string()));

            let bind = |property: &'static FerroProperty| {
                presenter.bind_binding(property, &TemplateBinding::new(property));
            };
            bind(TemplatedControl::background_property().as_property());
            bind(TemplatedControl::background_sizing_property().as_property());
            bind(TemplatedControl::border_brush_property().as_property());
            bind(TemplatedControl::border_thickness_property().as_property());
            bind(TemplatedControl::corner_radius_property().as_property());
            bind(ContentControl::content_template_property().as_property());
            bind(ContentControl::content_property().as_property());
            bind(TemplatedControl::padding_property().as_property());
            bind(ContentControl::vertical_content_alignment_property().as_property());
            bind(ContentControl::horizontal_content_alignment_property().as_property());

            presenter.register_in_name_scope(&**ns).upcast()
        });
        TemplatedControl::template_property().override_default_value::<ContentControl>(Some(template));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), presenter: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The content to display.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// The data template used to display the content of the control.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }

    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value)
    }

    /// The presenter from the control's template.
    pub fn presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.presenter.borrow().clone()
    }

    /// The horizontal alignment of the content within the control.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// The vertical alignment of the content within the control.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    fn content_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        if let Some(old_child) = old_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).remove(&old_child);
        }

        if let Some(new_child) = new_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).add(new_child);
        }

        self.update_pseudo_classes();
    }

    fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(PC_EMPTY, self.content().is_none());
    }
}
