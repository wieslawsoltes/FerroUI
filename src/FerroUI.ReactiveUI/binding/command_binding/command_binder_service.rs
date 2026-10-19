//! `CommandBinderService` (ReactiveUI.Binding, `CommandBinding/CommandBinderService.cs`), with the
//! check of `CommandBindingAffinityChecker` (`Fallback/CommandBindingAffinityChecker.cs`).

use crate::binding::interfaces::ICreatesCommandBinding;
use ferroui_splat::AppLocator;
use std::any::TypeId;
use std::rc::Rc;

/// Finds the [`ICreatesCommandBinding`] with the highest affinity for a type of control
/// (`CommandBinderService`), as the property observation finds its observer.
pub struct CommandBinderService;

impl CommandBinderService {
    /// The registered binder with the highest positive affinity for the control type `T`; of equal
    /// affinities the first registered wins; `None` when every binder answers zero or less
    /// (`GetBinder<T>(bool hasEventTarget)`).
    pub fn get_binder<T: ?Sized + 'static>(has_event_target: bool) -> Option<Rc<dyn ICreatesCommandBinding>> {
        Self::get_binder_for(TypeId::of::<T>(), has_event_target)
    }

    /// [`get_binder`](Self::get_binder) for the control type `target_type`.
    pub fn get_binder_for(target_type: TypeId, has_event_target: bool) -> Option<Rc<dyn ICreatesCommandBinding>> {
        let mut best_binder = None;
        let mut best_affinity = 0;
        for binder in AppLocator::get_services::<dyn ICreatesCommandBinding>() {
            let affinity = binder.get_affinity_for_object(target_type, has_event_target);
            if affinity <= best_affinity {
                continue;
            }
            best_affinity = affinity;
            best_binder = Some(binder);
        }
        best_binder
    }

    /// The registered binder with the highest affinity for `target_type` when it outranks
    /// `own_affinity`, the affinity of the binding the caller would make itself
    /// (`CommandBindingAffinityChecker.HasHigherAffinityPlugin<T>` and `GetBinder<T>`).
    pub fn get_binder_above(target_type: TypeId, own_affinity: i32, has_event_target: bool) -> Option<Rc<dyn ICreatesCommandBinding>> {
        let binder = Self::get_binder_for(target_type, has_event_target)?;
        (binder.get_affinity_for_object(target_type, has_event_target) > own_affinity).then_some(binder)
    }
}
