use super::{
    MountedGroup, Route, StateRule, StateRuleKind, StateSetter, ViewState, ViewStateGroup,
};
use crate::{animation, style, KeyFrameBuilder, StyleBuilder};
use ferroui_base::animation::FillMode;
use ferroui_base::styling::Style;
use ferroui_base::Ref;
use std::collections::BTreeMap;
use std::time::Duration;

const NATIVE_TICK: Duration = Duration::from_nanos(100);

pub(super) fn build_style(group: &ViewStateGroup, anchor: &str) -> (Ref<Style>, MountedGroup) {
    let mut root = style(&format!(".{anchor}"));
    let mut rules = Vec::new();
    let mut add = |state: &ViewState, route: Option<&Route>| {
        let class = format!("{anchor}_{}", rules.len());
        let previous = route.and_then(|route| {
            group
                .states
                .iter()
                .find(|item| Some(item.name.as_str()) == route.from.as_deref())
        });
        let kind = route.map_or(StateRuleKind::Endpoint, |route| StateRuleKind::Transition {
            from: route.from.clone(),
        });
        rules.push(StateRule {
            class: class.clone(),
            state: state.name.clone(),
            kind,
        });
        (class, previous)
    };
    for state in &group.states {
        let (class, _) = add(state, None);
        for child in state_styles(&class, state, None, None) {
            root = root.nested(child);
        }
    }
    for route in &group.routes {
        let state = group
            .states
            .iter()
            .find(|state| state.name == route.to)
            .expect("validated state route");
        let (class, previous) = add(state, Some(route));
        for child in state_styles(&class, state, previous, Some(route)) {
            root = root.nested(child);
        }
    }
    (
        root.build(),
        MountedGroup {
            name: group.name.clone(),
            rules,
        },
    )
}

fn state_styles(
    class: &str,
    state: &ViewState,
    previous: Option<&ViewState>,
    route: Option<&Route>,
) -> Vec<StyleBuilder> {
    let mut targets = BTreeMap::<&str, Vec<&StateSetter>>::new();
    for setter in &state.setters {
        targets.entry(&setter.target).or_default().push(setter);
    }
    targets
        .into_iter()
        .map(|(target, setters)| {
            let selector = if target.is_empty() {
                format!("^.{class}")
            } else {
                format!("^.{class} #{target}")
            };
            let duration = route.map_or(Duration::ZERO, |route| route.duration);
            let mut clip = animation(duration.max(NATIVE_TICK))
                .fill_mode(FillMode::Forward)
                .fill_at_animation_priority();
            if let Some(route) = route {
                clip = clip.easing(route.easing.clone());
            }
            if duration.is_zero() {
                clip = clip.key_frame(0.0, |key| key_setters(key, setters.iter().copied()));
            } else if let Some(previous) = previous {
                let start = setters
                    .iter()
                    .filter_map(|end| {
                        previous.setters.iter().find(|start| {
                            start.target == end.target && std::ptr::eq(start.property, end.property)
                        })
                    })
                    .collect::<Vec<_>>();
                if !start.is_empty() {
                    clip = clip.key_frame(0.0, |key| key_setters(key, start));
                }
            }
            style(&selector).animation(clip.key_frame(1.0, |key| key_setters(key, setters)))
        })
        .collect()
}

fn key_setters<'a>(
    key: KeyFrameBuilder,
    setters: impl IntoIterator<Item = &'a StateSetter>,
) -> KeyFrameBuilder {
    setters.into_iter().fold(key, |key, setter| {
        key.set_boxed(setter.property, setter.value.clone())
    })
}
