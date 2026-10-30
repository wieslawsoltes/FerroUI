//! Port of `Core/ReversibleActionExecution.cs`.

use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_xaml_interactivity::{ActionCollection, IAction, IReversibleAction};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

/// Executes the actions of a collection, the reversible ones reversibly (an internal class of the
/// reference).
pub struct ReversibleActionExecution;

/// Whether the result of an action is `false` (`result is false`).
fn is_false(result: Option<&BoxedValue>) -> bool {
    result.and_then(|result| result.downcast_ref::<bool>()).is_some_and(|value| !*value)
}

impl ReversibleActionExecution {
    /// Executes the actions of `actions`: a reversible action with
    /// [`IReversibleAction::execute_reversibly`], any other action with [`IAction::execute`].
    /// Returns the reversible actions that applied (whose result is not `false`), in the order of
    /// the actions.
    ///
    /// # Panics
    /// Re-raises the panic of an action, after reverting the reversible actions that applied
    /// before it, last first (a panic while reverting is dropped, so that the panic of the
    /// action is the one raised).
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the applied actions are returned as the objects of
    /// the actions (a contract of an object is found through the object).
    pub fn execute(
        sender: Option<&BoxedValue>,
        actions: Option<&ActionCollection>,
        parameter: Option<&BoxedValue>,
    ) -> Vec<Ref<FerroObject>> {
        let mut applied_actions: Vec<Ref<FerroObject>> = Vec::new();
        let Some(actions) = actions else {
            return applied_actions;
        };

        let result = catch_unwind(AssertUnwindSafe(|| {
            for item in actions.snapshot().iter() {
                if let Some(reversible_action) = <dyn IReversibleAction>::of(item) {
                    let result = reversible_action.execute_reversibly(sender, parameter);
                    if !is_false(result.as_ref()) {
                        applied_actions.push(item.clone());
                    }
                } else if let Some(action) = <dyn IAction>::of(item) {
                    action.execute(sender, parameter);
                }
            }
        }));

        if let Err(payload) = result {
            for applied_action in applied_actions.iter().rev() {
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    if let Some(reversible_action) = <dyn IReversibleAction>::of(applied_action) {
                        reversible_action.revert(sender, parameter);
                    }
                }));
                // Preserve the panic raised while applying the action sequence.
            }

            resume_unwind(payload);
        }

        applied_actions
    }

    /// Reverts `applied_actions`, last first (the loop of the triggers that revert the actions
    /// they applied).
    pub fn revert(applied_actions: &[Ref<FerroObject>], sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) {
        for applied_action in applied_actions.iter().rev() {
            if let Some(reversible_action) = <dyn IReversibleAction>::of(applied_action) {
                reversible_action.revert(sender, parameter);
            }
        }
    }
}
