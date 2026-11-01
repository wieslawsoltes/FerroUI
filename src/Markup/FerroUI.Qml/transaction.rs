use super::*;
use ferroui_qml_compiler::Binding;
use std::collections::VecDeque;

/// The undo log only touches slots affected by this write, including dependency edges.
/// A failed handler or binding therefore cannot leave a half-applied native update.
pub(crate) struct Transaction<'a> {
    pub(crate) incubating: bool,
    pub(crate) current_binding: Option<SlotId>,
    pub(crate) guest_return_type: Option<String>,
    pub(crate) guest_parameters: Vec<String>,
    pub(crate) used_engine: bool,
    pub(crate) script_deadline: Option<std::time::Instant>,
    pub(crate) failed_host_call: Option<Error>,
    pub(crate) nested_retirements: Vec<ObjectId>,
    pub(crate) model_edits: Vec<(ObjectId, list_model::Undo)>,
    model_modes: BTreeSet<ObjectId>,
    pub(crate) record_changes: records::Journal,
    pub(crate) model: &'a mut Instance,
    original: BTreeMap<SlotId, Slot>,
    emitted: Vec<SignalEmission>,
    pub(crate) pending: BTreeSet<SlotId>,
    pending_bindings: BTreeSet<ObjectId>,
    pub(crate) state_pending: BTreeMap<ObjectId, bool>,
    evaluating: BTreeSet<SlotId>,
    notifications: VecDeque<(ObjectId, String)>,
    hold_notifications: bool,
    pub(crate) initializing: bool,
    pub(crate) animations: animations::Turn,
    scope: Option<u32>,
    pub(crate) depth: usize,
    steps: usize,
    pub(crate) connection_changes: signal_callbacks::Journal,
    pub(crate) deferred: Vec<deferred::Task>,
    pub(crate) structure: dynamic::Checkpoint,
    pub(crate) worker_sources: BTreeSet<ObjectId>,
    pub(crate) network_loads: Vec<(ObjectId, url::Url)>,
    pub(crate) worker_actions: Vec<workers::Action>,
    pub(crate) loader_changes: loader::Changes,
    pub(crate) repeater_changes: repeater::Changes,
    pub(crate) timer_changes: timers::TimerChanges,
}

impl Instance {
    /// A failed asynchronous job has already rolled back its subtree. Its
    /// terminal status must survive an exception in an error observer. Settle
    /// and validate the status first, then deliver its changes transactionally.
    pub(crate) fn report_loader_failure(&mut self, id: ObjectId) -> Result<(), Error> {
        self.publish_then_notify(|tx| tx.fail_loader(id))
    }

    pub(crate) fn publish_then_notify(
        &mut self,
        change: impl FnOnce(&mut Transaction<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let notifications = self.transaction(|tx| {
            tx.hold_notifications = true;
            change(tx)?;
            tx.settle()?;
            Ok(std::mem::take(&mut tx.notifications))
        })?;
        if notifications.is_empty() {
            return Ok(());
        }
        self.transaction(|tx| {
            tx.notifications = notifications;
            Ok(())
        })
    }

    pub(crate) fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut Transaction<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let structure = self.checkpoint();
        let mut tx = Transaction {
            structure,
            incubating: false,
            current_binding: None,
            guest_return_type: None,
            guest_parameters: Vec::new(),
            used_engine: false,
            script_deadline: None,
            failed_host_call: None,
            record_changes: self.records.journal(),
            model: self,
            nested_retirements: Vec::new(),
            model_edits: Vec::new(),
            model_modes: BTreeSet::new(),
            original: BTreeMap::new(),
            emitted: Vec::new(),
            pending: BTreeSet::new(),
            pending_bindings: BTreeSet::new(),
            state_pending: Default::default(),
            evaluating: BTreeSet::new(),
            notifications: VecDeque::new(),
            hold_notifications: false,
            initializing: false,
            animations: Default::default(),
            scope: None,
            depth: 0,
            steps: 0,
            connection_changes: Default::default(),
            deferred: Vec::new(),
            loader_changes: Default::default(),
            repeater_changes: Default::default(),
            timer_changes: Default::default(),
            network_loads: Vec::new(),
            worker_actions: Default::default(),
            worker_sources: Default::default(),
        };
        let result = operation(&mut tx).and_then(|value| {
            tx.settle()?;
            if let Some(error) = tx.failed_host_call.take() {
                return Err(error);
            }
            tx.model.validate_connections()?;
            tx.model.validate_model_modes(&tx.model_modes)?;
            if let Some(validate) = &tx.model.validator {
                let changes = tx
                    .original
                    .iter()
                    .filter_map(|(key, old)| {
                        let value = &tx.model.slots[*key].value;
                        if old.value.unchanged(value)
                            && old.assigned == tx.model.slots[*key].assigned
                        {
                            return None;
                        }
                        let property = &tx.model.properties[*key];
                        if tx.model.node(property.owner).is_none()
                            || property.name.starts_with("$qml:")
                        {
                            return None;
                        }
                        Some(Change {
                            object: property.owner,
                            property: property.name.clone(),
                            value: value.clone(),
                        })
                    })
                    .collect::<Vec<_>>();
                validate(tx.model, &changes)?;
            }
            Ok(value)
        });
        if result.is_err() {
            tx.model
                .javascript
                .connections
                .rollback(std::mem::take(&mut tx.connection_changes));
            for (id, undo) in tx.model_edits.into_iter().rev() {
                undo.restore(tx.model.models.get_mut(&id).unwrap());
            }
            tx.model
                .records
                .rollback(std::mem::take(&mut tx.record_changes));
            std::mem::take(&mut tx.loader_changes).rollback(tx.model);
            std::mem::take(&mut tx.repeater_changes).rollback(tx.model);
            tx.timer_changes.rollback(&mut tx.model.timers);
            std::mem::take(&mut tx.animations).rollback(tx.model);
            for (key, old) in tx.original {
                tx.model.slots[key] = old;
            }
            tx.model.rollback_creation(tx.structure);
        } else {
            tx.model
                .javascript
                .deferred
                .post(std::mem::take(&mut tx.deferred));
            tx.model
                .commit_workers(std::mem::take(&mut tx.worker_actions));
            tx.model.emissions.append(&mut tx.emitted);
            for (key, old) in tx.original {
                let value = &tx.model.slots[key].value;
                if tx.model.node(tx.model.properties[key].owner).is_none()
                    || tx.model.properties[key].name.starts_with("$qml:")
                {
                    continue;
                }
                if !old.value.unchanged(value) || old.assigned != tx.model.slots[key].assigned {
                    tx.model.changes.insert(key, value.clone());
                }
            }
            tx.model.retire_objects(&tx.structure.removed);
            for (id, url) in tx.network_loads {
                if tx.model.node(id).is_some() {
                    let transport = tx
                        .model
                        .dynamic
                        .registry
                        .as_ref()
                        .and_then(|r| r.component_transport.as_deref())
                        .expect("registered component transport");
                    tx.model.network.start(transport, id, url);
                }
            }
        }
        if tx.used_engine {
            if let Some(engine) = &tx.model.javascript.engine {
                engine.end_transaction(tx.model.script_instance(), result.is_ok());
            }
        }
        drop(tx.record_changes);
        tx.model.records.maybe_collect();
        result
    }
}

impl Transaction<'_> {
    pub(crate) fn declaration_scope(&self) -> Option<u32> {
        self.scope
    }

    pub(crate) fn scoped_key(&self, object: ObjectId, name: &str) -> Result<SlotId, Error> {
        self.model.key_in(object, name, self.scope)
    }

    pub(crate) fn with_scope<T>(
        &mut self,
        scope: Option<u32>,
        operation: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let previous = std::mem::replace(&mut self.scope, scope);
        let result = operation(self);
        self.scope = previous;
        result
    }

    pub(crate) fn touched(&mut self, key: SlotId) -> &mut Slot {
        self.original
            .entry(key)
            .or_insert_with(|| self.model.slots[key].clone());
        &mut self.model.slots[key]
    }

    pub(crate) fn budget(&mut self) -> Result<(), Error> {
        self.steps += 1;
        // A fixed overhead handles event feedback from small components; the
        // per-slot term permits initialization of legitimately large documents.
        if self.steps > 4096 + self.model.slots.len().saturating_mul(64) {
            return Err(Error::new(
                "binding or signal feedback exceeded the component work limit",
            ));
        }
        Ok(())
    }

    pub(crate) fn initialize(&mut self) -> Result<(), Error> {
        self.initialize_range(0, 0)
    }
    pub(crate) fn initialize_range(
        &mut self,
        first: usize,
        first_slot: usize,
    ) -> Result<(), Error> {
        let end = self.model.nodes.len();
        let previous = self.initializing;
        self.initializing = true;
        self.initialize_workers(first);
        self.initialize_states(first);
        self.initialize_loaders(first);
        self.repeater_changes.pending.extend(
            self.model
                .dynamic
                .repeaters
                .keys()
                .filter(|id| id.index >= first),
        );
        self.model_modes.extend(
            self.model
                .models
                .keys()
                .copied()
                .filter(|id| id.index >= first),
        );
        self.pending_bindings.extend(
            self.model
                .bindings
                .keys()
                .copied()
                .filter(|id| id.index >= first),
        );
        self.pending.extend(
            self.model
                .slots
                .iter()
                .enumerate()
                .skip(first_slot)
                .filter(|(_, s)| s.dirty)
                .map(|(i, _)| i),
        );
        self.settle()?;
        self.initializing = previous;
        self.animations.pending.extend(
            self.model.nodes[first..end]
                .iter()
                .map(|n| n.id)
                .filter(|&id| self.model.standalone_animation(id)),
        );
        self.timer_changes
            .pending
            .extend(self.model.timers.objects().filter(|id| id.index >= first));
        self.settle()?;
        // Completion handlers run after construction. A new synchronous
        // incubator requested from onCompleted can therefore finish immediately.
        let was_incubating = std::mem::take(&mut self.incubating);
        let result = (first..end).rev().try_for_each(|index| {
            self.dispatch(self.model.nodes[index].id, "Component.completed", Vec::new())?;
            self.settle()
        });
        self.incubating = was_incubating;
        result
    }

    pub(crate) fn settle(&mut self) -> Result<(), Error> {
        loop {
            while let Some(key) = self.pending.pop_first() {
                self.evaluate(key)?;
            }
            if self.stage_state()? || self.stage_animation()? {
                continue;
            }
            if let Some(binding) = self.pending_bindings.pop_first() {
                self.update_binding(binding)?;
                continue;
            }
            if self.stage_repeater()? || self.stage_loader()? {
                continue;
            }
            if self.release_finished_values() {
                continue;
            }
            if !self.worker_sources.is_empty() {
                self.stage_workers()?;
                // Loading a new source resets ready. Process its dependent
                // bindings before validating or returning this transaction.
                continue;
            }
            if !self.initializing {
                if let Some(timer) = self.timer_changes.pending.pop_first() {
                    self.schedule_timer(timer)?;
                    continue;
                }
            }
            if self.hold_notifications {
                break;
            }
            let Some((object, name)) = self.notifications.pop_front() else {
                if !self.nested_retirements.is_empty() {
                    let retired = std::mem::take(&mut self.nested_retirements);
                    self.destroy_subtrees(&retired)?;
                    continue;
                }
                break;
            };
            self.budget()?;
            self.dispatch(object, &name, Vec::new())?;
        }
        Ok(())
    }

    pub(crate) fn read(
        &mut self,
        key: SlotId,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        self.evaluate(key)?;
        reads.insert(key);
        Ok(self.model.slots[key].value.clone())
    }

    fn evaluate(&mut self, key: SlotId) -> Result<(), Error> {
        if !self.model.slots[key].dirty {
            return Ok(());
        }
        self.with_stack(4, |tx| tx.evaluate_chain(key))
    }

    /// Only the first mandatory property read is scheduled ahead of its owner.
    /// Every expression and Rust callback still runs exactly once.
    fn evaluate_chain(&mut self, mut key: SlotId) -> Result<(), Error> {
        let mut chain = Vec::new();
        loop {
            self.budget()?;
            if !self.evaluating.insert(key) {
                return Err(Error::new(format!(
                    "binding cycle at '{}'",
                    self.model.properties[key].name
                )));
            }
            chain.push(key);
            let Some(next) = self
                .first_dependency(key)
                .filter(|next| self.model.slots[*next].dirty)
            else {
                break;
            };
            key = next;
        }
        for key in chain.into_iter().rev() {
            let previous = self.current_binding.replace(key);
            let result = self.with_scope(self.model.slots[key].scope, |tx| tx.evaluate_one(key));
            self.current_binding = previous;
            result?;
        }
        Ok(())
    }

    fn first_dependency(&self, key: SlotId) -> Option<SlotId> {
        if let Some(overlay) = &self.model.slots[key].overlay {
            return overlay.immediate_input();
        }
        if let Some(Alias::Slot(target)) = self.model.properties[key].alias {
            return Some(target);
        }
        let binding = match self.model.slots[key].binding.as_ref()? {
            InstalledBinding::PropertyRead(source) => return Some(*source),
            InstalledBinding::Expression(binding) => binding,
            InstalledBinding::Guarded { when, .. } => return Some(*when),
            InstalledBinding::Guest { .. } | InstalledBinding::StateExpression { .. } => {
                return None
            }
        };
        let Binding::Expr(expr) = binding.as_ref() else {
            return None;
        };
        let (name, scope) = match expr {
            ir::Expr::Native(index) => self
                .model
                .native_first_reads
                .get(*index)
                .copied()
                .flatten()?,
            other => other.first_read()?,
        };
        if matches!(name, "parent" | "this" | "NaN" | "Infinity") {
            return None;
        }
        // QML ids shadow a same-named property even on the binding owner.
        if self.model.ids.contains_key(name) {
            return None;
        }
        let owner = self.model.properties[key].owner;
        if let Ok(slot) = self.model.key_in(owner, name, self.model.slots[key].scope) {
            return Some(slot);
        }
        if self.model.nodes[owner.index]
            .type_scope
            .member(name)
            .is_some()
        {
            return None;
        }
        let root = match scope {
            Some(name) => *self.model.ids.get(name)?,
            None => self.model.root(),
        };
        self.model
            .key_in(root, name, self.model.slots[key].scope)
            .or_else(|_| {
                self.model
                    .key_in(self.model.root(), name, self.model.slots[key].scope)
            })
            .ok()
    }

    pub(crate) fn with_stack<T>(
        &mut self,
        cost: usize,
        operation: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        // Account for simultaneously active expression, method and binding
        // frames; opaque/native evaluators may still perform dynamic reads.
        if self.depth + cost > 80 {
            return Err(Error::new("QML evaluation stack limit exceeded"));
        }
        self.depth += cost;
        let result = operation(self);
        self.depth -= cost;
        result
    }

    fn evaluate_one(&mut self, key: SlotId) -> Result<(), Error> {
        let owner = self.model.properties[key].owner;
        let mut reads = BTreeSet::new();
        let input = match &self.model.slots[key].overlay {
            Some(overlay) => match overlay.frozen() {
                Some(value) => Some(value.clone()),
                None => Some(self.read(overlay.input(), &mut reads)?),
            },
            None => None,
        };
        let superseded = self.model.slots[key]
            .overlay
            .as_ref()
            .is_some_and(|o| o.superseded);
        let value = match (input, &self.model.properties[key].alias) {
            (Some(value), _) if !superseded => value,
            (_, Some(Alias::Slot(target))) => self.read(*target, &mut reads)?,
            (_, Some(Alias::Object(target))) => Value::Object(*target),
            _ => {
                let binding = self.model.slots[key]
                    .binding
                    .clone()
                    .ok_or_else(|| Error::new("missing dirty binding"))?;
                match binding {
                    InstalledBinding::PropertyRead(source) => self.read(source, &mut reads)?,
                    InstalledBinding::StateExpression {
                        receiver,
                        expression,
                        ..
                    } => {
                        let value = self.binding_value(receiver, &expression, &mut reads)?;
                        self.check_binding_edges(key, &reads)?;
                        value
                    }
                    InstalledBinding::Guest {
                        callback,
                        receiver,
                        scope,
                    } => {
                        let ty = self.model.properties[key].ty.clone();
                        let value = self.with_scope(scope, |tx| {
                            tx.with_external(receiver, &callback, &mut reads, |engine, ctx| {
                                if !engine.is_reference_live(&callback) {
                                    return Err(Error::new("JavaScript binding expired after worker reset. Assign a value or install a new binding"));
                                }
                                engine.evaluate_binding(ctx, &callback, receiver, &ty)
                            })
                        })?;
                        self.check_binding_edges(key, &reads)?;
                        value
                    }
                    InstalledBinding::Guarded { when, expression } => {
                        if self.read(when, &mut reads)? == Value::Bool(true) {
                            let value = self.binding_value(owner, &expression, &mut reads)?;
                            self.check_binding_edges(key, &reads)?;
                            value
                        } else {
                            // Retain the previous input until the overlay is detached.
                            // Only the guard remains subscribed while disabled.
                            self.model.slots[key].value.clone()
                        }
                    }
                    InstalledBinding::Expression(binding) => {
                        self.binding_value(owner, &binding, &mut reads)?
                    }
                }
            }
        };
        if self.model.slots[key].overlay.is_some() {
            self.check_binding_edges(key, &reads)?;
        }
        let value = self.model.typed(&self.model.properties[key].ty, value)?;
        self.replace_reads(key, reads);
        self.touched(key).dirty = false;
        self.evaluating.remove(&key);
        self.set_value(key, value);
        Ok(())
    }

    fn binding_value(
        &mut self,
        owner: ObjectId,
        binding: &Binding,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        match binding {
            Binding::Expr(expr) => self.expr(owner, expr, &mut Locals::new(), reads),
            Binding::Script(body) => self
                .statements(owner, body, &mut Locals::new(), reads)?
                .value(),
            _ => Err(Error::new("unresolved object-valued binding")),
        }
    }

    // A cached alias or derived property can read cleanly during installation
    // even though it already depends on the target. Reject that edge before it
    // becomes a cycle, including when the computed value would stay unchanged.
    fn check_binding_edges(&mut self, key: SlotId, reads: &BTreeSet<SlotId>) -> Result<(), Error> {
        let slot = &self.model.slots[key];
        if slot.dependents.is_empty() || reads.is_subset(&slot.reads) {
            return Ok(());
        }
        let mut stack: Vec<_> = slot.dependents.iter().copied().collect();
        let mut seen = BTreeSet::new();
        while let Some(dep) = stack.pop() {
            if seen.insert(dep) {
                self.budget()?;
                if reads.contains(&dep) {
                    return Err(Error::new(format!(
                        "binding cycle at '{}'",
                        self.model.properties[key].name
                    )));
                }
                stack.extend(&self.model.slots[dep].dependents);
            }
        }
        Ok(())
    }

    pub(crate) fn replace_reads(&mut self, key: SlotId, reads: BTreeSet<SlotId>) {
        let old = std::mem::replace(&mut self.touched(key).reads, reads.clone());
        for dep in old.difference(&reads) {
            self.touched(*dep).dependents.remove(&key);
        }
        for dep in reads.difference(&old) {
            self.touched(*dep).dependents.insert(key);
        }
    }

    pub(crate) fn set_value(&mut self, key: SlotId, value: Value) {
        if !self.intercept_animation(key, &value) {
            self.set_value_raw(key, value);
        }
    }

    pub(crate) fn set_value_raw(&mut self, key: SlotId, value: Value) {
        if self.model.slots[key].value.unchanged(&value) {
            return;
        }
        let property = &self.model.properties[key];
        if !self.model.dynamic.loaders.is_empty() {
            if let Some(name) = loader::observed_property(&property.name) {
                self.loader_write(property.owner, name, &value);
            }
        }
        self.touched(key).value = value;
        self.observe_states(key);
        self.observe_animation(key);
        if let Some(target) = self.model.slots[key].active_target {
            if self.model.slots[target]
                .overlay
                .as_ref()
                .is_some_and(|o| o.is_delayed(key))
            {
                self.deferred
                    .push(deferred::Task::Binding(self.model.properties[key].owner));
            } else if self.model.slots[target]
                .overlay
                .as_ref()
                .is_some_and(|o| o.superseded && o.input() == key)
            {
                let slot = self.touched(target);
                slot.overlay.as_mut().expect("active overlay").superseded = false;
                slot.binding = None;
            }
        }
        // Eagerly marking the transitive closure lets code in the same handler
        // read a derived value before the outer transaction is settled.
        let mut stack: Vec<_> = self.model.slots[key].dependents.iter().copied().collect();
        while let Some(dep) = stack.pop() {
            if !self.model.slots[dep].dirty && !self.evaluating.contains(&dep) {
                let slot = self.touched(dep);
                slot.dirty = true;
                stack.extend(slot.dependents.iter().copied());
                self.pending.insert(dep);
            }
        }
        let property = &self.model.properties[key];
        if property.name == "source" && self.model.workers.contains(property.owner) {
            self.worker_sources.insert(property.owner);
        }
        if self
            .model
            .dynamic
            .repeaters
            .get(&property.owner)
            .is_some_and(|v| v.is_list_view())
        {
            if property.name == "model" {
                self.repeater_changes.list_models.insert(property.owner);
            }
            if matches!(
                property.name.as_str(),
                "width"
                    | "height"
                    | "contentX"
                    | "contentY"
                    | "cacheBuffer"
                    | "currentIndex"
                    | "spacing"
                    | "orientation"
            ) {
                self.repeater_changes.pending.insert(property.owner);
            }
        }
        if matches!(
            property.name.as_str(),
            "width" | "height" | "implicitWidth" | "implicitHeight"
        ) {
            if let Some(parent) = self.model.parent(property.owner) {
                if self
                    .model
                    .dynamic
                    .repeaters
                    .get(&parent)
                    .is_some_and(|state| state.is_list_view())
                    && !self.repeater_changes.busy.contains(&parent)
                {
                    self.repeater_changes.pending.insert(parent);
                }
            }
        }
        if matches!(property.name.as_str(), "model" | "delegate")
            && self.model.dynamic.repeaters.contains_key(&property.owner)
        {
            self.repeater_changes.pending.insert(property.owner);
        }
        if !self.model.dynamic.repeaters.is_empty()
            && self.model.nodes[property.owner.index]
                .visual_children
                .as_ref()
                == Some(&property.name)
        {
            let candidates = self.model.children(property.owner).iter().filter(|id| {
                self.model.dynamic.repeaters.contains_key(id)
                    && !self.repeater_changes.busy.contains(id)
            });
            self.repeater_changes.pending.extend(candidates);
        }
        if property.name == "dynamicRoles" && self.model.models.contains_key(&property.owner) {
            self.model_modes.insert(property.owner);
        }
        if self.model.bindings.contains_key(&property.owner)
            && matches!(
                property.name.as_str(),
                "when" | "target" | "property" | "restoreMode" | "delayed"
            )
        {
            self.pending_bindings.insert(property.owner);
        }
        if !self.initializing {
            if self.model.timers.is_timer(property.owner)
                && matches!(
                    property.name.as_str(),
                    "running" | "interval" | "repeat" | "triggeredOnStart"
                )
            {
                self.timer_changes.pending.insert(property.owner);
            }
            let signal = format!("{}Changed", property.name);
            if self.model.nodes[property.owner.index]
                .handlers
                .contains_key(&signal)
                || self.model.listeners.contains_key(&signal)
                || self
                    .model
                    .javascript
                    .connections
                    .has(property.owner, &signal)
                || self
                    .model
                    .watched
                    .contains(&(property.owner, signal.clone()))
            {
                self.notifications.push_back((property.owner, signal));
            }
        }
    }

    pub(crate) fn write(&mut self, key: SlotId, value: Value) -> Result<(), Error> {
        if self.in_binding() {
            return Err(Error::new(
                "a binding cannot imperatively modify the object graph",
            ));
        }
        if self.write_delegate_input(key, &value)? {
            return Ok(());
        }
        let key = self.model.write_target(key)?;
        let fallback = self.loader_dimension_assignment(key, value == Value::Undefined);
        let reset = fallback.is_some();
        let value = self
            .model
            .typed(&self.model.properties[key].ty, fallback.unwrap_or(value))?;
        let inputs = self.model.slots[key]
            .overlay
            .as_ref()
            .and_then(|o| o.immediate_input())
            .into_iter()
            .collect();
        self.replace_reads(key, inputs);
        let slot = self.touched(key);
        slot.assigned = !reset;
        slot.binding = None;
        if let Some(overlay) = &mut slot.overlay {
            overlay.superseded = false;
        }
        slot.dirty = false;
        self.pending.remove(&key);
        self.set_value(key, value);
        Ok(())
    }

    pub(crate) fn install_guest_binding(
        &mut self,
        key: SlotId,
        callback: ExternalValue,
    ) -> Result<(), Error> {
        if self.in_binding() {
            return Err(Error::new(
                "a binding cannot imperatively modify the object graph",
            ));
        }
        if callback.instance() != self.model.script_instance() {
            return Err(Error::new("foreign JavaScript binding"));
        }
        // The alias owner supplies `this`, even when the physical slot belongs to another object.
        let receiver = self.model.properties[key].owner;
        let target = self.model.write_target(key)?;
        let scope = self.declaration_scope();
        let slot = self.touched(target);
        slot.binding = Some(InstalledBinding::Guest {
            callback,
            receiver,
            scope,
        });
        slot.assigned = true;
        slot.dirty = true;
        if let Some(overlay) = &mut slot.overlay {
            overlay.superseded = true;
        }
        self.pending.insert(target);
        self.settle()
    }

    pub(crate) fn in_binding(&self) -> bool {
        !self.evaluating.is_empty()
    }

    pub(crate) fn signal(
        &mut self,
        owner: ObjectId,
        name: &str,
        arguments: Vec<Value>,
    ) -> Result<(), Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot emit signals"));
        }
        let node = self
            .model
            .node(owner)
            .ok_or_else(|| Error::new("foreign or unknown object"))?;
        let formals = node
            .signal(name)
            .ok_or_else(|| Error::new(format!("unknown signal '{name}'")))?;
        let arguments = self.arguments(formals, arguments)?;
        self.dispatch(owner, name, arguments)
    }

    pub(crate) fn arguments(
        &self,
        parameters: &[Parameter],
        arguments: Vec<Value>,
    ) -> Result<Vec<Value>, Error> {
        if parameters.len() != arguments.len() {
            return Err(Error::new(format!(
                "expected {} arguments, got {}",
                parameters.len(),
                arguments.len()
            )));
        }
        parameters
            .iter()
            .zip(arguments)
            .map(|(p, v)| match &p.ty {
                Some(ty) => self.model.typed(ty, v),
                None => self.model.typed("var", v),
            })
            .collect()
    }

    pub(crate) fn dispatch(
        &mut self,
        owner: ObjectId,
        signal: &str,
        arguments: Vec<Value>,
    ) -> Result<(), Error> {
        let boundary = self.model.javascript.connections.boundary();
        if !self.model.watched.is_empty()
            && self.model.watched.contains(&(owner, signal.to_owned()))
        {
            self.emitted.push(SignalEmission {
                object: owner,
                signal: signal.to_owned(),
                arguments: arguments.clone(),
            });
        }
        if let Some(handler) = self.model.nodes[owner.index].handlers.get(signal).cloned() {
            self.run(owner, &handler, arguments.clone(), &mut BTreeSet::new())?;
        }
        self.dispatch_connections(owner, signal, &arguments)?;
        self.dispatch_script_callbacks(owner, signal, &arguments, boundary)
    }
}
