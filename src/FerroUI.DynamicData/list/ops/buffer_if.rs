//! `BufferIf` (`List/ObservableListEx.BufferIf.cs`).

macro_rules! buffer_if_methods {
    () => {
        /// The change sets, held back while `pause_if_true_selector` last gave `true` and delivered
        /// as one when it gives `false`; the signals are observed on the default scheduler
        /// (`BufferIf(IObservable<bool>, IScheduler? = null)`).
        fn buffer_if(&self, pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, false, None, None)
        }

        /// [`buffer_if`](Self::buffer_if) on `scheduler`.
        fn buffer_if_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, false, None, Some(scheduler))
        }

        /// [`buffer_if`](Self::buffer_if), paused from the start when `initial_pause_state` is true
        /// (`BufferIf(IObservable<bool>, bool, IScheduler? = null)`).
        fn buffer_if_with_initial(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, initial_pause_state, None, None)
        }

        /// [`buffer_if_with_initial`](Self::buffer_if_with_initial) on `scheduler`.
        fn buffer_if_with_initial_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, initial_pause_state, None, Some(scheduler))
        }

        /// [`buffer_if`](Self::buffer_if) whose pause ends by itself after `time_out`
        /// (`BufferIf(IObservable<bool>, TimeSpan?, IScheduler? = null)`).
        fn buffer_if_with_time_out(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            time_out: Option<std::time::Duration>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, false, time_out, None)
        }

        /// [`buffer_if_with_time_out`](Self::buffer_if_with_time_out) on `scheduler`.
        fn buffer_if_with_time_out_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            time_out: Option<std::time::Duration>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, false, time_out, Some(scheduler))
        }

        /// [`buffer_if_with_time_out`](Self::buffer_if_with_time_out), paused from the start when
        /// `initial_pause_state` is true (`BufferIf(IObservable<bool>, bool, TimeSpan?, IScheduler? =
        /// null)`).
        fn buffer_if_with_initial_and_time_out(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            time_out: Option<std::time::Duration>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, initial_pause_state, time_out, None)
        }

        /// [`buffer_if_with_initial_and_time_out`](Self::buffer_if_with_initial_and_time_out) on
        /// `scheduler`.
        fn buffer_if_with_initial_and_time_out_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            time_out: Option<std::time::Duration>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::buffer_if(self.change_sets(), pause_if_true_selector, initial_pause_state, time_out, Some(scheduler))
        }
    };
}
pub(crate) use buffer_if_methods;
