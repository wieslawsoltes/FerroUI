use super::ColorPickerDisplayMode;
use crate::{ColorModel, ColorSpectrumComponents, ColorView, ColorViewImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
    VisualImpl,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, ToggleButton,
};
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{ContentControl, Control, ControlImpl, TextBlock};
use std::cell::RefCell;
use std::rc::Rc;

/// The template parts of the inline presentation ([`ColorPickerDisplayMode::Inline`]) that the
/// control updates; all of them are optional, and the drop down presentation has none.
#[derive(Default)]
struct InlineParts {
    /// `PART_MoreButton`: shows and hides the text inputs when the More button is visible.
    more_button: Option<Ref<ToggleButton>>,
    /// `PART_MoreButtonLabel`: the label of the More button ("More" or "Less").
    more_button_label: Option<Ref<TextBlock>>,
    /// `PART_TextEntryGrid`: the color model, channel, alpha and hex inputs.
    text_entry_grid: Option<Ref<Control>>,
    /// `PART_Component1TextInput` to `PART_Component3TextInput`: the channel inputs, named for
    /// automation by the selected color model.
    component_inputs: [Option<Ref<Control>>; 3],
    /// `PART_ThirdDimensionSlider`: the slider of the component the spectrum does not show,
    /// named for automation by that component.
    third_dimension_slider: Option<Ref<Control>>,
    /// The handler connected to the More button.
    subscription: Option<Subscription>,
}

/// Presents a color for user editing using a spectrum, palette and component sliders within a drop down.
/// Editing is available when the drop down flyout is opened; otherwise, only the preview content area is shown.
///
/// With [`display_mode`](Self::display_mode) set to [`ColorPickerDisplayMode::Inline`] the
/// editors are shown in place instead, laid out as WinUI's `ColorPicker` (#1266).
#[repr(C)]
pub struct ColorPicker {
    base: ColorView,
    inline_parts: RefCell<InlineParts>,
}

ferro_class!(ColorPicker: ColorView);
ferro_impl_classes!(
    ColorPicker: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
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

    ferro_property!(
        /// Defines the `DisplayMode` property.
        pub fn display_mode_property() -> StyledProperty<ColorPickerDisplayMode> {
            FerroProperty::register::<ColorPicker, _>("DisplayMode", ColorPickerDisplayMode::DropDown)
        }
    );

    ferro_property!(
        /// Defines the `IsMoreButtonVisible` property.
        pub fn is_more_button_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorPicker, _>("IsMoreButtonVisible", false)
        }
    );

    ferro_property!(
        /// Defines the `IsAlphaSliderVisible` property.
        pub fn is_alpha_slider_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorPicker, _>("IsAlphaSliderVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsAlphaTextInputVisible` property.
        pub fn is_alpha_text_input_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorPicker, _>("IsAlphaTextInputVisible", true)
        }
    );
} }

impl FerroObjectImpl for ColorPicker {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::is_more_button_visible_property().as_property()
            || change.property() == ColorView::color_model_property().as_property()
            || change.property() == ColorView::color_spectrum_components_property().as_property()
        {
            this.update_inline_parts();
        }
    }
}

impl TemplatedControlImpl for ColorPicker {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let previous = std::mem::take(&mut *this.inline_parts.borrow_mut());
        if let Some(subscription) = previous.subscription {
            subscription.dispose();
        }

        let name_scope = e.name_scope();
        let more_button = name_scope.find_as::<ToggleButton>("PART_MoreButton");
        let subscription = more_button.as_ref().map(|more_button| {
            let weak = this.to_ref().downgrade();
            more_button.add_disposable_handler(
                ToggleButton::is_checked_changed_event(),
                move |_, _: &RoutedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.update_inline_parts();
                    }
                },
            )
        });
        *this.inline_parts.borrow_mut() = InlineParts {
            more_button,
            more_button_label: name_scope.find_as::<TextBlock>("PART_MoreButtonLabel"),
            text_entry_grid: name_scope.find_as::<Control>("PART_TextEntryGrid"),
            component_inputs: [
                name_scope.find_as::<Control>("PART_Component1TextInput"),
                name_scope.find_as::<Control>("PART_Component2TextInput"),
                name_scope.find_as::<Control>("PART_Component3TextInput"),
            ],
            third_dimension_slider: name_scope.find_as::<Control>("PART_ThirdDimensionSlider"),
            subscription,
        };
        this.update_inline_parts();

        Self::parent_on_apply_template(this, e);
    }
}

impl ColorPicker {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ColorView::construct(), inline_parts: RefCell::new(InlineParts::default()) }
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

    /// Gets how the editors are presented: in a drop down (the default) or in place.
    pub fn display_mode(&self) -> ColorPickerDisplayMode {
        self.get_value(Self::display_mode_property())
    }

    /// Sets [`display_mode`](Self::display_mode).
    pub fn set_display_mode(&self, value: ColorPickerDisplayMode) {
        self.set_value(Self::display_mode_property(), value)
    }

    /// Gets whether the inline presentation shows the More button, which then shows and hides
    /// the text inputs. When it is hidden (the default) the text inputs are always shown.
    pub fn is_more_button_visible(&self) -> bool {
        self.get_value(Self::is_more_button_visible_property())
    }

    /// Sets [`is_more_button_visible`](Self::is_more_button_visible).
    pub fn set_is_more_button_visible(&self, value: bool) {
        self.set_value(Self::is_more_button_visible_property(), value)
    }

    /// Gets whether the inline presentation shows the alpha slider; it is shown only when
    /// alpha is also enabled and visible.
    pub fn is_alpha_slider_visible(&self) -> bool {
        self.get_value(Self::is_alpha_slider_visible_property())
    }

    /// Sets [`is_alpha_slider_visible`](Self::is_alpha_slider_visible).
    pub fn set_is_alpha_slider_visible(&self, value: bool) {
        self.set_value(Self::is_alpha_slider_visible_property(), value)
    }

    /// Gets whether the inline presentation shows the alpha text input; it is shown only when
    /// alpha is also enabled and visible.
    pub fn is_alpha_text_input_visible(&self) -> bool {
        self.get_value(Self::is_alpha_text_input_visible_property())
    }

    /// Sets [`is_alpha_text_input_visible`](Self::is_alpha_text_input_visible).
    pub fn set_is_alpha_text_input_visible(&self, value: bool) {
        self.set_value(Self::is_alpha_text_input_visible_property(), value)
    }

    /// Updates the inline parts from the state of the control, as WinUI's `UpdateVisualState`:
    /// the text inputs are shown unless the More button is visible and not checked, the More
    /// button is labelled "More" or "Less", and the channel inputs are named for automation by
    /// the color model, and the third dimension slider by the component it edits (WinUI's
    /// names).
    fn update_inline_parts(&self) {
        let parts = self.inline_parts.borrow();
        let expanded = parts.more_button.as_ref().and_then(|button| button.is_checked()) == Some(true);
        if let Some(grid) = &parts.text_entry_grid {
            grid.set_is_visible(!self.is_more_button_visible() || expanded);
        }
        let label = if expanded { "Less" } else { "More" };
        if let Some(text_block) = &parts.more_button_label {
            text_block.set_text(Some(label));
        }
        if let Some(button) = &parts.more_button {
            AutomationProperties::set_name(button, Some(label));
        }
        let names = match self.color_model() {
            ColorModel::Rgba => ["Red", "Green", "Blue"],
            ColorModel::Hsva => ["Hue", "Saturation", "Value"],
        };
        for (input, name) in parts.component_inputs.iter().zip(names) {
            if let Some(input) = input {
                AutomationProperties::set_name(input, Some(name));
            }
        }
        if let Some(slider) = &parts.third_dimension_slider {
            let name = match self.color_spectrum_components() {
                ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => "Value (Brightness)",
                ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => "Saturation",
                ColorSpectrumComponents::SaturationValue | ColorSpectrumComponents::ValueSaturation => "Hue",
            };
            AutomationProperties::set_name(slider, Some(name));
        }
    }
}
