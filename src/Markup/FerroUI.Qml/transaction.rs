use super::*;
use ferroui_qml_compiler::Binding;
use std::collections::VecDeque;

/// The undo log only touches slots affected by this write, including dependency edges.
/// A failed handler or binding therefore cannot leave a half-applied native update.
pub(crate) struct Transaction<'a> {
    pub(crate) model: &'a mut Instance,
    original: BTreeMap<SlotId, Slot>,
    pending: BTreeSet<SlotId>,
    evaluating: BTreeSet<SlotId>,
    notifications: VecDeque<(ObjectId, String)>,
    initializing: bool,
    pub(crate) depth: usize,
    steps: usize,
}

impl Instance {
    pub(crate) fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut Transaction<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut tx = Transaction {
            model: self,
            original: BTreeMap::new(),
            pending: BTreeSet::new(),
            evaluating: BTreeSet::new(),
            notifications: VecDeque::new(),
            initializing: false,
            depth: 0,
            steps: 0,
        };
        let result = operation(&mut tx).and_then(|value| {
            tx.settle()?;
            Ok(value)
        });
        if result.is_err() {
            for (key, old) in tx.original {
                tx.model.slots[key] = old;
            }
        } else {
            for (key, old) in tx.original {
                let value = &tx.model.slots[key].value;
                if !old.value.unchanged(value) {
                    tx.model.changes.insert(key, value.clone());
                }
            }
        }
        result
    }
}

impl Transaction<'_> {
    fn touched(&mut self, key: SlotId) -> &mut Slot {
        self.original
            .entry(key)
            .or_insert_with(|| self.model.slots[key].clone());
        &mut self.model.slots[key]
    }

    fn budget(&mut self) -> Result<(), Error> {
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
        self.initializing = true;
        self.pending.extend(
            self.model
                .slots
                .iter()
                .enumerate()
                .filter(|(_, s)| s.dirty)
                .map(|(i, _)| i),
        );
        self.settle()?;
        self.initializing = false;
        for index in (0..self.model.nodes.len()).rev() {
            let object = ObjectId {
                instance: self.model.identity,
                index,
            };
            self.dispatch(object, "Component.completed", Vec::new())?;
            self.settle()?;
        }
        Ok(())
    }

    fn settle(&mut self) -> Result<(), Error> {
        loop {
            while let Some(key) = self.pending.pop_first() {
                self.evaluate(key)?;
            }
            let Some((object, name)) = self.notifications.pop_front() else {
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
        self.budget()?;
        if self.evaluating.len() >= 256 {
            return Err(Error::new(
                "binding dependency chain exceeds the nesting limit",
            ));
        }
        if !self.evaluating.insert(key) {
            return Err(Error::new(format!(
                "binding cycle at '{}'",
                self.model.properties[key].name
            )));
        }
        let owner = self.model.properties[key].owner;
        let mut reads = BTreeSet::new();
        let value = match self.model.properties[key].alias {
            Some(Alias::Slot(target)) => self.read(target, &mut reads)?,
            Some(Alias::Object(target)) => Value::Object(target),
            None => {
                let binding = self.model.slots[key]
                    .binding
                    .clone()
                    .ok_or_else(|| Error::new("missing dirty binding"))?;
                match binding.as_ref() {
                    Binding::Expr(expr) => {
                        self.expr(owner, expr, &mut Locals::new(), &mut reads)?
                    }
                    Binding::Script(body) => self
                        .statements(owner, body, &mut Locals::new(), &mut reads)?
                        .value(),
                    _ => return Err(Error::new("unresolved object-valued binding")),
                }
            }
        };
        let value = self.model.typed(&self.model.properties[key].ty, value)?;
        self.replace_reads(key, reads);
        self.touched(key).dirty = false;
        self.evaluating.remove(&key);
        self.set_value(key, value);
        Ok(())
    }

    fn replace_reads(&mut self, key: SlotId, reads: BTreeSet<SlotId>) {
        let old = std::mem::replace(&mut self.touched(key).reads, reads.clone());
        for dep in old.difference(&reads) {
            self.touched(*dep).dependents.remove(&key);
        }
        for dep in reads.difference(&old) {
            self.touched(*dep).dependents.insert(key);
        }
    }

    fn set_value(&mut self, key: SlotId, value: Value) {
        if self.model.slots[key].value.unchanged(&value) {
            return;
        }
        self.touched(key).value = value;
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
        if !self.initializing {
            let property = &self.model.properties[key];
            let signal = format!("{}Changed", property.name);
            if self.model.nodes[property.owner.index]
                .handlers
                .contains_key(&signal)
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
        let key = self.model.write_target(key)?;
        let value = self.model.typed(&self.model.properties[key].ty, value)?;
        self.replace_reads(key, BTreeSet::new());
        let slot = self.touched(key);
        slot.binding = None;
        slot.dirty = false;
        self.pending.remove(&key);
        self.set_value(key, value);
        Ok(())
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
            .signals
            .get(name)
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
                None => Ok(v),
            })
            .collect()
    }

    fn dispatch(
        &mut self,
        owner: ObjectId,
        signal: &str,
        arguments: Vec<Value>,
    ) -> Result<(), Error> {
        if let Some(handler) = self.model.nodes[owner.index].handlers.get(signal).cloned() {
            self.run(owner, &handler, arguments, &mut BTreeSet::new())?;
        }
        Ok(())
    }
}
