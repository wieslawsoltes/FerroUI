//! Qt Quick Loader uses the same dynamic object arena and transaction as its
//! owning component. The retained visual host consumes normal Item.data changes.
use super::*;
use crate::transaction::Transaction;

// A hot-path slot write only needs the borrowed name if the Loader contract observes it.
pub(crate) fn observed_property(name: &str) -> Option<&'static str> {
    match name {
        "source" => Some("source"),
        "sourceComponent" => Some("sourceComponent"),
        "active" => Some("active"),
        "asynchronous" => Some("asynchronous"),
        "status" => Some("status"),
        "width" => Some("width"),
        "height" => Some("height"),
        "implicitWidth" => Some("implicitWidth"),
        "implicitHeight" => Some("implicitHeight"),
        _ => None,
    }
}

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
    owned_factory: Option<(String, Option<String>, ObjectId)>,
    initial: Rc<BTreeMap<String, InitialProperty>>,
    source_origin: Option<String>,
    reload: bool,
}

#[derive(Clone, PartialEq, Eq)]
enum Selection {
    File(String),
    Component(Option<ObjectId>),
}

#[derive(Default)]
pub(crate) struct Changes {
    sizing: BTreeMap<(ObjectId, Axis), bool>,
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
        if let Some(axis) = Axis::from_property(name) {
            if self.model.dynamic.loaders.contains_key(&owner) && name == axis.dimension() {
                self.mark_axis(owner, axis, true);
            }
            if let Some(parent) = self.model.parent(owner) {
                if self.model.dynamic.loaders.contains_key(&parent)
                    && self.model.get(parent, "item").ok() == Some(Value::Object(owner))
                {
                    self.mark_axis(parent, axis, false);
                }
            }
        }
        if name == "status" && self.model.is_component(owner) {
            let waiting = self
                .model
                .dynamic
                .loaders
                .keys()
                .filter(|id| {
                    self.model.get(**id, "sourceComponent").ok() == Some(Value::Object(owner))
                })
                .copied();
            self.loader_changes.pending.extend(waiting);
        }
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

    pub(crate) fn set_loader_source(
        &mut self,
        loader: ObjectId,
        url: String,
        origin: Option<String>,
        properties: BTreeMap<String, InitialProperty>,
    ) -> Result<(), Error> {
        if self.in_binding() || !self.model.dynamic.loaders.contains_key(&loader) {
            return Err(Error::new("invalid Loader.setSource call"));
        }
        let key = self.model.key(loader, "source")?;
        self.write(key, Value::String(url.clone()))?;
        self.select_loader(loader, Selection::File(url));
        let state = self.loader_state(loader);
        state.initial = Rc::new(properties);
        state.source_origin = origin;
        Ok(())
    }

    pub(crate) fn loader_call(&mut self, loader: ObjectId, args: &[Value]) -> Result<Value, Error> {
        let [Value::String(url), properties @ ..] = args else {
            return Err(Error::new(
                "Loader.setSource expects a URL and optional initial properties",
            ));
        };
        let props = match properties {
            [] | [Value::Undefined | Value::Null] => BTreeMap::new(),
            [Value::Record(record)] if !record.is_array() => self
                .model
                .record_entries(record)?
                .iter()
                .map(|(k, v)| (k.clone(), InitialProperty::Value(v.clone())))
                .collect(),
            _ => return Err(Error::new("invalid Loader initial properties")),
        };
        self.set_loader_source(loader, url.clone(), None, props)?;
        self.settle()?;
        Ok(Value::Undefined)
    }

    fn select_loader(&mut self, id: ObjectId, selection: Selection) {
        let state = self.loader_state(id);
        state.selection = Some(selection);
        state.initial = Default::default();
        state.source_origin = None;
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
        let Some(id) = next else {
            return self.stage_loader_size();
        };
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
        let selection = selection.expect("selected Loader input");
        let state = &self.model.dynamic.loaders[&id];
        let origin = state.source_origin.clone();
        if let Some((old_url, old_origin, factory)) = state.owned_factory.clone() {
            if !matches!(&selection,Selection::File(url) if *url==old_url && old_origin==origin) {
                self.loader_state(id).owned_factory = None;
                self.publish_loader_property(id, "sourceComponent", Value::Null)?;
                if self.model.node(factory).is_some() {
                    self.destroy_object(factory)?;
                }
            }
        }
        let component = match selection {
            Selection::Component(component) => {
                self.publish_loader_property(id, "source", Value::String(String::new()))?;
                component
            }
            Selection::File(url) if url.is_empty() => {
                self.publish_loader_property(id, "sourceComponent", Value::Null)?;
                None
            }
            Selection::File(url) => {
                let cached = self.model.dynamic.loaders[&id]
                    .owned_factory
                    .as_ref()
                    .map(|entry| entry.2)
                    .filter(|&factory| self.model.node(factory).is_some());
                if let Some(factory) = cached {
                    Some(factory)
                } else if active {
                    let factory = self.url_factory(id, origin.as_deref(), &url, false, Some(id))?;
                    self.loader_state(id).owned_factory = Some((url, origin, factory));
                    self.publish_loader_property(id, "sourceComponent", Value::Object(factory))?;
                    Some(factory)
                } else {
                    None
                }
            }
        };
        if !active {
            return self.publish_status(id, 0, None);
        }
        let Some(component) = component else {
            return self.publish_status(id, 0, Some(0.));
        };
        match self.model.get(component, "status")? {
            Value::Number(1.) => {}
            Value::Number(2.) => return self.publish_status(id, 2, Some(0.)),
            _ => return self.publish_status(id, 3, Some(1.)),
        }
        if self.model.get(id, "asynchronous")? == Value::Bool(true) && !finish {
            self.publish_status(id, 2, Some(0.))?;
            self.deferred.push(deferred::Task::Loader(id));
            return Ok(());
        }
        let Some(item) = self.try_create_with_properties(
            component,
            Some(id),
            (*self.model.dynamic.loaders[&id].initial).clone(),
        )?
        else {
            return self.publish_status(id, 3, Some(1.));
        };
        self.publish_loader_property(id, "item", Value::Object(item))?;
        self.size_loader(id, Axis::Width, true)?;
        self.size_loader(id, Axis::Height, true)?;
        self.publish_status(id, 1, Some(1.))?;
        self.signal(id, "loaded", vec![])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Axis {
    Width,
    Height,
}
impl Axis {
    fn from_property(name: &str) -> Option<Self> {
        match name {
            "width" | "implicitWidth" => Some(Self::Width),
            "height" | "implicitHeight" => Some(Self::Height),
            _ => None,
        }
    }
    fn dimension(self) -> &'static str {
        match self {
            Self::Width => "width",
            Self::Height => "height",
        }
    }
    fn implicit(self) -> &'static str {
        match self {
            Self::Width => "implicitWidth",
            Self::Height => "implicitHeight",
        }
    }
}

impl Transaction<'_> {
    fn mark_axis(&mut self, loader: ObjectId, axis: Axis, enforce: bool) {
        let marked = self
            .loader_changes
            .sizing
            .entry((loader, axis))
            .or_default();
        *marked |= enforce;
    }

    pub(crate) fn loader_dimension_assignment(
        &mut self,
        key: SlotId,
        reset: bool,
    ) -> Option<Value> {
        let property = &self.model.properties[key];
        let owner = property.owner;
        let axis = Axis::from_property(&property.name)?;
        if axis.dimension() != property.name || !self.model.dynamic.loaders.contains_key(&owner) {
            return None;
        }
        self.mark_axis(owner, axis, true);
        if reset {
            self.model.get(owner, axis.implicit()).ok()
        } else {
            None
        }
    }

    fn stage_loader_size(&mut self) -> Result<bool, Error> {
        let Some(((id, axis), enforce)) = self.loader_changes.sizing.pop_first() else {
            return Ok(false);
        };
        if self.model.node(id).is_some() {
            self.size_loader(id, axis, enforce)?;
        }
        Ok(true)
    }

    fn size_loader(&mut self, id: ObjectId, axis: Axis, enforce: bool) -> Result<(), Error> {
        let Value::Object(item) = self.model.get(id, "item")? else {
            return Ok(());
        };
        if self
            .model
            .node(item)
            .and_then(|node| node.visual_children.as_ref())
            .is_none()
        {
            return Ok(());
        }
        let declared = self.model.is_assigned(id, axis.dimension());
        let intrinsic = match self.model.get(item, axis.implicit()) {
            Ok(Value::Number(v)) if v.is_finite() => v,
            _ => 0.,
        };
        if declared {
            if enforce {
                let value = self.model.get(id, axis.dimension())?;
                self.publish_loader_property(item, axis.dimension(), value)?;
            }
            self.publish_loader_property(id, axis.implicit(), Value::Number(intrinsic))?;
        } else {
            let natural = match self.model.get(item, axis.dimension())? {
                Value::Number(v) if v.is_finite() => v,
                _ => intrinsic,
            };
            self.publish_loader_property(id, axis.dimension(), Value::Number(natural))?;
            self.publish_loader_property(id, axis.implicit(), Value::Number(natural))?;
        }
        Ok(())
    }
}
