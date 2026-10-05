use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str::FromStr;

use crate::media::fonts::OpenTypeTag;
use crate::utilities::FormatError;

/// A single variable-font axis setting in user-space coordinates, e.g.
/// `wght=650`.
#[derive(Clone, Copy, Debug)]
pub struct FontVariation {
    /// The four-character axis tag (e.g. `wght`).
    pub tag: OpenTypeTag,
    /// The axis value in user-space units (e.g. `650` for `wght`).
    pub value: f64,
}

impl FontVariation {
    /// Creates an axis setting.
    pub const fn new(tag: OpenTypeTag, value: f64) -> Self {
        Self { tag, value }
    }
}

// Values compare like C# `double.Equals`: a NaN equals a NaN.
impl PartialEq for FontVariation {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && (self.value == other.value || (self.value.is_nan() && other.value.is_nan()))
    }
}

impl Eq for FontVariation {}

impl Hash for FontVariation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
        state.write_u64(normalized_bits(self.value));
    }
}

/// The bits of a value such that equal values have equal bits.
fn normalized_bits(value: f64) -> u64 {
    if value == 0.0 {
        0
    } else if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

/// Returns the `tag=value` form of the variation (e.g. `wght=650`), using the
/// invariant culture.
impl fmt::Display for FontVariation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=", self.tag)?;
        write_invariant(f, self.value)
    }
}

/// Writes a value as the shortest text that parses back to it, in the
/// notation of the invariant culture (`Infinity`, `1E+21`, `1E-05`).
fn write_invariant(f: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
    if value.is_nan() {
        return f.write_str("NaN");
    }

    if value.is_infinite() {
        return f.write_str(if value > 0.0 { "Infinity" } else { "-Infinity" });
    }

    if value == 0.0 {
        return f.write_str(if value.is_sign_negative() { "-0" } else { "0" });
    }

    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);

    if (-4..15).contains(&exponent) {
        return write!(f, "{value}");
    }

    write!(f, "{mantissa}E{}{:02}", if exponent < 0 { '-' } else { '+' }, exponent.abs())
}

/// An immutable, value-equal set of variable-font axis settings in user-space
/// coordinates (e.g. `wght=650,wdth=85`). This is the public input form;
/// values are normalized against a specific font's `fvar`/`avar` when applied.
///
/// Cloning is cheap (one reference count). Variations are stored sorted by
/// axis tag so that equality and hashing are independent of the order they
/// were specified in.
#[derive(Clone, Debug)]
pub struct FontVariationSettings {
    variations: Rc<[FontVariation]>,
    hash_code: u64,
}

impl FontVariationSettings {
    fn from_sorted(sorted_variations: Vec<FontVariation>) -> Self {
        let hash_code = Self::compute_hash_code(&sorted_variations);

        Self { variations: Rc::from(sorted_variations), hash_code }
    }

    /// The empty settings: no axis is varied.
    pub fn empty() -> FontVariationSettings {
        thread_local! {
            static EMPTY: FontVariationSettings = FontVariationSettings::from_sorted(Vec::new());
        }

        EMPTY.with(Clone::clone)
    }

    /// Initializes a new instance of the `FontVariationSettings` class.
    ///
    /// When a tag appears more than once, the last occurrence wins.
    ///
    /// Panics when a variation's value is NaN.
    pub fn new(variations: impl IntoIterator<Item = FontVariation>) -> Self {
        let mut builder: Vec<FontVariation> = Vec::new();

        for variation in variations {
            // Infinite values are usable - they clamp to the axis range like any other
            // out-of-range value when the settings are applied. NaN has no such meaning.
            if variation.value.is_nan() {
                panic!("Value for axis '{}' must not be NaN.", variation.tag);
            }

            // Last-wins for duplicate tags, matching CSS font-variation-settings.
            match builder.iter_mut().find(|existing| existing.tag == variation.tag) {
                Some(existing) => *existing = variation,
                None => builder.push(variation),
            }
        }

        builder.sort_by_key(|variation| variation.tag.value());

        Self::from_sorted(builder)
    }

    /// The axis settings, sorted by axis tag.
    pub fn variations(&self) -> &[FontVariation] {
        &self.variations
    }

    /// Gets whether no axis is varied.
    pub fn is_empty(&self) -> bool {
        self.variations.is_empty()
    }

    /// Tries to get the user-space value for an axis.
    pub fn try_get_value(&self, tag: OpenTypeTag) -> Option<f64> {
        self.variations.iter().find(|variation| variation.tag == tag).map(|variation| variation.value)
    }

    /// Parses a comma-separated list of `tag=value` pairs, e.g.
    /// `"wght=650,wdth=85"`. Values use the invariant culture.
    pub fn parse(s: &str) -> Result<FontVariationSettings, FormatError> {
        if s.trim().is_empty() {
            return Ok(Self::empty());
        }

        let mut variations = Vec::new();

        for part in s.split(',') {
            let pair = part.trim();

            if pair.is_empty() {
                continue;
            }

            let (tag_text, value_text) = match pair.split_once('=') {
                Some((tag_text, value_text)) if !tag_text.is_empty() && !value_text.is_empty() => {
                    (tag_text.trim(), value_text.trim())
                }
                _ => {
                    return Err(FormatError::from_string(format!(
                        "Invalid font variation '{pair}': expected tag=value."
                    )));
                }
            };

            if tag_text.is_empty() || tag_text.encode_utf16().count() > 4 {
                return Err(FormatError::from_string(format!("Invalid font variation axis tag '{tag_text}'.")));
            }

            // The value text is already trimmed, and no whitespace is allowed inside the number itself.
            let value = match parse_invariant(value_text) {
                Some(value) if !value.is_nan() => value,
                _ => {
                    return Err(FormatError::from_string(format!(
                        "Invalid font variation value '{value_text}' for axis '{tag_text}'."
                    )));
                }
            };

            variations.push(FontVariation::new(OpenTypeTag::parse(tag_text), value));
        }

        Ok(if variations.is_empty() { Self::empty() } else { Self::new(variations) })
    }

    fn compute_hash_code(variations: &[FontVariation]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        for variation in variations {
            variation.hash(&mut hasher);
        }

        hasher.finish()
    }
}

/// Parses a number written with an optional leading sign, a decimal point and
/// an exponent (invariant culture), or one of the symbols `Infinity`,
/// `-Infinity` and `NaN`.
fn parse_invariant(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let mut index = 0;

    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }

    let integer_start = index;

    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }

    let mut digits = index - integer_start;

    if bytes.get(index) == Some(&b'.') {
        index += 1;

        let fraction_start = index;

        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }

        digits += index - fraction_start;
    }

    let mut is_number = digits > 0;

    if is_number && matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;

        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }

        let exponent_start = index;

        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }

        is_number = index > exponent_start;
    }

    if is_number && index == bytes.len() {
        return text.parse().ok();
    }

    let (negative, symbol) = match bytes.first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };

    if symbol.eq_ignore_ascii_case("Infinity") {
        Some(if negative { f64::NEG_INFINITY } else { f64::INFINITY })
    } else if symbol.eq_ignore_ascii_case("NaN") {
        Some(f64::NAN)
    } else {
        None
    }
}

impl Default for FontVariationSettings {
    fn default() -> Self {
        Self::empty()
    }
}

impl FromStr for FontVariationSettings {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Returns the settings as a string that round-trips through
/// [`FontVariationSettings::parse`].
impl fmt::Display for FontVariationSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, variation) in self.variations.iter().enumerate() {
            if index > 0 {
                f.write_str(",")?;
            }

            fmt::Display::fmt(variation, f)?;
        }

        Ok(())
    }
}

impl PartialEq for FontVariationSettings {
    fn eq(&self, other: &Self) -> bool {
        if Rc::ptr_eq(&self.variations, &other.variations) {
            return true;
        }

        if self.hash_code != other.hash_code {
            return false;
        }

        self.variations == other.variations
    }
}

impl Eq for FontVariationSettings {}

impl Hash for FontVariationSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash_code);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn wght() -> OpenTypeTag {
        OpenTypeTag::parse("wght")
    }

    fn wdth() -> OpenTypeTag {
        OpenTypeTag::parse("wdth")
    }

    fn opsz() -> OpenTypeTag {
        OpenTypeTag::parse("opsz")
    }

    fn hash_of(settings: &FontVariationSettings) -> u64 {
        let mut hasher = DefaultHasher::new();
        settings.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn empty_is_empty_and_round_trips() {
        assert!(FontVariationSettings::empty().is_empty());
        assert!(FontVariationSettings::empty().variations().is_empty());
        assert_eq!(FontVariationSettings::empty().to_string(), "");
        assert_eq!(FontVariationSettings::parse("").unwrap(), FontVariationSettings::empty());
        assert_eq!(FontVariationSettings::parse("   ").unwrap(), FontVariationSettings::empty());
    }

    #[test]
    fn parse_reads_comma_separated_tag_value_pairs() {
        let settings = FontVariationSettings::parse(" wght = 700 , wdth=85.5 ").unwrap();

        assert_eq!(settings.variations().len(), 2);
        assert_eq!(settings.try_get_value(wght()), Some(700.0));
        assert_eq!(settings.try_get_value(wdth()), Some(85.5));
    }

    #[test]
    fn parse_rejects_malformed_input() {
        for input in ["wght", "wght=", "=700", "weight=700", "wght=seven", "wght=NaN", "wght=7 0", "wght=inf", "wght=1e"]
        {
            assert!(FontVariationSettings::parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn to_string_round_trips_through_parse() {
        let settings = FontVariationSettings::parse("opsz=14.25,wght=650").unwrap();

        let round_tripped = FontVariationSettings::parse(&settings.to_string()).unwrap();

        assert_eq!(settings, round_tripped);
        assert_eq!(settings.to_string(), "opsz=14.25,wght=650");
    }

    #[test]
    fn equality_is_order_independent_with_matching_hashes() {
        let a = FontVariationSettings::new([FontVariation::new(wght(), 700.0), FontVariation::new(opsz(), 36.0)]);
        let b = FontVariationSettings::new([FontVariation::new(opsz(), 36.0), FontVariation::new(wght(), 700.0)]);

        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b));
        assert_ne!(a, FontVariationSettings::parse("wght=700").unwrap());
    }

    #[test]
    fn duplicate_tags_collapse_to_the_last_value() {
        // CSS font-variation-settings behavior: the last occurrence wins.
        let settings = FontVariationSettings::parse("wght=400,wght=700").unwrap();

        assert_eq!(settings.variations().len(), 1);
        assert_eq!(settings.try_get_value(wght()), Some(700.0));
    }

    #[test]
    fn variations_are_sorted_by_tag() {
        let settings = FontVariationSettings::parse("wght=700,opsz=14,wdth=85").unwrap();

        assert_eq!(settings.variations()[0].tag, opsz());
        assert_eq!(settings.variations()[1].tag, wdth());
        assert_eq!(settings.variations()[2].tag, wght());
    }

    #[test]
    #[should_panic(expected = "must not be NaN")]
    fn constructor_rejects_nan() {
        FontVariationSettings::new([FontVariation::new(wght(), f64::NAN)]);
    }

    #[test]
    fn infinite_values_are_accepted_and_round_trip() {
        // Infinities clamp to the axis range when applied, like any out-of-range value.
        let settings = FontVariationSettings::new([
            FontVariation::new(wght(), f64::INFINITY),
            FontVariation::new(opsz(), f64::NEG_INFINITY),
        ]);

        assert_eq!(settings.try_get_value(wght()), Some(f64::INFINITY));

        let round_tripped = FontVariationSettings::parse(&settings.to_string()).unwrap();

        assert_eq!(settings, round_tripped);
        assert_eq!(round_tripped.try_get_value(opsz()), Some(f64::NEG_INFINITY));
    }

    #[test]
    fn try_get_value_misses_report_none() {
        let settings = FontVariationSettings::parse("wght=700").unwrap();

        assert_eq!(settings.try_get_value(opsz()), None);
    }

    #[test]
    fn variation_to_string_is_the_pair_form() {
        assert_eq!(FontVariation::new(wght(), 700.0).to_string(), "wght=700");
        assert_eq!(FontVariation::new(opsz(), 14.25).to_string(), "opsz=14.25");
    }

    #[test]
    fn values_use_the_invariant_number_format() {
        for (value, text) in [
            (1e15, "wght=1E+15"),
            (1.5e21, "wght=1.5E+21"),
            (123456789012345.0, "wght=123456789012345"),
            (0.0001, "wght=0.0001"),
            (0.00001, "wght=1E-05"),
            (-0.5, "wght=-0.5"),
            (0.0, "wght=0"),
        ] {
            let variation = FontVariation::new(wght(), value);

            assert_eq!(variation.to_string(), text);
            assert_eq!(FontVariationSettings::parse(text).unwrap().try_get_value(wght()), Some(value));
        }

        assert_eq!(FontVariationSettings::parse("wght=+.5e1").unwrap().try_get_value(wght()), Some(5.0));
        assert_eq!(FontVariationSettings::parse("wght=5.").unwrap().try_get_value(wght()), Some(5.0));
    }
}
