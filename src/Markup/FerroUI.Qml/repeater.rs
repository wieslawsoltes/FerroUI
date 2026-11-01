//! Eager Qt Quick delegates share the enclosing transaction. ListModel row IDs
//! survive reordering, independently of the delegate's current index.
use super::*;
use crate::transaction::Transaction;

#[path = "list_view.rs"]
mod list_view;

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
            .property("delegateModelAccess", "int", Value::Number(0.))
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
    list: Option<Box<list_view::State>>,
    roles: Option<Rc<[String]>>,
    rows: BTreeMap<RowId, (usize, ObjectId)>,
    factory: Option<ObjectId>,
    model: Option<Value>,
    entries: Vec<(Key, Option<ObjectId>)>,
}

impl State {
    pub(crate) fn list_view() -> Self {
        Self {
            list: Some(Default::default()),
            ..Default::default()
        }
    }

    pub(crate) fn is_list_view(&self) -> bool {
        self.list.is_some()
    }

    pub(crate) fn section_role_for(&self, model: ObjectId) -> Option<&str> {
        if self.model != Some(Value::Object(model)) {
            return None;
        }
        self.list.as_ref()?.section_role(self.roles.as_deref())
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.entries
            .iter()
            .filter_map(|(_, id)| *id)
            .chain(self.list.iter().flat_map(|state| state.items()))
    }
}

#[derive(Default)]
pub(crate) struct Changes {
    pub(crate) reveal_header: BTreeSet<ObjectId>,
    pub(crate) section_changes: BTreeSet<ObjectId>,
    rebound: BTreeMap<ObjectId, RowId>,
    deferred_role_edits: BTreeSet<ObjectId>,
    pub(crate) list_models: BTreeSet<ObjectId>,
    pub(crate) list_structure: BTreeSet<ObjectId>,
    pub(crate) pending: BTreeSet<ObjectId>,
    pub(crate) busy: BTreeSet<ObjectId>,
    before: BTreeMap<ObjectId, State>,
}

impl Changes {
    pub(crate) fn rollback(self, model: &mut Instance) {
        model.dynamic.repeaters.extend(self.before);
        for (id, source) in self.rebound {
            if let Some(row) = model
                .dynamic
                .contexts
                .get_mut(&id)
                .and_then(|ctx| ctx.row_revision.as_mut())
            {
                row.source = source;
            }
        }
    }
}

pub(crate) struct Inputs {
    attached: Option<BTreeMap<String, Value>>,
    view: ObjectId,
    values: BTreeMap<String, Value>,
    roles: Option<Rc<[String]>>,
}

pub(crate) struct ProjectedRow {
    view: ObjectId,
    pub(crate) source: RowId,
    pub(crate) revision: SlotId,
    index: SlotId,
    required: bool,
    roles: Rc<[String]>,
}

impl ProjectedRow {
    pub(crate) fn keys(&self) -> Vec<String> {
        self.roles
            .iter()
            .cloned()
            .chain(
                [
                    "index",
                    "row",
                    "column",
                    "model",
                    "modelData",
                    "hasModelChildren",
                ]
                .map(str::to_owned),
            )
            .collect()
    }

    fn has_role(&self, name: &str) -> bool {
        self.roles
            .binary_search_by(|role| role.as_str().cmp(name))
            .is_ok()
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

    pub(crate) fn delegate_row(&self, owner: ObjectId) -> Result<&ProjectedRow, Error> {
        self.node(owner)
            .and_then(|_| self.dynamic.contexts.get(&owner)?.row_revision.as_ref())
            .ok_or_else(|| Error::new("delegate row is no longer live"))
    }

    fn delegate_inputs(
        &self,
        repeater: ObjectId,
        index: usize,
        value: &Value,
        owner: ObjectId,
    ) -> Result<Inputs, Error> {
        let mut values = BTreeMap::new();
        let roles = self.dynamic.repeaters[&repeater].roles.clone();
        if let Value::ModelRow(row) = value {
            let names = roles.as_deref().expect("view role schema");
            for name in names {
                values.insert(name.clone(), self.model_role(*row, name)?);
            }
            let data = match names {
                [role] => self.model_role(*row, role)?,
                _ => Value::DelegateRow(owner),
            };
            values.insert("modelData".into(), data);
            values.insert("model".into(), Value::ModelRow(*row));
        } else {
            values.insert("modelData".into(), value.clone());
        }
        values.insert("index".into(), Value::Number(index as f64));
        Ok(Inputs {
            attached: None,
            view: repeater,
            values,
            roles,
        })
    }
}

impl Transaction<'_> {
    pub(crate) fn read_delegate_role(
        &mut self,
        owner: ObjectId,
        name: &str,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        let projected = self.model.delegate_row(owner)?;
        let source = projected.source;
        let revision = projected.revision;
        match name {
            "index" | "row" => return self.read(projected.index, reads),
            "column" => return Ok(Value::Number(0.)),
            "model" => return Ok(Value::DelegateRow(owner)),
            "hasModelChildren" => return Ok(Value::Bool(false)),
            "modelData" => match projected.roles.as_ref() {
                [role] => {
                    let role = role.clone();
                    self.read(revision, reads)?;
                    return self.projected_field(source, &role);
                }
                _ if projected.required => return Ok(Value::DelegateRow(owner)),
                _ => return Ok(Value::Undefined),
            },
            _ if !projected.has_role(name) => return Ok(Value::Undefined),
            _ => {}
        }
        self.read(revision, reads)?;
        self.projected_field(source, name)
    }

    pub(crate) fn write_delegate_role(
        &mut self,
        owner: ObjectId,
        name: &str,
        value: Value,
    ) -> Result<(), Error> {
        let projected = self.model.delegate_row(owner)?;
        if self.model.get(projected.view, "delegateModelAccess")? == Value::Number(1.) {
            return Err(Error::new("the delegate model is read-only"));
        }
        if !projected.has_role(name) {
            return Err(Error::new(format!(
                "unknown or readonly delegate role '{name}'"
            )));
        }
        self.write_role(projected.source, name, value)
    }

    pub(crate) fn write_delegate_input(
        &mut self,
        key: SlotId,
        value: &Value,
    ) -> Result<bool, Error> {
        let property = &self.model.properties[key];
        let Some(context) = self.model.dynamic.contexts.get(&property.owner) else {
            return Ok(false);
        };
        let Some(projected) = &context.row_revision else {
            return Ok(false);
        };
        let name = property
            .name
            .strip_prefix(CONTEXT)
            .unwrap_or(&property.name);
        if context
            .delegate_inputs
            .as_ref()
            .and_then(|map| map.get(name))
            != Some(&key)
        {
            return Ok(false);
        }
        let role = if name == "modelData" && projected.roles.len() == 1 {
            &projected.roles[0]
        } else if projected.has_role(name) {
            name
        } else {
            return Ok(false);
        };
        let access = self.model.get(projected.view, "delegateModelAccess")?;
        if projected.required && access != Value::Number(2.) {
            // Assigning a required property breaks its model binding locally.
            self.touched(key).delegate_local = true;
            return Ok(false);
        }
        if access == Value::Number(1.) {
            return Err(Error::new("the delegate model is read-only"));
        }
        let row = projected.source;
        let role = role.to_owned();
        self.write_role(row, &role, value.clone())?;
        Ok(true)
    }

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
        inputs: &Inputs,
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
        let required = self.model.dynamic.contexts[&root].bound
            || node
                .properties
                .values()
                .any(|key| self.model.properties[*key].required);
        if let Some(fields) = &inputs.attached {
            let mut scope = (*self.model.nodes[root.index].type_scope).clone();
            scope.attach_list_view(root, fields.keys());
            self.model.nodes[root.index].type_scope = Rc::new(scope);
            for (name, value) in fields {
                self.model.allocate_property(
                    Property {
                        owner: root,
                        name: format!("ListView.{name}"),
                        ty: "var".into(),
                        readonly: true,
                        required: false,
                        alias: None,
                    },
                    value.clone(),
                )?;
            }
        }
        if let Some(Value::ModelRow(row)) = inputs.values.get("model") {
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
            let index = self.model.allocate_property(
                Property {
                    owner: root,
                    name: "$qml:delegateIndex".into(),
                    ty: "int".into(),
                    readonly: true,
                    required: false,
                    alias: None,
                },
                inputs.values["index"].clone(),
            )?;
            self.model
                .dynamic
                .contexts
                .get_mut(&root)
                .expect("delegate context")
                .row_revision = Some(ProjectedRow {
                view: inputs.view,
                source: *row,
                revision: slot,
                index,
                required,
                roles: inputs.roles.clone().expect("view role schema"),
            });
        }
        for (name, original) in &inputs.values {
            let value = projected_input(root, required, name, original);
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
            self.model.slots[key].value =
                self.model.typed(&self.model.properties[key].ty, value)?;
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

    fn recycle_delegate(&mut self, item: ObjectId, inputs: Inputs) -> Result<(), Error> {
        if let Some(Value::ModelRow(next)) = inputs.values.get("model") {
            let row = self
                .model
                .dynamic
                .contexts
                .get_mut(&item)
                .unwrap()
                .row_revision
                .as_mut()
                .unwrap();
            self.repeater_changes
                .rebound
                .entry(item)
                .or_insert(row.source);
            row.source = *next;
        }
        for name in inputs.values.keys() {
            if let Some(key) = self.model.delegate_context_key(item, name) {
                if self.model.slots[key].delegate_local {
                    self.touched(key).delegate_local = false;
                }
            }
        }
        self.update_delegate(item, inputs)
    }

    fn update_delegate(&mut self, item: ObjectId, inputs: Inputs) -> Result<(), Error> {
        let row = self.model.dynamic.contexts[&item].row_revision.as_ref();
        let required = row.is_some_and(|row| row.required);
        if let Some(row) = row {
            self.set_value(row.index, inputs.values["index"].clone());
        }
        for (name, original) in inputs.values {
            if let Some(key) = self.model.delegate_context_key(item, &name) {
                if self.model.slots[key].delegate_local {
                    continue;
                }
                let value = projected_input(item, required, &name, &original);
                let value = self.model.typed(&self.model.properties[key].ty, value)?;
                self.set_value(key, value);
            }
        }
        if let Some(slot) = self.model.dynamic.contexts[&item]
            .row_revision
            .as_ref()
            .map(|p| p.revision)
        {
            let Value::Number(revision) = self.model.slots[slot].value else {
                unreachable!()
            };
            self.set_value(slot, Value::Number(revision + 1.));
        }
        Ok(())
    }

    /// Role edits preserve the topology and the view’s captured roles. Structural
    /// edits and writes made during construction need a full reconciliation.
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
                        let inputs =
                            self.model
                                .delegate_inputs(id, index, &Value::ModelRow(row), item)?;
                        self.update_delegate(item, inputs)?;
                    }
                    if self.repeater_changes.section_changes.contains(&id) {
                        self.repeater_changes.pending.insert(id);
                    }
                    continue;
                }
            }
            if edited.is_some() && !self.model.dynamic.repeaters[&id].is_list_view() {
                self.repeater_changes.deferred_role_edits.insert(id);
            }
            self.repeater_changes.pending.insert(id);
            self.repeater_changes.list_structure.insert(id);
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
            let result = self.with_stack(4, |tx| {
                if tx.model.dynamic.repeaters[&id].is_list_view() {
                    tx.refresh_list_view(id)
                } else {
                    tx.refresh_repeater(id)
                }
            });
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
        // A ListModel role edit updates its realized delegate directly, except
        // when the view is already being reconciled. Only those deferred edits
        // need another projection at an otherwise unchanged row and position.
        let deferred_edit = self.repeater_changes.deferred_role_edits.remove(&id);
        let unchanged_inputs =
            !deferred_edit && matches!(&model, Value::Object(o) if self.model.is_list_model(*o));
        let state = &self.model.dynamic.repeaters[&id];
        let model_changed = !state
            .model
            .as_ref()
            .is_some_and(|old| old.unchanged(&model));
        let reset = model_changed || state.factory != factory;
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
        if model_changed {
            state.roles = None;
        }
        if state.roles.is_none() && !values.is_empty() {
            if let Value::Object(model) = model {
                state.roles = self.model.models.get(&model).map(|m| m.view_roles());
            }
        }
        state.model = Some(model);
        state.entries = values
            .iter()
            .map(|(key, _)| (*key, retained.get(key).copied()))
            .collect();
        if let (Some(factory), Some(slot)) = (factory, siblings) {
            let template = self.model.dynamic.templates[&factory].clone();
            for (index, (key, value)) in values.into_iter().enumerate() {
                if unchanged_inputs
                    && matches!(previous.get(index), Some((old_key, Some(_))) if *old_key == key)
                    && !reset
                {
                    continue;
                }
                let owner = retained
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| self.model.next_object());
                let inputs = self.model.delegate_inputs(id, index, &value, owner)?;
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

fn projected_input(owner: ObjectId, required: bool, name: &str, value: &Value) -> Value {
    match (name, value) {
        ("model", Value::ModelRow(_)) => Value::DelegateRow(owner),
        ("modelData", Value::DelegateRow(_)) if !required => Value::Undefined,
        _ => value.clone(),
    }
}
