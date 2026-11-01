//! Qt Quick Loader uses the same dynamic object arena and transaction as its
//! owning component. The retained visual host consumes normal Item.data changes.
use super::*;
use crate::transaction::Transaction;

impl TypeDefinition {
    /// Native registration for QtQuick.Loader. The host supplies QtQuick.Item.
    pub fn quick_loader() -> Self {
        Self::default()
            .inherits("QtQuick", "Item")
            .property("active", "bool", Value::Bool(true))
            .property("asynchronous", "bool", Value::Bool(false))
            .property("source", "url", Value::String(String::new()))
            .property("sourceComponent", "Component", Value::Null)
            .readonly_property("item", "QtObject", Value::Null)
            .readonly_property("status", "int", Value::Number(0.))
            .readonly_property("progress", "real", Value::Number(0.))
            .property("width", "real", Value::Number(0.))
            .property("height", "real", Value::Number(0.))
            .readonly_property("implicitWidth", "real", Value::Number(0.))
            .readonly_property("implicitHeight", "real", Value::Number(0.))
            .signal("loaded", vec![])
            .enumeration(
                "Status",
                &[("Null", 0), ("Ready", 1), ("Loading", 2), ("Error", 3)],
            )
            .expect("Loader status enumeration")
    }
}

#[derive(Clone, Default)]
pub(crate) struct State {
    selection: Option<Selection>,
    reload: bool,
}

#[derive(Clone, PartialEq, Eq)]
enum Selection {
    File(String),
    Component(Option<ObjectId>),
}

#[derive(Default)]
pub(crate) struct Changes {
    before: BTreeMap<ObjectId, State>,
    pending: BTreeSet<ObjectId>,
    busy: BTreeSet<ObjectId>,
    projecting: bool,
}

impl Changes {
    pub(crate) fn rollback(self, model: &mut Instance) {
        for (id, state) in self.before {
            model.dynamic.loaders.insert(id, state);
        }
    }
}

impl Transaction<'_> {
    fn loader_state(&mut self, id: ObjectId) -> &mut State {
        let state = self
            .model
            .dynamic
            .loaders
            .get_mut(&id)
            .expect("registered Loader");
        self.loader_changes
            .before
            .entry(id)
            .or_insert_with(|| state.clone());
        state
    }

    pub(crate) fn initialize_loaders(&mut self, first: usize) {
        self.loader_changes.pending.extend(
            self.model
                .dynamic
                .loaders
                .keys()
                .filter(|id| id.index >= first),
        );
    }

    pub(crate) fn loader_write(&mut self, owner: ObjectId, name: &str, value: &Value) {
        if self.loader_changes.projecting {
            return;
        }
        if self.model.dynamic.loaders.contains_key(&owner) {
            match (name, value) {
                ("source", Value::String(url)) => {
                    self.select_loader(owner, Selection::File(url.clone()))
                }
                ("sourceComponent", Value::Object(component)) => {
                    self.select_loader(owner, Selection::Component(Some(*component)))
                }
                ("sourceComponent", Value::Null | Value::Undefined) => {
                    self.select_loader(owner, Selection::Component(None))
                }
                ("active" | "asynchronous", _) => {
                    self.loader_changes.pending.insert(owner);
                }
                _ => {}
            }
        }
    }

    fn select_loader(&mut self, id: ObjectId, selection: Selection) {
        let state = self.loader_state(id);
        state.selection = Some(selection);
        state.reload = true;
        self.loader_changes.pending.insert(id);
    }

    fn publish_loader_property(
        &mut self,
        id: ObjectId,
        name: &str,
        value: Value,
    ) -> Result<(), Error> {
        let key = self.model.key(id, name)?;
        let old = std::mem::replace(&mut self.loader_changes.projecting, true);
        self.set_value(key, value);
        self.loader_changes.projecting = old;
        Ok(())
    }

    fn publish_status(
        &mut self,
        id: ObjectId,
        status: u8,
        progress: Option<f64>,
    ) -> Result<(), Error> {
        if let Some(progress) = progress {
            self.publish_loader_property(id, "progress", Value::Number(progress))?;
        }
        self.publish_loader_property(id, "status", Value::Number(f64::from(status)))
    }

    pub(crate) fn stage_loader(&mut self) -> Result<bool, Error> {
        let next = self
            .loader_changes
            .pending
            .iter()
            .copied()
            .find(|id| !self.loader_changes.busy.contains(id));
        let Some(id) = next else { return Ok(false) };
        self.loader_changes.pending.remove(&id);
        if self.model.node(id).is_some() {
            self.loader_changes.busy.insert(id);
            let result = self.with_stack(4, |tx| tx.refresh_loader(id, false));
            self.loader_changes.busy.remove(&id);
            result?;
        }
        Ok(true)
    }

    pub(crate) fn finish_loader(&mut self, id: ObjectId) -> Result<(), Error> {
        if self.model.node(id).is_none() || self.model.get(id, "status")? != Value::Number(2.) {
            return Ok(());
        }
        self.loader_changes.busy.insert(id);
        let result = self.with_stack(4, |tx| tx.refresh_loader(id, true));
        self.loader_changes.busy.remove(&id);
        result
    }

    fn refresh_loader(&mut self, id: ObjectId, finish: bool) -> Result<(), Error> {
        self.budget()?;
        let active = self.model.get(id, "active")? == Value::Bool(true);
        let mut selection = self.model.dynamic.loaders[&id].selection.clone();
        if selection.is_none() {
            selection = Some(match self.model.get(id, "sourceComponent")? {
                Value::Object(component) => Selection::Component(Some(component)),
                _ => match self.model.get(id, "source")? {
                    Value::String(source) => Selection::File(source),
                    _ => Selection::Component(None),
                },
            });
            self.loader_state(id).selection = selection.clone();
        }
        let reload = std::mem::take(&mut self.loader_state(id).reload);
        if let Value::Object(item) = self.model.get(id, "item")? {
            if !reload && active {
                return Ok(());
            }
            self.publish_loader_property(id, "item", Value::Null)?;
            self.destroy_object(item)?;
        }
        let component = match selection.expect("selected Loader input") {
            Selection::Component(component) => {
                self.publish_loader_property(id, "source", Value::String(String::new()))?;
                component
            }
            Selection::File(source) => {
                self.publish_loader_property(id, "sourceComponent", Value::Null)?;
                if !source.is_empty() && active {
                    return self.publish_status(id, 3, Some(0.));
                }
                None
            }
        };
        if !active {
            return self.publish_status(id, 0, None);
        }
        let Some(component) = component else {
            return self.publish_status(id, 0, Some(0.));
        };
        if !self.model.is_component(component) {
            return self.publish_status(id, 3, Some(0.));
        }
        if self.model.get(id, "asynchronous")? == Value::Bool(true) && !finish {
            self.publish_status(id, 2, Some(0.))?;
            self.deferred.push(deferred::Task::Loader(id));
            return Ok(());
        }
        let item = self.create_object(component, Some(id), BTreeMap::new())?;
        self.publish_loader_property(id, "item", Value::Object(item))?;
        self.publish_status(id, 1, Some(1.))?;
        self.signal(id, "loaded", vec![])
    }
}
