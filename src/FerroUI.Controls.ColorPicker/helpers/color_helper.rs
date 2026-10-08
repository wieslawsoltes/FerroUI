use super::round_digits;
use ferroui_base::media::{Color, HsvColor, KnownColor, KnownColors};
use ferroui_base::utilities::CultureInfo;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};

/// The key of a rounded [`HsvColor`] in the display name cache: the bits of
/// its components (the original keys a dictionary with the color itself;
/// DEVIATIONS.md, Colour picker).
type HsvColorKey = [u64; 4];

fn hsv_color_key(color: &HsvColor) -> HsvColorKey {
    [color.a.to_bits(), color.h.to_bits(), color.s.to_bits(), color.v.to_bits()]
}

static CACHED_DISPLAY_NAMES: LazyLock<Mutex<HashMap<HsvColorKey, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CACHED_KNOWN_COLOR_NAMES: LazyLock<Mutex<HashMap<KnownColor, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Contains helpers useful when working with colors.
pub struct ColorHelper;

impl ColorHelper {
    /// Gets the relative (perceptual) luminance/brightness of the given color.
    /// 1 is closer to white while 0 is closer to black.
    pub fn get_relative_luminance(color: Color) -> f64 {
        // The equation for relative luminance is given by
        //
        // L = 0.2126 * Rg + 0.7152 * Gg + 0.0722 * Bg
        //
        // where Xg = { X/3294 if X <= 10, (R/269 + 0.0513)^2.4 otherwise }
        //
        // If L is closer to 1, then the color is closer to white; if it is closer to 0,
        // then the color is closer to black.  This is based on the fact that the human
        // eye perceives green to be much brighter than red, which in turn is perceived to be
        // brighter than blue.

        let rg = if color.r <= 10 { color.r as f64 / 3294.0 } else { (color.r as f64 / 269.0 + 0.0513).powf(2.4) };
        let gg = if color.g <= 10 { color.g as f64 / 3294.0 } else { (color.g as f64 / 269.0 + 0.0513).powf(2.4) };
        let bg = if color.b <= 10 { color.b as f64 / 3294.0 } else { (color.b as f64 / 269.0 + 0.0513).powf(2.4) };

        0.2126 * rg + 0.7152 * gg + 0.0722 * bg
    }

    /// Determines if color display names are supported based on the current thread culture.
    ///
    /// Only English names are currently supported following known color names.
    /// In the future known color names could be localized.
    pub fn to_display_name_exists() -> bool {
        let culture = CultureInfo::current_ui_culture();
        let name = culture.name();
        name.len() >= 2 && name.as_bytes()[..2].eq_ignore_ascii_case(b"EN")
    }

    /// Determines an approximate display name for the given color.
    pub fn to_display_name(color: Color) -> String {
        let hsv_color = color.to_hsv();

        // Handle extremes that are outside the below algorithm
        if color.a == 0x00 {
            return Self::get_display_name(KnownColor::TRANSPARENT);
        }

        // HSV ----------------------------------------------------------------------
        //
        // There are far too many possible HSV colors to cache and search through
        // for performance reasons. Therefore, the HSV color is rounded.
        // Rounding is tolerable in this algorithm because it is perception based.
        // Hue is the most important for user perception so is rounded the least.
        // Then there is a lot of loss in rounding the saturation and value components
        // which are not as closely related to perceived color.
        //
        //         Hue : Round to nearest int (0..360)
        //  Saturation : Round to the nearest 1/10 (0..1)
        //       Value : Round to the nearest 1/10 (0..1)
        //       Alpha : Is ignored in this algorithm
        //
        // Rounding results in ~36_000 values to cache in the worse case.
        //
        // RGB ----------------------------------------------------------------------
        //
        // The original algorithm worked in RGB color space.
        // If this code is every adjusted to work in RGB again note the following:
        //
        // Without rounding, there are 16_777_216 possible RGB colors (without alpha).
        // This is too many to cache and search through for performance reasons.
        // It is also needlessly large as there are only ~140 known/named colors.
        // Therefore, rounding of the input color's component values is done to
        // reduce the color space into something more useful.
        //
        // The rounding value of 5 is specially chosen.
        // It is a factor of 255 and therefore evenly divisible which improves
        // the quality of the calculations.
        let rounded_hsv_color = HsvColor::new(
            1.0,
            round_digits(hsv_color.h, 0, false),
            round_digits(hsv_color.s, 1, false),
            round_digits(hsv_color.v, 1, false),
        );

        // Attempt to use a previously cached display name
        {
            let cache = CACHED_DISPLAY_NAMES.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(display_name) = cache.get(&hsv_color_key(&rounded_hsv_color)) {
                return display_name.clone();
            }
        }

        // Build the KnownColor name cache if it doesn't already exist
        {
            let mut cache = CACHED_KNOWN_COLOR_NAMES.lock().unwrap_or_else(PoisonError::into_inner);
            if cache.is_empty() {
                // Skip 'None' (the values do not include it)
                for known_color in KnownColors::values() {
                    // Some known colors have the same numerical value. For example:
                    //  - Aqua = 0xff00ffff
                    //  - Cyan = 0xff00ffff
                    //
                    // This is not possible to represent in a dictionary which requires
                    // unique values. Therefore, only the first value is used.

                    cache.entry(known_color).or_insert_with(|| Self::get_display_name(known_color));
                }
            }
        }

        // Find the closest known color by measuring 3D Euclidean distance (ignore alpha)
        // This is done in HSV color space to most closely match user-perception
        let mut closest_known_color = KnownColor::NONE;
        let mut closest_known_color_distance = f64::INFINITY;

        // Skip 'None' (the values do not include it)
        for known_color in KnownColors::values() {
            // Transparent is skipped since alpha is ignored making it equivalent to White
            if known_color != KnownColor::TRANSPARENT {
                let known_hsv_color = KnownColors::to_color(known_color).to_hsv();

                let distance = ((rounded_hsv_color.h - known_hsv_color.h).powf(2.0)
                    + (rounded_hsv_color.s - known_hsv_color.s).powf(2.0)
                    + (rounded_hsv_color.v - known_hsv_color.v).powf(2.0))
                .sqrt();

                if distance < closest_known_color_distance {
                    closest_known_color = known_color;
                    closest_known_color_distance = distance;
                }
            }
        }

        // Return the closest known color as the display name
        // Cache results for next time as well
        if closest_known_color != KnownColor::NONE {
            let display_name = {
                let cache = CACHED_KNOWN_COLOR_NAMES.lock().unwrap_or_else(PoisonError::into_inner);
                match cache.get(&closest_known_color) {
                    Some(display_name) => display_name.clone(),
                    None => Self::get_display_name(closest_known_color),
                }
            };

            CACHED_DISPLAY_NAMES
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(hsv_color_key(&rounded_hsv_color), display_name.clone());

            display_name
        } else {
            String::new()
        }
    }

    /// Gets the human-readable display name for the given [`KnownColor`].
    ///
    /// This currently uses the name of the known color directly which limits
    /// it to the EN language only. In the future this should be localized to
    /// other cultures.
    fn get_display_name(known_color: KnownColor) -> String {
        // The name of a value that two known colors share is the name declared first (the
        // original takes the enumeration member name of the value).
        let name = KnownColors::get_known_color_name(known_color.value()).unwrap_or("None");
        let mut sb = String::with_capacity(name.len() + 4);

        // Add spaces converting PascalCase to human-readable names
        for (i, c) in name.chars().enumerate() {
            if i != 0 && c.is_uppercase() {
                sb.push(' ');
            }

            sb.push(c);
        }

        sb
    }
}
