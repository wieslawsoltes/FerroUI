use crate::utils::debug_display::append_optional_boxed_value;
use super::TemplatedControlImpl;
use crate::metadata::TemplatePartAttribute;
use crate::presenters::ContentPresenter;
use crate::templates::IDataTemplate;
use crate::{ContentControl, ContentControlImpl, ContentControlImplExt, Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElement, StyledElementImpl, StyledProperty,
    VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A [`ContentControl`] with a header.
#[repr(C)]
pub struct HeaderedContentControl {
    base: ContentControl,
    header_presenter: RefCell<Option<Ref<ContentPresenter>>>,
}

ferro_class!(HeaderedContentControl: ContentControl);
ferroui_base::ferro_class_info!(HeaderedContentControl { new: HeaderedContentControl::new });
ferro_impl_classes!(
    HeaderedContentControl: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for HeaderedContentControl {}

impl ContentControlImpl for HeaderedContentControl {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        let mut result = Self::parent_register_content_presenter(this, presenter);

        if presenter.name().as_deref() == Some("PART_HeaderPresenter") {
            *this.header_presenter.borrow_mut() = Some(presenter.to_ref());
            result = true;
        }

        result
    }
}

impl HeaderedContentControl {
    /// The named parts expected in the control template, in addition to
    /// those of the base class.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_HeaderPresenter", <ContentPresenter as StaticType>::TYPE)];
}

ferroui_base::ferro_properties! { impl HeaderedContentControl {
    ferro_property!(
        /// Defines the `Header` property.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<HeaderedContentControl, _>("Header", None)
        }
    );

    ferro_property!(
        /// Defines the `HeaderTemplate` property.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<HeaderedContentControl, _>("HeaderTemplate", None)
        }
    );
} }

impl HeaderedContentControl {
    fn static_constructor() {
        Self::header_property().changed().add_class_handler::<HeaderedContentControl>(|x, e| x.header_changed(e));
        crate::i_headered::register_headered::<HeaderedContentControl>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct(), header_presenter: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The header content.
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value)
    }

    /// The header presenter from the control's template.
    pub fn header_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.header_presenter.borrow().clone()
    }

    /// The data template used to display the header content of the control.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    fn header_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        if let Some(old_child) = old_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).remove(&old_child);
        }

        if let Some(new_child) = new_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).add(new_child);
        }
    }
}

impl crate::i_headered::IHeadered for HeaderedContentControl {
    fn header(&self) -> Option<BoxedValue> {
        HeaderedContentControl::header(self)
    }

    fn set_header(&self, value: Option<BoxedValue>) {
        HeaderedContentControl::set_header(self, value)
    }
}

impl HeaderedContentControl {
    /// Appends the text that describes the control in diagnostics: the description of the
    /// content control and, with the content, its header.
    pub(crate) fn build_debug_display(&self, builder: &mut String, include_content: bool) {
        let base: &ContentControl = self;
        base.build_debug_display(builder, include_content);

        if include_content {
            append_optional_boxed_value(builder, "Header", self.header().as_ref(), true);
        }
    }
}
