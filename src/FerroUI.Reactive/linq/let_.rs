//! `Let` (`Linq/QueryLanguageEx.cs`, the experimental `ObservableEx.Let`): a function of the
//! sequence, applied at once.

/// The methods of `Let` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! let_methods {
    () => {
        /// The sequence that `function` makes of the sequence, called at once, so that the sequence
        /// may be named more than once in an expression
        /// (`Let<TSource, TResult>(IObservable<TSource>, Func<IObservable<TSource>, IObservable<TResult>>)`).
        fn let_<R: 'static>(
            &self,
            function: impl FnOnce(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            function(self.observable())
        }
    };
}
pub(crate) use let_methods;
