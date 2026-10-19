//! `BatchIf` (`Cache/ObservableCacheEx.BatchIf.cs`).

macro_rules! batch_if_methods {
    () => {
        /// The change sets, held back while `pause_if_true_selector` last gave `true` and delivered
        /// as one when it gives `false` (`BatchIf(IObservable<bool>)`).
        fn batch_if(&self, pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, false, None, None)
        }

        /// [`batch_if`](Self::batch_if) with the timers on `scheduler` (`BatchIf(IObservable<bool>,
        /// IScheduler?)`).
        fn batch_if_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, false, None, Some(scheduler))
        }

        /// [`batch_if`](Self::batch_if), paused from the start when `initial_pause_state` is true
        /// (`BatchIf(IObservable<bool>, bool, IScheduler? = null)`).
        fn batch_if_with_initial(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, initial_pause_state, None, None)
        }

        /// [`batch_if_with_initial`](Self::batch_if_with_initial) on `scheduler`.
        fn batch_if_with_initial_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, initial_pause_state, None, Some(scheduler))
        }

        /// [`batch_if`](Self::batch_if) whose pause ends by itself after `time_out`
        /// (`BatchIf(IObservable<bool>, TimeSpan?, IScheduler? = null)`).
        fn batch_if_with_time_out(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            time_out: Option<std::time::Duration>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, time_out, false, None, None)
        }

        /// [`batch_if_with_time_out`](Self::batch_if_with_time_out) on `scheduler`.
        fn batch_if_with_time_out_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            time_out: Option<std::time::Duration>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, time_out, false, None, Some(scheduler))
        }

        /// [`batch_if_with_time_out`](Self::batch_if_with_time_out), paused from the start when
        /// `initial_pause_state` is true (`BatchIf(IObservable<bool>, bool, TimeSpan?, IScheduler? =
        /// null)`).
        fn batch_if_with_initial_and_time_out(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            time_out: Option<std::time::Duration>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, time_out, initial_pause_state, None, None)
        }

        /// [`batch_if_with_initial_and_time_out`](Self::batch_if_with_initial_and_time_out) on
        /// `scheduler`.
        fn batch_if_with_initial_and_time_out_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            time_out: Option<std::time::Duration>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, time_out, initial_pause_state, None, Some(scheduler))
        }

        /// [`batch_if`](Self::batch_if) that also delivers, and pauses again, at each value of
        /// `timer`; the pause ends when `timer` ends (`BatchIf(IObservable<bool>, IScheduler?,
        /// IObservable<Unit>?)`).
        fn batch_if_with_timer(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            timer: Option<std::rc::Rc<dyn ferroui_system::IObservable<()>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, false, timer, None)
        }

        /// [`batch_if_with_timer`](Self::batch_if_with_timer) on `scheduler`.
        fn batch_if_with_timer_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            timer: Option<std::rc::Rc<dyn ferroui_system::IObservable<()>>>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, false, timer, Some(scheduler))
        }

        /// [`batch_if_with_timer`](Self::batch_if_with_timer), paused from the start when
        /// `initial_pause_state` is true (`BatchIf(IObservable<bool>, bool, IObservable<Unit>?,
        /// IScheduler? = null)`).
        fn batch_if_with_initial_and_timer(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            timer: Option<std::rc::Rc<dyn ferroui_system::IObservable<()>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, initial_pause_state, timer, None)
        }

        /// [`batch_if_with_initial_and_timer`](Self::batch_if_with_initial_and_timer) on `scheduler`.
        fn batch_if_with_initial_and_timer_on(
            &self,
            pause_if_true_selector: std::rc::Rc<dyn ferroui_system::IObservable<bool>>,
            initial_pause_state: bool,
            timer: Option<std::rc::Rc<dyn ferroui_system::IObservable<()>>>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::batch_if(self.change_sets(), pause_if_true_selector, None, initial_pause_state, timer, Some(scheduler))
        }
    };
}
pub(crate) use batch_if_methods;
