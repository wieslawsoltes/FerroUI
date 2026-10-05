use crate::media::IDashStyle;
use std::any::Any;
use std::rc::Rc;

/// Represents the sequence of dashes and gaps that will be applied by an
/// [`ImmutablePen`](crate::media::immutable::ImmutablePen).
#[derive(Clone, Debug)]
pub struct ImmutableDashStyle {
    dashes: Box<[f64]>,
    offset: f64,
}

impl ImmutableDashStyle {
    /// Creates a dash style. `None` dashes produce an empty sequence.
    pub fn new(dashes: Option<&[f64]>, offset: f64) -> Self {
        Self { dashes: dashes.unwrap_or(&[]).into(), offset }
    }

    /// The length of alternating dashes and gaps.
    #[inline]
    pub fn dashes(&self) -> &[f64] {
        &self.dashes
    }

    /// How far in the dash sequence the stroke will start.
    #[inline]
    pub fn offset(&self) -> f64 {
        self.offset
    }

    fn sequence_equal(left: &[f64], right: Option<&[f64]>) -> bool {
        match right {
            Some(right) => left == right,
            None => false,
        }
    }
}

impl PartialEq for ImmutableDashStyle {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset && self.dashes == other.dashes
    }
}

impl IDashStyle for ImmutableDashStyle {
    fn dashes(&self) -> Option<Vec<f64>> {
        Some(self.dashes.to_vec())
    }

    #[inline]
    fn offset(&self) -> f64 {
        self.offset
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_immutable_dash_style(self: Rc<Self>) -> Rc<ImmutableDashStyle> {
        self
    }

    fn equals(&self, other: &dyn IDashStyle) -> bool {
        if self.reference_id() == other.reference_id() {
            return true;
        }

        if let Some(other) = other.as_any().downcast_ref::<ImmutableDashStyle>() {
            return self == other;
        }

        self.offset == other.offset() && Self::sequence_equal(&self.dashes, other.dashes().as_deref())
    }
}
