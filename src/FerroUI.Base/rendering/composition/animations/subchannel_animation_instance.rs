//! Animations of components of a property: WinUI's sub-channel targets of
//! `StartAnimation` and `StopAnimation` (`Offset.X`, `Scale.XY`,
//! `Translation.Z`).
//!
//! The animation of a sub-channel animates a value of the sub-channel's type
//! (a scalar for one component, a `Vector2` for two, a `Vector3` for three, a
//! `Vector4` for four). It is started as an animation of the whole property:
//! the UI thread wraps the instance of the animation in a
//! `SubchannelAnimation`, which the generated start path of the property
//! sends like any animation, and the server runs a
//! [`SubchannelAnimationInstance`] that evaluates the inner animation and
//! writes its result into the components of the current value of the
//! property. Animations of other components of the same property started
//! later are merged into the running instance on the server, so `Offset.X`
//! and `Offset.Y` run together; an animation of the whole property replaces
//! them all.

use super::{AnimationInstanceFactory, IAnimationInstance, ICompositionAnimation, ICompositionAnimationBase};
use crate::numerics::{Vector2, Vector3, Vector4};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::{CompositionProperty, IServerClockItem, ServerCompositor, ServerObjectId};
use crate::{Vector, Vector3D};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Splits a sub-channel target (`Offset.X`, `scale.xy`) into the property
/// name and the indices of the components (X = 0, Y = 1, Z = 2, W = 3).
/// Returns `None` for a name without a `.`, and for one whose part after
/// the `.` is not made of the letters X, Y, Z and W: a dotted property name
/// of the class itself (the animatable properties of an effect brush,
/// `Blur.BlurAmount`), which the class resolves as a whole. Panics when the
/// part after the `.` is more than four or repeated components (WinUI
/// throws).
///
/// The components are case-insensitive; the property name is matched as
/// given.
pub(crate) fn parse_subchannel(property_name: &str) -> Option<(&str, Vec<usize>)> {
    let (property, channel) = property_name.split_once('.')?;
    if channel.is_empty() || !channel.chars().all(|c| matches!(c.to_ascii_uppercase(), 'X' | 'Y' | 'Z' | 'W')) {
        return None;
    }
    let mut components = Vec::with_capacity(channel.len());
    for c in channel.chars() {
        let index = match c.to_ascii_uppercase() {
            'X' => 0,
            'Y' => 1,
            'Z' => 2,
            'W' => 3,
            _ => panic!("Unknown property {property_name}"),
        };
        if components.contains(&index) {
            panic!("Unknown property {property_name}");
        }
        components.push(index);
    }
    if components.is_empty() || components.len() > 4 || property.is_empty() {
        panic!("Unknown property {property_name}");
    }
    Some((property, components))
}

/// The components of a vector value, with their number.
fn vector_components(value: &ExpressionVariant) -> Option<([f64; 4], usize)> {
    match *value {
        ExpressionVariant::Vector2(v) => Some(([v.x as f64, v.y as f64, 0.0, 0.0], 2)),
        ExpressionVariant::Vector(v) => Some(([v.x, v.y, 0.0, 0.0], 2)),
        ExpressionVariant::Vector3(v) => Some(([v.x as f64, v.y as f64, v.z as f64, 0.0], 3)),
        ExpressionVariant::Vector3D(v) => Some(([v.x, v.y, v.z, 0.0], 3)),
        ExpressionVariant::Vector4(v) => Some(([v.x as f64, v.y as f64, v.z as f64, v.w as f64], 4)),
        _ => None,
    }
}

/// The components of the value of a sub-channel: a scalar or a vector.
fn channel_components(value: &ExpressionVariant) -> Option<([f64; 4], usize)> {
    match *value {
        ExpressionVariant::Double(v) => Some(([v, 0.0, 0.0, 0.0], 1)),
        _ => vector_components(value),
    }
}

/// `value` with its components replaced, of the same type.
fn with_components(value: ExpressionVariant, c: [f64; 4]) -> ExpressionVariant {
    match value {
        ExpressionVariant::Vector2(_) => Vector2::new(c[0] as f32, c[1] as f32).into(),
        ExpressionVariant::Vector(_) => Vector::new(c[0], c[1]).into(),
        ExpressionVariant::Vector3(_) => Vector3::new(c[0] as f32, c[1] as f32, c[2] as f32).into(),
        ExpressionVariant::Vector3D(_) => Vector3D::new(c[0], c[1], c[2]).into(),
        ExpressionVariant::Vector4(_) => Vector4::new(c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32).into(),
        other => other,
    }
}

/// The value of the sub-channel `components` of `value`: a scalar for one
/// component, a `Vector2`, `Vector3` or `Vector4` for more. `Invalid` when
/// `value` is not a vector with these components.
pub(crate) fn extract_subchannel(value: ExpressionVariant, components: &[usize]) -> ExpressionVariant {
    let Some((c, n)) = vector_components(&value) else { return ExpressionVariant::Invalid };
    if components.iter().any(|&i| i >= n) {
        return ExpressionVariant::Invalid;
    }
    let at = |k: usize| c[components[k]];
    match components.len() {
        1 => ExpressionVariant::Double(at(0)),
        2 => Vector2::new(at(0) as f32, at(1) as f32).into(),
        3 => Vector3::new(at(0) as f32, at(1) as f32, at(2) as f32).into(),
        4 => Vector4::new(at(0) as f32, at(1) as f32, at(2) as f32, at(3) as f32).into(),
        _ => ExpressionVariant::Invalid,
    }
}

/// `value` with the sub-channel `components` set from `channel_value`.
/// Components the value does not have, and a channel value that is not a
/// scalar or a vector, leave it unchanged.
pub(crate) fn write_subchannel(
    value: ExpressionVariant,
    components: &[usize],
    channel_value: ExpressionVariant,
) -> ExpressionVariant {
    let Some((mut c, n)) = vector_components(&value) else { return value };
    let Some((o, on)) = channel_components(&channel_value) else { return value };
    for (k, &i) in components.iter().enumerate() {
        if k < on && i < n {
            c[i] = o[k];
        }
    }
    with_components(value, c)
}

/// The animation of a sub-channel on its way to the start path of the
/// property: what the generated `start_animation` of the property is given
/// in place of the animation. It holds the factory of the inner instance,
/// created when the animation was started, and wraps it in a
/// [`SubchannelAnimationInstance`].
///
/// `previous` is the animation of the property that is waiting for the same
/// batch (started earlier in the frame), which the start path replaces: when
/// it animates other sub-channels, the new sub-channel joins it on the
/// server, as for an animation that runs already.
pub(crate) struct SubchannelAnimation {
    components: Vec<usize>,
    inner: RefCell<Option<AnimationInstanceFactory>>,
    previous: RefCell<Option<AnimationInstanceFactory>>,
}

impl SubchannelAnimation {
    pub(crate) fn new(
        components: Vec<usize>,
        inner: AnimationInstanceFactory,
        previous: Option<AnimationInstanceFactory>,
    ) -> SubchannelAnimation {
        SubchannelAnimation { components, inner: RefCell::new(Some(inner)), previous: RefCell::new(previous) }
    }
}

impl ICompositionAnimationBase for SubchannelAnimation {
    fn as_composition_animation(&self) -> Option<&dyn ICompositionAnimation> {
        Some(self)
    }
}

impl ICompositionAnimation for SubchannelAnimation {
    fn target(&self) -> Option<String> {
        None
    }

    /// Wraps the inner instance; `final_value` was applied to the inner
    /// instance when it was created. Called once by the start path.
    fn create_instance(&self, target_object: ServerObjectId, _final_value: Option<ExpressionVariant>) -> AnimationInstanceFactory {
        let Some(inner) = self.inner.borrow_mut().take() else {
            panic!("the instance of a sub-channel animation is created once");
        };
        let previous = self.previous.borrow_mut().take();
        let components = self.components.clone();
        AnimationInstanceFactory::new(move || {
            let instance = SubchannelAnimationInstance::new(target_object, components, inner.create());
            if let Some(previous) = previous {
                // A previous animation of the whole property is replaced
                // (dropped); one of sub-channels takes the new sub-channel.
                let previous = previous.create();
                let merged = match previous.as_subchannel() {
                    Some(running) => {
                        running.absorb(&instance);
                        true
                    }
                    None => false,
                };
                if merged {
                    return previous;
                }
            }
            let instance: Rc<dyn IAnimationInstance> = instance;
            instance
        })
    }
}

/// One running animation of a sub-channel.
struct Subchannel {
    components: Vec<usize>,
    instance: Rc<dyn IAnimationInstance>,
}

/// The server instance of the animations of sub-channels of one property:
/// it evaluates the animation of each sub-channel and writes the result into
/// the components of the current value of the property.
///
/// The inner instances are initialized with the property itself, so their
/// invalidation marks this instance (the animation of the property) dirty.
pub struct SubchannelAnimationInstance {
    target: ServerObjectId,
    channels: RefCell<Vec<Subchannel>>,
    property: Cell<Option<&'static CompositionProperty>>,
    starting_value: Cell<ExpressionVariant>,
    active: Cell<bool>,
}

impl SubchannelAnimationInstance {
    /// An instance with one sub-channel, `components` of the property,
    /// animated by `instance`.
    pub fn new(
        target: ServerObjectId,
        components: Vec<usize>,
        instance: Rc<dyn IAnimationInstance>,
    ) -> Rc<SubchannelAnimationInstance> {
        Rc::new(SubchannelAnimationInstance {
            target,
            channels: RefCell::new(vec![Subchannel { components, instance }]),
            property: Cell::new(None),
            starting_value: Cell::new(ExpressionVariant::Invalid),
            active: Cell::new(false),
        })
    }

    /// The component sets of the running sub-channels.
    pub fn components(&self) -> Vec<Vec<usize>> {
        self.channels.borrow().iter().map(|c| c.components.clone()).collect()
    }

    /// Stops and drops the channels that share a component with
    /// `components`, without applying their stop behavior.
    // Deviation (winui-composition.md): a running sub-channel that shares
    // only some components with a new one (`Offset.XY`, then `Offset.X`) is
    // stopped as a whole.
    fn drop_overlapping(&self, components: &[usize]) {
        let removed: Vec<Subchannel> = {
            let mut channels = self.channels.borrow_mut();
            let (removed, kept) = std::mem::take(&mut *channels)
                .into_iter()
                .partition(|c| c.components.iter().any(|i| components.contains(i)));
            *channels = kept;
            removed
        };
        for channel in removed {
            if self.active.get() {
                channel.instance.deactivate();
            }
            channel.instance.release_completion();
        }
    }

    /// Takes over the sub-channels of `other` before either is initialized:
    /// they replace the channels that share a component with them.
    pub(crate) fn absorb(&self, other: &SubchannelAnimationInstance) {
        let incoming = std::mem::take(&mut *other.channels.borrow_mut());
        for channel in incoming {
            self.drop_overlapping(&channel.components);
            self.channels.borrow_mut().push(channel);
        }
    }

    /// Takes over the sub-channels of `other`, an instance started later on
    /// the same property and resolved already: they replace the channels
    /// that share a component with them, and start from `current_value`.
    pub(crate) fn merge(&self, other: &SubchannelAnimationInstance, committed_at: Duration, current_value: ExpressionVariant) {
        let incoming = std::mem::take(&mut *other.channels.borrow_mut());
        for channel in incoming {
            self.drop_overlapping(&channel.components);
            if let Some(property) = self.property.get() {
                channel.instance.initialize(
                    committed_at,
                    extract_subchannel(current_value, &channel.components),
                    property,
                );
            }
            if self.active.get() {
                channel.instance.activate();
            }
            self.channels.borrow_mut().push(channel);
        }
    }

    /// Stops the channels that share a component with `components`
    /// (`StopAnimation("Offset.X")`), applying their stop behavior to
    /// `current_value`. Returns the value to set, if a stop behavior changes
    /// it, and whether no channel is left.
    pub(crate) fn stop_components(
        &self,
        components: &[usize],
        current_value: ExpressionVariant,
    ) -> (Option<ExpressionVariant>, bool) {
        let removed: Vec<Subchannel> = {
            let mut channels = self.channels.borrow_mut();
            let (removed, kept) = std::mem::take(&mut *channels)
                .into_iter()
                .partition(|c| c.components.iter().any(|i| components.contains(i)));
            *channels = kept;
            removed
        };
        let mut value = current_value;
        let mut changed = false;
        for channel in removed {
            let channel_value = extract_subchannel(value, &channel.components);
            if let Some(stop) = channel.instance.value_on_stop(channel_value) {
                value = write_subchannel(value, &channel.components, stop);
                changed = true;
            }
            if self.active.get() {
                channel.instance.deactivate();
            }
            channel.instance.release_completion();
        }
        (changed.then_some(value), self.channels.borrow().is_empty())
    }
}

impl IServerClockItem for SubchannelAnimationInstance {
    /// The inner instances are on the clock themselves.
    fn on_tick(&self) {}
}

impl IAnimationInstance for SubchannelAnimationInstance {
    fn target_object(&self) -> ServerObjectId {
        self.target
    }

    fn resolve(&self, compositor: &Rc<ServerCompositor>) {
        for channel in self.channels.borrow().iter() {
            channel.instance.resolve(compositor);
        }
    }

    fn evaluate(&self, now: Duration, current_value: ExpressionVariant) -> ExpressionVariant {
        // The first evaluation has no current value yet: the value of the
        // property when the animation started stands in for it.
        let mut value = if current_value == ExpressionVariant::Invalid { self.starting_value.get() } else { current_value };
        let channels: Vec<(Vec<usize>, Rc<dyn IAnimationInstance>)> =
            self.channels.borrow().iter().map(|c| (c.components.clone(), c.instance.clone())).collect();
        for (components, instance) in channels {
            let channel_value = instance.evaluate(now, extract_subchannel(value, &components));
            value = write_subchannel(value, &components, channel_value);
        }
        value
    }

    fn initialize(&self, started_at: Duration, starting_value: ExpressionVariant, property: &'static CompositionProperty) {
        self.property.set(Some(property));
        self.starting_value.set(starting_value);
        for channel in self.channels.borrow().iter() {
            channel.instance.initialize(started_at, extract_subchannel(starting_value, &channel.components), property);
        }
    }

    fn activate(&self) {
        self.active.set(true);
        let instances: Vec<_> = self.channels.borrow().iter().map(|c| c.instance.clone()).collect();
        for instance in instances {
            instance.activate();
        }
    }

    fn deactivate(&self) {
        self.active.set(false);
        let instances: Vec<_> = self.channels.borrow().iter().map(|c| c.instance.clone()).collect();
        for instance in instances {
            instance.deactivate();
        }
    }

    fn invalidate(&self) {
        let instances: Vec<_> = self.channels.borrow().iter().map(|c| c.instance.clone()).collect();
        for instance in instances {
            instance.invalidate();
        }
    }

    fn value_on_stop(&self, current_value: ExpressionVariant) -> Option<ExpressionVariant> {
        let mut value = current_value;
        let mut changed = false;
        for channel in self.channels.borrow().iter() {
            let channel_value = extract_subchannel(value, &channel.components);
            if let Some(stop) = channel.instance.value_on_stop(channel_value) {
                value = write_subchannel(value, &channel.components, stop);
                changed = true;
            }
        }
        changed.then_some(value)
    }

    fn release_completion(&self) {
        let instances: Vec<_> = self.channels.borrow().iter().map(|c| c.instance.clone()).collect();
        for instance in instances {
            instance.release_completion();
        }
    }

    /// Every running sub-channel is put under the controller.
    fn attach_controller(&self, controller: ServerObjectId) {
        let instances: Vec<_> = self.channels.borrow().iter().map(|c| c.instance.clone()).collect();
        for instance in instances {
            instance.attach_controller(controller);
        }
    }

    fn as_subchannel(&self) -> Option<&SubchannelAnimationInstance> {
        Some(self)
    }
}
