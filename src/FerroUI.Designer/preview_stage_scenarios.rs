//! Native visual-state endpoints for independent preview-stage candidates.
//! The authored document is read-only here. Classes, native state animations and
//! disposable preview values belong exclusively to the candidate RuntimePreview.
use crate::runtime::{PreviewStatus, RuntimePreview};
use ferroui_base::animation::{IClock, PlayState, TimeSpan};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{Ref, StyledElement, WeakRef};
use ferroui_controls::ContentControl;
use ferroui_designer_animation::{Sample, VisualStateGroup};
use ferroui_designer_core::{Document, NodeId};
use ferroui_inspect::Inspector;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The board resolves persistent source anchors against the accepted document before
/// passing a scenario here. One group on one owner can have one selected endpoint.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedStageState<'a> {
    pub owner: NodeId,
    pub group: &'a str,
    pub state: &'a str,
}

/// Retain beside the candidate RuntimePreview, and drop before dropping that preview.
/// RuntimePreview owns literal-value disposables; this guard owns the sampled native
/// clock. Endpoint scenarios intentionally receive no advancing application clock.
#[derive(Default)]
pub(crate) struct StageScenarioGuard {
    clock: Option<Rc<StageStateClock>>,
    host: Option<WeakRef<ContentControl>>,
    previous_clock: Option<Rc<dyn IClock>>,
    endpoint: f64,
}

impl Drop for StageScenarioGuard {
    fn drop(&mut self) {
        let Some(clock) = self.clock.take() else {
            return;
        };
        clock.set_play_state(PlayState::Stop);
        clock.subject.on_next(TimeSpan::from_seconds(self.endpoint));
        let installed: Rc<dyn IClock> = clock;
        if let Some(host) = self.host.take().and_then(|host| host.upgrade()) {
            if host
                .clock()
                .is_some_and(|current| Rc::ptr_eq(&current, &installed))
            {
                host.set_clock(self.previous_clock.take());
            }
        }
    }
}

struct StateClasses {
    owner: Ref<StyledElement>,
    controlled: Vec<String>,
    selected: String,
    transition: Option<String>,
}

/// Apply complete endpoint scenarios to a newly constructed stage candidate.
///
/// Resolve and validate every group, state, namescope target and native literal value
/// before changing any classes. Binding/resource endpoints run through their authored
/// native styles and animation keyframes, and require a rooted logical tree. Literal
/// endpoints also work before attachment using RuntimePreview's animation-value layer.
/// The board publishes the candidate only after this function succeeds; failures leave
/// the previously accepted stage untouched.
pub(crate) fn apply_stage_states(
    document: &Document,
    preview: &mut RuntimePreview,
    states: &[ResolvedStageState<'_>],
) -> Result<StageScenarioGuard, String> {
    if states.is_empty() {
        return Ok(StageScenarioGuard::default());
    }
    if !document.is_valid()
        || !matches!(preview.status, PreviewStatus::Ready { revision } if revision == document.revision())
    {
        return Err("State scenarios require an accepted preview of the current XAML".into());
    }

    let groups = VisualStateGroup::read(document);
    let mut ordered = states.to_vec();
    ordered.sort_by_key(|selection| (selection.owner, selection.group));
    let mut selected_groups = BTreeSet::new();
    let mut classes = Vec::with_capacity(ordered.len());
    let mut samples = BTreeMap::new();
    let mut endpoint = 0.000_000_1_f64;

    for selection in ordered {
        if !selected_groups.insert((selection.owner, selection.group)) {
            return Err(format!(
                "Choose one state for group '{}' on each preview owner",
                selection.group
            ));
        }
        let mut matching = groups
            .iter()
            .filter(|group| group.owner == selection.owner && group.name == selection.group);
        let group = matching.next().ok_or_else(|| {
            format!(
                "Unknown state group '{}' on preview owner {}",
                selection.group, selection.owner
            )
        })?;
        if matching.next().is_some() {
            return Err(format!(
                "State group '{}' is ambiguous on preview owner {}",
                selection.group, selection.owner
            ));
        }
        group.validate().map_err(|error| error.to_string())?;
        let state = group
            .states
            .iter()
            .find(|state| state.name == selection.state)
            .ok_or_else(|| {
                format!(
                    "Unknown state '{}' in group '{}'",
                    selection.state, selection.group
                )
            })?;
        let owner = preview
            .unique_scenario_visual(selection.owner)?
            .and_then(|visual| visual.cast::<StyledElement>())
            .ok_or("State owner has no native control in this preview stage")?;
        let expression = state
            .setters
            .iter()
            .any(|setter| setter.value.trim_start().starts_with('{'));
        if expression && !owner.is_attached_to_logical_tree() {
            return Err(
                "Attach the preview stage to a Window or rooted logical tree before sampling state bindings or resources"
                    .into(),
            );
        }
        for setter in &state.setters {
            let target = ferroui_designer_animation::resolve_state_target(
                document,
                selection.owner,
                &setter.target,
            )
            .map_err(|error| error.to_string())?;
            let visual = preview
                .visual(target)
                .ok_or("A state target has no native control in this preview stage")?;
            let property = Inspector::property(&visual, &setter.property)?;
            if property.is_direct() || property.is_read_only() {
                return Err(format!(
                    "{} does not support a native state animation value layer",
                    setter.property
                ));
            }
            if setter.value.trim_start().starts_with('{') {
                continue;
            }
            let value = ferroui_designer_animation::state_value(&setter.property, &setter.value)
                .map_err(|error| error.to_string())?;
            samples.insert(
                (target, setter.property.clone()),
                Sample {
                    target,
                    property: setter.property.clone(),
                    value,
                },
            );
        }

        let from = group
            .states
            .iter()
            .find(|state| owner.classes().contains(&group.class_name(&state.name)))
            .map(|state| state.name.as_str());
        let transition = group
            .transitions
            .iter()
            .find(|transition| transition.to == state.name && transition.from.as_deref() == from)
            .or_else(|| {
                group
                    .transitions
                    .iter()
                    .find(|transition| transition.to == state.name && transition.from.is_none())
            });
        let duration = transition.map_or(0.000_000_1, |transition| {
            transition.duration.max(0.000_000_1)
        });
        if !duration.is_finite() || duration * 10_000_000.0 >= i64::MAX as f64 {
            return Err("State endpoint is outside the native clock range".into());
        }
        endpoint = endpoint.max(duration);
        classes.push(StateClasses {
            controlled: group.classes(),
            selected: group.class_name(&state.name),
            transition: transition
                .and_then(|transition| transition.from.as_deref())
                .map(|from| group.transition_class(from, &state.name)),
            owner,
        });
    }

    let samples = samples.into_values().collect::<Vec<_>>();
    // The shared native planner checks all property conversions and validators before
    // installing any value. This temporary validation layer is removed before styles run.
    preview.preview_samples(&samples)?;
    preview.clear_animation();
    let clock = Rc::new(StageStateClock::default());
    let guard = StageScenarioGuard {
        clock: Some(clock.clone()),
        host: Some(preview.host.downgrade()),
        previous_clock: preview.host.clock(),
        endpoint,
    };
    preview.host.set_clock(Some(clock.clone()));
    for change in classes {
        let collection = change.owner.classes();
        collection.remove_all(change.controlled);
        if let Some(transition) = change.transition {
            collection.add(&transition);
        }
        collection.add(&change.selected);
    }
    preview.host.update_layout();
    clock.subject.on_next(TimeSpan::ZERO);
    clock.subject.on_next(TimeSpan::from_seconds(endpoint));
    // Literal endpoints retain the same deterministic overlay as preview_state_at.
    // Expressions are never evaluated or replaced here: native animations own them.
    preview.preview_samples(&samples)?;
    preview.host.update_layout();
    Ok(guard)
}

struct StageStateClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}

impl Default for StageStateClock {
    fn default() -> Self {
        Self {
            subject: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        }
    }
}

impl IObservable<TimeSpan> for StageStateClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for StageStateClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}

#[cfg(test)]
#[path = "preview_stage_scenarios_tests.rs"]
mod tests;
