// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use super::hsv::Hsv;
use super::increment_amount::IncrementAmount;
use super::increment_direction::IncrementDirection;
use super::ColorHelper;
use crate::{ColorComponent, ColorModel, HsvComponent};
use ferroui_base::layout::Orientation;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, Colors, HsvColor};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{PixelSize, Vector};

/// `Convert.ToByte(double)` of the .NET base library for a value already in
/// the range of a byte: rounds half to even.
fn to_byte(value: f64) -> u8 {
    value.round_ties_even() as u8
}

/// `Math.Sign(double)` of the .NET base library.
fn sign(value: f64) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

/// The component `component` of `hsv`: the counterpart of the `ref` local
/// the original points at one of the fields.
fn component_value(hsv: &Hsv, component: HsvComponent) -> f64 {
    match component {
        HsvComponent::Hue => hsv.h,
        HsvComponent::Saturation => hsv.s,
        HsvComponent::Value => hsv.v,
        HsvComponent::Alpha => panic!("Invalid HsvComponent."),
    }
}

/// Writes the component `component` of `hsv`; see [`component_value`].
fn set_component_value(hsv: &mut Hsv, component: HsvComponent, value: f64) {
    match component {
        HsvComponent::Hue => hsv.h = value,
        HsvComponent::Saturation => hsv.s = value,
        HsvComponent::Value => hsv.v = value,
        HsvComponent::Alpha => panic!("Invalid HsvComponent."),
    }
}

/// Contains internal, special-purpose helpers used with the color picker.
pub(crate) struct ColorPickerHelpers;

impl ColorPickerHelpers {
    /// Generates a new bitmap of the specified size by changing a specific color component.
    /// This will produce a gradient representing a sweep of all possible values of the color component.
    ///
    /// - `bgra_pixel_data`: the target buffer of `width * height * 4` bytes,
    ///   returned with the bitmap data.
    /// - `width`, `height`: the pixel width (X, horizontal) and height (Y, vertical) of the resulting bitmap.
    /// - `orientation`: the orientation of the resulting bitmap (gradient direction).
    /// - `color_model`: the color model being used: RGBA or HSVA.
    /// - `component`: the specific color component to sweep.
    /// - `base_hsv_color`: the base HSV color used for components not being changed.
    /// - `is_alpha_visible`: whether the alpha component is visible and rendered in the bitmap.
    ///   This property is ignored when the alpha component itself is being rendered.
    /// - `is_perceptive`: whether the slider adapts rendering to improve user-perception over exactness.
    ///   This will ensure colors are always discernible.
    ///
    /// Deviation (DEVIATIONS.md, Colour picker): the original fills the
    /// pooled buffer it is given in place on a thread-pool thread
    /// (`Task.Run`). The port has no thread pool (the browser has no
    /// threads): the buffer is moved into a job of the UI thread at
    /// background priority and handed back as the result of the future,
    /// which is `None` when the job was canceled (the dispatcher shut down;
    /// the original never resumes after an abandoned task).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_component_bitmap_async(
        bgra_pixel_data: Vec<u8>,
        width: i32,
        height: i32,
        orientation: Orientation,
        color_model: ColorModel,
        component: ColorComponent,
        base_hsv_color: HsvColor,
        is_alpha_visible: bool,
        is_perceptive: bool,
    ) -> Option<Vec<u8>> {
        if width == 0 || height == 0 {
            return Some(bgra_pixel_data);
        }

        let mut bgra_pixel_data = bgra_pixel_data;
        let operation = Dispatcher::current_dispatcher().invoke_async_local_with_priority(
            move || {
                Self::fill_component_bitmap(
                    &mut bgra_pixel_data,
                    width,
                    height,
                    orientation,
                    color_model,
                    component,
                    base_hsv_color,
                    is_alpha_visible,
                    is_perceptive,
                );
                bgra_pixel_data
            },
            DispatcherPriority::BACKGROUND,
        );

        operation.await.ok()
    }

    /// The body of the task of [`create_component_bitmap_async`](Self::create_component_bitmap_async).
    #[allow(clippy::too_many_arguments)]
    fn fill_component_bitmap(
        bgra_pixel_data: &mut [u8],
        width: i32,
        height: i32,
        orientation: Orientation,
        color_model: ColorModel,
        component: ColorComponent,
        mut base_hsv_color: HsvColor,
        is_alpha_visible: bool,
        is_perceptive: bool,
    ) {
        let mut pixel_data_index: usize = 0;
        let component_step: f64;
        let mut base_rgb_color = Colors::WHITE;
        let mut rgb_color: Color;

        // BGRA formatted color components 1 byte each (4 bytes in a pixel)
        let _bgra_pixel_data_height = (height * 4) as usize;
        let bgra_pixel_data_width = (width * 4) as usize;

        // Maximize alpha component value
        if !is_alpha_visible && component != ColorComponent::Alpha {
            base_hsv_color = HsvColor::new(1.0, base_hsv_color.h, base_hsv_color.s, base_hsv_color.v);
        }

        // Convert HSV to RGB once
        if color_model == ColorModel::Rgba {
            base_rgb_color = base_hsv_color.to_rgb();
        }

        // Apply any perceptive adjustments to the color
        if is_perceptive && component != ColorComponent::Alpha {
            if color_model == ColorModel::Hsva {
                // Maximize Saturation and Value components
                match component {
                    ColorComponent::Component1 => {
                        // Hue
                        base_hsv_color = HsvColor::new(base_hsv_color.a, base_hsv_color.h, 1.0, 1.0);
                    }
                    ColorComponent::Component2 => {
                        // Saturation
                        base_hsv_color = HsvColor::new(base_hsv_color.a, base_hsv_color.h, base_hsv_color.s, 1.0);
                    }
                    ColorComponent::Component3 => {
                        // Value
                        base_hsv_color = HsvColor::new(base_hsv_color.a, base_hsv_color.h, 1.0, base_hsv_color.v);
                    }
                    ColorComponent::Alpha => {}
                }
            } else {
                // Minimize component values other than the current one
                match component {
                    ColorComponent::Component1 => {
                        // Red
                        base_rgb_color = Color::new(base_rgb_color.a, base_rgb_color.r, 0, 0);
                    }
                    ColorComponent::Component2 => {
                        // Green
                        base_rgb_color = Color::new(base_rgb_color.a, 0, base_rgb_color.g, 0);
                    }
                    ColorComponent::Component3 => {
                        // Blue
                        base_rgb_color = Color::new(base_rgb_color.a, 0, 0, base_rgb_color.b);
                    }
                    ColorComponent::Alpha => {}
                }
            }
        }

        let get_color = |component_value: f64| -> Color {
            match component {
                ColorComponent::Component1 => {
                    if color_model == ColorModel::Hsva {
                        // Sweep hue
                        HsvColor::hsv_to_rgb(
                            MathUtilities::clamp(component_value, 0.0, 360.0),
                            base_hsv_color.s,
                            base_hsv_color.v,
                            base_hsv_color.a,
                        )
                    } else {
                        // Sweep red
                        Color::new(
                            base_rgb_color.a,
                            to_byte(MathUtilities::clamp(component_value, 0.0, 255.0)),
                            base_rgb_color.g,
                            base_rgb_color.b,
                        )
                    }
                }
                ColorComponent::Component2 => {
                    if color_model == ColorModel::Hsva {
                        // Sweep saturation
                        HsvColor::hsv_to_rgb(
                            base_hsv_color.h,
                            MathUtilities::clamp(component_value, 0.0, 1.0),
                            base_hsv_color.v,
                            base_hsv_color.a,
                        )
                    } else {
                        // Sweep green
                        Color::new(
                            base_rgb_color.a,
                            base_rgb_color.r,
                            to_byte(MathUtilities::clamp(component_value, 0.0, 255.0)),
                            base_rgb_color.b,
                        )
                    }
                }
                ColorComponent::Component3 => {
                    if color_model == ColorModel::Hsva {
                        // Sweep value
                        HsvColor::hsv_to_rgb(
                            base_hsv_color.h,
                            base_hsv_color.s,
                            MathUtilities::clamp(component_value, 0.0, 1.0),
                            base_hsv_color.a,
                        )
                    } else {
                        // Sweep blue
                        Color::new(
                            base_rgb_color.a,
                            base_rgb_color.r,
                            base_rgb_color.g,
                            to_byte(MathUtilities::clamp(component_value, 0.0, 255.0)),
                        )
                    }
                }
                ColorComponent::Alpha => {
                    if color_model == ColorModel::Hsva {
                        // Sweep alpha
                        HsvColor::hsv_to_rgb(
                            base_hsv_color.h,
                            base_hsv_color.s,
                            base_hsv_color.v,
                            MathUtilities::clamp(component_value, 0.0, 1.0),
                        )
                    } else {
                        // Sweep alpha
                        Color::new(
                            to_byte(MathUtilities::clamp(component_value, 0.0, 255.0)),
                            base_rgb_color.r,
                            base_rgb_color.g,
                            base_rgb_color.b,
                        )
                    }
                }
            }
        };

        // Premultiplies a color into the pixel at `index`.
        let write_pixel = |data: &mut [u8], index: usize, color: Color| {
            data[index] = (color.b as u32 * color.a as u32 / 255) as u8;
            data[index + 1] = (color.g as u32 * color.a as u32 / 255) as u8;
            data[index + 2] = (color.r as u32 * color.a as u32 / 255) as u8;
            data[index + 3] = color.a;
        };

        // Create the color component gradient
        if orientation == Orientation::Horizontal {
            // Determine the numerical increment of the color steps within the component
            if color_model == ColorModel::Hsva {
                if component == ColorComponent::Component1 {
                    component_step = 360.0 / width as f64;
                } else {
                    component_step = 1.0 / width as f64;
                }
            } else {
                component_step = 255.0 / width as f64;
            }

            for y in 0..height {
                for x in 0..width {
                    if y == 0 {
                        rgb_color = get_color(x as f64 * component_step);

                        // Get a new color
                        write_pixel(bgra_pixel_data, pixel_data_index, rgb_color);
                    } else {
                        // Use the color in the row above
                        // Remember the pixel data is 1 dimensional instead of 2
                        bgra_pixel_data.copy_within(
                            pixel_data_index - bgra_pixel_data_width..pixel_data_index - bgra_pixel_data_width + 4,
                            pixel_data_index,
                        );
                    }

                    pixel_data_index += 4;
                }
            }
        } else {
            // Determine the numerical increment of the color steps within the component
            if color_model == ColorModel::Hsva {
                if component == ColorComponent::Component1 {
                    component_step = 360.0 / height as f64;
                } else {
                    component_step = 1.0 / height as f64;
                }
            } else {
                component_step = 255.0 / height as f64;
            }

            for y in 0..height {
                for x in 0..width {
                    if x == 0 {
                        // The lowest component value should be at the 'bottom' of the bitmap
                        rgb_color = get_color((height - 1 - y) as f64 * component_step);

                        // Get a new color
                        write_pixel(bgra_pixel_data, pixel_data_index, rgb_color);
                    } else {
                        // Use the color in the column to the left
                        // Remember the pixel data is 1 dimensional instead of 2
                        bgra_pixel_data.copy_within(pixel_data_index - 4..pixel_data_index, pixel_data_index);
                    }

                    pixel_data_index += 4;
                }
            }
        }
    }

    /// Increments the component `component` of `original_hsv` by a small or
    /// large step, or to the next named color.
    pub fn increment_color_component(
        original_hsv: Hsv,
        component: HsvComponent,
        direction: IncrementDirection,
        amount: IncrementAmount,
        should_wrap: bool,
        mut min_bound: f64,
        mut max_bound: f64,
    ) -> Hsv {
        let mut new_hsv = original_hsv;

        if amount == IncrementAmount::Small || !ColorHelper::to_display_name_exists() {
            // In order to avoid working with small values that can incur rounding issues,
            // we'll multiple saturation and value by 100 to put them in the range of 0-100 instead of 0-1.
            new_hsv.s *= 100.0;
            new_hsv.v *= 100.0;

            // If we're adding a small increment, then we'll just add or subtract 1.
            // If we're adding a large increment, then we want to snap to the next
            // or previous major value - for hue, this is every increment of 30;
            // for saturation and value, this is every increment of 10.
            let increment_amount = match component {
                HsvComponent::Hue => {
                    if amount == IncrementAmount::Small {
                        1.0
                    } else {
                        30.0
                    }
                }
                HsvComponent::Saturation | HsvComponent::Value => {
                    if amount == IncrementAmount::Small {
                        1.0
                    } else {
                        10.0
                    }
                }
                HsvComponent::Alpha => panic!("Invalid HsvComponent."),
            };

            let previous_value = component_value(&new_hsv, component);

            let mut value_to_increment = previous_value
                + if direction == IncrementDirection::Lower { -increment_amount } else { increment_amount };

            // If the value has reached outside the bounds, we were previous at the boundary, and we should wrap,
            // then we'll place the selection on the other side of the spectrum.
            // Otherwise, we'll place it on the boundary that was exceeded.
            if value_to_increment < min_bound {
                value_to_increment = if should_wrap && previous_value == min_bound { max_bound } else { min_bound };
            }

            if value_to_increment > max_bound {
                value_to_increment = if should_wrap && previous_value == max_bound { min_bound } else { max_bound };
            }

            set_component_value(&mut new_hsv, component, value_to_increment);

            // We multiplied saturation and value by 100 previously, so now we want to put them back in the 0-1 range.
            new_hsv.s /= 100.0;
            new_hsv.v /= 100.0;
        } else {
            // While working with named colors, we're going to need to be working in actual HSV units,
            // so we'll divide the min bound and max bound by 100 in the case of saturation or value,
            // since we'll have received units between 0-100 and we need them within 0-1.
            if component == HsvComponent::Saturation || component == HsvComponent::Value {
                min_bound /= 100.0;
                max_bound /= 100.0;
            }

            new_hsv = Self::find_next_named_color(original_hsv, component, direction, should_wrap, min_bound, max_bound);
        }

        new_hsv
    }

    /// Finds the color with the next display name in the direction
    /// `direction` of the component `component`.
    pub fn find_next_named_color(
        original_hsv: Hsv,
        component: HsvComponent,
        direction: IncrementDirection,
        should_wrap: bool,
        min_bound: f64,
        max_bound: f64,
    ) -> Hsv {
        // There's no easy way to directly get the next named color, so what we'll do
        // is just iterate in the direction that we want to find it until we find a color
        // in that direction that has a color name different than our current color name.
        // Once we find a new color name, then we'll iterate across that color name until
        // we find its bounds on the other side, and then select the color that is exactly
        // in the middle of that color's bounds.
        let mut new_hsv = original_hsv;

        let original_color_name = ColorHelper::to_display_name(original_hsv.to_rgb().to_color(1.0));
        let mut new_color_name = original_color_name.clone();

        let (original_value, increment_amount) = match component {
            HsvComponent::Hue => (original_hsv.h, 1.0),
            HsvComponent::Saturation => (original_hsv.s, 0.01),
            HsvComponent::Value => (original_hsv.v, 0.01),
            HsvComponent::Alpha => panic!("Invalid HsvComponent."),
        };
        let step = if direction == IncrementDirection::Lower { -1.0 } else { 1.0 };

        let mut should_find_mid_point = true;

        while new_color_name == original_color_name {
            let previous_value = component_value(&new_hsv, component);
            let mut new_value = previous_value + step * increment_amount;

            let mut just_wrapped = false;

            // If we've hit a boundary, then either we should wrap or we shouldn't.
            // If we should, then we'll perform that wrapping if we were previously up against
            // the boundary that we've now hit.  Otherwise, we'll stop at that boundary.
            if new_value > max_bound {
                if should_wrap {
                    new_value = min_bound;
                    just_wrapped = true;
                } else {
                    set_component_value(&mut new_hsv, component, max_bound);
                    should_find_mid_point = false;
                    new_color_name = ColorHelper::to_display_name(new_hsv.to_rgb().to_color(1.0));
                    break;
                }
            } else if new_value < min_bound {
                if should_wrap {
                    new_value = max_bound;
                    just_wrapped = true;
                } else {
                    set_component_value(&mut new_hsv, component, min_bound);
                    should_find_mid_point = false;
                    new_color_name = ColorHelper::to_display_name(new_hsv.to_rgb().to_color(1.0));
                    break;
                }
            }

            set_component_value(&mut new_hsv, component, new_value);

            if !just_wrapped
                && previous_value != original_value
                && sign(new_value - original_value) != sign(previous_value - original_value)
            {
                // If we've wrapped all the way back to the start and have failed to find a new color name,
                // then we'll just quit - there isn't a new color name that we're going to find.
                should_find_mid_point = false;
                break;
            }

            new_color_name = ColorHelper::to_display_name(new_hsv.to_rgb().to_color(1.0));
        }

        if should_find_mid_point {
            let start_hsv = new_hsv;
            let mut current_hsv = start_hsv;
            let mut start_end_offset = 0.0;
            let mut current_color_name = new_color_name.clone();

            let wrap_increment = match component {
                HsvComponent::Hue => 360.0,
                HsvComponent::Saturation | HsvComponent::Value => 1.0,
                HsvComponent::Alpha => panic!("Invalid HsvComponent."),
            };

            while new_color_name == current_color_name {
                let mut current_value = component_value(&current_hsv, component) + step * increment_amount;

                // If we've hit a boundary, then either we should wrap or we shouldn't.
                // If we should, then we'll perform that wrapping if we were previously up against
                // the boundary that we've now hit.  Otherwise, we'll stop at that boundary.
                if current_value > max_bound {
                    if should_wrap {
                        current_value = min_bound;
                        start_end_offset = max_bound - min_bound;
                    } else {
                        set_component_value(&mut current_hsv, component, max_bound);
                        break;
                    }
                } else if current_value < min_bound {
                    if should_wrap {
                        current_value = max_bound;
                        start_end_offset = min_bound - max_bound;
                    } else {
                        set_component_value(&mut current_hsv, component, min_bound);
                        break;
                    }
                }

                set_component_value(&mut current_hsv, component, current_value);
                current_color_name = ColorHelper::to_display_name(current_hsv.to_rgb().to_color(1.0));
            }

            let start_value = component_value(&start_hsv, component);
            let current_value = component_value(&current_hsv, component);
            let mut new_value = (start_value + current_value + start_end_offset) / 2.0;

            // Dividing by 2 may have gotten us halfway through a single step, so we'll
            // remove that half-step if it exists.
            let mut leftover_value = new_value.abs();

            while leftover_value > increment_amount {
                leftover_value -= increment_amount;
            }

            new_value -= leftover_value;

            while new_value < min_bound {
                new_value += wrap_increment;
            }

            while new_value > max_bound {
                new_value -= wrap_increment;
            }

            set_component_value(&mut new_hsv, component, new_value);
        }

        new_hsv
    }

    /// Converts the given raw BGRA pre-multiplied alpha pixel data into a bitmap.
    ///
    /// `bgra_pixel_data` is the bitmap (in raw BGRA pre-multiplied alpha
    /// pixels) of `pixel_width` by `pixel_height` pixels.
    pub fn create_bitmap_from_pixel_data(bgra_pixel_data: &[u8], pixel_width: i32, pixel_height: i32) -> Bitmap {
        Bitmap::from_pixels(
            PixelFormat::BGRA8888,
            AlphaFormat::Premul,
            bgra_pixel_data,
            PixelSize::new(pixel_width, pixel_height),
            Vector::new(96.0, 96.0),
            pixel_width * 4,
        )
    }
}
