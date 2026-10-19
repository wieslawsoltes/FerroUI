//! `FerroViewThreadInvoker` (ReactiveUI.Binding's integration, `...ViewThreadInvoker.cs`).

use crate::{FerroObjectHandles, FerroScheduler};
use ferroui_base::{FerroObject, Ref};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::IViewThreadInvoker;
use std::any::Any;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// The shared invoker (`Instance`).
    static INSTANCE: Rc<FerroViewThreadInvoker> = Rc::new(FerroViewThreadInvoker);
}

/// Routes the writes of the bindings to a FerroUI object onto the [`FerroScheduler`] of the
/// dispatcher that owns it (`FerroViewThreadInvoker`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream derives from
/// `SequencerViewThreadInvoker<FerroObject, FerroScheduler>`, which claims the targets of the
/// type `FerroObject`; a FerroUI object comes in several untyped forms
/// ([`FerroObjectHandles::object_of`]), so the invoker implements the trait itself, with the same
/// members: the scheduler of a target is `FerroScheduler::for_dispatcher` of its dispatcher. The
/// shared instance is per thread, as the objects are.
#[derive(Debug, Default)]
pub struct FerroViewThreadInvoker;

impl FerroViewThreadInvoker {
    /// The shared instance, which the bindings route their writes to FerroUI objects through
    /// (`Instance`).
    pub fn instance() -> Rc<FerroViewThreadInvoker> {
        INSTANCE.with(Rc::clone)
    }

    /// The scheduler of the dispatcher that owns `target` (`SequencerFor`).
    pub fn sequencer_for(target: &FerroObject) -> Rc<FerroScheduler> {
        FerroScheduler::for_dispatcher(target.dispatcher())
    }

    /// `target` as a FerroUI object; anything else panics, as upstream's cast throws
    /// `InvalidCastException`.
    fn typed(target: &Rc<dyn Any>) -> Ref<FerroObject> {
        match FerroObjectHandles::object_of(&**target) {
            Some(object) => object,
            None => panic!("Unable to cast the target of a binding to FerroObject"),
        }
    }
}

impl IViewThreadInvoker for FerroViewThreadInvoker {
    /// Whether `target` is a FerroUI object (`Claims`).
    fn claims(&self, target: &Rc<dyn Any>) -> bool {
        FerroObjectHandles::object_of(&**target).is_some()
    }

    /// Whether the calling thread owns the dispatcher of `target` (`CheckAccess`).
    fn check_access(&self, target: &Rc<dyn Any>) -> bool {
        Self::sequencer_for(&Self::typed(target)).check_access()
    }

    /// Queues `callback` on the scheduler of the dispatcher of `target` (`Post`).
    fn post(&self, target: &Rc<dyn Any>, callback: Box<dyn FnOnce()>) {
        let sequencer = Self::sequencer_for(&Self::typed(target));
        let _ = sequencer.schedule(Box::new(move |_: &Rc<dyn IScheduler>| {
            callback();
            Disposable::empty()
        }));
    }
}
