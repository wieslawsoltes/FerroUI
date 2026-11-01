//! The owner event loop drains a bounded snapshot. Jobs posted from within
//! callbacks belong to a subsequent turn, including a callback reposting itself.
use super::*;
use crate::transaction::Transaction;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
enum Key {
    Binding(ObjectId),
    Function(u64),
    Method(MethodRef),
}

pub(crate) enum Task {
    Method(MethodRef, Vec<Value>),
    Binding(ObjectId),
    Call {
        owner: ObjectId,
        function: ExternalValue,
        arguments: Vec<Value>,
    },
}

impl Task {
    fn key(&self) -> Key {
        match self {
            Self::Binding(owner) => Key::Binding(*owner),
            Self::Method(method, _) => Key::Method(method.clone()),
            Self::Call { function, .. } => Key::Function(function.id()),
        }
    }
}

#[derive(Default)]
pub(crate) struct Queue {
    sequence: u64,
    jobs: BTreeMap<u64, Task>,
    positions: BTreeMap<Key, u64>,
}

impl Queue {
    pub(crate) fn post(&mut self, tasks: Vec<Task>) {
        for task in tasks {
            let key = task.key();
            if let Some(previous) = self.positions.get(&key) {
                if matches!(key, Key::Binding(_)) {
                    continue;
                }
                // Reposting a function changes its place in the queue and its arguments.
                self.jobs.remove(previous);
            }
            self.sequence += 1;
            self.positions.insert(key, self.sequence);
            self.jobs.insert(self.sequence, task);
        }
    }

    fn pop(&mut self, boundary: u64) -> Option<Task> {
        if *self.jobs.first_key_value()?.0 > boundary {
            return None;
        }
        let (_, task) = self.jobs.pop_first()?;
        self.positions.remove(&task.key());
        Some(task)
    }
}

impl Instance {
    pub fn has_deferred_work(&self) -> bool {
        !self.javascript.deferred.jobs.is_empty()
    }

    /// Execute work enqueued before this turn began. Each job has a separate
    /// transaction. Failure preserves the remaining jobs for the next turn.
    pub fn process_deferred(&mut self, limit: usize) -> Result<usize, Error> {
        let boundary = self.javascript.deferred.sequence;
        let mut processed = 0;
        while processed < limit {
            let Some(task) = self.javascript.deferred.pop(boundary) else {
                break;
            };
            self.transaction(|tx| tx.dispatch_deferred(task))?;
            processed += 1;
        }
        Ok(processed)
    }
}

impl Transaction<'_> {
    fn dispatch_deferred(&mut self, task: Task) -> Result<(), Error> {
        match task {
            Task::Method(method, arguments) if self.model.node(method.receiver()).is_some() => {
                self.invoke(
                    method.receiver(),
                    method.name(),
                    arguments,
                    &mut BTreeSet::new(),
                )?;
                Ok(())
            }
            Task::Binding(owner) if self.model.node(owner).is_some() => self.publish_delayed(owner),
            Task::Call {
                owner,
                function,
                arguments,
            } if self.model.node(owner).is_some() => {
                if self
                    .model
                    .javascript
                    .engine
                    .as_ref()
                    .is_some_and(|e| e.is_reference_live(&function))
                {
                    self.with_external(owner, &function, &mut BTreeSet::new(), |engine, ctx| {
                        engine.call_external(ctx, &function, Value::Undefined, arguments)
                    })?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
