//! The transitions between visual states: the choice of the `VisualTransition` (the transition
//! lookup of WinUI's `CVisualStateManager2`) and the storyboard generated for it
//! (`dxaml/xcp/components/vsm/DynamicTransitionStoryboardGenerator.cpp` of microsoft-ui-xaml
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation,
//! see NOTICE.md).

use super::applied_setters::{key_of, ResolvedSetter, SetterKey};
use super::{VisualState, VisualTransition};
use crate::animation::{
    ColorAnimation, ColorAnimationUsingKeyFrames, DoubleAnimation, DoubleAnimationUsingKeyFrames, EasingFunctionBase,
    PointAnimation, PointAnimationUsingKeyFrames, ResolvedAnimation, Storyboard, Timeline,
};
use ferroui_base::animation::TimelineDuration;
use ferroui_base::collections::FerroList;
use ferroui_base::media::{Color, Transform};
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::{BoxedValue, FerroLocator, FerroObject, FerroProperty, LocatorExtensions, Point, Ref, Visual};
use std::any::TypeId;
use std::time::Duration;

/// The transition of `transitions` from `from` to `to` (WinUI's lookup): a transition whose `From`
/// names the current state scores 1, whose `To` names the new state scores 2; one that names
/// another state is out; the default transition (neither) scores 0; the first of the best wins.
pub(crate) fn find_transition(
    transitions: &FerroList<Ref<VisualTransition>>,
    from: Option<&Ref<VisualState>>,
    to: &Ref<VisualState>,
) -> Option<Ref<VisualTransition>> {
    let from_name = from.map(|state| state.name());
    let to_name = to.name();
    let mut best: Option<(i32, Ref<VisualTransition>)> = None;
    for transition in transitions.snapshot().iter() {
        let mut score = 0;
        let transition_from = transition.from();
        if !transition_from.is_empty() {
            if from_name.as_deref() != Some(transition_from.as_str()) {
                continue;
            }
            score += 1;
        }
        let transition_to = transition.to();
        if !transition_to.is_empty() {
            if transition_to != to_name {
                continue;
            }
            score += 2;
        }
        if best.as_ref().is_none_or(|(best_score, _)| score > *best_score) {
            best = Some((score, transition.clone()));
        }
    }
    best.map(|(_, transition)| transition)
}

/// Whether the platform plays animations (with system animations off, WinUI does not create the
/// storyboards of transitions: the states change at once).
pub(crate) fn animations_enabled() -> bool {
    FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .is_none_or(|settings| settings.animations_enabled())
}

/// A value that a state animates or sets, as the generator sees it.
struct Node {
    key: SetterKey,
    object: Ref<FerroObject>,
    property: &'static FerroProperty,
    /// The value it goes to (`None`: back to the value without animations).
    value: Option<BoxedValue>,
}

/// The value an animation of a state animates to, for a transition into that state (WinUI's
/// `GenerateToAnimation`: `From`, else `To`, of an animation from, to or by a value; the value of
/// the first key frame of a key frame animation).
fn to_value(animation: &Timeline) -> Option<BoxedValue> {
    fn boxed<T: PartialEq + 'static>(value: Option<T>) -> Option<BoxedValue> {
        value.map(|value| std::rc::Rc::new(value) as BoxedValue)
    }
    if let Some(animation) = animation.downcast_ref::<DoubleAnimation>() {
        return boxed(animation.from().or(animation.to()));
    }
    if let Some(animation) = animation.downcast_ref::<ColorAnimation>() {
        return boxed(animation.from().or(animation.to()));
    }
    if let Some(animation) = animation.downcast_ref::<PointAnimation>() {
        return boxed(animation.from().or(animation.to()));
    }
    if let Some(animation) = animation.downcast_ref::<DoubleAnimationUsingKeyFrames>() {
        return boxed(animation.key_frames().snapshot().first().map(|frame| frame.value()));
    }
    if let Some(animation) = animation.downcast_ref::<ColorAnimationUsingKeyFrames>() {
        return boxed(animation.key_frames().snapshot().first().map(|frame| frame.value()));
    }
    if let Some(animation) = animation.downcast_ref::<PointAnimationUsingKeyFrames>() {
        return boxed(animation.key_frames().snapshot().first().map(|frame| frame.value()));
    }
    None
}

fn animation_nodes(animations: &[ResolvedAnimation], to: bool) -> Vec<Node> {
    animations
        .iter()
        .filter_map(|resolved| {
            let value = if to { Some(to_value(&resolved.animation)?) } else { None };
            Some(Node {
                key: key_of(&resolved.object, resolved.property),
                object: resolved.object.clone(),
                property: resolved.property,
                value,
            })
        })
        .collect()
}

fn setter_nodes(setters: &[ResolvedSetter], to: bool) -> Vec<Node> {
    setters
        .iter()
        .filter(|setter| setter.value.is_some())
        .map(|setter| Node {
            key: key_of(&setter.object, setter.property),
            object: setter.object.clone(),
            property: setter.property,
            value: if to { setter.value.clone() } else { None },
        })
        .collect()
}

/// What the old and the new state animate and set, for [`generate`].
pub(crate) struct StateValues<'a> {
    pub(crate) animations: &'a [ResolvedAnimation],
    pub(crate) setters: &'a [ResolvedSetter],
}

/// Whether an animation of `property` runs for the full duration of a transition: WinUI's
/// independent animations (the opacity, and the values of transforms).
fn is_independent(property: &'static FerroProperty) -> bool {
    property == Visual::opacity_property().as_property() || Transform::TYPE.is_assignable_from(property.owner_type())
}

/// The generated storyboard of a transition (WinUI's `GenerateDynamicTransitionAnimations`): it
/// lasts the generated duration, and animates the double, colour and point values the new state
/// animates or sets to their values, and those the old state animated or set that the new one does
/// not back to the values without animations, leaving out what the transition's own storyboard
/// animates. Only independent animations take the full duration, eased; the others jump.
pub(crate) fn generate(
    transition: &VisualTransition,
    transition_animations: &[ResolvedAnimation],
    old: StateValues<'_>,
    new: StateValues<'_>,
) -> Ref<Storyboard> {
    let duration = transition.generated_duration().to_duration().unwrap_or(Duration::ZERO);
    let easing = transition.generated_easing_function();
    let storyboard = Storyboard::new();
    storyboard.set_duration(TimelineDuration::Time(duration));
    let explicit: Vec<SetterKey> =
        transition_animations.iter().map(|resolved| key_of(&resolved.object, resolved.property)).collect();
    let mut to_nodes = animation_nodes(new.animations, true);
    to_nodes.extend(setter_nodes(new.setters, true));
    to_nodes.retain(|node| !explicit.contains(&node.key));
    // The new values: what the new state animates and sets (all of it, also without a generated
    // animation, keeps the old state's value from being animated back).
    let mut new_keys: Vec<SetterKey> =
        new.animations.iter().map(|resolved| key_of(&resolved.object, resolved.property)).collect();
    new_keys.extend(new.setters.iter().map(|setter| key_of(&setter.object, setter.property)));
    let mut from_nodes = animation_nodes(old.animations, false);
    from_nodes.extend(setter_nodes(old.setters, false));
    from_nodes.retain(|node| !explicit.contains(&node.key) && !new_keys.contains(&node.key));
    let mut seen: Vec<SetterKey> = Vec::new();
    for node in to_nodes.into_iter().chain(from_nodes) {
        if seen.contains(&node.key) {
            continue;
        }
        seen.push(node.key);
        if let Some(animation) = generated_animation(&node, duration, easing.as_ref()) {
            storyboard.children().add(animation);
        }
    }
    storyboard
}

/// An animation of `node` to its value, or back to the value without animations.
fn generated_animation(
    node: &Node,
    duration: Duration,
    easing: Option<&Ref<EasingFunctionBase>>,
) -> Option<Ref<Timeline>> {
    let type_ = node.property.property_type();
    let value = node.value.as_ref();
    let full = is_independent(node.property);
    let easing = if full { easing.cloned() } else { None };
    let animation: Ref<Timeline> = if type_ == TypeId::of::<f64>() {
        let animation = DoubleAnimation::new();
        animation.set_to(value.and_then(|value| value.downcast_ref::<f64>().copied()));
        animation.set_easing_function(easing);
        animation.upcast()
    } else if type_ == TypeId::of::<Color>() {
        let animation = ColorAnimation::new();
        animation.set_to(value.and_then(|value| value.downcast_ref::<Color>().copied()));
        animation.set_easing_function(easing);
        animation.upcast()
    } else if type_ == TypeId::of::<Point>() {
        let animation = PointAnimation::new();
        animation.set_to(value.and_then(|value| value.downcast_ref::<Point>().copied()));
        animation.set_easing_function(easing);
        animation.upcast()
    } else {
        return None;
    };
    Storyboard::set_target(&animation, &node.object);
    animation.set_explicit_property(Some(node.property));
    animation.set_duration(TimelineDuration::Time(if full { duration } else { Duration::ZERO }));
    Some(animation)
}
