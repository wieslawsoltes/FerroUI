use std::cell::RefCell;
use std::rc::Rc;

/// The result of an operation that either has completed already or completes
/// later, in which case its continuations run synchronously at completion.
#[derive(Clone)]
pub struct SynchronousCompletionAsyncResult<T: Clone> {
    state: State<T>,
}

#[derive(Clone)]
enum State<T: Clone> {
    Result(T),
    Source(SynchronousCompletionAsyncResultSource<T>),
}

impl<T: Clone + 'static> SynchronousCompletionAsyncResult<T> {
    /// Creates a completed result.
    pub fn new(result: T) -> Self {
        Self { state: State::Result(result) }
    }

    pub fn is_completed(&self) -> bool {
        match &self.state {
            State::Result(_) => true,
            State::Source(source) => source.is_completed(),
        }
    }

    /// The result. Panics if the operation is not yet completed.
    pub fn get_result(&self) -> T {
        match &self.state {
            State::Result(result) => result.clone(),
            State::Source(source) => source.result(),
        }
    }

    /// Runs `continuation` now if the operation has completed without a
    /// source, otherwise when the source completes.
    pub fn on_completed(&self, continuation: impl FnOnce() + 'static) {
        match &self.state {
            State::Result(_) => continuation(),
            State::Source(source) => source.on_completed(Box::new(continuation)),
        }
    }
}

struct SourceInner<T> {
    result: Option<T>,
    continuations: Vec<Box<dyn FnOnce()>>,
}

/// The producer side of a [`SynchronousCompletionAsyncResult`].
pub struct SynchronousCompletionAsyncResultSource<T> {
    inner: Rc<RefCell<SourceInner<T>>>,
}

impl<T> Clone for SynchronousCompletionAsyncResultSource<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static> Default for SynchronousCompletionAsyncResultSource<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> SynchronousCompletionAsyncResultSource<T> {
    pub fn new() -> Self {
        Self { inner: Rc::new(RefCell::new(SourceInner { result: None, continuations: Vec::new() })) }
    }

    /// The consumer side of the operation.
    pub fn async_result(&self) -> SynchronousCompletionAsyncResult<T> {
        SynchronousCompletionAsyncResult { state: State::Source(self.clone()) }
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.inner.borrow().result.is_some()
    }

    pub(crate) fn result(&self) -> T {
        self.inner.borrow().result.clone().expect("Asynchronous operation is not yet completed")
    }

    pub(crate) fn on_completed(&self, continuation: Box<dyn FnOnce()>) {
        self.inner.borrow_mut().continuations.push(continuation);
    }

    /// Completes the operation and runs its continuations. Panics if it is
    /// already completed.
    pub fn set_result(&self, result: T) {
        let continuations = {
            let mut inner = self.inner.borrow_mut();
            if inner.result.is_some() {
                panic!("Asynchronous operation is already completed");
            }
            inner.result = Some(result);
            std::mem::take(&mut inner.continuations)
        };
        for continuation in continuations {
            continuation();
        }
    }

    /// Completes the operation unless it is already completed.
    pub fn try_set_result(&self, result: T) {
        if self.is_completed() {
            return;
        }
        self.set_result(result);
    }
}
