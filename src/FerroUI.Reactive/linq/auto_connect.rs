//! `AutoConnect` (`Linq/Observable/AutoConnect.cs`): a connectable observable connected once it
//! has a given number of observers, and never disconnected by them.

use crate::subjects::IConnectableObservable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) type OnConnect = Box<dyn FnOnce(Rc<dyn IDisposable>)>;

/// `AutoConnect<T>`: connects the source when the number of observers reaches `min_observers`, and
/// hands the connection to `on_connect`.
struct AutoConnect<T> {
    source: Rc<dyn IConnectableObservable<T>>,
    min_observers: usize,
    on_connect: RefCell<Option<OnConnect>>,
    count: Cell<usize>,
}

impl<T: 'static> IObservable<T> for AutoConnect<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let subscription = self.source.subscribe(observer);

        if self.count.get() < self.min_observers {
            let count = self.count.get() + 1;
            self.count.set(count);
            if count == self.min_observers {
                let connection = self.source.connect();
                let on_connect = self.on_connect.borrow_mut().take();
                if let Some(on_connect) = on_connect {
                    on_connect(connection);
                }
            }
        }
        subscription
    }
}

/// `AutoConnect(IConnectableObservable<TSource>, int, Action<IDisposable>)`: with no observers
/// required, the source is connected at once and returned.
pub(crate) fn auto_connect<T: 'static>(
    source: Rc<dyn IConnectableObservable<T>>,
    min_observers: usize,
    on_connect: Option<OnConnect>,
) -> Rc<dyn IObservable<T>> {
    if min_observers == 0 {
        let connection = source.connect();
        if let Some(on_connect) = on_connect {
            on_connect(connection);
        }
        return source;
    }
    Rc::new(AutoConnect { source, min_observers, on_connect: RefCell::new(on_connect), count: Cell::new(0) })
}

/// The methods of `AutoConnect` (provided methods of
/// [`ConnectableObservableExt`](super::multicast::ConnectableObservableExt)). Upstream's one
/// overload has default arguments (`minObservers = 1`, `onConnect = null`); the port has a method
/// for each.
macro_rules! auto_connect_methods {
    () => {
        /// A sequence that connects the source when it gets its first observer (`AutoConnect()`).
        fn auto_connect(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::auto_connect::auto_connect(self.connectable(), 1, None)
        }

        /// A sequence that connects the source when it has `min_observers` observers; zero connects
        /// it at once (`AutoConnect(int)`).
        fn auto_connect_min(&self, min_observers: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::auto_connect::auto_connect(self.connectable(), min_observers, None)
        }

        /// [`auto_connect_min`](Self::auto_connect_min) that hands the connection to `on_connect`,
        /// with which the source can be disconnected (`AutoConnect(int, Action<IDisposable>)`).
        fn auto_connect_with(
            &self,
            min_observers: usize,
            on_connect: impl FnOnce(std::rc::Rc<dyn ferroui_system::IDisposable>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::auto_connect::auto_connect(self.connectable(), min_observers, Some(Box::new(on_connect)))
        }
    };
}
pub(crate) use auto_connect_methods;
