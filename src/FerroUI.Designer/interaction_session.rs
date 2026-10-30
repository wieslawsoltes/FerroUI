//! Visual-state and behavior authoring through native styles/classes and Xaml.Behaviors.
use crate::DesignerSession;
use ferroui_base::animation::{IClock, PlayState, TimeSpan};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{BoxedValue, StyledElement};
use ferroui_designer_animation::{BehaviorAction, BehaviorModel, Sample, VisualStateGroup};
use ferroui_designer_core::NodeId;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct InteractionSession {
    original_classes: BTreeMap<(NodeId, String), Vec<String>>,
    active_states: BTreeMap<(NodeId, String), String>,
    behavior_modified: bool,
    clock: Option<Rc<StatePreviewClock>>,
    clock_time: f64,
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
        let edits = group
            .plan(&self.document)
            .map_err(|error| error.to_string())?;
        self.apply("Edit visual state group", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn remove_state_group(&mut self, owner: NodeId, name: &str) -> Result<(), String> {
        let edits = VisualStateGroup::remove(&self.document, owner, name)
            .map_err(|error| error.to_string())?;
        self.apply("Remove visual state group", edits)
            .map_err(|error| error.to_string())?;
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
        if elapsed.is_some_and(|time| {
            !time.is_finite() || time < 0.0 || time * 10_000_000.0 >= i64::MAX as f64
        }) {
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
        let key = (owner, name.to_owned());
        let from = group
            .states
            .iter()
            .find(|state| {
                owner_visual
                    .classes()
                    .contains(&group.class_name(&state.name))
            })
            .map(|state| state.name.clone())
            .or_else(|| self.interactions.active_states.get(&key).cloned());
        let mut next = self.interactions.active_states.clone();
        if let Some(state) = selected {
            next.insert(key.clone(), state.name.clone());
        } else {
            next.remove(&key);
        }
        let duration = selected
            .and_then(|state| {
                group
                    .transitions
                    .iter()
                    .find(|transition| transition.to == state.name && transition.from == from)
                    .or_else(|| {
                        group.transitions.iter().find(|transition| {
                            transition.to == state.name && transition.from.is_none()
                        })
                    })
            })
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
            if let Some(from) = from.filter(|from| {
                group.transitions.iter().any(|transition| {
                    transition.from.as_deref() == Some(from.as_str()) && transition.to == state.name
                })
            }) {
                classes.add(&group.transition_class(&from, &state.name));
            }
            classes.add(&group.class_name(&state.name));
        } else if let Some(original) = self.interactions.original_classes.remove(&key) {
            for class in original {
                classes.add(&class);
            }
        }
        self.interactions.active_states = next;
        self.preview.host.update_layout();
        clock.subject.on_next(TimeSpan::ZERO);
        clock.subject.on_next(TimeSpan::from_seconds(time));
        self.interactions.clock = Some(clock);
        self.interactions.clock_time = time;
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
        let time = self.interactions.clock_time + seconds;
        if !seconds.is_finite()
            || seconds < 0.0
            || !time.is_finite()
            || time * 10_000_000.0 >= i64::MAX as f64
        {
            return Err("State clock delta is outside the native clock range".into());
        }
        let clock = self
            .interactions
            .clock
            .as_ref()
            .ok_or("No state preview is active")?;
        self.preview.clear_animation();
        clock.subject.on_next(TimeSpan::from_seconds(time));
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
    pub fn behaviors(&self, owner: NodeId) -> Vec<BehaviorModel> {
        BehaviorModel::read(&self.document, owner).unwrap_or_default()
    }
    pub fn upsert_behavior(&mut self, model: BehaviorModel) -> Result<NodeId, String> {
        let before = self
            .behaviors(model.owner)
            .iter()
            .filter_map(|model| model.id)
            .collect::<BTreeSet<_>>();
        let edits = model
            .plan(&self.document)
            .map_err(|error| error.to_string())?;
        self.apply("Edit behavior", edits)
            .map_err(|error| error.to_string())?;
        if let Some(id) = model.id {
            return Ok(id);
        }
        self.behaviors(model.owner)
            .into_iter()
            .filter_map(|model| model.id)
            .find(|id| !before.contains(id))
            .ok_or_else(|| "Created behavior was not found".into())
    }
    pub fn remove_behavior(&mut self, id: NodeId) -> Result<(), String> {
        let edits = BehaviorModel::remove(&self.document, id).map_err(|error| error.to_string())?;
        self.apply("Remove behavior", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn set_behavior_source(&mut self, id: NodeId, source: &str) -> Result<(), String> {
        let owner = self.behavior_owner(id)?;
        let mut model = BehaviorModel::from_source(source, owner, Some(id))
            .map_err(|error| error.to_string())?;
        model.id = Some(id);
        self.upsert_behavior(model)?;
        Ok(())
    }
    /// Execute the real attached trigger's actions, preserving bindings, name scopes and
    /// custom IAction implementations. The host must be attached to a rooted logical tree
    /// so native action styles, bindings and lifecycle hooks have run. Stop/reload restores
    /// the authored preview afterwards.
    pub fn preview_behavior(&mut self, id: NodeId) -> Result<(), String> {
        let owner = self.behavior_owner(id)?;
        let models = self.behaviors(owner);
        let index = models
            .iter()
            .position(|model| model.id == Some(id))
            .ok_or("Unknown behavior")?;
        let visual = self
            .preview
            .visual(owner)
            .ok_or("Behavior owner has no live preview")?;
        if visual
            .cast::<StyledElement>()
            .is_some_and(|element| !element.is_attached_to_logical_tree())
        {
            return Err("Attach the preview host to a Window or rooted logical tree before executing behaviors; native action styles and bindings require attachment".into());
        }
        let behaviors = ferroui_xaml_interactivity::Interaction::get_behaviors(&visual);
        let snapshot = behaviors.snapshot();
        let behavior = snapshot
            .get(index)
            .ok_or("The authored behavior is not attached in the current preview")?;
        let trigger=<dyn ferroui_xaml_interactivity::ITrigger>::of(behavior).ok_or("This behavior is not an action trigger; use Interact to exercise its native event/lifecycle behavior")?;
        let sender: BoxedValue = Rc::new(visual.clone());
        ferroui_xaml_interactivity::Interaction::execute_actions(
            Some(&sender),
            trigger.actions().as_ref(),
            None,
        );
        self.interactions.behavior_modified = true;
        for group in self.state_groups() {
            let Some(owner) = self
                .preview
                .visual(group.owner)
                .and_then(|visual| visual.cast::<StyledElement>())
            else {
                continue;
            };
            let key = (group.owner, group.name.clone());
            if let Some(state) = group
                .states
                .iter()
                .find(|state| owner.classes().contains(&group.class_name(&state.name)))
            {
                self.interactions
                    .active_states
                    .insert(key, state.name.clone());
            } else {
                self.interactions.active_states.remove(&key);
            }
        }
        self.import_animations();
        self.generation += 1;
        Ok(())
    }
    fn behavior_owner(&self, id: NodeId) -> Result<NodeId, String> {
        let mut current = self.document.ast().node(id).and_then(|node| node.parent);
        while let Some(id) = current {
            let node = self
                .document
                .ast()
                .node(id)
                .ok_or("Unknown behavior ancestor")?;
            if node.local_name() == "Interaction.Behaviors" {
                return node
                    .parent
                    .ok_or_else(|| "Behavior collection has no owner".into());
            }
            current = node.parent;
        }
        Err("Source node is not a behavior".into())
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

struct StatePreviewClock {
    subject: LightweightSubject<TimeSpan>,
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
impl IObservable<TimeSpan> for StatePreviewClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
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
