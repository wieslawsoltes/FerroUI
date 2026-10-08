use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::{CompositionProperty, IServerClockItem, ServerCompositor, ServerObjectId};
use std::rc::Rc;
use std::time::Duration;

/// A running animation of one property of a server object.
///
/// An instance is created on the UI thread by the animation it belongs to,
/// crosses to the server in the batch that starts it, and is driven there.
/// It names its target, and the objects its reference parameters refer to,
/// by id: the UI thread never holds a server object. The ids are resolved
/// on the server by [`resolve`](Self::resolve), which the animations of the
/// target call before [`initialize`](Self::initialize).
pub trait IAnimationInstance: IServerClockItem + 'static {
    /// The object whose property is animated.
    fn target_object(&self) -> ServerObjectId;

    /// Resolves the server objects the instance refers to by id, on the
    /// compositor that drives it. Called once, on the server, before the
    /// instance is initialized; an instance that refers to no object needs
    /// nothing.
    fn resolve(&self, _compositor: &Rc<ServerCompositor>) {}

    /// The value of the animation at `now`.
    fn evaluate(&self, now: Duration, current_value: ExpressionVariant) -> ExpressionVariant;

    /// Attaches the instance to the property it animates.
    fn initialize(&self, started_at: Duration, starting_value: ExpressionVariant, property: &'static CompositionProperty);

    fn activate(&self);

    fn deactivate(&self);

    fn invalidate(&self);
}

/// An animation instance on its way to the server.
///
/// An instance belongs to the server: it resolves server objects and keeps
/// its state in cells, so it cannot cross threads. What the UI thread
/// creates and a batch carries is the factory of the instance, which holds
/// what the instance is made of (all of it `Send`) and is called on the
/// server.
pub struct AnimationInstanceFactory(Box<dyn FnOnce() -> Rc<dyn IAnimationInstance> + Send>);

impl AnimationInstanceFactory {
    pub fn new(create: impl FnOnce() -> Rc<dyn IAnimationInstance> + Send + 'static) -> Self {
        Self(Box::new(create))
    }

    /// Creates the instance. Called on the thread that runs it.
    pub fn create(self) -> Rc<dyn IAnimationInstance> {
        (self.0)()
    }
}
