use crate::{Easing, StateSetter, StateTransition, VisualStateGroup, resolve_state_target};
use ferroui_base::animation::easings::{Easing as NativeEasing, SpringEasing};
use ferroui_base::animation::{
    Animatable, Animation, Cue, FillMode, IAnimationSetter, KeyFrame, SignedDuration,
};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::styling::Setter;
use ferroui_base::{BoxedValue, FerroPropertyRegistry, Ref, StyledElement};
use ferroui_designer_core::{Document, NodeId};
use std::collections::BTreeMap;
use std::rc::Rc;

/// A designer-controlled route bound to the application’s actual animators.
/// Applying the clips through NativePlayback gives each route a disposable value layer.
pub struct NativeStateRoute {
    bindings: BTreeMap<NodeId, (Ref<Animation>, Ref<Animatable>)>,
}

impl NativeStateRoute {
    pub fn bind(
        document: &Document,
        group: &VisualStateGroup,
        transition: &StateTransition,
        mut resolve: impl FnMut(NodeId) -> Option<Ref<StyledElement>>,
    ) -> Result<Self, String> {
        let end = group
            .states
            .iter()
            .find(|state| state.name == transition.to)
            .ok_or("The destination state is unavailable")?;
        let start = transition
            .from
            .as_ref()
            .and_then(|name| group.states.iter().find(|state| &state.name == name));
        let duration = transition.duration;
        if !duration.is_finite() || duration < 0.0 || duration * 10_000_000.0 >= i64::MAX as f64 {
            return Err("The state duration is outside the native clock range".into());
        }
        let easing = native_easing(&transition.easing)?;
        let mut targets = BTreeMap::<&str, Vec<&StateSetter>>::new();
        for setter in &end.setters {
            targets.entry(&setter.target).or_default().push(setter);
        }
        let mut bindings = BTreeMap::new();
        for (name, setters) in targets {
            let id = resolve_state_target(document, group.owner, name)
                .map_err(|error| error.to_string())?;
            let target =
                resolve(id).ok_or("A state target is absent or ambiguous in the application")?;
            let mut first = Vec::new();
            let mut last = Vec::new();
            for setter in setters {
                let endpoint = native_setter(&target, setter)?;
                if duration == 0.0 {
                    first.push(endpoint.clone());
                } else if let Some(previous) = start.and_then(|state| {
                    state.setters.iter().find(|previous| {
                        previous.target == setter.target && previous.property == setter.property
                    })
                }) {
                    first.push(native_setter(&target, previous)?);
                }
                last.push(endpoint);
            }
            let animation = Animation::new();
            animation.set_duration(SignedDuration::from_seconds(duration.max(0.000_000_1)));
            animation.set_fill_mode(FillMode::Forward);
            animation.set_easing(easing.clone());
            if !first.is_empty() {
                animation
                    .children()
                    .add(KeyFrame::with_cue(Cue::new(0.0), first));
            }
            animation
                .children()
                .add(KeyFrame::with_cue(Cue::new(1.0), last));
            bindings.insert(id, (animation, target.upcast::<Animatable>()));
        }
        if bindings.is_empty() {
            return Err("The state has no mapped animation setters".into());
        }
        Ok(Self { bindings })
    }

    pub fn targets(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.bindings.keys().copied()
    }

    pub fn bindings(&self) -> Vec<(Ref<Animation>, Ref<Animatable>)> {
        self.bindings.values().cloned().collect()
    }
}

fn native_setter(
    target: &StyledElement,
    setter: &StateSetter,
) -> Result<Rc<dyn IAnimationSetter>, String> {
    let name = &setter.property;
    let property = (!name.contains('.'))
        .then(|| FerroPropertyRegistry::instance().find_registered_for(target, name))
        .flatten()
        .filter(|property| !property.is_read_only() && Animation::can_animate_property(property))
        .ok_or_else(|| format!("No application animator is registered for {name}"))?;
    let source: BoxedValue = Rc::new(setter.value.clone());
    let value = ValueTypes::try_convert(
        Some(&source),
        ValueType::new(property.property_type(), property.property_type_name()),
    )
    .flatten()
    .filter(|value| property.is_valid_value(value.as_any()))
    .ok_or_else(|| format!("Invalid state animation value for {name}"))?;
    Ok(Setter::new_untyped(property, value))
}

fn native_easing(easing: &Easing) -> Result<NativeEasing, String> {
    if let Easing::Spring {
        mass,
        stiffness,
        damping,
        initial_velocity,
    } = easing
    {
        return Ok(NativeEasing::new(SpringEasing::with_values(
            *mass,
            *stiffness,
            *damping,
            *initial_velocity,
        )));
    }
    NativeEasing::parse(&easing.to_xaml()).map_err(|error| error.to_string())
}
