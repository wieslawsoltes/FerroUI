//! WinUI's `Timeline` (`Microsoft.UI.Xaml.Media.Animation.Timeline`;
//! `dxaml/xcp/core/animation/Timeline.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md). The timing arithmetic is `ferroui_base::animation::TimelineTiming`.

use ferroui_base::animation::{FillBehavior, RepeatBehavior, SignedDuration, TimelineDuration, TimelineTiming};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, FerroObject, FerroObjectImpl, FerroProperty,
    Ref, StyledProperty, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

type CompletedHandler = dyn Fn(&Ref<Timeline>);

/// The base class of the storyboard family: the timing of a [`Storyboard`](super::Storyboard)
/// and of the animations it holds.
///
/// `BeginTime` (`None`: never starts), `Duration`, `AutoReverse`, `RepeatBehavior`, `SpeedRatio`
/// and `FillBehavior` have WinUI's meaning and defaults; the times of a timeline are relative to
/// its parent storyboard.
#[repr(C)]
pub struct Timeline {
    base: FerroObject,
    completed: HandlerList<CompletedHandler>,
    // The target object given in code (WinUI's `Storyboard.SetTarget`).
    target: RefCell<Option<WeakRef<FerroObject>>>,
    // The target property given in code (the generated animations of a visual transition).
    property: Cell<Option<&'static FerroProperty>>,
}

ferro_class!(Timeline: FerroObject);
ferro_class_info!(Timeline { markup: { namespace: "FerroUI.Animation" } });
ferro_impl_classes!(Timeline: FerroObjectImpl);

ferro_properties! { impl Timeline {
    /// Defines the `BeginTime` property: when the timeline starts, relative to the start of its
    /// parent. Default zero; `None` never starts.
    pub fn begin_time_property() -> StyledProperty<Option<SignedDuration>> {
        FerroProperty::register::<Timeline, _>("BeginTime", Some(SignedDuration::ZERO))
    }

    /// Defines the `Duration` property: the length of one iteration. Default
    /// [`TimelineDuration::Automatic`] (one second for an animation from, to or by a value, the
    /// last key time for a key frame animation, the end of the last child for a storyboard).
    pub fn duration_property() -> StyledProperty<TimelineDuration> {
        FerroProperty::register::<Timeline, _>("Duration", TimelineDuration::Automatic)
    }

    /// Defines the `AutoReverse` property: whether each iteration runs back after it runs
    /// forward. Default `false`.
    pub fn auto_reverse_property() -> StyledProperty<bool> {
        FerroProperty::register::<Timeline, _>("AutoReverse", false)
    }

    /// Defines the `RepeatBehavior` property. Default one iteration (`1x`).
    pub fn repeat_behavior_property() -> StyledProperty<RepeatBehavior> {
        FerroProperty::register::<Timeline, _>("RepeatBehavior", RepeatBehavior::Count(1.0))
    }

    /// Defines the `SpeedRatio` property: the rate of the time of the timeline relative to its
    /// parent (a ratio that is not positive is 1). Default 1.
    pub fn speed_ratio_property() -> StyledProperty<f64> {
        FerroProperty::register::<Timeline, _>("SpeedRatio", 1.0)
    }

    /// Defines the `FillBehavior` property: what the timeline does after its active period.
    /// Default [`FillBehavior::HoldEnd`].
    pub fn fill_behavior_property() -> StyledProperty<FillBehavior> {
        FerroProperty::register::<Timeline, _>("FillBehavior", FillBehavior::HoldEnd)
    }
} }

impl Timeline {
    /// Creates the class data of a derived timeline.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            completed: HandlerList::new(),
            target: RefCell::new(None),
            property: Cell::new(None),
        }
    }

    pub fn begin_time(&self) -> Option<SignedDuration> {
        self.get_value(Self::begin_time_property())
    }

    pub fn set_begin_time(&self, value: Option<SignedDuration>) {
        self.set_value(Self::begin_time_property(), value);
    }

    pub fn duration(&self) -> TimelineDuration {
        self.get_value(Self::duration_property())
    }

    pub fn set_duration(&self, value: TimelineDuration) {
        self.set_value(Self::duration_property(), value);
    }

    pub fn auto_reverse(&self) -> bool {
        self.get_value(Self::auto_reverse_property())
    }

    pub fn set_auto_reverse(&self, value: bool) {
        self.set_value(Self::auto_reverse_property(), value);
    }

    pub fn repeat_behavior(&self) -> RepeatBehavior {
        self.get_value(Self::repeat_behavior_property())
    }

    pub fn set_repeat_behavior(&self, value: RepeatBehavior) {
        self.set_value(Self::repeat_behavior_property(), value);
    }

    pub fn speed_ratio(&self) -> f64 {
        self.get_value(Self::speed_ratio_property())
    }

    pub fn set_speed_ratio(&self, value: f64) {
        self.set_value(Self::speed_ratio_property(), value);
    }

    pub fn fill_behavior(&self) -> FillBehavior {
        self.get_value(Self::fill_behavior_property())
    }

    pub fn set_fill_behavior(&self, value: FillBehavior) {
        self.set_value(Self::fill_behavior_property(), value);
    }

    /// The timing of the timeline, from its properties.
    pub fn timing(&self) -> TimelineTiming {
        TimelineTiming {
            // A negative begin time is not a time of the timing model; it starts at once.
            begin_time: self
                .begin_time()
                .map(|begin| begin.to_duration().unwrap_or(Duration::ZERO)),
            duration: self.duration(),
            auto_reverse: self.auto_reverse(),
            repeat_behavior: self.repeat_behavior(),
            speed_ratio: self.speed_ratio(),
            fill_behavior: self.fill_behavior(),
        }
    }

    /// Adds a handler of `Completed`, raised when the active period of the timeline ends (not
    /// when it is stopped).
    ///
    /// Dropping the returned guard removes the handler; [`Subscription::detach`] keeps it for the
    /// life of the timeline.
    pub fn completed(&self, handler: impl Fn(&Ref<Timeline>) + 'static) -> Subscription {
        let token = self.completed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.completed.remove(token);
            }
        }))
    }

    pub(crate) fn raise_completed(&self) {
        let this = self.to_ref();
        for (_, handler) in self.completed.snapshot().iter() {
            handler(&this);
        }
    }

    /// The target given in code, while it is alive.
    pub(crate) fn explicit_target(&self) -> Option<Ref<FerroObject>> {
        self.target.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_explicit_target(&self, target: Option<&FerroObject>) {
        *self.target.borrow_mut() = target.map(|target| target.to_ref().downgrade());
    }

    pub(crate) fn explicit_property(&self) -> Option<&'static FerroProperty> {
        self.property.get()
    }

    pub(crate) fn set_explicit_property(&self, property: Option<&'static FerroProperty>) {
        self.property.set(property);
    }
}
