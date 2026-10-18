//! `DoWhile` (`Linq/Observable/DoWhile.cs`), and its overload of `QueryLanguage.Imperative.cs`.

use crate::linq::catch::{tail_recursive, SourceIterator, Sources, TailRecursiveMode};
use crate::linq::while_::{repetitions, Condition};
use ferroui_system::IObservable;
use std::rc::Rc;

/// `DoWhile<TSource>`: `source`, then its repetitions for as long as `condition` holds after each
/// of them (`GetSources`), one after another, run by a `ConcatSink<TSource>`.
pub(crate) fn do_while<T: 'static>(source: Rc<dyn IObservable<T>>, condition: Condition) -> Rc<dyn IObservable<T>> {
    let sources: Sources<T> = Rc::new(move || {
        let first = std::iter::once(Ok(source.clone()));
        Ok(Box::new(first.chain(repetitions(condition.clone(), source.clone()))) as SourceIterator<T>)
    });
    tail_recursive(sources, TailRecursiveMode::Concat)
}

/// The methods of `DoWhile` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! do_while_methods {
    () => {
        /// The values of the sequence, repeated for as long as `condition` holds after each
        /// repetition (`DoWhile<TSource>(IObservable<TSource>, Func<bool>)`).
        fn do_while(&self, condition: impl Fn() -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_while::do_while(self.observable(), std::rc::Rc::new(move || Ok(condition())))
        }

        /// [`do_while`](Self::do_while) with a condition that may fail; a failure ends the sequence
        /// with its error (`DoWhile` with a condition that throws).
        fn try_do_while(
            &self,
            condition: impl Fn() -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_while::do_while(self.observable(), std::rc::Rc::new(condition))
        }
    };
}
pub(crate) use do_while_methods;
