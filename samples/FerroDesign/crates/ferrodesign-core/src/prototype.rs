//! Deterministic prototype state, event matching and activation-scoped delays.
//! The host owns its clock and external actions. Timer tokens are capabilities
//! for one active frame/overlay instance and cannot fire after that instance
//! exits, even when a native timer callback was already queued.

use crate::*;
use std::collections::{BTreeMap, BTreeSet};

#[path = "prototype_animation.rs"]
mod animation;
pub use animation::*;
#[path = "prototype_scroll.rs"]
mod scroll;
pub use scroll::*;

const MAX_PROTOTYPE_HISTORY: usize = 1000;
const MAX_PROTOTYPE_OVERLAYS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PrototypeTimer {
    pub activation: u64,
    pub root: NodeId,
    pub node: NodeId,
    pub interaction: usize,
    pub delay_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Activation {
    id: u64,
    root: NodeId,
}

#[derive(Clone, Debug, PartialEq)]
struct TransientPrototypeState {
    trigger: Trigger,
    source: NodeId,
    document: Document,
    player: Box<PrototypePlayer>,
    transition: Transition,
}

/// A player's variable overrides belong to its preview session; authored
/// variable definitions are never changed. The host should render a disposable
/// document copy because applying an override updates the copy's bound nodes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrototypePlayer {
    pub current: Option<NodeId>,
    pub overlays: Vec<(NodeId, Point)>,
    pub history: Vec<NodeId>,
    pub last_transition: Transition,
    pub last_action: Option<PrototypeAction>,
    pub pending_url: Option<String>,
    pub scroll_target: Option<NodeId>,
    pub scroll: PrototypeScrollState,
    variable_overrides: BTreeMap<VariableId, VariableValue>,
    activations: Vec<Activation>,
    next_activation: u64,
    fired_timers: BTreeSet<(u64, NodeId, usize)>,
    transients: Vec<TransientPrototypeState>,
}

impl PrototypePlayer {
    pub fn start(&mut self, document: &Document, page: NodeId) -> bool {
        let root = document.page(page).and_then(|page| {
            page.prototype_start.or_else(|| {
                page.children
                    .iter()
                    .find(|id| {
                        document
                            .nodes
                            .get(id)
                            .is_some_and(|node| node.kind == NodeKind::Frame && node.visible)
                    })
                    .copied()
            })
        });
        root.is_some_and(|root| self.start_at(document, root))
    }

    pub fn start_at(&mut self, document: &Document, root: NodeId) -> bool {
        if !document.nodes.contains_key(&root) || !document.effective_visible(root) {
            return false;
        }
        self.stop();
        self.current = Some(root);
        self.reconcile_activations();
        true
    }

    pub fn stop(&mut self) {
        self.current = None;
        self.overlays.clear();
        self.history.clear();
        self.last_transition = Transition::default();
        self.last_action = None;
        self.pending_url = None;
        self.scroll_target = None;
        self.scroll.clear();
        self.variable_overrides.clear();
        self.activations.clear();
        self.fired_timers.clear();
        self.transients.clear();
        // Do not reuse activation IDs after restarting this player.
        self.next_activation = self.next_activation.wrapping_add(1);
    }

    /// The top overlay is modal. Design-time locking does not disable prototype
    /// interactions, but hidden nodes and nodes outside this root cannot fire.
    pub fn interaction_root(&self) -> Option<NodeId> {
        self.overlays
            .last()
            .map(|overlay| overlay.0)
            .or(self.current)
    }

    pub fn variable_overrides(&self) -> &BTreeMap<VariableId, VariableValue> {
        &self.variable_overrides
    }

    pub fn scroll_by(&mut self, document: &mut Document, node: NodeId, delta: Point) -> bool {
        let Some(root) = self.interaction_root() else { return false };
        self.scroll.scroll_by(document, root, node, delta)
    }

    pub fn trigger(&mut self, document: &mut Document, node: NodeId, trigger: Trigger) -> bool {
        if matches!(trigger, Trigger::Key | Trigger::AfterDelay) {
            return false;
        }
        self.trigger_matching(document, node, |interaction| {
            interaction.trigger == trigger
                || (trigger == Trigger::Hover && interaction.trigger == Trigger::WhileHover)
                || (trigger == Trigger::Press && interaction.trigger == Trigger::WhilePress)
        })
    }

    pub fn has_transient(&self, trigger: Trigger) -> bool {
        self.transients.iter().any(|state| state.trigger == trigger)
    }

    pub fn has_transients(&self) -> bool {
        !self.transients.is_empty()
    }

    /// Hover is tested against the original activation's shape and overlay
    /// placement. Changing frames or opening an overlay cannot accidentally
    /// end a while-hover action merely because the picked node ID changed.
    pub fn transient_contains(&self, trigger: Trigger, world: Point) -> bool {
        let Some(state) = self.transients.iter().find(|state| state.trigger == trigger) else { return false };
        let mut point = world;
        if let (Some(current), Some(&(overlay, position))) = (state.player.current, state.player.overlays.last()) {
            if is_within(&state.document, state.source, overlay) {
                if let (Some(current), Some(overlay)) = (state.document.world_bounds(current), state.document.world_bounds(overlay)) {
                    point = point - Point::new(current.x + position.x - overlay.x, current.y + position.y - overlay.y);
                }
            }
        }
        state.document.hit_test_node(state.source, point, 0.0)
    }

    /// Restores the exact preview state before a reversible trigger, including
    /// variable-bound nodes, scroll offsets, history, and the overlay stack.
    /// Ending an outer hover also ends a press started inside that hover.
    pub fn end_transient(&mut self, document: &mut Document, trigger: Trigger) -> bool {
        let Some(index) = self.transients.iter().position(|state| state.trigger == trigger) else { return false };
        let next_activation = self.next_activation;
        let mut previous = std::mem::take(&mut self.transients);
        let state = previous.remove(index);
        previous.truncate(index);
        *document = state.document;
        *self = *state.player;
        self.transients = previous;
        self.last_action = None;
        self.last_transition = reverse_transition(state.transition);
        // Never revive an already queued timer capability on restoration.
        self.next_activation = self.next_activation.max(next_activation);
        self.activations.clear();
        self.fired_timers.clear();
        self.reconcile_activations();
        true
    }

    pub fn trigger_key(&mut self, document: &mut Document, node: NodeId, key: &str) -> bool {
        let Some(key) = normalize_prototype_key(key) else {
            return false;
        };
        self.trigger_matching(document, node, |interaction| {
            interaction.trigger == Trigger::Key
                && interaction
                    .key
                    .as_deref()
                    .and_then(normalize_prototype_key)
                    .is_some_and(|expected| expected == key)
        })
    }

    fn trigger_matching(
        &mut self,
        document: &mut Document,
        node: NodeId,
        matches: impl Fn(&Interaction) -> bool,
    ) -> bool {
        self.reconcile_activations();
        let Some(root) = self.interaction_root() else {
            return false;
        };
        if !is_within(document, node, root) || !document.effective_visible(node) {
            return false;
        }
        let interaction = document
            .nodes
            .get(&node)
            .and_then(|node| {
                node.interactions
                    .iter()
                    .find(|interaction| matches(interaction))
            })
            .cloned();
        let Some(interaction) = interaction else { return false };
        if matches!(interaction.trigger, Trigger::WhileHover | Trigger::WhilePress) {
            if self.has_transient(interaction.trigger)
                || matches!(interaction.action, PrototypeAction::OpenUrl { .. })
            {
                return false;
            }
            let snapshot = TransientPrototypeState {
                trigger: interaction.trigger,
                source: node,
                document: document.clone(),
                player: Box::new(self.checkpoint()),
                transition: interaction.transition.clone(),
            };
            if self.execute(document, interaction) {
                self.transients.push(snapshot);
                return true;
            }
            false
        } else {
            let persistent = matches!(interaction.trigger, Trigger::Click | Trigger::Key | Trigger::Drag);
            let changed = self.execute(document, interaction);
            if changed && persistent {
                self.transients.clear();
            }
            changed
        }
    }

    fn checkpoint(&self) -> Self {
        // Do not recursively clone previous transient document snapshots.
        Self {
            current: self.current,
            overlays: self.overlays.clone(),
            history: self.history.clone(),
            last_transition: self.last_transition.clone(),
            last_action: self.last_action.clone(),
            pending_url: self.pending_url.clone(),
            scroll_target: self.scroll_target,
            scroll: self.scroll.clone(),
            variable_overrides: self.variable_overrides.clone(),
            activations: self.activations.clone(),
            next_activation: self.next_activation,
            fired_timers: self.fired_timers.clone(),
            transients: Vec::new(),
        }
    }

    /// All not-yet-fired delays for visible nodes of active frame/overlay
    /// instances, sorted by delay and stable document traversal. Opening an
    /// overlay preserves the underlying frame's elapsed timers; reopening a
    /// closed overlay gives that new instance fresh timer identities.
    pub fn pending_timers(&mut self, document: &Document) -> Vec<PrototypeTimer> {
        self.reconcile_activations();
        let mut timers = Vec::new();
        for activation in &self.activations {
            for node in document.descendants(activation.root) {
                if !document.effective_visible(node) {
                    continue;
                }
                for (index, interaction) in document.nodes[&node].interactions.iter().enumerate() {
                    if interaction.trigger != Trigger::AfterDelay
                        || self.fired_timers.contains(&(activation.id, node, index))
                    {
                        continue;
                    }
                    timers.push(PrototypeTimer {
                        activation: activation.id,
                        root: activation.root,
                        node,
                        interaction: index,
                        delay_ms: interaction.delay_ms,
                    });
                }
            }
        }
        timers.sort_by_key(|timer| timer.delay_ms);
        timers
    }

    /// Fires an identified delay after the host reports its elapsed time. Early,
    /// duplicate, edited, hidden and exited-instance callbacks are ignored.
    /// Invalid authored actions are consumed once, avoiding a retry timer loop.
    pub fn fire_timer(
        &mut self,
        document: &mut Document,
        timer: PrototypeTimer,
        elapsed_ms: u64,
    ) -> bool {
        if elapsed_ms < timer.delay_ms as u64 {
            return false;
        }
        self.reconcile_activations();
        if !self
            .activations
            .iter()
            .any(|activation| activation.id == timer.activation && activation.root == timer.root)
            || !is_within(document, timer.node, timer.root)
            || !document.effective_visible(timer.node)
        {
            return false;
        }
        let Some(interaction) = document
            .nodes
            .get(&timer.node)
            .and_then(|node| node.interactions.get(timer.interaction))
            .filter(|interaction| {
                interaction.trigger == Trigger::AfterDelay && interaction.delay_ms == timer.delay_ms
            })
            .cloned()
        else {
            return false;
        };
        if !self
            .fired_timers
            .insert((timer.activation, timer.node, timer.interaction))
        {
            return false;
        }
        self.execute(document, interaction)
    }

    pub fn back(&mut self, document: &Document) -> bool {
        if self.overlays.pop().is_some() {
            self.reconcile_activations();
            self.pending_url = None;
            self.scroll_target = None;
            self.last_action = Some(PrototypeAction::Back);
            return true;
        }
        let Some(index) = self
            .history
            .iter()
            .rposition(|id| document.nodes.contains_key(id))
        else {
            return false;
        };
        let previous = self.history[index];
        self.history.truncate(index);
        self.current = Some(previous);
        self.pending_url = None;
        self.scroll_target = None;
        self.activations.clear();
        self.fired_timers.clear();
        self.reconcile_activations();
        self.last_action = Some(PrototypeAction::Back);
        true
    }

    fn execute(&mut self, document: &mut Document, interaction: Interaction) -> bool {
        let transition = interaction.transition;
        let action = interaction.action.clone();
        match interaction.action {
            PrototypeAction::NavigateTo { target } => {
                if !document.nodes.contains_key(&target) || !document.effective_visible(target) {
                    return false;
                }
                if let Some(current) = self.current.filter(|current| *current != target) {
                    self.history.push(current);
                    if self.history.len() > MAX_PROTOTYPE_HISTORY {
                        self.history.remove(0);
                    }
                }
                self.current = Some(target);
                self.scroll.reset_subtree(document, target);
                self.overlays.clear();
                self.activations.clear();
                self.fired_timers.clear();
                self.pending_url = None;
                self.scroll_target = None;
                self.reconcile_activations();
            }
            PrototypeAction::OpenOverlay { target, position } => {
                if !document.nodes.contains_key(&target)
                    || !document.effective_visible(target)
                    || !position.x.is_finite()
                    || !position.y.is_finite()
                    || self.overlays.len() >= MAX_PROTOTYPE_OVERLAYS
                {
                    return false;
                }
                self.overlays.push((target, position));
                self.scroll.reset_subtree(document, target);
                self.reconcile_activations();
            }
            PrototypeAction::CloseOverlay => {
                if self.overlays.pop().is_none() {
                    return false;
                }
                self.reconcile_activations();
            }
            PrototypeAction::Back => {
                if !self.back(document) {
                    return false;
                }
            }
            PrototypeAction::SetVariable { variable, value } => {
                if !document.variables.contains_key(&variable)
                    || !valid_variable_value(document, variable, &value, &self.variable_overrides)
                {
                    return false;
                }
                let previous = self.variable_overrides.insert(variable, value);
                if variable_affects_layout(document, variable, &self.variable_overrides) {
                    // Spacing can resize Hug ancestors and move siblings. Work
                    // on one candidate so failed validation or reflow cannot
                    // partially change either the preview or its override map.
                    let mut candidate = document.clone();
                    if !self.apply_variable_overrides(&mut candidate, true) {
                        if let Some(previous) = previous {
                            self.variable_overrides.insert(variable, previous);
                        } else {
                            self.variable_overrides.remove(&variable);
                        }
                        return false;
                    }
                    *document = candidate;
                } else {
                    // Preserve the existing allocation-light path for color,
                    // text and other actions unrelated to spacing bindings.
                    self.apply_variable_overrides(document, false);
                }
            }
            PrototypeAction::OpenUrl { url } => {
                let Some(url) = prototype_external_url(&url) else {
                    return false;
                };
                self.pending_url = Some(url);
            }
            PrototypeAction::ScrollTo { target } => {
                if !document.nodes.contains_key(&target)
                    || !document.effective_visible(target)
                    || !self.interaction_root().is_some_and(|root| is_within(document, target, root))
                {
                    return false;
                }
                self.scroll_target = Some(target);
                if let Some(root) = self.interaction_root() {
                    self.scroll.scroll_to(document, root, target);
                }
            }
        }
        self.last_transition = transition;
        self.last_action = Some(action);
        true
    }

    fn reconcile_activations(&mut self) {
        let roots: Vec<_> = self
            .current
            .into_iter()
            .chain(self.overlays.iter().map(|overlay| overlay.0))
            .collect();
        let prefix = self
            .activations
            .iter()
            .zip(&roots)
            .take_while(|(activation, root)| activation.root == **root)
            .count();
        self.activations.truncate(prefix);
        for root in roots.into_iter().skip(prefix) {
            self.next_activation = self.next_activation.wrapping_add(1);
            self.activations.push(Activation {
                id: self.next_activation,
                root,
            });
        }
        self.fired_timers
            .retain(|(id, _, _)| self.activations.iter().any(|scope| scope.id == *id));
    }

    fn apply_variable_overrides(&self, document: &mut Document, reflow_layout: bool) -> bool {
        let mode = document.variable_mode.clone();
        let mut authored = Vec::with_capacity(self.variable_overrides.len());
        for (id, value) in &self.variable_overrides {
            if let Some(variable) = document.variables.get_mut(id) {
                authored.push((*id, variable.values.insert(mode.clone(), value.clone())));
            }
        }
        // Validate while temporary mode values are installed; restoring the
        // authored table first would hide negative or wrongly typed overrides.
        let valid = if reflow_layout && document.validate_variables().is_err() {
            false
        } else {
            document.apply_variables();
            !reflow_layout
                || (document.layout_all().errors.is_empty() && document.validate().is_ok())
        };
        for (id, value) in authored {
            if let Some(variable) = document.variables.get_mut(&id) {
                if let Some(value) = value {
                    variable.values.insert(mode.clone(), value);
                } else {
                    variable.values.remove(&mode);
                }
            }
        }
        valid
    }
}

/// Follow the effective alias path, including prior session overrides, before
/// deciding whether an action needs an atomic document-wide layout candidate.
/// A color-only action does not clone/reflow unrelated auto-layout trees.
fn variable_affects_layout(
    document: &Document,
    changed: VariableId,
    overrides: &BTreeMap<VariableId, VariableValue>,
) -> bool {
    let bindings: BTreeSet<_> = document.nodes.values().flat_map(|node| {
        node.variable_bindings.iter().filter_map(|(property, &id)| {
            crate::parse_layout_variable_binding(property).map(|_| id)
        })
    }).collect();
    for mut id in bindings {
        let mut seen = BTreeSet::new();
        while seen.insert(id) {
            if id == changed { return true; }
            let value = overrides.get(&id).or_else(|| {
                let variable = document.variables.get(&id)?;
                variable.values.get(&document.variable_mode)
                    .or_else(|| variable.values.get("Default"))
                    .or_else(|| variable.values.values().next())
            });
            let Some(VariableValue::Alias(target)) = value else { break; };
            id = *target;
        }
    }
    false
}

fn is_within(document: &Document, mut node: NodeId, root: NodeId) -> bool {
    for _ in 0..MAX_DOCUMENT_DEPTH {
        if node == root {
            return true;
        }
        let Some(parent) = document.nodes.get(&node).and_then(|node| node.parent) else {
            return false;
        };
        node = parent;
    }
    false
}

fn reverse_transition(mut transition: Transition) -> Transition {
    transition.kind = match transition.kind {
        TransitionKind::SlideLeft | TransitionKind::Push => TransitionKind::SlideRight,
        TransitionKind::SlideRight => TransitionKind::SlideLeft,
        TransitionKind::SlideUp => TransitionKind::SlideDown,
        TransitionKind::SlideDown => TransitionKind::SlideUp,
        TransitionKind::MoveIn => TransitionKind::MoveOut,
        TransitionKind::MoveOut => TransitionKind::MoveIn,
        kind => kind,
    };
    transition.easing = match transition.easing {
        Easing::EaseIn => Easing::EaseOut,
        Easing::EaseOut => Easing::EaseIn,
        easing => easing,
    };
    transition
}

fn valid_variable_value(
    document: &Document,
    variable: VariableId,
    value: &VariableValue,
    overrides: &BTreeMap<VariableId, VariableValue>,
) -> bool {
    let mut value = value;
    let mut seen = BTreeSet::from([variable]);
    loop {
        match value {
            VariableValue::Number(number) => return number.is_finite(),
            VariableValue::Alias(id) => {
                if !seen.insert(*id) {
                    return false;
                }
                value = if let Some(value) = overrides.get(id) {
                    value
                } else {
                    let Some(variable) = document.variables.get(id) else {
                        return false;
                    };
                    let Some(value) = variable
                        .values
                        .get(&document.variable_mode)
                        .or_else(|| variable.values.get("Default"))
                        .or_else(|| variable.values.values().next())
                    else {
                        return false;
                    };
                    value
                };
            }
            _ => return true,
        }
    }
}

/// Normalizes key chords independently of a browser/native key enum.
pub fn normalize_prototype_key(key: &str) -> Option<String> {
    let key = if key == " " { "space" } else { key.trim() };
    if key.is_empty() {
        return None;
    }
    let plus_key = key.ends_with('+');
    let mut modifiers = [false; 4];
    let mut primary: Option<String> = None;
    for token in key
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let token = token.to_lowercase();
        match token.as_str() {
            "ctrl" | "control" => modifiers[0] = true,
            "alt" | "option" => modifiers[1] = true,
            "shift" => modifiers[2] = true,
            "meta" | "cmd" | "command" | "super" | "win" | "windows" => modifiers[3] = true,
            _ => {
                if primary.is_some() {
                    return None;
                }
                primary = Some(
                    match token.as_str() {
                        "return" => "enter",
                        "esc" => "escape",
                        "spacebar" => "space",
                        "arrowup" => "up",
                        "arrowdown" => "down",
                        "arrowleft" => "left",
                        "arrowright" => "right",
                        "back" => "backspace",
                        "del" => "delete",
                        "pgup" | "page up" => "pageup",
                        "pgdn" | "page down" => "pagedown",
                        _ => &token,
                    }
                    .to_string(),
                );
            }
        }
    }
    if plus_key {
        if primary.is_some() {
            return None;
        }
        primary = Some("+".to_string());
    }
    let primary = primary?;
    // A printable symbol already includes the layout's Shift transformation
    // (for example '+' instead of '='). Keep Shift for named and letter keys,
    // while matching Ctrl+Shift+'+' to the authored Ctrl+'+' chord.
    if primary.chars().count() == 1
        && primary.chars().next().is_some_and(|symbol| {
            !symbol.is_alphanumeric() && !symbol.is_whitespace() && !symbol.is_control()
        })
    {
        modifiers[2] = false;
    }
    let mut normalized = String::new();
    for (index, modifier) in ["ctrl", "alt", "shift", "meta"].iter().enumerate() {
        if modifiers[index] {
            normalized.push_str(modifier);
            normalized.push('+');
        }
    }
    normalized.push_str(&primary);
    Some(normalized)
}

/// Allowed external-navigation requests; emitting one does not imply that a
/// host opened it. Script/data schemes and control characters are rejected.
pub fn prototype_external_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() || url.chars().any(char::is_control) {
        return None;
    }
    let (scheme, rest) = url.split_once(':')?;
    match scheme.to_ascii_lowercase().as_str() {
        "http" | "https" => {
            let authority = rest.strip_prefix("//")?.split(['/', '?', '#']).next()?;
            if authority.is_empty() || authority.chars().any(char::is_whitespace) {
                return None;
            }
        }
        "mailto" | "tel" if !rest.is_empty() => {}
        _ => return None,
    }
    Some(url.to_string())
}

/// Validates an authored rule without requiring a running presentation. Runtime
/// execution additionally checks the active frame/overlay and available history.
pub fn validate_prototype_interaction(
    document: &Document,
    source: NodeId,
    interaction: &Interaction,
) -> Result<(), String> {
    if !document.nodes.contains_key(&source) {
        return Err("The source layer no longer exists.".into());
    }
    if document.effective_locked(source) {
        return Err("Unlock the source layer before editing its interactions.".into());
    }
    if matches!(interaction.trigger, Trigger::WhileHover | Trigger::WhilePress)
        && matches!(interaction.action, PrototypeAction::OpenUrl { .. })
    {
        return Err("Choose a reversible action for a while-hover or while-press trigger.".into());
    }
    if interaction.trigger == Trigger::Key
        && interaction
            .key
            .as_deref()
            .and_then(normalize_prototype_key)
            .is_none()
    {
        return Err("Enter a key or chord, such as Enter, Escape, or Ctrl+Shift+A.".into());
    }
    match &interaction.action {
        PrototypeAction::NavigateTo { target } | PrototypeAction::ScrollTo { target } => {
            if !document.nodes.contains_key(target) {
                return Err("Choose an existing destination layer.".into());
            }
        }
        PrototypeAction::OpenOverlay { target, position } => {
            if !document.nodes.contains_key(target) {
                return Err("Choose an existing overlay layer.".into());
            }
            if !position.x.is_finite() || !position.y.is_finite() {
                return Err("Overlay coordinates must be finite numbers.".into());
            }
        }
        PrototypeAction::SetVariable { variable, value } => {
            if !document.variables.contains_key(variable) {
                return Err("Choose an existing variable.".into());
            }
            if !valid_variable_value(document, *variable, value, &BTreeMap::new()) {
                return Err(
                    "Use a finite variable value or an existing alias without a cycle.".into(),
                );
            }
        }
        PrototypeAction::OpenUrl { url } => {
            if prototype_external_url(url).is_none() {
                return Err("Enter a complete http, https, mailto, or tel URL.".into());
            }
        }
        PrototypeAction::CloseOverlay | PrototypeAction::Back => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "prototype_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "prototype_layout_variables_tests.rs"]
mod layout_variable_tests;
