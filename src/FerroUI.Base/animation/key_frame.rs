use crate::animation::{Cue, IAnimationSetter, KeySpline, TimeSpan};
use crate::collections::FerroList;
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// How the position of a key frame within an iteration is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub(crate) enum KeyFrameTimingMode {
    #[default]
    Unset = 0,
    TimeSpan = 1,
    Cue = 2,
}

/// Stores data regarding a specific key point and value in an animation.
#[repr(C)]
pub struct KeyFrame {
    base: FerroObject,
    k_time_span: Cell<TimeSpan>,
    k_cue: Cell<Cue>,
    k_key_spline: RefCell<Option<Ref<KeySpline>>>,
    setters: FerroList<Rc<dyn IAnimationSetter>>,
    timing_mode: Cell<KeyFrameTimingMode>,
}

ferro_class!(KeyFrame: FerroObject);
crate::ferro_class_info!(KeyFrame { new: KeyFrame::new });
ferro_impl_classes!(KeyFrame: FerroObjectImpl);

impl KeyFrame {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            k_time_span: Cell::new(TimeSpan::ZERO),
            k_cue: Cell::new(Cue::default()),
            k_key_spline: RefCell::new(None),
            setters: FerroList::new(),
            timing_mode: Cell::new(KeyFrameTimingMode::Unset),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame at `cue` with the given setters.
    pub fn with_cue(cue: Cue, setters: impl IntoIterator<Item = Rc<dyn IAnimationSetter>>) -> Ref<Self> {
        let result = Self::new();
        result.set_cue(cue);
        result.setters.add_range(setters);
        result
    }

    /// Creates a key frame at `key_time` with the given setters.
    pub fn with_key_time(key_time: TimeSpan, setters: impl IntoIterator<Item = Rc<dyn IAnimationSetter>>) -> Ref<Self> {
        let result = Self::new();
        result.set_key_time(key_time);
        result.setters.add_range(setters);
        result
    }

    /// The setters of the key frame: the values the animated properties
    /// have at its position.
    pub fn setters(&self) -> FerroList<Rc<dyn IAnimationSetter>> {
        self.setters.clone()
    }

    pub(crate) fn timing_mode(&self) -> KeyFrameTimingMode {
        self.timing_mode.get()
    }

    /// The key time of this key frame.
    pub fn key_time(&self) -> TimeSpan {
        self.k_time_span.get()
    }

    /// Sets the key time of this key frame. Panics when the cue has been
    /// set: only one of them can be.
    pub fn set_key_time(&self, value: TimeSpan) {
        if self.timing_mode.get() == KeyFrameTimingMode::Cue {
            panic!("You can only set either KeyTime or Cue.");
        }
        self.timing_mode.set(KeyFrameTimingMode::TimeSpan);
        self.k_time_span.set(value);
    }

    /// The cue of this key frame.
    pub fn cue(&self) -> Cue {
        self.k_cue.get()
    }

    /// Sets the cue of this key frame. Panics when the key time has been
    /// set: only one of them can be.
    pub fn set_cue(&self, value: Cue) {
        if self.timing_mode.get() == KeyFrameTimingMode::TimeSpan {
            panic!("You can only set either KeyTime or Cue.");
        }
        self.timing_mode.set(KeyFrameTimingMode::Cue);
        self.k_cue.set(value);
    }

    /// The key spline of this key frame.
    pub fn key_spline(&self) -> Option<Ref<KeySpline>> {
        self.k_key_spline.borrow().clone()
    }

    /// Sets the key spline of this key frame. Panics when the spline is not
    /// valid; it is stored nevertheless.
    pub fn set_key_spline(&self, value: Option<Ref<KeySpline>>) {
        let valid = value.as_ref().is_none_or(|value| value.is_valid());
        *self.k_key_spline.borrow_mut() = value;
        if !valid {
            panic!("KeySpline must have X coordinates >= 0.0 and <= 1.0.");
        }
    }
}
