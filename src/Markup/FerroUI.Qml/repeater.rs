//! Eager Qt Quick delegates share the enclosing transaction. ListModel row IDs
//! survive reordering, independently of the delegate's current index.
use super::*;
use crate::transaction::Transaction;

const MAX_ROWS: usize = 65_536;
const REVISION: &str = "$qml:repeatedItems";
const CONTEXT: &str = "$qml:delegate:";

impl TypeDefinition {
    /// Register the eager model/delegate item on a host that supplies QtQuick.Item.
    pub fn quick_repeater() -> Self {
        let index = Parameter {
            name: "index".into(),
            ty: Some("int".into()),
        };
        let item = Parameter {
            name: "item".into(),
            ty: Some("QtQuick.Item".into()),
        };
        let parameters = vec![index.clone(), item];
        Self::default()
            .inherits("QtQuick", "Item")
            .property("model", "var", Value::Undefined)
            .property("delegate", "Component", Value::Null)
            .default_property("delegate")
            .readonly_property("count", "int", Value::Number(0.))
            .readonly_property(REVISION, "real", Value::Number(0.))
            .signal("itemAdded", parameters.clone())
            .signal("itemRemoved", parameters)
            .context_method("itemAt", vec![index], "QtQuick.Item", |ctx, args| {
                let [Value::Number(index)] = args else {
                    unreachable!("typed index")
                };
                ctx.repeated_item(*index as i32)
            })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Position(usize),
    Row(RowId),
}

#[derive(Clone, Default)]
pub(crate) struct State {
    rows: BTreeMap<RowId, (usize, ObjectId)>,
    factory: Option<ObjectId>,
    model: Option<Value>,
    entries: Vec<(Key, Option<ObjectId>)>,
}

impl State {
    pub(crate) fn items(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.entries.iter().filter_map(|(_, id)| *id)
    }
}

#[derive(Default)]
pub(crate) struct Changes {
    pub(crate) pending: BTreeSet<ObjectId>,
    pub(crate) busy: BTreeSet<ObjectId>,
    before: BTreeMap<ObjectId, State>,
}

impl Changes {
    pub(crate) fn rollback(self, model: &mut Instance) {
        model.dynamic.repeaters.extend(self.before);
    }
}

impl Instance {
    fn delegate_inputs_in_scope(&self, mut owner: ObjectId) -> Option<&BTreeMap<String, SlotId>> {
        loop {
            let root = self.node(owner)?.context?;
            let context = self.dynamic.contexts.get(&root)?;
            if let Some(inputs) = &context.delegate_inputs {
                return Some(inputs);
            }
            // A component instantiated inside a delegate captures the scope
            // of its declaration, regardless of its visual or ownership parent.
            owner = context.declaration;
        }
    }

    pub(crate) fn delegate_context_key(&self, owner: ObjectId, name: &str) -> Option<SlotId> {
        self.delegate_inputs_in_scope(owner)?.get(name).copied()
    }

    pub(crate) fn delegate_context_names(&self, owner: ObjectId, names: &mut BTreeSet<String>) {
        if let Some(inputs) = self.delegate_inputs_in_scope(owner) {
            names.extend(inputs.keys().cloned());
        }
    }

    fn repeater_values(&self, value: &Value) -> Result<Vec<(Key, Value)>, Error> {
        let indexed = |values: Vec<Value>| {
            values
                .into_iter()
                .enumerate()
                .map(|(index, v)| (Key::Position(index), v))
                .collect()
        };
        match value {
            Value::Number(n) if n.is_finite() && *n >= 0. && *n <= i32::MAX as f64 => {
                // Do not allocate an unbounded vector from an untrusted numeric model.
                if *n > MAX_ROWS as f64 {
                    return Err(Error::new("Repeater model exceeds the row limit"));
                }
                Ok((0..*n as usize)
                    .map(|index| (Key::Position(index), Value::Number(index as f64)))
                    .collect())
            }
            Value::List(list) => Ok(indexed(list.clone())),
            Value::Record(array) if array.is_array() => Ok(indexed(self.array_values(array)?)),
            Value::Object(id) if self.models.contains_key(id) => Ok(self.models[id]
                .row_ids(*id)
                .map(|row| (Key::Row(row), Value::ModelRow(row)))
                .collect()),
            _ => Ok(vec![]),
        }
    }

    fn delegate_inputs(
        &self,
        index: usize,
        value: &Value,
    ) -> Result<BTreeMap<String, Value>, Error> {
        let mut inputs = BTreeMap::new();
        if let Value::ModelRow(row) = value {
            let roles = self.model_keys(*row)?;
            for name in &roles {
                inputs.insert(name.clone(), self.model_role(*row, name)?);
            }
            let model_data = if roles.len() == 1 {
                self.model_role(*row, &roles[0])?
            } else {
                Value::Undefined
            };
            inputs.insert("modelData".into(), model_data);
            inputs.insert("model".into(), value.clone());
        } else {
            inputs.insert("modelData".into(), value.clone());
        }
        inputs.insert("index".into(), Value::Number(index as f64));
        Ok(inputs)
    }
}

impl Transaction<'_> {
    pub(crate) fn repeated_item(
        &mut self,
        id: ObjectId,
        index: i32,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        self.read(self.model.key(id, REVISION)?, reads)?;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| self.model.dynamic.repeaters[&id].entries.get(i))
            .and_then(|(_, item)| *item)
            .map_or(Value::Null, Value::Object))
    }

    pub(crate) fn initialize_delegate(
        &mut self,
        root: ObjectId,
        inputs: &BTreeMap<String, Value>,
        assigned: &mut BTreeSet<SlotId>,
    ) -> Result<(), Error> {
        let node = &self.model.nodes[root.index];
        if !(node.module == "QtQuick" && node.name == "Item"
            || node
                .lineage
                .iter()
                .any(|(module, name)| module == "QtQuick" && name == "Item"))
        {
            return Err(Error::new("Repeater delegates must inherit QtQuick.Item"));
        }
        let required = node
            .properties
            .values()
            .any(|key| self.model.properties[*key].required);
        if let Some(Value::ModelRow(row)) = inputs.get("model") {
            let slot = self.model.allocate_property(
                Property {
                    owner: root,
                    name: "$qml:delegateRevision".into(),
                    ty: "real".into(),
                    readonly: true,
                    required: false,
                    alias: None,
                },
                Value::Number(0.),
            )?;
            self.model
                .dynamic
                .contexts
                .get_mut(&root)
                .expect("delegate context")
                .row_revision = Some((*row, slot));
        }
        for (name, value) in inputs {
            let key = if required {
                let Some(&key) = self.model.nodes[root.index].properties.get(name) else {
                    continue;
                };
                if !self.model.properties[key].required {
                    continue;
                }
                key
            } else {
                self.model.allocate_property(
                    Property {
                        owner: root,
                        name: format!("{CONTEXT}{name}"),
                        ty: "var".into(),
                        readonly: true,
                        required: false,
                        alias: None,
                    },
                    Value::Undefined,
                )?
            };
            self.model.slots[key].value = self
                .model
                .typed(&self.model.properties[key].ty, value.clone())?;
            self.model
                .dynamic
                .contexts
                .get_mut(&root)
                .expect("delegate context")
                .delegate_inputs
                .as_mut()
                .expect("delegate inputs")
                .insert(name.clone(), key);
            assigned.insert(key);
        }
        Ok(())
    }

    fn update_delegate(
        &mut self,
        item: ObjectId,
        inputs: BTreeMap<String, Value>,
    ) -> Result<(), Error> {
        for (name, value) in inputs {
            if let Some(key) = self.model.delegate_context_key(item, &name) {
                let value = self.model.typed(&self.model.properties[key].ty, value)?;
                self.set_value(key, value);
            }
        }
        if let Some((_, slot)) = self.model.dynamic.contexts[&item].row_revision {
            let Value::Number(revision) = self.model.slots[slot].value else {
                unreachable!()
            };
            self.set_value(slot, Value::Number(revision + 1.));
        }
        Ok(())
    }

    /// Role edits preserve the topology. Only structural edits, schema changes,
    /// and writes made during construction need a complete reconciliation.
    pub(crate) fn model_repeaters_changed(
        &mut self,
        model: ObjectId,
        edited: Option<RowId>,
    ) -> Result<(), Error> {
        let views: Vec<_> = self
            .model
            .dynamic
            .repeaters
            .iter()
            .filter_map(|(&id, state)| (state.model == Some(Value::Object(model))).then_some(id))
            .collect();
        for id in views {
            if let Some(row) = edited {
                if !self.repeater_changes.busy.contains(&id)
                    && !self.repeater_changes.pending.contains(&id)
                {
                    if let Some(&(index, item)) = self.model.dynamic.repeaters[&id].rows.get(&row) {
                        let inputs = self.model.delegate_inputs(index, &Value::ModelRow(row))?;
                        self.update_delegate(item, inputs)?;
                    }
                    continue;
                }
            }
            self.repeater_changes.pending.insert(id);
        }
        Ok(())
    }

    pub(crate) fn stage_repeater(&mut self) -> Result<bool, Error> {
        let changes = &mut self.repeater_changes;
        let Some(id) = changes
            .pending
            .iter()
            .copied()
            .find(|id| !changes.busy.contains(id))
        else {
            return Ok(false);
        };
        changes.pending.remove(&id);
        if self.model.node(id).is_some() {
            changes.busy.insert(id);
            changes
                .before
                .entry(id)
                .or_insert_with(|| self.model.dynamic.repeaters[&id].clone());
            let result = self.with_stack(4, |tx| tx.refresh_repeater(id));
            self.repeater_changes.busy.remove(&id);
            result?;
        }
        Ok(true)
    }

    fn refresh_repeater(&mut self, id: ObjectId) -> Result<(), Error> {
        self.budget()?;
        let model = self.model.get(id, "model")?;
        let mut values = if let Value::External(array) = &model {
            let snapshot = self.with_external(id, array, &mut BTreeSet::new(), |engine, ctx| {
                engine.array_elements(ctx, array, MAX_ROWS)
            })?;
            snapshot
                .unwrap_or_default()
                .into_iter()
                .enumerate()
                .map(|(i, v)| (Key::Position(i), v))
                .collect()
        } else {
            self.model.repeater_values(&model)?
        };
        if values.len() > MAX_ROWS {
            return Err(Error::new("Repeater model exceeds the row limit"));
        }
        let count = values.len();
        let parent = self.model.parent(id);
        let siblings = parent.and_then(|p| self.model.node(p)?.visual_children.as_ref().and_then(|name| self.model.key(p,name).ok()))
            .filter(|&slot| matches!(&self.model.slots[slot].value, Value::List(list) if list.contains(&Value::Object(id))));
        let factory = match self.model.get(id, "delegate")? {
            Value::Object(factory)
                if siblings.is_some() && self.model.dynamic.templates.contains_key(&factory) =>
            {
                Some(factory)
            }
            _ => None,
        };
        if factory.is_none() {
            values.clear();
        }
        let state = &self.model.dynamic.repeaters[&id];
        let reset = state.factory != factory
            || !state
                .model
                .as_ref()
                .is_some_and(|old| old.unchanged(&model));
        let previous = state.entries.clone();
        let wanted: BTreeSet<_> = values.iter().map(|(key, _)| *key).collect();
        let mut retained = BTreeMap::new();
        let mut retired = Vec::new();
        for (index, &(key, item)) in previous.iter().enumerate().rev() {
            let Some(item) = item else { continue };
            if !reset && wanted.contains(&key) {
                retained.insert(key, item);
            } else {
                // While delivering itemRemoved the old position is still readable.
                self.signal(
                    id,
                    "itemRemoved",
                    vec![Value::Number(index as f64), Value::Object(item)],
                )?;
                self.model.dynamic.repeaters.get_mut(&id).unwrap().entries[index].1 = None;
                retired.push(item);
            }
        }
        self.destroy_subtrees(&retired)?;
        self.set_value(self.model.key(id, "count")?, Value::Number(count as f64));
        let state = self.model.dynamic.repeaters.get_mut(&id).unwrap();
        state.factory = factory;
        state.model = Some(model);
        state.entries = values
            .iter()
            .map(|(key, _)| (*key, retained.get(key).copied()))
            .collect();
        if let (Some(factory), Some(slot)) = (factory, siblings) {
            let template = self.model.dynamic.templates[&factory].clone();
            for (index, (key, value)) in values.into_iter().enumerate() {
                let inputs = self.model.delegate_inputs(index, &value)?;
                if let Some(&item) = retained.get(&key) {
                    self.update_delegate(item, inputs)?;
                } else {
                    let item = self.construct_object(
                        factory,
                        parent,
                        &template,
                        BTreeMap::new(),
                        Some(&inputs),
                        Default::default(),
                    )?;
                    self.model.dynamic.repeaters.get_mut(&id).unwrap().entries[index].1 =
                        Some(item);
                    self.signal(
                        id,
                        "itemAdded",
                        vec![Value::Number(index as f64), Value::Object(item)],
                    )?;
                }
            }
            self.place_delegates(id, slot);
        }
        if previous != self.model.dynamic.repeaters[&id].entries {
            let state = self.model.dynamic.repeaters.get_mut(&id).unwrap();
            state.rows = state
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| match entry {
                    (Key::Row(row), Some(item)) => Some((*row, (index, *item))),
                    _ => None,
                })
                .collect();
            let revision = self.model.key(id, REVISION)?;
            let Value::Number(n) = self.model.slots[revision].value else {
                unreachable!()
            };
            self.set_value(revision, Value::Number(n + 1.));
        }
        Ok(())
    }

    fn place_delegates(&mut self, id: ObjectId, slot: SlotId) {
        let delegates: Vec<_> = self.model.dynamic.repeaters[&id].items().collect();
        let managed: BTreeSet<_> = delegates.iter().copied().collect();
        let Value::List(mut siblings) = self.model.slots[slot].value.clone() else {
            unreachable!("visual list")
        };
        siblings.retain(|value| !matches!(value, Value::Object(child) if managed.contains(child)));
        if let Some(at) = siblings.iter().position(|v| *v == Value::Object(id)) {
            siblings.splice(at + 1..at + 1, delegates.into_iter().map(Value::Object));
        }
        self.set_value(slot, Value::List(siblings));
    }
}
