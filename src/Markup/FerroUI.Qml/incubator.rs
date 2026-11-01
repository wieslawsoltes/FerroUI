//! A Component incubator is a JavaScript handle, independent of its result's
//! QObject parent. Its fields live in the transactional record heap. The queue
//! roots a pending handle even if its creator drops the last external reference.
use super::*;
use crate::transaction::Transaction;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IncubationMode {
    #[default]
    Asynchronous,
    Synchronous,
}

impl IncubationMode {
    fn from_qml(value: Option<&Value>) -> Self {
        if matches!(value, Some(Value::Number(1.))) {
            Self::Synchronous
        } else {
            Self::Asynchronous
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncubationStatus {
    Loading,
    Ready,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incubator(Record);

impl Incubator {
    pub fn from_record(handle: Record) -> Option<Self> {
        handle.is_incubator().then_some(Self(handle))
    }

    pub fn record(&self) -> &Record {
        &self.0
    }

    pub fn status(&self, instance: &Instance) -> Result<IncubationStatus, Error> {
        match instance.record_entries(&self.0)?.get("status") {
            Some(Value::Number(1.)) => Ok(IncubationStatus::Ready),
            Some(Value::Number(2.)) => Ok(IncubationStatus::Loading),
            Some(Value::Number(3.)) => Ok(IncubationStatus::Error),
            _ => Err(Error::new("invalid incubator state")),
        }
    }

    pub fn object(&self, instance: &Instance) -> Result<Option<ObjectId>, Error> {
        Ok(match instance.record_entries(&self.0)?.get("object") {
            Some(Value::Object(id)) if instance.node(*id).is_some() => Some(*id),
            _ => None,
        })
    }

    pub fn force_completion(&self, instance: &mut Instance) -> Result<(), Error> {
        instance.finish_incubation(&self.0, Default::default(), false)
    }

    /// End an unfinished request with Error and notify its registered callback.
    pub fn cancel(&self, instance: &mut Instance) -> Result<(), Error> {
        instance.finish_incubation(&self.0, Default::default(), true)
    }
}

pub(crate) struct Request {
    component: ObjectId,
    parent: Option<ObjectId>,
    initial: BTreeMap<String, InitialProperty>,
}

impl Instance {
    pub fn incubate_object(
        &mut self,
        component: ObjectId,
        parent: Option<ObjectId>,
        initial: BTreeMap<String, InitialProperty>,
        mode: IncubationMode,
    ) -> Result<Option<Incubator>, Error> {
        self.transaction(|tx| tx.start_incubation(component, parent, initial, mode))
    }

    pub(crate) fn advance_incubator(
        &mut self,
        handle: Record,
        previous: Option<Box<incubation::Preparation>>,
        has_time: &mut impl FnMut() -> bool,
    ) -> Result<(), Error> {
        let Some(request) = self.records.request(&handle) else {
            return Ok(());
        };
        let template = self.dynamic.templates.get(&request.component);
        let prepared = if let Some(template) = template {
            let mut preparation = previous
                .unwrap_or_else(|| Box::new(incubation::Preparation::new(template.clone())));
            match preparation.advance(
                self.dynamic.registry.as_ref().expect("type registry"),
                has_time,
            ) {
                Ok(false) => {
                    self.javascript
                        .deferred
                        .post(vec![deferred::DeferredWork::Incubator(
                            handle,
                            Some(preparation),
                        )]);
                    return Ok(());
                }
                Ok(true) => Some(preparation.into_nodes()),
                Err(_) => None,
            }
        } else {
            None
        };
        let cancelled = prepared.is_none();
        self.finish_incubation(&handle, prepared.unwrap_or_default(), cancelled)
    }

    fn finish_incubation(
        &mut self,
        handle: &Record,
        prepared: incubation::PreparedNodes,
        cancelled: bool,
    ) -> Result<(), Error> {
        // Publish the terminal result before invoking guest code. An exception
        // in the callback rolls back its own writes; the finished object stands.
        let activated =
            self.transaction(|tx| tx.complete_incubator(handle, prepared, cancelled, true));
        let changed = match activated {
            Ok(changed) => changed,
            Err(error) => {
                if self.transaction(|tx| {
                    tx.complete_incubator(handle, Default::default(), true, true)
                })? {
                    if let Err(reporting) = self.transaction(|tx| tx.notify_incubator(handle)) {
                        return Err(Error::new(format!(
                            "{error} (reporting incubator failure: {reporting})"
                        )));
                    }
                }
                return Err(error);
            }
        };
        if changed {
            self.transaction(|tx| tx.notify_incubator(handle))?;
        }
        Ok(())
    }
}

impl Transaction<'_> {
    pub(crate) fn start_incubation(
        &mut self,
        component: ObjectId,
        parent: Option<ObjectId>,
        initial: BTreeMap<String, InitialProperty>,
        mode: IncubationMode,
    ) -> Result<Option<Incubator>, Error> {
        if self.in_binding() || parent.is_some_and(|p| self.model.node(p).is_none()) {
            return Err(Error::new("invalid incubator parent or creation context"));
        }
        if !self.model.is_component(component)
            || self.model.get(component, "status")? != Value::Number(1.)
        {
            return Ok(None);
        }
        let handle = self.model.records.new_incubator(
            self.model.identity,
            Request {
                component,
                parent,
                initial,
            },
        );
        if mode == IncubationMode::Synchronous && !self.incubating {
            self.complete_incubator(&handle, Default::default(), false, false)?;
        } else {
            self.deferred
                .push(deferred::DeferredWork::Incubator(handle.clone(), None));
        }
        Ok(Some(Incubator(handle)))
    }

    pub(crate) fn incubate_method(
        &mut self,
        component: ObjectId,
        args: &[Value],
    ) -> Result<Value, Error> {
        if args.len() > 3 {
            return Err(Error::new(
                "incubateObject expects a parent, initial properties and an incubation mode",
            ));
        }
        let parent = match args.first() {
            None | Some(Value::Null | Value::Undefined) => None,
            Some(Value::Object(id)) => Some(*id),
            _ => return Err(Error::new("invalid incubator parent")),
        };
        let initial = match args.get(1) {
            None | Some(Value::Null | Value::Undefined) => BTreeMap::new(),
            Some(Value::Record(r)) if !r.is_array() => self
                .model
                .record_entries(r)?
                .iter()
                .map(|(key, value)| (key.clone(), InitialProperty::Value(value.clone())))
                .collect(),
            _ => return Err(Error::new("invalid incubator initial properties")),
        };
        Ok(self
            .start_incubation(
                component,
                parent,
                initial,
                IncubationMode::from_qml(args.get(2)),
            )?
            .map_or(Value::Null, |h| Value::Record(h.0)))
    }

    pub(crate) fn force_incubator(&mut self, handle: &Record) -> Result<(), Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot force incubation"));
        }
        if self.complete_incubator(handle, Default::default(), false, true)? {
            self.notify_incubator(handle)?;
        }
        Ok(())
    }

    fn complete_incubator(
        &mut self,
        handle: &Record,
        prepared: incubation::PreparedNodes,
        cancel: bool,
        async_context: bool,
    ) -> Result<bool, Error> {
        self.model.record_entries(handle)?;
        if !handle.is_incubator() {
            return Err(Error::new("expected an incubator"));
        }
        let Some(request) = self
            .model
            .records
            .take_request(handle, &mut self.record_changes)
        else {
            return Ok(false);
        };
        let result = if !cancel {
            if let Some(template) = self
                .model
                .dynamic
                .templates
                .get(&request.component)
                .cloned()
            {
                let revision = self.model.structure_revision();
                let previous = std::mem::replace(&mut self.incubating, async_context);
                let result = self.construct_object(
                    request.component,
                    request.parent.filter(|&id| self.model.node(id).is_some()),
                    &template,
                    request.initial.clone(),
                    None,
                    prepared,
                );
                self.incubating = previous;
                match result {
                    Ok(id) => Some(id),
                    Err(_) if self.model.structure_revision() == revision => None,
                    Err(error) => return Err(error),
                }
            } else {
                None
            }
        } else {
            None
        };
        self.store_field(
            handle,
            "object",
            Some(result.map_or(Value::Null, Value::Object)),
        )?;
        self.store_field(
            handle,
            "status",
            Some(Value::Number(if result.is_some() { 1. } else { 3. })),
        )?;
        self.deferred
            .push(deferred::DeferredWork::CancelIncubator(handle.clone()));
        Ok(true)
    }

    fn notify_incubator(&mut self, handle: &Record) -> Result<(), Error> {
        let fields = self.model.record_entries(handle)?;
        let callback = fields.get("onStatusChanged").cloned();
        let args = vec![fields["status"].clone()];
        match callback {
            Some(Value::Method(method)) if self.model.node(method.receiver()).is_some() => {
                self.invoke(method.receiver(), method.name(), args, &mut BTreeSet::new())?;
            }
            Some(Value::External(function)) => {
                self.with_external(
                    self.model.root(),
                    &function,
                    &mut BTreeSet::new(),
                    |engine, ctx| {
                        engine.call_external(ctx, &function, Value::Record(handle.clone()), args)
                    },
                )?;
            }
            _ => {}
        }
        Ok(())
    }
}
