use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use ferroui_controls::application_lifetimes::ActivatedEventArgs;
use std::rc::Rc;

/// An activity that shows content of the framework.
///
/// The reference contract also carries the results of activities and the
/// navigation service of the back button (`IActivityResultHandler`,
/// `IActivityNavigationService`): stage 2 of
/// docs/porting/android-platform.md.
pub trait IFerroActivity {
    /// The content of the view of the activity.
    fn content(&self) -> Option<BoxedValue>;

    /// Sets the content of the view of the activity.
    fn set_content(&self, value: Option<BoxedValue>);

    /// Raised when the activity is started or receives an intent.
    ///
    /// Disposing the returned value removes the handler.
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Raised when the activity is stopped.
    ///
    /// Disposing the returned value removes the handler.
    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable>;

    /// `(activity as Activity)?.MoveTaskToBack(nonRoot)`: moves the task of
    /// the activity to the back of the activity stack, when the object
    /// behind the contract is an activity of the system.
    fn move_task_to_back(&self, _non_root: bool) -> bool {
        false
    }

    /// The number of the activity object of this backend, when the object
    /// behind the contract is one: what tells two activities apart (the
    /// reference compares the objects).
    fn as_ferro_activity_handle(&self) -> Option<i64> {
        None
    }
}
