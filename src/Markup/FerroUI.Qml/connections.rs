//! Generalized signal listeners. Indexing by signal keeps ordinary updates
//! independent of the size of the object tree.
use super::*;
use crate::transaction::Transaction;
use ferroui_qml_compiler::{MemberKind, Object};

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

    pub(crate) fn validate_connections(&self) -> Result<(), Error> {
        for (signal, listeners) in &self.listeners {
            for listener in listeners {
                let Value::Object(target) = self.slots[listener.target].value else {
                    continue;
                };
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
        }
        Ok(())
    }
}

impl Transaction<'_> {
    pub(crate) fn dispatch_connections(
        &mut self,
        sender: ObjectId,
        signal: &str,
        arguments: &[Value],
    ) -> Result<(), Error> {
        let Some(listeners) = self.model.listeners.get(signal).cloned() else {
            return Ok(());
        };
        for listener in listeners {
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
