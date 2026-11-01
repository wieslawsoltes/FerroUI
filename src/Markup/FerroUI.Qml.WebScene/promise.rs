use super::*;
use ferroui_qml::PromiseState;

impl Realm {
    pub(super) fn observe(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        value: &ExternalValue,
        timeout: Duration,
    ) -> Result<PromiseState, Error> {
        if self.deadline.get().is_some() {
            return Err(Error::new(
                "a Promise can only be observed after the active JavaScript call returns",
            ));
        }
        // Allow the worker to deliver its final status before the outer hard
        // deadline. The hard deadline also covers a runaway V8 microtask.
        let remaining = ctx
            .deadline()
            .map(|limit| limit.saturating_duration_since(Instant::now()));
        let budget = remaining.map_or(self.timeout, |left| self.timeout.min(left));
        let available = budget.saturating_sub(Duration::from_millis(5));
        let milliseconds = timeout.min(available).as_millis().min(u64::MAX as u128) as u64;
        let command =
            json!({"op":"await","target":["js",self.handle(value)?],"waitMs":milliseconds});
        let result = self.invoke(ctx, command)?;
        let Value::List(mut items) = result else {
            return Err(Error::new("invalid WebScene Promise result"));
        };
        if items.len() != 2 {
            return Err(Error::new("invalid WebScene Promise state"));
        }
        let result = items.pop().expect("checked length");
        match items.pop() {
            Some(Value::String(tag)) => match tag.as_str() {
                "other" => Ok(PromiseState::NotPromise),
                "pending" => Ok(PromiseState::Pending),
                "fulfilled" => Ok(PromiseState::Fulfilled(result)),
                "rejected" => match result {
                    Value::String(reason) => Ok(PromiseState::Rejected(reason)),
                    _ => Err(Error::new("invalid Promise rejection reason")),
                },
                _ => Err(Error::new("unknown Promise state")),
            },
            _ => Err(Error::new("invalid Promise state tag")),
        }
    }
}
