use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::media::fonts::OpenTypeTag;

/// A single normalized variation coordinate: an axis tag paired with a value
/// in `[-1, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NormalizedVariationCoordinate {
    pub axis: OpenTypeTag,
    pub normalized_value: f32,
}

impl NormalizedVariationCoordinate {
    pub const fn new(axis: OpenTypeTag, normalized_value: f32) -> Self {
        Self { axis, normalized_value }
    }
}

/// Why a set of coordinates is not a valid [`NormalizedVariationPosition`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NormalizedVariationPositionError {
    /// A coordinate is NaN or outside of `[-1, 1]` (C# `ArgumentOutOfRangeException`).
    OutOfRange { axis: OpenTypeTag, value: f32 },
    /// An axis appears more than once (C# `ArgumentException`).
    DuplicateAxis { axis: OpenTypeTag },
}

impl fmt::Display for NormalizedVariationPositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { axis, value } => {
                write!(f, "Normalized coordinate for axis '{axis}' must be in [-1, 1]; was {value}.")
            }
            Self::DuplicateAxis { axis } => write!(f, "Duplicate axis '{axis}' in coordinates."),
        }
    }
}

impl std::error::Error for NormalizedVariationPositionError {}

/// An immutable, value-equal position in a variable font's normalized design
/// space: per-axis coordinates in `[-1, 1]` (the post-`avar` form consumed
/// directly by `gvar`, `HVAR`, `MVAR` and friends).
///
/// This is the structural identity of a variation - it is the cache key for
/// `gvar` deltas, and equal positions select equal instances. Coordinates are
/// sorted by axis tag so that equality and hashing are independent of
/// insertion order.
///
/// The default value (no coordinates) represents the "no variation" case -
/// every axis at its default. A coordinate whose normalized value is `0` is
/// equivalent to an absent one; the `from_coordinates` factories drop such
/// coordinates so that both forms yield structurally-equal positions.
#[derive(Clone, Debug, Default)]
pub(crate) struct NormalizedVariationPosition {
    coordinates: Option<Rc<[NormalizedVariationCoordinate]>>,
    hash_code: i32,
}

#[allow(dead_code)] // used by the variable font support, which is not ported yet
impl NormalizedVariationPosition {
    fn new(sorted_coordinates: Vec<NormalizedVariationCoordinate>) -> Self {
        let hash_code = Self::compute_hash_code(&sorted_coordinates);

        Self { coordinates: Some(Rc::from(sorted_coordinates)), hash_code }
    }

    /// The per-axis normalized coordinates in `[-1, 1]`, sorted by axis tag.
    /// Empty when the position is the default instance.
    pub fn coordinates(&self) -> &[NormalizedVariationCoordinate] {
        self.coordinates.as_deref().unwrap_or(&[])
    }

    /// `true` when no coordinates are set - the default instance.
    pub fn is_default(&self) -> bool {
        self.coordinates().is_empty()
    }

    /// Builds a position from a set of normalized coordinates. Each value
    /// must be in `[-1, 1]` and not NaN; zero-valued coordinates are dropped
    /// (they equal the axis default).
    pub fn from_coordinate_map(
        normalized_coordinates: &HashMap<OpenTypeTag, f32>,
    ) -> Result<NormalizedVariationPosition, NormalizedVariationPositionError> {
        if normalized_coordinates.is_empty() {
            return Ok(Self::default());
        }

        let mut builder = Vec::with_capacity(normalized_coordinates.len());

        for (&axis, &value) in normalized_coordinates {
            Self::validate_coordinate(value, axis)?;
            builder.push(NormalizedVariationCoordinate::new(axis, value));
        }

        Self::create_from_validated(builder)
    }

    /// Builds a position from a sequence of normalized coordinates. Each
    /// value must be in `[-1, 1]` and not NaN; duplicate axes are rejected;
    /// zero-valued coordinates are dropped (they equal the axis default).
    pub fn from_coordinates(
        normalized_coordinates: &[NormalizedVariationCoordinate],
    ) -> Result<NormalizedVariationPosition, NormalizedVariationPositionError> {
        if normalized_coordinates.is_empty() {
            return Ok(Self::default());
        }

        for coordinate in normalized_coordinates {
            Self::validate_coordinate(coordinate.normalized_value, coordinate.axis)?;
        }

        Self::create_from_validated(normalized_coordinates.to_vec())
    }

    fn create_from_validated(
        mut builder: Vec<NormalizedVariationCoordinate>,
    ) -> Result<NormalizedVariationPosition, NormalizedVariationPositionError> {
        builder.sort_by_key(|coordinate| coordinate.axis.value());

        for i in 1..builder.len() {
            if builder[i].axis == builder[i - 1].axis {
                return Err(NormalizedVariationPositionError::DuplicateAxis { axis: builder[i].axis });
            }
        }

        // A normalized value of 0 is the axis default: dropping it keeps explicitly-default
        // positions structurally equal to positions that omit the axis, so both produce one
        // cache key (and one variation clone) instead of two. Done after the duplicate check
        // so that duplicates are still rejected regardless of their values.
        builder.retain(|coordinate| coordinate.normalized_value != 0.0);

        if builder.is_empty() {
            return Ok(Self::default());
        }

        Ok(Self::new(builder))
    }

    /// Tries to get the normalized coordinate for an axis.
    pub fn try_get_coordinate(&self, axis: OpenTypeTag) -> Option<f32> {
        self.coordinates()
            .iter()
            .find(|coordinate| coordinate.axis == axis)
            .map(|coordinate| coordinate.normalized_value)
    }

    /// Returns the normalized coordinate for an axis, or `fallback` if absent.
    pub fn get_coordinate_or_default(&self, axis: OpenTypeTag, fallback: f32) -> f32 {
        self.try_get_coordinate(axis).unwrap_or(fallback)
    }

    /// The hash computed at construction; `0` for the default instance.
    pub fn get_hash_code(&self) -> i32 {
        self.hash_code
    }

    fn validate_coordinate(value: f32, axis: OpenTypeTag) -> Result<(), NormalizedVariationPositionError> {
        if value.is_nan() || !(-1.0..=1.0).contains(&value) {
            return Err(NormalizedVariationPositionError::OutOfRange { axis, value });
        }

        Ok(())
    }

    fn compute_hash_code(coordinates: &[NormalizedVariationCoordinate]) -> i32 {
        if coordinates.is_empty() {
            return 0;
        }

        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        for coordinate in coordinates {
            coordinate.axis.hash(&mut hasher);
            // Zero values are dropped, so equal values have equal bits.
            hasher.write_u32(coordinate.normalized_value.to_bits());
        }

        let hash = hasher.finish();

        (hash ^ (hash >> 32)) as u32 as i32
    }
}

impl PartialEq for NormalizedVariationPosition {
    fn eq(&self, other: &Self) -> bool {
        // Cheap reject via the cached hash first; then an allocation-free element-wise
        // compare. `coordinates` normalizes the default instance to an empty slice, so the
        // slices are always valid.
        if self.hash_code != other.hash_code {
            return false;
        }

        self.coordinates() == other.coordinates()
    }
}

impl Eq for NormalizedVariationPosition {}

impl Hash for NormalizedVariationPosition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.hash_code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NormalizedVariationPositionError::{DuplicateAxis, OutOfRange};

    fn wght() -> OpenTypeTag {
        OpenTypeTag::parse("wght")
    }

    fn wdth() -> OpenTypeTag {
        OpenTypeTag::parse("wdth")
    }

    fn ital() -> OpenTypeTag {
        OpenTypeTag::parse("ital")
    }

    fn from_pairs(pairs: &[(OpenTypeTag, f32)]) -> NormalizedVariationPosition {
        NormalizedVariationPosition::from_coordinate_map(&pairs.iter().copied().collect()).unwrap()
    }

    #[test]
    fn default_struct_is_the_no_variation_case() {
        let settings = NormalizedVariationPosition::default();

        assert!(settings.is_default());
        assert!(settings.coordinates().is_empty());
        assert_eq!(settings.get_hash_code(), 0);
    }

    #[test]
    fn default_structs_are_equal() {
        assert_eq!(NormalizedVariationPosition::default(), NormalizedVariationPosition::default());
    }

    #[test]
    fn from_coordinates_dictionary_rejects_out_of_range_or_nan() {
        for value in [f32::NAN, -1.0001, 1.0001, f32::INFINITY, f32::NEG_INFINITY] {
            let coords = HashMap::from([(wght(), value)]);

            assert!(matches!(
                NormalizedVariationPosition::from_coordinate_map(&coords),
                Err(OutOfRange { axis, .. }) if axis == wght()
            ));
        }
    }

    #[test]
    fn from_coordinates_dictionary_accepts_boundary_values() {
        for value in [-1.0f32, 1.0] {
            let settings = from_pairs(&[(wght(), value)]);

            assert_eq!(settings.coordinates(), [NormalizedVariationCoordinate::new(wght(), value)]);
        }
    }

    #[test]
    fn from_coordinates_drops_zero_coordinates() {
        // 0 is the axis default: an explicit wght=0 must produce the same value (and the
        // same variation-cache key downstream) as settings that omit the axis entirely.
        let from_dictionary = from_pairs(&[(wght(), 0.0)]);

        assert!(from_dictionary.is_default());
        assert_eq!(from_dictionary, NormalizedVariationPosition::default());

        let from_span = NormalizedVariationPosition::from_coordinates(&[
            NormalizedVariationCoordinate::new(wght(), 0.0),
            NormalizedVariationCoordinate::new(wdth(), -0.25),
        ])
        .unwrap();

        assert_eq!(from_span.coordinates().len(), 1);
        assert_eq!(from_span.coordinates()[0].axis, wdth());
    }

    #[test]
    fn from_coordinates_span_rejects_duplicate_axes_even_when_zero_valued() {
        // Canonicalization must not weaken validation: the duplicate check runs before
        // zero-valued coordinates are dropped.
        let result = NormalizedVariationPosition::from_coordinates(&[
            NormalizedVariationCoordinate::new(wght(), 0.0),
            NormalizedVariationCoordinate::new(wght(), 0.5),
        ]);

        assert_eq!(result, Err(DuplicateAxis { axis: wght() }));
    }

    #[test]
    fn from_coordinates_dictionary_empty_returns_default_struct() {
        let settings = NormalizedVariationPosition::from_coordinate_map(&HashMap::new()).unwrap();

        assert!(settings.is_default());
        assert_eq!(settings, NormalizedVariationPosition::default());
    }

    #[test]
    fn from_coordinates_dictionary_sorts_by_axis_tag() {
        // Insertion order shouldn't matter - coordinates land sorted by the tag value
        // so equality and hashing are insertion-order-independent.
        let settings = from_pairs(&[(wght(), 0.5), (ital(), 1.0), (wdth(), -0.25)]);

        // Sorted: ital (0x6974616c), wdth (0x77647468), wght (0x77676874).
        let axes: Vec<OpenTypeTag> = settings.coordinates().iter().map(|coordinate| coordinate.axis).collect();

        assert_eq!(axes, [ital(), wdth(), wght()]);
    }

    #[test]
    fn from_coordinates_dictionary_defensively_copies_the_input() {
        let mut mutable = HashMap::from([(wght(), 0.5f32)]);

        let settings = NormalizedVariationPosition::from_coordinate_map(&mutable).unwrap();

        mutable.insert(wght(), 0.9);
        mutable.insert(wdth(), -0.25);

        assert_eq!(settings.coordinates(), [NormalizedVariationCoordinate::new(wght(), 0.5)]);
    }

    #[test]
    fn from_coordinates_span_empty_returns_default_struct() {
        assert!(NormalizedVariationPosition::from_coordinates(&[]).unwrap().is_default());
    }

    #[test]
    fn from_coordinates_span_sorts_and_validates() {
        let settings = NormalizedVariationPosition::from_coordinates(&[
            NormalizedVariationCoordinate::new(wght(), 0.5),
            NormalizedVariationCoordinate::new(ital(), 1.0),
            NormalizedVariationCoordinate::new(wdth(), -0.25),
        ])
        .unwrap();

        let axes: Vec<OpenTypeTag> = settings.coordinates().iter().map(|coordinate| coordinate.axis).collect();

        assert_eq!(axes, [ital(), wdth(), wght()]);
    }

    #[test]
    fn from_coordinates_span_rejects_duplicate_axes() {
        let result = NormalizedVariationPosition::from_coordinates(&[
            NormalizedVariationCoordinate::new(wght(), 0.5),
            NormalizedVariationCoordinate::new(wght(), -0.5),
        ]);

        assert_eq!(result, Err(DuplicateAxis { axis: wght() }));
    }

    #[test]
    fn from_coordinates_span_rejects_out_of_range_value() {
        let result = NormalizedVariationPosition::from_coordinates(&[NormalizedVariationCoordinate::new(wght(), 2.0)]);

        assert_eq!(result, Err(OutOfRange { axis: wght(), value: 2.0 }));
        assert!(result.unwrap_err().to_string().contains("must be in [-1, 1]"));
    }

    #[test]
    fn try_get_coordinate_returns_value_for_present_axis() {
        let settings = from_pairs(&[(wght(), 0.5), (wdth(), -0.25)]);

        assert_eq!(settings.try_get_coordinate(wght()), Some(0.5));
        assert_eq!(settings.try_get_coordinate(wdth()), Some(-0.25));
    }

    #[test]
    fn try_get_coordinate_returns_none_for_absent_axis() {
        assert_eq!(from_pairs(&[(wght(), 0.5)]).try_get_coordinate(ital()), None);
    }

    #[test]
    fn try_get_coordinate_returns_none_for_default_struct() {
        assert_eq!(NormalizedVariationPosition::default().try_get_coordinate(wght()), None);
    }

    #[test]
    fn get_coordinate_or_default_returns_fallback_for_absent_axis() {
        let settings = from_pairs(&[(wght(), 0.5)]);

        assert_eq!(settings.get_coordinate_or_default(wght(), 0.0), 0.5);
        assert_eq!(settings.get_coordinate_or_default(ital(), 0.0), 0.0);
        assert_eq!(settings.get_coordinate_or_default(ital(), -1.0), -1.0);
    }

    #[test]
    fn equality_is_reflexive() {
        let settings = from_pairs(&[(wght(), 0.5)]);

        assert_eq!(settings, settings.clone());
    }

    #[test]
    fn equality_is_structural_for_identical_coordinates() {
        let a = from_pairs(&[(wght(), 0.5), (wdth(), -0.25)]);
        let b = from_pairs(&[(wght(), 0.5), (wdth(), -0.25)]);

        assert_eq!(a, b);
        assert_eq!(b, a);
        assert_eq!(a.get_hash_code(), b.get_hash_code());
    }

    #[test]
    fn equality_ignores_insertion_order() {
        let a = NormalizedVariationPosition::from_coordinates(&[
            NormalizedVariationCoordinate::new(wght(), 0.5),
            NormalizedVariationCoordinate::new(wdth(), -0.25),
        ])
        .unwrap();
        let b = NormalizedVariationPosition::from_coordinates(&[
            NormalizedVariationCoordinate::new(wdth(), -0.25),
            NormalizedVariationCoordinate::new(wght(), 0.5),
        ])
        .unwrap();

        assert_eq!(a, b);
        assert_eq!(a.get_hash_code(), b.get_hash_code());
    }

    #[test]
    fn equality_differs_when_a_coordinate_value_differs() {
        assert_ne!(from_pairs(&[(wght(), 0.5)]), from_pairs(&[(wght(), 0.6)]));
    }

    #[test]
    fn equality_differs_when_a_coordinate_key_differs() {
        assert_ne!(from_pairs(&[(wght(), 0.5)]), from_pairs(&[(wdth(), 0.5)]));
    }

    #[test]
    fn equality_differs_when_coordinate_counts_differ() {
        // The second coordinate must be non-zero: zero coordinates canonicalize away,
        // which would make these two values deliberately equal.
        assert_ne!(from_pairs(&[(wght(), 0.5)]), from_pairs(&[(wght(), 0.5), (wdth(), -0.25)]));
    }

    #[test]
    fn equality_differs_between_default_and_populated() {
        let a = NormalizedVariationPosition::default();
        let b = from_pairs(&[(wght(), 0.5)]);

        assert_ne!(a, b);
        assert_ne!(b, a);
    }

    #[test]
    fn hash_is_cached_and_stable_across_equal_instances() {
        // The hash is computed once at construction. Two structurally-equal positions
        // must have the same hash regardless of how the coordinates were inserted.
        let a = from_pairs(&[(wght(), 0.5), (wdth(), -0.25), (ital(), 1.0)]);
        let b = from_pairs(&[(ital(), 1.0), (wght(), 0.5), (wdth(), -0.25)]);

        let hash_a = a.get_hash_code();

        assert_eq!(hash_a, a.get_hash_code());
        assert_eq!(hash_a, b.get_hash_code());
    }

    #[test]
    fn equality_operators_match_equals() {
        let a = from_pairs(&[(ital(), 1.0)]);
        let b = from_pairs(&[(ital(), 1.0)]);
        let c = from_pairs(&[(ital(), 0.0)]);

        assert!(a == b);
        assert!(a != c);
    }

    #[test]
    fn normalized_variation_coordinate_has_structural_equality() {
        let a = NormalizedVariationCoordinate::new(wght(), 0.5);
        let b = NormalizedVariationCoordinate::new(wght(), 0.5);
        let c = NormalizedVariationCoordinate::new(wght(), 0.6);
        let d = NormalizedVariationCoordinate::new(wdth(), 0.5);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }
}
