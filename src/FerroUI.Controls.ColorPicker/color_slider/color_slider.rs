use crate::helpers::color_picker_helpers::ColorPickerHelpers;
use crate::helpers::round_digits;
use crate::primitives::ColorHelper;
use crate::{ColorChangedEventArgs, ColorComponent, ColorModel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, LayoutableImpl};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, HsvColor, IBrush, IImageBrushSource, ImageBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{HandlerList, MathUtilities};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs,
    Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::metadata::PseudoClassesAttribute;
use ferroui_controls::primitives::{RangeBase, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, Slider, SliderImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Defines the maximum hue component value
/// (other components are always 0..100 or 0.255).
///
/// This should match the default [`ColorSpectrum::max_hue`](crate::primitives::ColorSpectrum::max_hue) property.
const MAX_HUE: f64 = 359.0;

/// `Convert.ToInt32(double)` of the .NET base library: rounds half to even.
fn to_int32(value: f64) -> i32 {
    value.round_ties_even() as i32
}

/// A slider with a background that represents a single color component.
#[repr(C)]
pub struct ColorSlider {
    base: Slider,
    color_changed: HandlerList<dyn Fn(&ColorChangedEventArgs)>,
    ignore_property_changed: Cell<bool>,
    background_bitmap: RefCell<Option<Rc<Bitmap>>>,
}

ferro_class! {
    ColorSlider: Slider, virtuals ColorSliderImpl: SliderImpl {
        /// Raises the color changed event.
        ///
        /// `e` defines the old/new colors.
        fn on_color_changed(this, e: &ColorChangedEventArgs);
    }
}
ferro_impl_classes!(
    ColorSlider: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SliderImpl
);
ferroui_base::ferro_class_info!(ColorSlider { new: ColorSlider::new });

impl FerroObjectImpl for ColorSlider {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if this.ignore_property_changed.get() {
            Self::parent_on_property_changed(this, change);
            return;
        }

        if change.property() == Self::color_property().as_property() {
            this.ignore_property_changed.set(true);

            // Always keep the two color properties in sync
            this.set_current_value(Self::hsv_color_property(), this.color().to_hsv());

            this.set_color_to_slider_values();
            this.update_background();
            this.update_pseudo_classes();

            this.on_color_changed(&ColorChangedEventArgs::new(
                change.get_old_value::<Color>().unwrap_or_default(),
                change.get_new_value::<Color>(),
            ));

            this.ignore_property_changed.set(false);
        } else if change.property() == Self::color_component_property().as_property()
            || change.property() == Self::color_model_property().as_property()
            || change.property() == Self::is_alpha_visible_property().as_property()
            || change.property() == Self::is_perceptive_property().as_property()
        {
            this.ignore_property_changed.set(true);

            this.set_color_to_slider_values();
            this.update_background();
            this.update_pseudo_classes();

            this.ignore_property_changed.set(false);
        } else if change.property() == Self::hsv_color_property().as_property() {
            this.ignore_property_changed.set(true);

            // Always keep the two color properties in sync
            this.set_current_value(Self::color_property(), this.hsv_color().to_rgb());

            this.set_color_to_slider_values();
            this.update_background();
            this.update_pseudo_classes();

            this.on_color_changed(&ColorChangedEventArgs::new(
                change.get_old_value::<HsvColor>().unwrap_or_default().to_rgb(),
                change.get_new_value::<HsvColor>().to_rgb(),
            ));

            this.ignore_property_changed.set(false);
        } else if change.property() == Self::is_rounding_enabled_property().as_property() {
            this.set_color_to_slider_values();
        } else if change.property() == Visual::bounds_property().as_property() {
            // If the control's overall dimensions have changed the background bitmap size also needs to change.
            // This means the existing bitmap must be released to be recreated correctly in UpdateBackground().
            if let Some(background_bitmap) = this.background_bitmap.borrow_mut().take() {
                background_bitmap.dispose();
            }

            this.update_background();
            this.update_pseudo_classes();
        } else if change.property() == RangeBase::value_property().as_property()
            || change.property() == RangeBase::minimum_property().as_property()
            || change.property() == RangeBase::maximum_property().as_property()
        {
            this.ignore_property_changed.set(true);

            let old_color = this.color();
            let (color, hsv_color) = this.get_color_from_slider_values();

            if this.color_model() == ColorModel::Hsva {
                this.set_current_value(Self::hsv_color_property(), hsv_color);
                this.set_current_value(Self::color_property(), hsv_color.to_rgb());
            } else {
                this.set_current_value(Self::color_property(), color);
                this.set_current_value(Self::hsv_color_property(), color.to_hsv());
            }

            this.update_pseudo_classes();
            this.on_color_changed(&ColorChangedEventArgs::new(old_color, this.color()));

            this.ignore_property_changed.set(false);
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl ColorSliderImpl for ColorSlider {
    fn on_color_changed(this: &Self, e: &ColorChangedEventArgs) {
        for (_, handler) in this.color_changed.snapshot().iter() {
            handler(e);
        }
    }
}

impl ColorSlider {
    /// The pseudo-class of a dark selector thumb.
    pub const PC_DARK_SELECTOR: &'static str = ":dark-selector";
    /// The pseudo-class of a light selector thumb.
    pub const PC_LIGHT_SELECTOR: &'static str = ":light-selector";

    /// The pseudo-classes set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[Self::PC_DARK_SELECTOR, Self::PC_LIGHT_SELECTOR]);

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Slider::construct(),
            color_changed: HandlerList::new(),
            ignore_property_changed: Cell::new(false),
            background_bitmap: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the [`ColorSlider`] class.
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

    /// Whether property changes are ignored while the slider updates its own
    /// properties (a protected field of the original).
    pub fn ignore_property_changed(&self) -> bool {
        self.ignore_property_changed.get()
    }

    /// Sets [`ignore_property_changed`](Self::ignore_property_changed).
    pub fn set_ignore_property_changed(&self, value: bool) {
        self.ignore_property_changed.set(value);
    }

    /// Updates the visual state of the control by applying latest PseudoClasses.
    fn update_pseudo_classes(&self) {
        // The slider itself can be transparent for certain color values.
        // This causes an issue where a white selector thumb over a light window background or
        // a black selector thumb over a dark window background is not visible.
        // This means under a certain alpha threshold, neither a white or black selector thumb
        // should be shown and instead the default slider thumb color should be used instead.
        if self.color().a < 128 && (self.is_alpha_visible() || self.color_component() == ColorComponent::Alpha) {
            self.pseudo_classes().set(Self::PC_DARK_SELECTOR, false);
            self.pseudo_classes().set(Self::PC_LIGHT_SELECTOR, false);
        } else {
            let perceived_color = if self.color_model() == ColorModel::Hsva {
                self.get_perceptive_background_hsv_color(self.hsv_color()).to_rgb()
            } else {
                self.get_perceptive_background_color(self.color())
            };

            if ColorHelper::get_relative_luminance(perceived_color) <= 0.5 {
                self.pseudo_classes().set(Self::PC_DARK_SELECTOR, false);
                self.pseudo_classes().set(Self::PC_LIGHT_SELECTOR, true);
            } else {
                self.pseudo_classes().set(Self::PC_DARK_SELECTOR, true);
                self.pseudo_classes().set(Self::PC_LIGHT_SELECTOR, false);
            }
        }
    }

    /// Generates a new background image for the color slider and applies it.
    ///
    /// C# `private async void UpdateBackground()`: runs up to its first
    /// await before this returns.
    fn update_background(&self) {
        let this = self.to_ref();
        Dispatcher::current_dispatcher().to_task_scheduler().start_local(async move {
            Self::update_background_async(this).await;
        });
    }

    async fn update_background_async(this: Ref<Self>) {
        // In FerroUI, Bounds returns the actual device-independent pixel size of a control.
        // However, this is not necessarily the size of the control rendered on a display.
        // A desktop or application scaling factor may be applied which must be accounted for here.
        // Remember bitmaps in FerroUI are rendered mapping to actual device pixels, not the device-
        // independent pixels of controls.

        let scale = LayoutHelper::get_layout_scale(&this);
        let pixel_width: i32;
        let pixel_height: i32;

        if let Some(track) = this.track() {
            pixel_width = to_int32(track.bounds().width * scale);
            pixel_height = to_int32(track.bounds().height * scale);
        } else {
            // As a fallback, attempt to calculate using the overall control size
            // This shouldn't happen as a track is a required template part of a slider
            // However, if it does, the spectrum gradient will still be shown
            pixel_width = to_int32(this.bounds().width * scale);
            pixel_height = to_int32(this.bounds().height * scale);
        }

        if pixel_width != 0 && pixel_height != 0 {
            // The buffer is sized to its capacity, because CreateComponentBitmapAsync sets bytes
            // via indexer over pre-allocated buffer.
            let bgra_pixel_data = vec![0u8; (pixel_width * pixel_height * 4) as usize];
            let Some(bgra_pixel_data) = ColorPickerHelpers::create_component_bitmap_async(
                bgra_pixel_data,
                pixel_width,
                pixel_height,
                this.orientation(),
                this.color_model(),
                this.color_component(),
                this.hsv_color(),
                this.is_alpha_visible(),
                this.is_perceptive(),
            )
            .await
            else {
                // A canceled job: the method of the original never resumes.
                return;
            };

            if let Some(background_bitmap) = this.background_bitmap.borrow_mut().take() {
                background_bitmap.dispose();
            }
            let background_bitmap = Rc::new(ColorPickerHelpers::create_bitmap_from_pixel_data(
                &bgra_pixel_data,
                pixel_width,
                pixel_height,
            ));
            *this.background_bitmap.borrow_mut() = Some(background_bitmap.clone());

            let source: Rc<dyn IImageBrushSource> = background_bitmap;
            let brush: Rc<dyn IBrush> = ImageBrush::with_source(Some(source)).into();
            this.set_background(Some(brush));
        }
    }

    /// Rounds the component values of the given [`HsvColor`].
    /// This is useful for user-display and to ensure a color matches user selection exactly.
    fn round_component_values(hsv_color: HsvColor) -> HsvColor {
        HsvColor::new(
            round_digits(hsv_color.a, 2, true),
            round_digits(hsv_color.h, 0, true),
            round_digits(hsv_color.s, 2, true),
            round_digits(hsv_color.v, 2, true),
        )
    }

    /// Updates the slider property values by applying the current color.
    ///
    /// Warning: This will trigger property changed updates.
    /// Consider using [`set_ignore_property_changed`](Self::set_ignore_property_changed) externally.
    fn set_color_to_slider_values(&self) {
        let component = self.color_component();

        if self.color_model() == ColorModel::Hsva {
            let mut hsv_color = self.hsv_color();

            if self.is_rounding_enabled() {
                hsv_color = Self::round_component_values(hsv_color);
            }

            // Note: Components converted into a usable range for the user
            match component {
                ColorComponent::Alpha => {
                    self.set_minimum(0.0);
                    self.set_maximum(100.0);
                    self.set_range_value(hsv_color.a * 100.0);
                }
                ColorComponent::Component1 => {
                    // Hue
                    self.set_minimum(0.0);
                    self.set_maximum(MAX_HUE);
                    self.set_range_value(hsv_color.h);
                }
                ColorComponent::Component2 => {
                    // Saturation
                    self.set_minimum(0.0);
                    self.set_maximum(100.0);
                    self.set_range_value(hsv_color.s * 100.0);
                }
                ColorComponent::Component3 => {
                    // Value
                    self.set_minimum(0.0);
                    self.set_maximum(100.0);
                    self.set_range_value(hsv_color.v * 100.0);
                }
            }
        } else {
            let rgb_color = self.color();

            match component {
                ColorComponent::Alpha => {
                    self.set_minimum(0.0);
                    self.set_maximum(255.0);
                    self.set_range_value(rgb_color.a as f64);
                }
                ColorComponent::Component1 => {
                    // Red
                    self.set_minimum(0.0);
                    self.set_maximum(255.0);
                    self.set_range_value(rgb_color.r as f64);
                }
                ColorComponent::Component2 => {
                    // Green
                    self.set_minimum(0.0);
                    self.set_maximum(255.0);
                    self.set_range_value(rgb_color.g as f64);
                }
                ColorComponent::Component3 => {
                    // Blue
                    self.set_minimum(0.0);
                    self.set_maximum(255.0);
                    self.set_range_value(rgb_color.b as f64);
                }
            }
        }
    }

    /// Gets the current color determined by the slider values.
    fn get_color_from_slider_values(&self) -> (Color, HsvColor) {
        let mut hsv_color: HsvColor;
        let rgb_color: Color;
        let slider_percent = self.value() / (self.maximum() - self.minimum());
        let component = self.color_component();

        if self.color_model() == ColorModel::Hsva {
            let base_hsv_color = self.hsv_color();

            match component {
                ColorComponent::Alpha => {
                    hsv_color = HsvColor::new(slider_percent, base_hsv_color.h, base_hsv_color.s, base_hsv_color.v);
                }
                ColorComponent::Component1 => {
                    hsv_color =
                        HsvColor::new(base_hsv_color.a, slider_percent * MAX_HUE, base_hsv_color.s, base_hsv_color.v);
                }
                ColorComponent::Component2 => {
                    hsv_color = HsvColor::new(base_hsv_color.a, base_hsv_color.h, slider_percent, base_hsv_color.v);
                }
                ColorComponent::Component3 => {
                    hsv_color = HsvColor::new(base_hsv_color.a, base_hsv_color.h, base_hsv_color.s, slider_percent);
                }
            }

            rgb_color = hsv_color.to_rgb();
        } else {
            let base_rgb_color = self.color();

            let component_value = MathUtilities::clamp(slider_percent * 255.0, 0.0, 255.0).round_ties_even() as u8;

            match component {
                ColorComponent::Alpha => {
                    rgb_color = Color::new(component_value, base_rgb_color.r, base_rgb_color.g, base_rgb_color.b);
                }
                ColorComponent::Component1 => {
                    rgb_color = Color::new(base_rgb_color.a, component_value, base_rgb_color.g, base_rgb_color.b);
                }
                ColorComponent::Component2 => {
                    rgb_color = Color::new(base_rgb_color.a, base_rgb_color.r, component_value, base_rgb_color.b);
                }
                ColorComponent::Component3 => {
                    rgb_color = Color::new(base_rgb_color.a, base_rgb_color.r, base_rgb_color.g, component_value);
                }
            }

            hsv_color = rgb_color.to_hsv();
        }

        if self.is_rounding_enabled() {
            hsv_color = Self::round_component_values(hsv_color);
        }

        (rgb_color, hsv_color)
    }

    /// Gets the actual background color displayed for the given HSV color.
    /// This can differ due to the effects of certain properties intended to improve perception.
    fn get_perceptive_background_hsv_color(&self, mut hsv_color: HsvColor) -> HsvColor {
        let component = self.color_component();
        let is_alpha_visible = self.is_alpha_visible();
        let is_perceptive = self.is_perceptive();

        if !is_alpha_visible && component != ColorComponent::Alpha {
            hsv_color = HsvColor::new(1.0, hsv_color.h, hsv_color.s, hsv_color.v);
        }

        if is_perceptive {
            match component {
                ColorComponent::Component1 => HsvColor::new(hsv_color.a, hsv_color.h, 1.0, 1.0),
                ColorComponent::Component2 => HsvColor::new(hsv_color.a, hsv_color.h, hsv_color.s, 1.0),
                ColorComponent::Component3 => HsvColor::new(hsv_color.a, hsv_color.h, 1.0, hsv_color.v),
                ColorComponent::Alpha => hsv_color,
            }
        } else {
            hsv_color
        }
    }

    /// Gets the actual background color displayed for the given RGB color.
    /// This can differ due to the effects of certain properties intended to improve perception.
    fn get_perceptive_background_color(&self, mut rgb_color: Color) -> Color {
        let component = self.color_component();
        let is_alpha_visible = self.is_alpha_visible();
        let is_perceptive = self.is_perceptive();

        if !is_alpha_visible && component != ColorComponent::Alpha {
            rgb_color = Color::new(255, rgb_color.r, rgb_color.g, rgb_color.b);
        }

        if is_perceptive {
            match component {
                ColorComponent::Component1 => Color::new(rgb_color.a, rgb_color.r, 0, 0),
                ColorComponent::Component2 => Color::new(rgb_color.a, 0, rgb_color.g, 0),
                ColorComponent::Component3 => Color::new(rgb_color.a, 0, 0, rgb_color.b),
                ColorComponent::Alpha => rgb_color,
            }
        } else {
            rgb_color
        }
    }
}
