use crate::color_palettes::IColorPalette;
use crate::converters::ColorToHexConverter;
use crate::ColorChangedEventArgs;
use ferroui_base::collections::FerroList;
use ferroui_base::input::{FocusChangedEventArgs, InputElement, InputElementImpl, Key, KeyEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Color, HsvColor};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, VisualImpl,
};
use ferroui_controls::metadata::TemplatePartAttribute;
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{ControlImpl, TextBox};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Presents a color for user editing using a spectrum, palette and component sliders.
#[repr(C)]
pub struct ColorView {
    base: TemplatedControl,
    color_changed: HandlerList<dyn Fn(&ColorChangedEventArgs)>,

    // XAML template parts
    hex_text_box: RefCell<Option<Ref<TextBox>>>,
    /// The handlers connected to the hex text box (the original removes its
    /// handlers one by one with `-=`).
    hex_text_box_disposables: RefCell<Vec<Rc<dyn IDisposable>>>,

    ignore_property_changed: Cell<bool>,
}

ferro_class! {
    ColorView: TemplatedControl, virtuals ColorViewImpl: TemplatedControlImpl {
        /// Obsolete. No-op. This method is no longer used and will be removed in a future
        /// release (the necessary validation is now handled by the tab control).
        ///
        /// This method does nothing and should not be overridden or relied upon.
        fn validate_selection(this);

        /// Raises the color changed event.
        ///
        /// `e` defines the old/new colors.
        fn on_color_changed(this, e: &ColorChangedEventArgs);

        /// Called when the `Color` property has to be coerced; returns the
        /// coerced value.
        fn on_coerce_color(this, value: Color) -> Color;

        /// Called when the `HsvColor` property has to be coerced; returns the
        /// coerced value.
        fn on_coerce_hsv_color(this, value: HsvColor) -> HsvColor;
    }
}
ferro_impl_classes!(ColorView: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferroui_base::ferro_class_info!(ColorView { new: ColorView::new });

impl FerroObjectImpl for ColorView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if this.ignore_property_changed.get() {
            Self::parent_on_property_changed(this, change);
            return;
        }

        // Always keep the two color properties in sync
        if change.property() == Self::color_property().as_property() {
            this.ignore_property_changed.set(true);

            this.set_current_value(Self::hsv_color_property(), this.color().to_hsv());
            this.set_color_to_hex_text_box();

            this.on_color_changed(&ColorChangedEventArgs::new(
                change.get_old_value::<Color>().unwrap_or_default(),
                change.get_new_value::<Color>(),
            ));

            this.ignore_property_changed.set(false);
        } else if change.property() == Self::hsv_color_property().as_property() {
            this.ignore_property_changed.set(true);

            this.set_current_value(Self::color_property(), this.hsv_color().to_rgb());
            this.set_color_to_hex_text_box();

            this.on_color_changed(&ColorChangedEventArgs::new(
                change.get_old_value::<HsvColor>().unwrap_or_default().to_rgb(),
                change.get_new_value::<HsvColor>().to_rgb(),
            ));

            this.ignore_property_changed.set(false);
        } else if change.property() == Self::palette_property().as_property() {
            let palette: Option<Rc<dyn IColorPalette>> = this.palette();

            // Any custom palette change must be automatically synced with the
            // bound properties controlling the palette grid
            if let Some(palette) = palette {
                this.set_current_value(Self::palette_column_count_property(), palette.color_count());

                let new_palette_colors = FerroList::new();
                for shade_index in 0..palette.shade_count() {
                    for color_index in 0..palette.color_count() {
                        new_palette_colors.add(palette.get_color(color_index, shade_index));
                    }
                }

                this.set_current_value(Self::palette_colors_property(), Some(new_palette_colors));
            }
        } else if change.property() == Self::is_alpha_enabled_property().as_property() {
            // Manually coerce the HsvColor value
            // (Color will be coerced automatically if HsvColor changes)
            this.set_current_value(Self::hsv_color_property(), this.on_coerce_hsv_color(this.hsv_color()));
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl TemplatedControlImpl for ColorView {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let disposables = std::mem::take(&mut *this.hex_text_box_disposables.borrow_mut());
        for disposable in disposables {
            disposable.dispose();
        }

        let hex_text_box = e.name_scope().find_as::<TextBox>("PART_HexTextBox");
        *this.hex_text_box.borrow_mut() = hex_text_box.clone();

        this.set_color_to_hex_text_box();

        if let Some(hex_text_box) = hex_text_box {
            let weak = this.to_ref().downgrade();
            let key_down = hex_text_box.add_disposable_handler(
                InputElement::key_down_event(),
                move |_, e: &KeyEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.hex_text_box_key_down(e);
                    }
                },
            );
            let weak = this.to_ref().downgrade();
            let lost_focus = hex_text_box.add_disposable_handler(
                InputElement::lost_focus_event(),
                move |_, e: &FocusChangedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.hex_text_box_lost_focus(e);
                    }
                },
            );
            *this.hex_text_box_disposables.borrow_mut() = vec![key_down, lost_focus];
        }

        Self::parent_on_apply_template(this, e);
    }
}

impl ColorViewImpl for ColorView {
    fn validate_selection(_this: &Self) {
        // Obsolete: no-op. Will be removed in a future release.
    }

    fn on_color_changed(this: &Self, e: &ColorChangedEventArgs) {
        for (_, handler) in this.color_changed.snapshot().iter() {
            handler(e);
        }
    }

    fn on_coerce_color(this: &Self, value: Color) -> Color {
        if !this.is_alpha_enabled() {
            return Color::new(255, value.r, value.g, value.b);
        }

        value
    }

    fn on_coerce_hsv_color(this: &Self, value: HsvColor) -> HsvColor {
        if !this.is_alpha_enabled() {
            return HsvColor::new(1.0, value.h, value.s, value.v);
        }

        value
    }
}

impl ColorView {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_HexTextBox", <TextBox as StaticType>::TYPE)];

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            color_changed: HandlerList::new(),
            hex_text_box: RefCell::new(None),
            hex_text_box_disposables: RefCell::new(Vec::new()),
            ignore_property_changed: Cell::new(false),
        }
    }

    /// Initializes a new instance of the [`ColorView`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Event for when the selected color changes within the slider.
    /// Disposing the returned handle unsubscribes.
    pub fn color_changed(&self, handler: impl Fn(&ColorChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.color_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.color_changed.remove(token);
            }
        })
    }

    /// Whether property changes are ignored while the view updates its own
    /// properties (a protected field of the original).
    pub fn ignore_property_changed(&self) -> bool {
        self.ignore_property_changed.get()
    }

    /// Sets [`ignore_property_changed`](Self::ignore_property_changed).
    pub fn set_ignore_property_changed(&self, value: bool) {
        self.ignore_property_changed.set(value);
    }

    /// Gets the value of the hex TextBox and sets it as the current [`color`](Self::color).
    /// If invalid, the TextBox hex text will revert back to the last valid color.
    fn get_color_from_hex_text_box(&self) {
        let hex_text_box = self.hex_text_box.borrow().clone();
        if let Some(hex_text_box) = hex_text_box {
            let converted_color = ColorToHexConverter::parse_hex_string(
                &hex_text_box.text().unwrap_or_default(),
                self.hex_input_alpha_position(),
            );

            if let Some(color) = converted_color {
                self.set_current_value(Self::color_property(), color);
            }

            // Re-apply the hex value
            // This ensure the hex color value is always valid and formatted correctly
            self.set_color_to_hex_text_box();
        }
    }

    /// Sets the current [`color`](Self::color) to the hex TextBox.
    fn set_color_to_hex_text_box(&self) {
        let hex_text_box = self.hex_text_box.borrow().clone();
        if let Some(hex_text_box) = hex_text_box {
            let text = ColorToHexConverter::to_hex_string(
                self.color(),
                self.hex_input_alpha_position(),
                self.is_alpha_enabled() && self.is_alpha_visible(),
                false,
            );
            hex_text_box.set_text(Some(&text));
        }
    }

    /// Coerces/validates the `Color` property value.
    pub(super) fn coerce_color(instance: &FerroObject, value: Color) -> Color {
        if let Some(color_view) = instance.to_ref().cast::<ColorView>() {
            return color_view.on_coerce_color(value);
        }

        value
    }

    /// Coerces/validates the `HsvColor` property value.
    pub(super) fn coerce_hsv_color(instance: &FerroObject, value: HsvColor) -> HsvColor {
        if let Some(color_view) = instance.to_ref().cast::<ColorView>() {
            return color_view.on_coerce_hsv_color(value);
        }

        value
    }

    /// Event handler for when a key is pressed within the Hex RGB value TextBox.
    /// This is used to trigger re-evaluation of the color based on the TextBox value.
    fn hex_text_box_key_down(&self, e: &KeyEventArgs) {
        if e.key == Key::Enter {
            self.get_color_from_hex_text_box();
        }
    }

    /// Event handler for when the Hex RGB value TextBox looses focus.
    /// This is used to trigger re-evaluation of the color based on the TextBox value.
    fn hex_text_box_lost_focus(&self, _e: &FocusChangedEventArgs) {
        self.get_color_from_hex_text_box();
    }
}
