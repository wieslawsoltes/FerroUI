//! `SequencerViewThreadInvoker<TTarget, TSequencer>`.

use crate::binding::interfaces::IViewThreadInvoker;
use crate::primitives::IThreadAffineSequencer;
use ferroui_reactive::disposables::Disposable;
use std::any::Any;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

/// Routes the writes to a view object of type `T` through the scheduler that owns the object's
/// thread (`SequencerViewThreadInvoker<TTarget, TSequencer>`).
///
/// A platform supplies `sequencer_for`, which names the scheduler that owns an object. The
/// scheduler answers whether the calling thread owns it, and queues work onto that thread. An
/// object with no owning scheduler belongs to no thread, so every thread may write to it.
// Deviation (DEVIATIONS.md, ReactiveUI): the abstract class with its one abstract member
// (`SequencerFor`) is a struct built from that member as a closure.
pub struct SequencerViewThreadInvoker<T: 'static, S: IThreadAffineSequencer + ?Sized + 'static> {
    sequencer_for: Rc<dyn Fn(&T) -> Option<Rc<S>>>,
    _target: PhantomData<fn(&T)>,
}

impl<T: 'static, S: IThreadAffineSequencer + ?Sized + 'static> SequencerViewThreadInvoker<T, S> {
    /// The invoker of the objects of type `T`, whose owning scheduler `sequencer_for` names
    /// (`None` for an object that belongs to no thread).
    pub fn new(sequencer_for: impl Fn(&T) -> Option<Rc<S>> + 'static) -> Self {
        Self { sequencer_for: Rc::new(sequencer_for), _target: PhantomData }
    }

    /// The scheduler that owns the thread of `target` (`SequencerFor`).
    pub fn sequencer_for(&self, target: &T) -> Option<Rc<S>> {
        (self.sequencer_for)(target)
    }

    /// `target` as a `T`; another type panics, as upstream's cast throws `InvalidCastException`.
    fn typed<'a>(target: &'a Rc<dyn Any>) -> &'a T {
        match (**target).downcast_ref::<T>() {
            Some(target) => target,
            None => panic!("Unable to cast the target of a binding to {}", std::any::type_name::<T>()),
        }
    }
}

impl<T: 'static, S: IThreadAffineSequencer + ?Sized + 'static> IViewThreadInvoker for SequencerViewThreadInvoker<T, S> {
    /// Whether `target` is a `T` (`Claims`).
    fn claims(&self, target: &Rc<dyn Any>) -> bool {
        (**target).is::<T>()
    }

    /// Whether the calling thread owns the scheduler of `target`, a `T`; `true` when it has none
    /// (`CheckAccess`).
    fn check_access(&self, target: &Rc<dyn Any>) -> bool {
        self.sequencer_for(Self::typed(target)).is_none_or(|sequencer| sequencer.check_access())
    }

    /// Queues `callback` on the scheduler of `target`, a `T`, or runs it at once when the target
    /// has none (`Post`).
    fn post(&self, target: &Rc<dyn Any>, callback: Box<dyn FnOnce()>) {
        let Some(sequencer) = self.sequencer_for(Self::typed(target)) else {
            callback();
            return;
        };
        let _ = sequencer.schedule(Box::new(move |_| {
            callback();
            Disposable::empty()
        }));
    }
}

impl<T: 'static, S: IThreadAffineSequencer + ?Sized + 'static> fmt::Debug for SequencerViewThreadInvoker<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SequencerViewThreadInvoker<{}>", std::any::type_name::<T>())
    }
}
