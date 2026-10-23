//! The inertia modifiers of an [`InteractionTracker`](super::InteractionTracker):
//! `InteractionTrackerInertiaModifier` and its subclasses.

use crate::rendering::composition::animations::ExpressionAnimation;
use crate::rendering::composition::Compositor;
use std::cell::RefCell;
use std::rc::Rc;

/// A modifier of the inertia of one axis of an interaction tracker
/// (`InteractionTrackerInertiaModifier`): what
/// [`configure_position_x_inertia_modifiers`](super::InteractionTracker::configure_position_x_inertia_modifiers)
/// and its siblings take.
///
/// When the tracker enters inertia it evaluates the conditions of the
/// modifiers of an axis in order; the first modifier whose condition is
/// `true` applies to the axis for the whole inertia.
///
/// Deviation (winui-composition.md): WinUI also has
/// `InteractionTrackerInertiaNaturalMotion`, which runs a
/// `ScalarNaturalMotionAnimation`; the natural motion animations are not
/// ported yet, so that subclass is missing.
#[derive(Clone)]
pub enum InteractionTrackerInertiaModifier {
    /// `InteractionTrackerInertiaRestingValue`: replaces the resting value.
    RestingValue(Rc<InteractionTrackerInertiaRestingValue>),
    /// `InteractionTrackerInertiaMotion`: replaces the motion.
    Motion(Rc<InteractionTrackerInertiaMotion>),
}

impl InteractionTrackerInertiaModifier {
    /// The condition of the modifier.
    pub fn condition(&self) -> Option<Rc<ExpressionAnimation>> {
        match self {
            Self::RestingValue(modifier) => modifier.condition(),
            Self::Motion(modifier) => modifier.condition(),
        }
    }
}

impl From<Rc<InteractionTrackerInertiaRestingValue>> for InteractionTrackerInertiaModifier {
    fn from(value: Rc<InteractionTrackerInertiaRestingValue>) -> Self {
        Self::RestingValue(value)
    }
}

impl From<Rc<InteractionTrackerInertiaMotion>> for InteractionTrackerInertiaModifier {
    fn from(value: Rc<InteractionTrackerInertiaMotion>) -> Self {
        Self::Motion(value)
    }
}

/// An inertia modifier that changes where the inertia comes to rest
/// (`InteractionTrackerInertiaRestingValue`): when `Condition` is `true`,
/// the axis comes to rest at the value of `RestingValue`, evaluated once
/// when the inertia starts.
///
/// Both expressions read the tracker as `this.Target`
/// (`this.Target.NaturalRestingPosition.X`, `this.Target.Position`, ...).
pub struct InteractionTrackerInertiaRestingValue {
    compositor: Rc<Compositor>,
    condition: RefCell<Option<Rc<ExpressionAnimation>>>,
    resting_value: RefCell<Option<Rc<ExpressionAnimation>>>,
}

impl InteractionTrackerInertiaRestingValue {
    /// `InteractionTrackerInertiaRestingValue.Create(compositor)`.
    pub fn create(compositor: &Rc<Compositor>) -> Rc<InteractionTrackerInertiaRestingValue> {
        Rc::new(InteractionTrackerInertiaRestingValue {
            compositor: compositor.clone(),
            condition: RefCell::new(None),
            resting_value: RefCell::new(None),
        })
    }

    /// The compositor of the modifier.
    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// The expression that decides whether the modifier applies: a
    /// boolean expression.
    pub fn condition(&self) -> Option<Rc<ExpressionAnimation>> {
        self.condition.borrow().clone()
    }

    pub fn set_condition(&self, value: Option<Rc<ExpressionAnimation>>) {
        *self.condition.borrow_mut() = value;
    }

    /// The expression of the resting value of the axis: a scalar
    /// expression.
    pub fn resting_value(&self) -> Option<Rc<ExpressionAnimation>> {
        self.resting_value.borrow().clone()
    }

    pub fn set_resting_value(&self, value: Option<Rc<ExpressionAnimation>>) {
        *self.resting_value.borrow_mut() = value;
    }
}

/// An inertia modifier that replaces the motion of the inertia
/// (`InteractionTrackerInertiaMotion`): when `Condition` is `true`, the
/// value of the axis is the value of `Motion`, evaluated every frame of the
/// inertia, until it stops changing.
///
/// Deviation (winui-composition.md): WinUI does not document what the value
/// of `Motion` is; here it is the value of the axis for the frame (a
/// position for the position axes, a scale for the scale axis).
pub struct InteractionTrackerInertiaMotion {
    compositor: Rc<Compositor>,
    condition: RefCell<Option<Rc<ExpressionAnimation>>>,
    motion: RefCell<Option<Rc<ExpressionAnimation>>>,
}

impl InteractionTrackerInertiaMotion {
    /// `InteractionTrackerInertiaMotion.Create(compositor)`.
    pub fn create(compositor: &Rc<Compositor>) -> Rc<InteractionTrackerInertiaMotion> {
        Rc::new(InteractionTrackerInertiaMotion {
            compositor: compositor.clone(),
            condition: RefCell::new(None),
            motion: RefCell::new(None),
        })
    }

    /// The compositor of the modifier.
    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// The expression that decides whether the modifier applies: a
    /// boolean expression.
    pub fn condition(&self) -> Option<Rc<ExpressionAnimation>> {
        self.condition.borrow().clone()
    }

    pub fn set_condition(&self, value: Option<Rc<ExpressionAnimation>>) {
        *self.condition.borrow_mut() = value;
    }

    /// The expression of the motion: a scalar expression.
    pub fn motion(&self) -> Option<Rc<ExpressionAnimation>> {
        self.motion.borrow().clone()
    }

    pub fn set_motion(&self, value: Option<Rc<ExpressionAnimation>>) {
        *self.motion.borrow_mut() = value;
    }
}
