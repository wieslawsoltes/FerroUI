//! Generalized signal listeners. Indexing by signal keeps ordinary updates
//! independent of the size of the object tree.
use super::*;
use crate::transaction::Transaction;
use ferroui_qml_compiler::{MemberKind, Object};

/// Positions in declaration order. Dispatch can resume the ordinary traversal
/// at its next position if a handler retargets a later listener.
pub(crate) type TargetIndex = BTreeMap<String, BTreeMap<ObjectId, Rc<Vec<usize>>>>;

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

    pub(crate) fn index_connections(&self) -> Result<TargetIndex, Error> {
        let mut index = TargetIndex::new();
        for (signal, listeners) in &self.listeners {
            let mut targets = BTreeMap::<ObjectId, Vec<usize>>::new();
            for (position, listener) in listeners.iter().enumerate() {
                let Value::Object(target) = self.slots[listener.target].value else {
                    continue;
                };
                targets.entry(target).or_default().push(position);
                if self.slots[listener.ignore_unknown].value == Value::Bool(true) {
                    continue;
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
            }
            index.insert(
                signal.clone(),
                targets
                    .into_iter()
                    .map(|(id, entries)| (id, Rc::new(entries)))
                    .collect(),
            );
        }
        Ok(index)
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
            .and_then(|index| index.get(signal)?.get(&sender))
            .cloned();
        let mut matches = matching.iter().flat_map(|indices| indices.iter().copied());
        let mut next = 0;
        while next < end {
            if !self.connection_targets_changed && self.model.listener_targets.is_some() {
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
