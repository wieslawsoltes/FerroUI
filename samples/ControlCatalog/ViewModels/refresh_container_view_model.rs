//! Port of `ViewModels/RefreshContainerViewModel.cs`.

use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::ItemsSource;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// The view model of the refresh container page: the items of its list.
pub struct RefreshContainerViewModel {
    base: ViewModelBase,
    items: Rc<BindableList<String>>,
}

impl PartialEq for RefreshContainerViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for RefreshContainerViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl RefreshContainerViewModel {
    pub fn new() -> Rc<RefreshContainerViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), items: BindableList::new((1..=200).map(|i| format!("Item {i}"))) })
    }

    pub fn items(&self) -> Rc<BindableList<String>> {
        self.items.clone()
    }

    pub async fn add_to_top(&self) {
        delay(Duration::from_millis(3000)).await;
        let count = self.items.items().count() as i32;
        self.items.items().insert(0, format!("Item {}", 200 - count));
    }
}

#[derive(Default)]
struct DelayState {
    elapsed: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// `Task.Delay(duration)`: completes when a dispatcher timer of `duration`
/// has ticked.
struct Delay(Rc<DelayState>);

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.elapsed.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn delay(duration: Duration) -> Delay {
    let state = Rc::new(DelayState::default());
    let timer_state = state.clone();
    DispatcherTimer::run_once(
        move || {
            timer_state.elapsed.set(true);
            let waker = timer_state.waker.borrow_mut().take();
            if let Some(waker) = waker {
                waker.wake();
            }
        },
        duration,
        DispatcherPriority::DEFAULT,
    );
    Delay(state)
}

ferro_markup_type!(class RefreshContainerViewModel {
    this: Rc<RefreshContainerViewModel>,
    handles: [RefreshContainerViewModel, Rc<RefreshContainerViewModel>, Option<Rc<RefreshContainerViewModel>>],
    constructors: [() => RefreshContainerViewModel::new],
    properties: [
        // A list a binding delivers to an items source property.
        Items: ItemsSource { get: |this: &Rc<RefreshContainerViewModel>| ItemsSource::from(this.items()) },
    ],
    notify_property_changed: RefreshContainerViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_list_starts_with_two_hundred_items() {
        let view_model = RefreshContainerViewModel::new();
        let items = view_model.items().items().to_vec();
        assert_eq!(200, items.len());
        assert_eq!("Item 1", items[0]);
        assert_eq!("Item 200", items[199]);
    }
}
