use crate::animation::easings::*;
use crate::animation::KeySpline;
use crate::utilities::FormatError;
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

/// An easing function: the shared, type-erased handle that animations and
/// transitions hold.
///
/// Any [`IEasing`] converts into an `Easing` (`LinearEasing::new().into()`).
/// To keep mutating an easing after handing it out (for example the control
/// points of a [`SplineEasing`]), create it behind an `Rc` and use
/// [`from_rc`](Self::from_rc). Handles compare by identity.
#[derive(Clone)]
pub struct Easing(Rc<dyn IEasing>);

impl Easing {
    /// Wraps an easing function.
    pub fn new(easing: impl IEasing) -> Self {
        Self(Rc::new(easing))
    }

    /// Wraps a shared easing function.
    pub fn from_rc(easing: Rc<dyn IEasing>) -> Self {
        Self(easing)
    }

    /// The wrapped easing function.
    pub fn as_rc(&self) -> &Rc<dyn IEasing> {
        &self.0
    }

    /// Returns the transformed progress for a linear progress.
    #[inline]
    pub fn ease(&self, progress: f64) -> f64 {
        self.0.ease(progress)
    }

    /// Parses an easing: either the four comma-separated control points of a
    /// spline easing, or the name of one of the easing types of this module.
    pub fn parse(e: &str) -> Result<Easing, FormatError> {
        if e.contains(',') {
            return Ok(Easing::new(SplineEasing::with_key_spline(KeySpline::parse(e)?)));
        }

        Ok(match e {
            "BackEaseIn" => Easing::new(BackEaseIn::new()),
            "BackEaseInOut" => Easing::new(BackEaseInOut::new()),
            "BackEaseOut" => Easing::new(BackEaseOut::new()),
            "BounceEaseIn" => Easing::new(BounceEaseIn::new()),
            "BounceEaseInOut" => Easing::new(BounceEaseInOut::new()),
            "BounceEaseOut" => Easing::new(BounceEaseOut::new()),
            "CircularEaseIn" => Easing::new(CircularEaseIn::new()),
            "CircularEaseInOut" => Easing::new(CircularEaseInOut::new()),
            "CircularEaseOut" => Easing::new(CircularEaseOut::new()),
            "CubicEaseIn" => Easing::new(CubicEaseIn::new()),
            "CubicEaseInOut" => Easing::new(CubicEaseInOut::new()),
            "CubicEaseOut" => Easing::new(CubicEaseOut::new()),
            "ElasticEaseIn" => Easing::new(ElasticEaseIn::new()),
            "ElasticEaseInOut" => Easing::new(ElasticEaseInOut::new()),
            "ElasticEaseOut" => Easing::new(ElasticEaseOut::new()),
            "ExponentialEaseIn" => Easing::new(ExponentialEaseIn::new()),
            "ExponentialEaseInOut" => Easing::new(ExponentialEaseInOut::new()),
            "ExponentialEaseOut" => Easing::new(ExponentialEaseOut::new()),
            "LinearEasing" => Easing::new(LinearEasing::new()),
            "QuadraticEaseIn" => Easing::new(QuadraticEaseIn::new()),
            "QuadraticEaseInOut" => Easing::new(QuadraticEaseInOut::new()),
            "QuadraticEaseOut" => Easing::new(QuadraticEaseOut::new()),
            "QuarticEaseIn" => Easing::new(QuarticEaseIn::new()),
            "QuarticEaseInOut" => Easing::new(QuarticEaseInOut::new()),
            "QuarticEaseOut" => Easing::new(QuarticEaseOut::new()),
            "QuinticEaseIn" => Easing::new(QuinticEaseIn::new()),
            "QuinticEaseInOut" => Easing::new(QuinticEaseInOut::new()),
            "QuinticEaseOut" => Easing::new(QuinticEaseOut::new()),
            "SineEaseIn" => Easing::new(SineEaseIn::new()),
            "SineEaseInOut" => Easing::new(SineEaseInOut::new()),
            "SineEaseOut" => Easing::new(SineEaseOut::new()),
            "SplineEasing" => Easing::new(SplineEasing::new()),
            "SpringEasing" => Easing::new(SpringEasing::new()),
            _ => {
                return Err(FormatError::from_string(format!(
                    "Easing \"{e}\" was not found in the easings module."
                )))
            }
        })
    }
}

impl IEasing for Easing {
    #[inline]
    fn ease(&self, progress: f64) -> f64 {
        self.0.ease(progress)
    }

    #[inline]
    fn derives_from_easing(&self) -> bool {
        self.0.derives_from_easing()
    }
}

impl Default for Easing {
    /// The linear easing.
    fn default() -> Self {
        Easing::new(LinearEasing::new())
    }
}

impl PartialEq for Easing {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl fmt::Debug for Easing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Easing")
    }
}

impl FromStr for Easing {
    type Err = FormatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Easing::parse(s)
    }
}

macro_rules! easing_from {
    ($($name:ident),+ $(,)?) => {
        $(impl From<$name> for Easing {
            fn from(value: $name) -> Self {
                Easing::new(value)
            }
        })+
    };
}

easing_from!(
    BackEaseIn, BackEaseInOut, BackEaseOut, BounceEaseIn, BounceEaseInOut, BounceEaseOut, CircularEaseIn, CircularEaseInOut, CircularEaseOut, CubicEaseIn, CubicEaseInOut, CubicEaseOut, ElasticEaseIn, ElasticEaseInOut, ElasticEaseOut, ExponentialEaseIn, ExponentialEaseInOut, ExponentialEaseOut, LinearEasing, QuadraticEaseIn, QuadraticEaseInOut, QuadraticEaseOut, QuarticEaseIn, QuarticEaseInOut, QuarticEaseOut, QuinticEaseIn, QuinticEaseInOut, QuinticEaseOut, SineEaseIn, SineEaseInOut, SineEaseOut, SplineEasing, SpringEasing
);
