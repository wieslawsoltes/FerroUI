//! Explicit, bounded Promise interop from the Rust owner thread. A retained
//! Promise belongs to one QML engine realm.
use super::*;
use std::time::Duration;

/// A rejected value is rendered by its JavaScript backend with available stack context.
#[derive(Clone, Debug, PartialEq)]
pub enum PromiseState {
    NotPromise,
    Pending,
    Fulfilled(Value),
    Rejected(String),
}

impl Instance {
    /// Inspect a native JavaScript Promise without waiting for future work.
    pub fn promise_state(&mut self, promise: &ExternalValue) -> Result<PromiseState, Error> {
        let owner = self.root();
        self.transaction(|tx| tx.promise(owner, promise, Duration::ZERO))
    }

    /// Wait for a retained Promise. A rejection or timeout rolls back any QML
    /// changes made within this settlement transaction.
    pub fn settle_promise(
        &mut self,
        promise: &ExternalValue,
        timeout: Duration,
    ) -> Result<Value, Error> {
        let owner = self.root();
        self.transaction(|tx| finish(tx.promise(owner, promise, timeout)?, timeout, None))
    }

    /// Call and settle in one transaction. A rejection also reverts synchronous
    /// QML writes performed before the method returned its Promise. Ordinary
    /// return values pass through directly.
    pub fn call_and_await(
        &mut self,
        owner: ObjectId,
        method: &str,
        args: Vec<Value>,
        timeout: Duration,
    ) -> Result<Value, Error> {
        self.transaction(|tx| {
            let result = tx.invoke(owner, method, args, &mut BTreeSet::new())?;
            tx.model.validate_handles(&result)?;
            match &result {
                Value::External(handle) => {
                    finish(tx.promise(owner, handle, timeout)?, timeout, Some(result))
                }
                _ => Ok(result),
            }
        })
    }
}

fn finish(
    state: PromiseState,
    timeout: Duration,
    non_promise: Option<Value>,
) -> Result<Value, Error> {
    match state {
        PromiseState::Fulfilled(value) => Ok(value),
        PromiseState::Rejected(reason) => {
            Err(Error::new(format!("JavaScript Promise rejected: {reason}")))
        }
        PromiseState::Pending => Err(Error::new(format!(
            "JavaScript Promise did not settle within {timeout:?}"
        ))),
        PromiseState::NotPromise => {
            non_promise.ok_or_else(|| Error::new("expected a JavaScript Promise"))
        }
    }
}

impl transaction::Transaction<'_> {
    fn promise(
        &mut self,
        owner: ObjectId,
        value: &ExternalValue,
        timeout: Duration,
    ) -> Result<PromiseState, Error> {
        let state = self.with_external(owner, value, &mut BTreeSet::new(), |engine, ctx| {
            engine.observe_promise(ctx, value, timeout)
        })?;
        if let PromiseState::Fulfilled(value) = &state {
            self.model.validate_handles(value)?;
        }
        Ok(state)
    }
}
