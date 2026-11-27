//! The add and remove accessors of the markup events of the ribbon over
//! `MarkupDelegate::combine` and `MarkupDelegate::remove`.
//!
//! Combining delegates of different nominal types is the `ArgumentException` of
//! `Delegate.Combine`. An event accessor has no result to return it in, so the error goes to
//! the dispatcher, as the failure of a markup handler does, and the handlers of the event stay
//! as they were.

use ferroui_base::metadata::{MarkupDelegate, MarkupInvokeError};
use ferroui_base::threading::Dispatcher;

fn or_report(
    result: Result<Option<MarkupDelegate>, MarkupInvokeError>,
    current: Option<&MarkupDelegate>,
) -> Option<MarkupDelegate> {
    result.unwrap_or_else(|error| {
        Dispatcher::current_dispatcher().raise_unhandled_exception(error);
        current.cloned()
    })
}

/// The handlers of an event after `handler` is added (`+=`).
pub fn add_handler(current: Option<&MarkupDelegate>, handler: Option<&MarkupDelegate>) -> Option<MarkupDelegate> {
    or_report(MarkupDelegate::combine(current, handler), current)
}

/// The handlers of an event after `handler` is removed (`-=`).
pub fn remove_handler(current: Option<&MarkupDelegate>, handler: Option<&MarkupDelegate>) -> Option<MarkupDelegate> {
    or_report(MarkupDelegate::remove(current, handler), current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mismatched_handler_is_reported_and_changes_nothing() {
        let _scope = Dispatcher::unit_test_scope();
        let first = MarkupDelegate::action(|| {});
        let second = MarkupDelegate::action(|| {});
        let both = add_handler(Some(&first), Some(&second)).expect("two handlers");
        assert_eq!(both.invocation_list().len(), 2);
        assert_eq!(remove_handler(Some(&both), Some(&second)), Some(first.clone()));
        let untyped = MarkupDelegate::new(|_| None);
        let reported = Dispatcher::current_dispatcher().capture_unhandled_exceptions(|| {
            let unchanged = add_handler(Some(&first), Some(&untyped));
            assert_eq!(unchanged, Some(first.clone()));
        });
        assert!(reported.unwrap_err().contains("nominal type"));
    }
}
