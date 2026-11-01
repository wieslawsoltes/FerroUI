//! Flat ListModel storage. Row identity is independent of display order.
//! Every edit records an inverse operation in the enclosing QML transaction.
use crate::transaction::Transaction;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

pub(crate) const MODULE: &str = "QtQml.Models";
const REVISION: &str = "$qml:rows";
type Roles = BTreeMap<String, Value>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RowId {
    model: ObjectId,
    serial: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Kind {
    Bool,
    Number,
    String,
    Object,
}
impl Kind {
    fn of(value: &Value) -> Result<Option<Self>, Error> {
        Ok(match value {
            Value::Bool(_) => Some(Self::Bool),
            Value::Number(_) => Some(Self::Number),
            Value::String(_) => Some(Self::String),
            Value::Object(_) => Some(Self::Object),
            Value::Null | Value::Undefined => None,
            _ => {
                return Err(Error::new(
                    "this ListModel supports scalar and object-reference roles",
                ))
            }
        })
    }
    fn default_value(self) -> Value {
        match self {
            Self::Bool => Value::Bool(false),
            Self::Number => Value::Number(0.),
            Self::String => Value::String(String::new()),
            Self::Object => Value::Null,
        }
    }
}

#[derive(Default)]
pub(crate) struct Model {
    worker_token: std::cell::OnceCell<u64>,
    order: Vec<u64>,
    rows: BTreeMap<u64, Roles>,
    roles: BTreeMap<String, Kind>,
    next: u64,
    dynamic: Option<bool>,
}

impl Model {
    fn read_field(&self, roles: &Roles, name: &str) -> Value {
        roles.get(name).cloned().unwrap_or_else(|| {
            if self.dynamic == Some(false) {
                self.roles
                    .get(name)
                    .map(|k| k.default_value())
                    .unwrap_or(Value::Undefined)
            } else {
                Value::Undefined
            }
        })
    }
    fn prepare(&self, fields: &Roles, dynamic: bool) -> Result<Vec<(String, Kind)>, Error> {
        if self.dynamic.is_some_and(|old| old != dynamic) {
            return Err(Error::new("set dynamicRoles before adding data"));
        }
        let mut added = Vec::new();
        for (name, value) in fields {
            if name.is_empty() || !name.starts_with(char::is_lowercase) {
                return Err(Error::new(format!("invalid model role '{name}'")));
            }
            if let Some(kind) = Kind::of(value)? {
                match self.roles.get(name) {
                    Some(old) if !dynamic && *old != kind => {
                        return Err(Error::new(format!(
                            "incompatible type for model role '{name}'"
                        )))
                    }
                    None => added.push((name.clone(), kind)),
                    _ => (),
                }
            }
        }
        Ok(added)
    }
    fn insert(&mut self, at: usize, roles: Roles) -> u64 {
        // A removed handle must never identify a subsequently inserted row.
        let id = self.next;
        self.next += 1;
        self.order.insert(at, id);
        self.rows.insert(id, roles);
        id
    }
    fn move_range(&mut self, from: usize, to: usize, count: usize) {
        if from < to {
            self.order[from..to + count].rotate_left(count);
        } else if from > to {
            self.order[to..from + count].rotate_right(count);
        }
    }
    pub(crate) fn static_row(&mut self, object: &ir::Object) -> Result<(), Error> {
        let mut fields = Roles::new();
        for member in &object.members {
            let ir::MemberKind::Binding {
                name,
                value: ir::Binding::Expr(expr),
            } = &member.kind
            else {
                return Err(Error::new("ListElement expects constant role assignments"));
            };
            let literal = expr.as_literal().ok_or_else(|| {
                Error::new(format!(
                    "ListElement role '{name}' requires a scalar constant"
                ))
            })?;
            fields.insert(name.clone(), Value::from(literal));
        }
        self.roles.extend(self.prepare(&fields, false)?);
        self.dynamic = Some(false);
        self.insert(self.order.len(), fields);
        Ok(())
    }
    pub(crate) fn row_ids(&self, model: ObjectId) -> impl Iterator<Item = RowId> + '_ {
        self.order
            .iter()
            .map(move |&serial| RowId { model, serial })
    }
    pub(crate) fn len(&self) -> usize {
        self.order.len()
    }
}

pub(crate) enum Undo {
    Schema(Option<bool>, Vec<String>),
    Insert(usize, u64),
    Remove(usize, Vec<u64>, BTreeMap<u64, Roles>),
    Move {
        from: usize,
        to: usize,
        count: usize,
    },
    Update(u64, Vec<(String, Option<Value>)>),
}

impl Undo {
    pub(crate) fn restore(self, model: &mut Model) {
        match self {
            Self::Schema(old, added) => {
                model.dynamic = old;
                for name in added {
                    model.roles.remove(&name);
                }
            }
            Self::Insert(at, id) => {
                model.order.remove(at);
                model.rows.remove(&id);
            }
            Self::Remove(at, order, rows) => {
                model.order.splice(at..at, order);
                model.rows.extend(rows);
            }
            Self::Move { from, to, count } => model.move_range(to, from, count),
            Self::Update(id, changes) => {
                let fields = model
                    .rows
                    .get_mut(&id)
                    .expect("undo order restores row before editing it");
                for (name, value) in changes {
                    if let Some(value) = value {
                        fields.insert(name, value);
                    } else {
                        fields.remove(&name);
                    }
                }
            }
        }
    }
}

pub(crate) fn register(types: &mut TypeRegistry) -> Result<(), Error> {
    types.register(
        MODULE,
        "ListModel",
        TypeDefinition::default()
            .inherits("QtQml", "QtObject")
            .readonly_property("count", "int", Value::Number(0.))
            .property("dynamicRoles", "bool", Value::Bool(false))
            .readonly_property(REVISION, "real", Value::Number(0.)),
    )?;
    types.register(MODULE, "ListElement", TypeDefinition::default())?;
    types.set_module_version(MODULE, 2, 15);
    types.set_module_version(MODULE, 6, 12);
    types.reexport("QtQml", MODULE)
}

pub(crate) fn is_method(name: &str) -> bool {
    matches!(
        name,
        "append" | "clear" | "get" | "insert" | "move" | "remove" | "set" | "setProperty" | "sync"
    )
}

impl Instance {
    pub(crate) fn worker_model_snapshot(
        &self,
        id: ObjectId,
    ) -> Result<(u64, u64, Vec<Roles>), Error> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let model = self
            .models
            .get(&id)
            .ok_or_else(|| Error::new("invalid ListModel"))?;
        let token = *model
            .worker_token
            .get_or_init(|| NEXT.fetch_add(1, Ordering::Relaxed));
        let rows = model
            .order
            .iter()
            .map(|&serial| {
                let row = RowId { model: id, serial };
                self.model_keys(row)?
                    .into_iter()
                    .map(|name| Ok((name.clone(), self.model_role(row, &name)?)))
                    .collect::<Result<Roles, Error>>()
            })
            .collect::<Result<_, _>>()?;
        Ok((token, self.worker_model_revision(id)?, rows))
    }
    pub(crate) fn worker_model_revision(&self, id: ObjectId) -> Result<u64, Error> {
        let Value::Number(revision) = self.slots[self.key(id, REVISION)?].value else {
            return Err(Error::new("invalid ListModel revision"));
        };
        Ok(revision as u64)
    }
    pub(crate) fn model_keys(&self, row: RowId) -> Result<Vec<String>, Error> {
        let model = self
            .models
            .get(&row.model)
            .ok_or_else(|| Error::new("foreign model handle"))?;
        let fields = model
            .rows
            .get(&row.serial)
            .ok_or_else(|| Error::new("model row was removed"))?;
        Ok(if model.dynamic == Some(false) {
            model
                .roles
                .keys()
                .chain(fields.keys())
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        } else {
            fields.keys().cloned().collect()
        })
    }

    /// Read a role through a row handle obtained by calling ListModel.get().
    pub fn model_role(&self, row: RowId, name: &str) -> Result<Value, Error> {
        let model = self
            .models
            .get(&row.model)
            .ok_or_else(|| Error::new("foreign model handle"))?;
        let roles = model
            .rows
            .get(&row.serial)
            .ok_or_else(|| Error::new("model row was removed"))?;
        Ok(model.read_field(roles, name))
    }
    pub(crate) fn validate_model_modes(&self, changed: &BTreeSet<ObjectId>) -> Result<(), Error> {
        for id in changed {
            if let Some(mode) = self.models[id].dynamic {
                if self.get(*id, "dynamicRoles")? != Value::Bool(mode) {
                    return Err(Error::new("set dynamicRoles before adding data"));
                }
            }
        }
        Ok(())
    }
}

impl Transaction<'_> {
    pub(crate) fn read_role(
        &mut self,
        row: RowId,
        name: &str,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        // A delegate bound to its own row subscribes to its local revision.
        // Other consumers retain the model-wide invalidation contract.
        let delegate = self
            .current_binding
            .and_then(|slot| self.model.node(self.model.properties[slot].owner)?.context)
            .and_then(|root| self.model.dynamic.contexts.get(&root)?.row_revision);
        if let Some((own_row, revision)) = delegate {
            if own_row == row {
                self.read(revision, reads)?;
                if !self.model.models[&row.model].rows.contains_key(&row.serial) {
                    // Count or other dependencies can still invalidate an outgoing
                    // delegate before its removal signal. Read its last row values.
                    if let Some(fields) =
                        self.model_edits
                            .iter()
                            .rev()
                            .find_map(|(id, undo)| match undo {
                                Undo::Remove(_, _, removed) if *id == row.model => {
                                    removed.get(&row.serial)
                                }
                                _ => None,
                            })
                    {
                        return Ok(self.model.models[&row.model].read_field(fields, name));
                    }
                }
                return self.model.model_role(row, name);
            }
        }
        self.read(self.model.key(row.model, REVISION)?, reads)?;
        self.model.model_role(row, name)
    }
    fn role_schema(&mut self, id: ObjectId, fields: &Roles, dynamic: bool) -> Result<(), Error> {
        for value in fields.values() {
            self.model.validate_handles(value)?;
        }
        let model = self.model.models.get_mut(&id).expect("registered model");
        let added = model.prepare(fields, dynamic)?;
        if !added.is_empty() || model.dynamic.is_none() {
            self.model_edits.push((
                id,
                Undo::Schema(
                    model.dynamic,
                    added.iter().map(|(n, _)| n.clone()).collect(),
                ),
            ));
            model.dynamic = Some(dynamic);
            model.roles.extend(added);
        }
        Ok(())
    }
    fn finish_model_edit(&mut self, id: ObjectId, row: Option<RowId>) -> Result<(), Error> {
        let count = self.model.models[&id].len();
        let size = self.model.key(id, "count")?;
        let revision = self.model.key(id, REVISION)?;
        let Value::Number(previous) = self.model.slots[revision].value else {
            return Err(Error::new("invalid ListModel revision"));
        };
        self.set_value(size, Value::Number(count as f64));
        self.set_value(revision, Value::Number(previous + 1.));
        self.model_repeaters_changed(id, row)?;
        self.settle()
    }
    fn model_dict(&self, value: &Value) -> Result<Roles, Error> {
        match value {
            Value::Record(record) if !record.is_array() => {
                Ok(self.model.record_entries(record)?.clone())
            }
            Value::ModelRow(row) => self
                .model
                .models
                .get(&row.model)
                .and_then(|m| m.rows.get(&row.serial))
                .cloned()
                .ok_or_else(|| Error::new("invalid model row")),
            _ => Err(Error::new("ListModel expects an object of role values")),
        }
    }
    fn insert_row(
        &mut self,
        id: ObjectId,
        at: usize,
        fields: Roles,
        dynamic: bool,
    ) -> Result<(), Error> {
        self.role_schema(id, &fields, dynamic)?;
        let serial = self.model.models.get_mut(&id).unwrap().insert(at, fields);
        self.model_edits.push((id, Undo::Insert(at, serial)));
        Ok(())
    }
    fn update_row(
        &mut self,
        row: RowId,
        fields: Roles,
        dynamic: bool,
    ) -> Result<Option<RowId>, Error> {
        if !self
            .model
            .models
            .get(&row.model)
            .is_some_and(|m| m.rows.contains_key(&row.serial))
        {
            return Err(Error::new("invalid model row"));
        }
        let roles = self.model.models[&row.model].roles.len();
        self.role_schema(row.model, &fields, dynamic)?;
        let previous = self
            .model
            .models
            .get_mut(&row.model)
            .unwrap()
            .rows
            .get_mut(&row.serial)
            .unwrap();
        let changes = fields
            .into_iter()
            .filter_map(|(name, value)| {
                if previous.get(&name).is_some_and(|old| old.unchanged(&value)) {
                    return None;
                }
                let old = previous.insert(name.clone(), value);
                Some((name, old))
            })
            .collect();
        self.model_edits
            .push((row.model, Undo::Update(row.serial, changes)));
        Ok((roles == self.model.models[&row.model].roles.len()).then_some(row))
    }
    pub(crate) fn write_role(&mut self, row: RowId, name: &str, value: Value) -> Result<(), Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot modify a ListModel"));
        }
        let dynamic = self.model.get(row.model, "dynamicRoles")? == Value::Bool(true);
        let edited = self.update_row(row, BTreeMap::from([(name.to_owned(), value)]), dynamic)?;
        self.finish_model_edit(row.model, edited)
    }
    fn remove_rows(&mut self, id: ObjectId, range: Range<usize>) {
        let model = self.model.models.get_mut(&id).unwrap();
        let at = range.start;
        let order: Vec<_> = model.order.drain(range).collect();
        let removed = order
            .iter()
            .map(|row| (*row, model.rows.remove(row).unwrap()))
            .collect();
        self.model_edits
            .push((id, Undo::Remove(at, order, removed)));
    }
    pub(crate) fn model_call(
        &mut self,
        id: ObjectId,
        method: &str,
        args: &[Value],
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        let model = &self.model.models[&id];
        let size = model.len();
        if let ("get", [index]) = (method, args) {
            let found = index_arg(index).and_then(|i| model.order.get(i).copied());
            self.read(self.model.key(id, REVISION)?, reads)?;
            return Ok(found.map_or(Value::Undefined, |serial| {
                Value::ModelRow(RowId { model: id, serial })
            }));
        }
        if self.in_binding() {
            return Err(Error::new("a binding cannot modify a ListModel"));
        }
        let dynamic = self.model.get(id, "dynamicRoles")? == Value::Bool(true);
        match (method, args) {
            ("append", [value]) | ("insert", [_, value]) => {
                let at = if method == "append" {
                    size
                } else {
                    position(&args[0], size, true)?
                };
                let batch = match value {
                    Value::Record(array) if array.is_array() => {
                        Some(self.model.array_values(array)?)
                    }
                    Value::List(rows) => Some(rows.clone()),
                    _ => None,
                };
                if let Some(rows) = batch {
                    if rows.is_empty() {
                        return Ok(Value::Undefined);
                    }
                    // Append contiguously then rotate once. Repeated insertions
                    // at the front would shift the pre-existing rows N times.
                    for (i, row) in rows.iter().enumerate() {
                        self.insert_row(id, size + i, self.model_dict(row)?, dynamic)?;
                    }
                    if at != size {
                        self.model
                            .models
                            .get_mut(&id)
                            .expect("registered model")
                            .move_range(size, at, rows.len());
                        self.model_edits.push((
                            id,
                            Undo::Move {
                                from: size,
                                to: at,
                                count: rows.len(),
                            },
                        ));
                    }
                } else {
                    self.insert_row(id, at, self.model_dict(value)?, dynamic)?;
                }
            }
            ("set", [index, value]) => {
                let at = position(index, size, true)?;
                let fields = self.model_dict(value)?;
                if at == size {
                    self.insert_row(id, at, fields, dynamic)?;
                } else {
                    let row = RowId {
                        model: id,
                        serial: self.model.models[&id].order[at],
                    };
                    let edited = self.update_row(row, fields, dynamic)?;
                    self.finish_model_edit(id, edited)?;
                    return Ok(Value::Undefined);
                }
            }
            ("setProperty", [index, Value::String(name), value]) => {
                let serial = self.model.models[&id].order[position(index, size, false)?];
                self.write_role(RowId { model: id, serial }, name, value.clone())?;
                return Ok(Value::Undefined);
            }
            ("clear", []) => self.remove_rows(id, 0..size),
            ("remove", [index]) | ("remove", [index, _]) => {
                let start = position(index, size, false)?;
                let count = args
                    .get(1)
                    .map_or(Some(1), index_arg)
                    .ok_or_else(|| Error::new("invalid remove count"))?;
                if count > size - start {
                    return Err(Error::new("invalid remove range"));
                }
                self.remove_rows(id, start..start + count);
            }
            ("move", [from, to, count]) => {
                let count = index_arg(count)
                    .filter(|n| *n <= size)
                    .ok_or_else(|| Error::new("invalid move count"))?;
                let from = position(from, size - count, true)?;
                let to = position(to, size - count, true)?;
                self.model
                    .models
                    .get_mut(&id)
                    .unwrap()
                    .move_range(from, to, count);
                self.model_edits.push((id, Undo::Move { from, to, count }));
            }
            ("sync", []) => return Err(Error::new("ListModel.sync requires WorkerScript")),
            _ => {
                return Err(Error::new(format!(
                    "invalid arguments to ListModel.{method}"
                )))
            }
        }
        self.finish_model_edit(id, None)?;
        Ok(Value::Undefined)
    }
}

fn index_arg(value: &Value) -> Option<usize> {
    if let Value::Number(n) = value {
        if n.is_finite() && *n >= 0. && n.fract() == 0. && *n <= i32::MAX as f64 {
            return Some(*n as usize);
        }
    }
    None
}
fn position(value: &Value, size: usize, end: bool) -> Result<usize, Error> {
    index_arg(value)
        .filter(|n| *n < size || (end && *n == size))
        .ok_or_else(|| Error::new("ListModel index out of range"))
}
