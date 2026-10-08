use crate::{ColorView, ColorViewImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, ferro_property, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{ContentControl, ControlImpl};
use std::rc::Rc;

/// Presents a color for user editing using a spectrum, palette and component sliders within a drop down.
/// Editing is available when the drop down flyout is opened; otherwise, only the preview content area is shown.
#[repr(C)]
pub struct ColorPicker {
    base: ColorView,
}

ferro_class!(ColorPicker: ColorView);
ferro_impl_classes!(
    ColorPicker: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ColorViewImpl
);
ferroui_base::ferro_class_info!(ColorPicker { new: ColorPicker::new });

ferro_properties! { impl ColorPicker {
    ferro_property!(
        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            ContentControl::content_property().add_owner::<ColorPicker>()
        }
    );

    ferro_property!(
        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            ContentControl::content_template_property().add_owner::<ColorPicker>()
        }
    );

    ferro_property!(
        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<ColorPicker>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<ColorPicker>()
        }
    );
} }

impl ColorPicker {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ColorView::construct() }
    }

    /// Initializes a new instance of the [`ColorPicker`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets any content displayed in the ColorPicker's preview content area.
    ///
    /// By default this should show a preview of the currently selected color.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    /// Sets [`content`](Self::content).
    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// Gets the data template used to display the content of the ColorPicker's preview content area.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }

    /// Sets [`content_template`](Self::content_template).
    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value)
    }

    /// Gets the horizontal alignment of the content within the ColorPicker's preview content area.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    /// Sets [`horizontal_content_alignment`](Self::horizontal_content_alignment).
    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// Gets the vertical alignment of the content within the ColorPicker's preview content area.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    /// Sets [`vertical_content_alignment`](Self::vertical_content_alignment).
    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }
}
