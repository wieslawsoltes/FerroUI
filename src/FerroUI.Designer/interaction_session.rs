//! Visual-state and behavior authoring through native styles/classes and Xaml.Behaviors.
use crate::DesignerSession;
use ferroui_base::animation::{IClock, PlayState, SignedDuration};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::StyledElement;
use ferroui_designer_animation::{BehaviorAction, Sample, VisualStateGroup};
use ferroui_designer_core::NodeId;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[path = "behavior_session.rs"]
mod behaviors;
mod state_triggers;

#[derive(Default)]
pub(crate) struct InteractionSession {
    original_classes: BTreeMap<(NodeId, String), Vec<String>>,
    active_states: BTreeMap<(NodeId, String), String>,
    behavior_modified: bool,
    active_behavior: Option<NodeId>,
    clock: Option<Rc<StatePreviewClock>>,
    clock_time: f64,
    transition: Option<TransitionPreview>,
}

#[derive(Clone)]
struct TransitionPreview {
    owner: NodeId,
    group: String,
    to: String,
    duration: f64,
}
impl DesignerSession {
    pub fn state_groups(&self) -> Vec<VisualStateGroup> {
        VisualStateGroup::read(&self.document)
    }
    /// The currently previewed native state endpoints. This is local presentation
    /// state and does not synthesize state declarations or edit the authored source.
    pub fn active_preview_states(&self) -> Vec<(NodeId, String, String)> {
        self.interactions
            .active_states
            .iter()
            .map(|((owner, group), state)| (*owner, group.clone(), state.clone()))
            .collect()
    }
    pub fn upsert_state_group(&mut self, group: VisualStateGroup) -> Result<(), String> {
        if self.uses_rust_source() {
            return self.edit_rust_state_group(group.owner, &group.name, Some(&group));
        }
        let edits = group
            .plan(&self.document)
            .map_err(|error| error.to_string())?;
        self.apply("Edit visual state group", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn remove_state_group(&mut self, owner: NodeId, name: &str) -> Result<(), String> {
        if self.uses_rust_source() {
            return self.edit_rust_state_group(owner, name, None);
        }
        let edits = VisualStateGroup::remove(&self.document, owner, name)
            .map_err(|error| error.to_string())?;
        self.apply("Remove visual state group", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub(crate) fn can_create_rust_state_group(&self, owner: NodeId) -> bool {
        self.can_manipulate()
            && self.document.ast().node(owner).zip(self.rust_map())
                .is_some_and(|(node, map)| map.can_create_state_group(node.span.start))
    }

    fn edit_rust_state_group(
        &mut self, owner: NodeId, name: &str, model: Option<&VisualStateGroup>,
    ) -> Result<(), String> {
        if !self.can_manipulate() {
            return Err("Open a current, editable Rust view".into());
        }
        let rust = self.rust.as_ref().ok_or("Open the current Rust view")?;
        let patches = rust.edit_state_group(&self.document, owner, name, model)?;
        if !patches.is_empty() {
            let label = if model.is_some() { "Edit Rust visual state group" } else { "Remove Rust visual state group" };
            self.commit_rust_patches(label, patches).map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }

    pub fn preview_state(
        &mut self,
        owner: NodeId,
        name: &str,
        state: Option<&str>,
    ) -> Result<(), String> {
        self.preview_state_at(owner, name, state, None)
    }
    /// Preview a state at a transition-relative time, or at its endpoint when elapsed is None.
    /// Native animation instances resolve resources and bindings; authored local values remain
    /// in the document and Stop rebuilds the authored preview.
    pub fn preview_state_at(
        &mut self,
        owner: NodeId,
        name: &str,
        state: Option<&str>,
        elapsed: Option<f64>,
    ) -> Result<(), String> {
        self.sample_state(owner, name, state, elapsed, None)
    }

    /// Sample an explicit authored route independently of the currently previewed state.
    /// Reconstruct the initial endpoint before sampling the destination so backward seeks
    /// and repeated seeks use the same native starting value and easing.
    pub fn preview_state_transition(
        &mut self, owner: NodeId, name: &str, from: Option<&str>, to: &str, elapsed: f64,
    ) -> Result<(), String> {
        let groups = self.state_groups();
        let group = groups.iter().find(|group| group.owner == owner && group.name == name)
            .ok_or("Unknown visual state group")?;
        if from == Some(to) || from.is_some_and(|from| !group.states.iter().any(|state| state.name == from))
            || !group.states.iter().any(|state| state.name == to)
        {
            return Err("Choose distinct authored endpoints for this visual state transition".into());
        }
        let duration = group.transition(from, to).ok_or("No authored transition connects these states")?.duration;
        if !valid_state_time(duration) || !valid_state_time(elapsed) || elapsed > duration {
            return Err("Choose a time within the native state transition duration".into());
        }
        if !self.preview.visual(owner).and_then(|visual| visual.cast::<StyledElement>())
            .is_some_and(|owner| owner.is_attached_to_logical_tree())
        {
            return Err("Attach the preview host to a rooted logical tree before sampling state transitions".into());
        }
        self.stop_animation_playback();
        // Resolve the exact starting endpoint without replaying any incoming route
        // from the state that happened to be visible before this seek.
        self.sample_state(owner, name, from, None, Some(None))?;
        let transition = TransitionPreview { owner, group: name.into(), to: to.into(), duration };
        if elapsed >= duration {
            return self.finish_state_transition(transition);
        }
        self.sample_state(owner, name, Some(to), Some(elapsed), Some(from))?;
        self.interactions.transition = Some(transition);
        Ok(())
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn state_transition_position(&self) -> Option<(f64, f64)> {
        Some((self.interactions.clock_time, self.interactions.transition.as_ref()?.duration))
    }

    fn finish_state_transition(&mut self, transition: TransitionPreview) -> Result<(), String> {
        // Legacy generated styles can start both the default and the directional
        // animations. Once the selected route ends, hold the native destination.
        self.sample_state(transition.owner, &transition.group, Some(&transition.to), None, Some(None))?;
        self.interactions.clock_time = transition.duration;
        self.interactions.transition = Some(transition);
        Ok(())
    }

    fn sample_state(
        &mut self, owner: NodeId, name: &str, state: Option<&str>, elapsed: Option<f64>,
        source_state: Option<Option<&str>>,
    ) -> Result<(), String> {
        if elapsed.is_some_and(|time| !valid_state_time(time)) {
            return Err("State preview time is outside the native clock range".into());
        }
        let groups = self.state_groups();
        let group = groups
            .iter()
            .find(|group| group.owner == owner && group.name == name)
            .ok_or("Unknown visual state group")?;
        if state.is_none() {
            let retained = self
                .interactions
                .active_states
                .iter()
                .filter(|((existing_owner, existing_name), _)| {
                    *existing_owner != owner || existing_name != name
                })
                .map(|(key, state)| (key.clone(), state.clone()))
                .collect::<Vec<_>>();
            self.clear_interaction_preview();
            for ((owner, name), state) in retained {
                self.preview_state(owner, &name, Some(&state))?;
            }
            return Ok(());
        }

        let selected = state
            .map(|name| {
                group
                    .states
                    .iter()
                    .find(|state| state.name == name)
                    .ok_or("Unknown visual state")
            })
            .transpose()?;
        let owner_visual = self
            .preview
            .visual(owner)
            .and_then(|visual| visual.cast::<StyledElement>())
            .ok_or("State owner has no live preview")?;
        if !owner_visual.is_attached_to_logical_tree()
            && (elapsed.is_some()
                || selected.is_some_and(|state| {
                    state
                        .setters
                        .iter()
                        .any(|setter| setter.value.trim_start().starts_with('{'))
                }))
        {
            return Err("Attach the preview host to a Window or rooted logical tree before sampling native state transitions, bindings or resources".into());
        }
        let projected = group.is_projected(&self.document);
        let key = (owner, name.to_owned());
        let from = source_state.unwrap_or_else(|| {
            group.states.iter().find(|state| owner_visual.classes().contains(&group.class_name(&state.name)))
                .map(|state| state.name.as_str()).or_else(|| self.interactions.active_states.get(&key).map(String::as_str))
        });
        let mut next = self.interactions.active_states.clone();
        if let Some(state) = selected {
            next.insert(key.clone(), state.name.clone());
        } else {
            next.remove(&key);
        }
        let transition = selected.and_then(|state| group.transition(from, &state.name));
        let duration = transition
            .map_or(0.000_000_1, |transition| {
                transition.duration.max(0.000_000_1)
            });
        let time = elapsed.unwrap_or(duration);
        // Resolve and validate all literal endpoints before any state/class change.
        let mut samples = BTreeMap::new();
        for ((owner, group_name), state_name) in &next {
            let Some(group) = groups
                .iter()
                .find(|group| group.owner == *owner && group.name == *group_name)
            else {
                continue;
            };
            let Some(state) = group.states.iter().find(|state| state.name == *state_name) else {
                continue;
            };
            for setter in &state.setters {
                let target = ferroui_designer_animation::resolve_state_target(
                    &self.document,
                    *owner,
                    &setter.target,
                )
                .map_err(|error| error.to_string())?;
                if setter.value.trim_start().starts_with('{') {
                    continue;
                }
                let value =
                    ferroui_designer_animation::state_value(&setter.property, &setter.value)
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
        }
        let samples = samples.into_values().collect::<Vec<_>>();
        self.preview.preview_samples(&samples)?;
        self.preview.clear_animation();
        let clock = Rc::new(StatePreviewClock::default());
        self.preview.host.set_clock(Some(clock.clone()));
        let classes = owner_visual.classes();
        let controlled = group.classes();
        self.interactions
            .original_classes
            .entry(key.clone())
            .or_insert_with(|| {
                controlled
                    .iter()
                    .filter(|class| classes.contains(class))
                    .cloned()
                    .collect()
            });
        for class in &controlled {
            classes.remove(class);
        }
        if let Some(state) = selected {
            let class = group.class_name(&state.name);
            let route = transition.and_then(|transition| transition.from.as_deref());
            if let Some(from) = route.filter(|_| source_state.is_some() && !projected) {
                // During an explicit seek, start the selected route last. Existing
                // XAML can match the default style along with the directional style.
                classes.add(&class);
                classes.add(&group.transition_class(from, &state.name));
            } else {
                if let Some(from) = route {
                    classes.add(&group.transition_class(from, &state.name));
                }
                classes.add(&class);
            }
        } else if let Some(original) = self.interactions.original_classes.remove(&key) {
            for class in original {
                classes.add(&class);
            }
        }
        self.interactions.active_states = next;
        self.preview.host.update_layout();
        clock.subject.on_next(SignedDuration::ZERO);
        clock.subject.on_next(SignedDuration::from_seconds(time));
        self.interactions.clock = Some(clock);
        self.interactions.clock_time = time;
        self.interactions.transition = None;
        self.interactions.behavior_modified = true;
        // Literal endpoint overlays remain deterministic even in a disconnected design tree.
        // Intermediate frames use the actual runtime animation to preserve native easing.
        if elapsed.is_none() {
            self.preview.preview_samples(&samples)?;
        }
        self.import_animations();
        self.generation += 1;
        Ok(())
    }
    pub fn advance_state_preview(&mut self, seconds: f64) -> Result<(), String> {
        let requested = self.interactions.clock_time + seconds;
        if !valid_state_time(seconds) || !valid_state_time(requested) {
            return Err("State clock delta is outside the native clock range".into());
        }
        if let Some(transition) = self.interactions.transition.as_ref().filter(|transition| requested >= transition.duration) {
            return if self.interactions.clock_time >= transition.duration { Ok(()) }
                else { self.finish_state_transition(transition.clone()) };
        }
        let time = requested;
        let clock = self
            .interactions
            .clock
            .as_ref()
            .ok_or("No state preview is active")?;
        self.preview.clear_animation();
        clock.subject.on_next(SignedDuration::from_seconds(time));
        self.interactions.clock_time = time;
        self.preview.host.update_layout();
        self.generation += 1;
        Ok(())
    }
    pub fn state_group_actions(
        &self,
        owner: NodeId,
        name: &str,
        to: &str,
    ) -> Result<Vec<BehaviorAction>, String> {
        self.state_group_transition_actions(owner, name, None, to)
    }
    pub fn state_group_transition_actions(
        &self,
        owner: NodeId,
        name: &str,
        from: Option<&str>,
        to: &str,
    ) -> Result<Vec<BehaviorAction>, String> {
        if self.uses_rust_source() {
            return Err("Use set_visual_state in the Rust handler for this state group".into());
        }
        let group = self
            .state_groups()
            .into_iter()
            .find(|group| group.owner == owner && group.name == name)
            .ok_or("Unknown state group")?;
        let owner_name = self
            .document
            .ast()
            .node(owner)
            .and_then(|node| node.xaml_name())
            .ok_or("Name the state owner before creating its transition actions")?;
        group
            .actions(owner_name, to, from)
            .map_err(|error| error.to_string())
    }
    pub fn clear_interaction_preview(&mut self) {
        let reset =
            self.interactions.behavior_modified || !self.interactions.original_classes.is_empty();
        if reset {
            self.preview.set_interactive(false);
            // Ordinary hot reload preserves user-entered text, selection and scroll state.
            // Stopping an interaction preview must rebuild the authored defaults instead.
            // Keep the retired graph until staging succeeds so an invalid draft keeps its
            // last successful preview rather than leaving the artboard empty.
            let previous = self.preview.host.content();
            self.preview.host.set_content(None);
            self.preview.reload(&self.document);
            if !matches!(self.preview.status, crate::PreviewStatus::Ready { .. }) {
                self.preview.host.set_content(previous);
            }
            self.generation += 1;
        }
        self.interactions = InteractionSession::default();
        if reset {
            self.import_animations();
        }
    }
    pub(crate) fn forget_interaction_preview(&mut self) {
        if self.interactions.clock.is_some() {
            self.preview.set_interactive(false);
        }
        self.interactions = InteractionSession::default();
    }
}

fn valid_state_time(time: f64) -> bool {
    time.is_finite() && time >= 0.0 && time * 10_000_000.0 < i64::MAX as f64
}

struct StatePreviewClock {
    subject: LightweightSubject<SignedDuration>,
    state: Cell<PlayState>,
}
impl Default for StatePreviewClock {
    fn default() -> Self {
        Self {
            subject: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        }
    }
}
impl IObservable<SignedDuration> for StatePreviewClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for StatePreviewClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
