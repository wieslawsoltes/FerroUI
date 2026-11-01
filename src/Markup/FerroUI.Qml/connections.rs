//! Generalized signal listeners. Indexing by signal keeps ordinary updates
//! independent of the size of the object tree.
use super::*;
use crate::transaction::Transaction;
use ferroui_qml_compiler::{MemberKind, Object};

/// Positions in declaration order. Dispatch can resume the ordinary traversal
/// at its next position if a handler retargets a later listener.
#[derive(Default)]
pub(crate) struct TargetIndex {
    routes: BTreeMap<String, BTreeMap<ObjectId, Rc<Vec<usize>>>>,
    receivers: BTreeMap<ObjectId, Receiver>,
}

struct Receiver {
    slot: SlotId,
    target: Option<ObjectId>,
    // (signal, position) also preserves the global signal validation order.
    handlers: Vec<(Rc<str>, usize)>,
}

pub(crate) enum IndexUpdate {
    Rebuild(TargetIndex),
    Retarget(Vec<(ObjectId, Option<ObjectId>)>),
}
impl IndexUpdate {
    pub(crate) fn commit(self, model: &mut Instance) {
        match self {
            Self::Rebuild(index) => model.listener_targets = Some(index),
            Self::Retarget(edits) => {
                let index = model
                    .listener_targets
                    .as_mut()
                    .expect("validated routing index");
                for (id, next) in edits {
                    let receiver = index.receivers.get_mut(&id).expect("registered receiver");
                    let old = std::mem::replace(&mut receiver.target, next);
                    for (signal, position) in &receiver.handlers {
                        let targets = index
                            .routes
                            .get_mut(signal.as_ref())
                            .expect("registered signal");
                        if let Some(old) = old {
                            let slots =
                                Rc::make_mut(targets.get_mut(&old).expect("previous target"));
                            slots.remove(
                                slots.binary_search(position).expect("registered position"),
                            );
                            if slots.is_empty() {
                                targets.remove(&old);
                            }
                        }
                        if let Some(next) = next {
                            let slots = Rc::make_mut(targets.entry(next).or_default());
                            let at = slots.partition_point(|p| p < position);
                            slots.insert(at, *position);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct Listener {
    pub(crate) receiver: ObjectId,
    target: SlotId,
    enabled: SlotId,
    ignore_unknown: SlotId,
    handler: Rc<Procedure>,
    implicit_parameters: bool,
}

fn signal_name(handler: &str) -> Option<String> {
    let mut suffix = handler.strip_prefix("on")?.chars();
    let first = suffix.next()?;
    if !first.is_uppercase() {
        return None;
    }
    Some(format!("{}{}", first.to_lowercase(), suffix.as_str()))
}

impl Instance {
    pub(crate) fn connect_handlers(
        &mut self,
        receiver: ObjectId,
        source: &Object,
    ) -> Result<(), Error> {
        let legacy = source.members.iter().any(
            |m| matches!(&m.kind, MemberKind::Handler {name,..} if signal_name(name).is_some()),
        );
        let target = self.key(receiver, "target")?;
        let enabled = self.key(receiver, "enabled")?;
        let ignore_unknown = self.key(receiver, "ignoreUnknownSignals")?;
        for member in &source.members {
            let (name, proc, implicit_parameters) = match &member.kind {
                MemberKind::Function { name, .. } if !legacy => (
                    name,
                    self.nodes[receiver.index].methods[name].clone(),
                    false,
                ),
                MemberKind::Handler {
                    name,
                    parameters,
                    body,
                } if legacy => (
                    name,
                    Rc::new(Procedure {
                        #[cfg(feature = "diagnostics")]
                        location: (diagnostics::Kind::Handler, name.clone(), member.span),
                        scope: member.scope,
                        parameters: parameters
                            .iter()
                            .map(|name| Parameter {
                                name: name.clone(),
                                ty: None,
                            })
                            .collect(),
                        result: None,
                        body: body.clone(),
                    }),
                    parameters.is_empty(),
                ),
                _ => continue,
            };
            let Some(name) = signal_name(name) else {
                continue;
            };
            self.listener_targets = None;
            self.listeners.entry(name).or_default().push(Listener {
                receiver,
                target,
                enabled,
                ignore_unknown,
                handler: proc,
                implicit_parameters,
            });
        }
        Ok(())
    }

    pub(crate) fn retain_connections(&mut self, mut keep: impl FnMut(&Listener) -> bool) {
        self.listeners.retain(|_, list| {
            let previous = list.len();
            list.retain(&mut keep);
            if list.len() != previous {
                self.listener_targets = None;
            }
            !list.is_empty()
        });
    }

    fn listener_target(&self, slot: SlotId) -> Option<ObjectId> {
        match self.slots[slot].value {
            Value::Object(id) => Some(id),
            _ => None,
        }
    }

    fn validate_listener(&self, signal: &str, listener: &Listener) -> Result<(), Error> {
        let Some(target) = self.listener_target(listener.target) else {
            return Ok(());
        };
        if self.slots[listener.ignore_unknown].value == Value::Bool(true) {
            return Ok(());
        }
        let node = &self.nodes[target.index];
        let parameters = node.signal(signal).ok_or_else(|| {
            Error::new(format!(
                "Connections target '{}' has no signal '{signal}'",
                node.name
            ))
        })?;
        if listener.handler.parameters.len() > parameters.len() {
            return Err(Error::new(format!(
                "too many parameters for Connections handler '{signal}'"
            )));
        }
        Ok(())
    }

    pub(crate) fn connection_updates(
        &self,
        changed: &BTreeSet<ObjectId>,
    ) -> Result<IndexUpdate, Error> {
        if let Some(index) = &self.listener_targets {
            let mut edits = Vec::new();
            let mut handlers = Vec::new();
            for id in changed {
                if let Some(receiver) = index.receivers.get(id) {
                    let target = self.listener_target(receiver.slot);
                    if target != receiver.target {
                        edits.push((*id, target));
                    }
                    handlers.extend(&receiver.handlers);
                }
            }
            handlers.sort_unstable();
            for (signal, position) in handlers {
                self.validate_listener(signal, &self.listeners[signal.as_ref()][*position])?;
            }
            return Ok(IndexUpdate::Retarget(edits));
        }
        let mut index = TargetIndex::default();
        for (signal, listeners) in &self.listeners {
            let name: Rc<str> = signal.as_str().into();
            let mut targets = BTreeMap::<ObjectId, Vec<usize>>::new();
            for (position, listener) in listeners.iter().enumerate() {
                self.validate_listener(signal, listener)?;
                let target = self.listener_target(listener.target);
                if let Some(id) = target {
                    targets.entry(id).or_default().push(position);
                }
                let receiver =
                    index
                        .receivers
                        .entry(listener.receiver)
                        .or_insert_with(|| Receiver {
                            slot: listener.target,
                            target,
                            handlers: Vec::new(),
                        });
                receiver.handlers.push((name.clone(), position));
            }
            index.routes.insert(
                signal.clone(),
                targets
                    .into_iter()
                    .map(|(id, entries)| (id, Rc::new(entries)))
                    .collect(),
            );
        }
        Ok(IndexUpdate::Rebuild(index))
    }
}

impl Transaction<'_> {
    pub(crate) fn dispatch_connections(
        &mut self,
        sender: ObjectId,
        signal: &str,
        arguments: &[Value],
    ) -> Result<(), Error> {
        let Some(listeners) = self.model.listeners.get(signal) else {
            return Ok(());
        };
        let end = listeners.len();
        let matching = self
            .model
            .listener_targets
            .as_ref()
            .and_then(|index| index.routes.get(signal)?.get(&sender))
            .cloned();
        let mut matches = matching.iter().flat_map(|indices| indices.iter().copied());
        let mut next = 0;
        while next < end {
            if self.changed_receivers.is_empty() && self.model.listener_targets.is_some() {
                let Some(position) = matches.next() else {
                    break;
                };
                next = position;
            }
            let listener = self.model.listeners[signal][next].clone();
            next += 1;
            let mut reads = BTreeSet::new();
            if self.read(listener.enabled, &mut reads)? != Value::Bool(true)
                || self.read(listener.target, &mut reads)? != Value::Object(sender)
            {
                continue;
            }
            self.budget()?;
            if listener.implicit_parameters {
                let mut handler = (*listener.handler).clone();
                handler.parameters = self.model.nodes[sender.index]
                    .signal(signal)
                    .unwrap_or_default()
                    .to_vec();
                self.run(listener.receiver, &handler, arguments.to_vec(), &mut reads)?;
            } else {
                self.run(
                    listener.receiver,
                    &listener.handler,
                    arguments.to_vec(),
                    &mut reads,
                )?;
            }
        }
        Ok(())
    }
}
