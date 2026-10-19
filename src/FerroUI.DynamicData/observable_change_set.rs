//! `ObservableChangeSet` (`ObservableChangeSet.cs`): sequences of change sets made by a function
//! that fills a cache or a list of their own, as `Observable.Create` makes a sequence of a
//! function that notifies an observer.
//!
//! Each subscription creates a `SourceCache` (keyed by `key_selector`) or a `SourceList`, passes it
//! to `subscribe`, and delivers its change sets. Disposing the subscription completes the
//! sequence, disposes the cache or list, and releases what `subscribe` returned: a disposable or
//! an action.
//!
//! The overloads of upstream map to the names of the Rx port's `Observable::create` forms:
//! - `Func<ISourceCache, IDisposable>` is `create_cache`, `Func<ISourceCache, Action>` is
//!   `create_cache_with_action`; their `try_` forms take a function that can fail, whose error ends
//!   the sequence (the exception that upstream catches).
//! - `Func<ISourceCache, Task>` is `create_cache_async`, `Func<ISourceCache, Task<IDisposable>>`
//!   is `create_cache_async_with_disposable`, `Func<ISourceCache, Task<Action>>` is
//!   `create_cache_async_with_action`; the `CancellationToken` forms take the suffix `_with_token`
//!   (the token is cancelled when the subscription is disposed). The function returns a future,
//!   which runs as a task (docs/porting/reactive.md, section 5.8); its error ends the sequence.
//! - The same with `list` for `ISourceList<T>`.
//!
//! The function receives the `SourceCache` or `SourceList` itself, a handle to the collection,
//! where upstream passes the interface `ISourceCache<TObject, TKey>` or `ISourceList<T>`.

use crate::cache::{ChangeSet, IConnectableCache, SourceCache};
use crate::list::{IObservableList, SourceList};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::future::Future;
use std::hash::Hash;
use std::rc::Rc;

/// The factories of sequences of change sets (`ObservableChangeSet`).
pub struct ObservableChangeSet;

/// The release of a subscription of an asynchronous form: the action or the disposable that the
/// task of `subscribe` gave, if any.
fn release(resource: Option<Rc<dyn IDisposable>>) {
    if let Some(resource) = resource {
        resource.dispose();
    }
}

impl ObservableChangeSet {
    // The forms of caches.

    /// A sequence of the change sets of a cache that `subscribe` fills; the disposable it returns
    /// is disposed with the subscription
    /// (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, IDisposable>, Func<TObject, TKey>)`).
    pub fn create_cache<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
        subscribe: impl Fn(SourceCache<T, K>) -> Rc<dyn IDisposable> + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        Self::try_create_cache(move |cache| Ok(subscribe(cache)), key_selector)
    }

    /// [`create_cache`](Self::create_cache) with a function that can fail: its error ends the
    /// sequence (the exception that upstream catches and delivers with `OnError`).
    pub fn try_create_cache<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
        subscribe: impl Fn(SourceCache<T, K>) -> Result<Rc<dyn IDisposable>, ObservableError> + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let key_selector: Rc<dyn Fn(&T) -> K> = Rc::new(key_selector);
        Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
            let key_selector = key_selector.clone();
            let cache = SourceCache::new(move |item: &T| key_selector(item));

            let disposable = match subscribe(cache.clone()) {
                Ok(disposable) => Some(disposable),
                Err(error) => {
                    observer.on_error(error);
                    None
                }
            };

            let completion = observer.clone();
            let connection = cache.connect().subscribe_safe(observer);
            Disposable::create(move || {
                release(disposable);
                completion.on_completed();
                connection.dispose();
                cache.dispose();
            })
        })
    }

    /// A sequence of the change sets of a cache that `subscribe` fills; the action it returns runs
    /// when the subscription is disposed
    /// (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, Action>, Func<TObject, TKey>)`).
    pub fn create_cache_with_action<T: Clone + 'static, K: Eq + Hash + Clone + 'static, A: FnOnce() + 'static>(
        subscribe: impl Fn(SourceCache<T, K>) -> A + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        Self::try_create_cache(move |cache| Ok(Disposable::create(subscribe(cache))), key_selector)
    }

    /// [`create_cache_with_action`](Self::create_cache_with_action) with a function that can fail:
    /// its error ends the sequence.
    pub fn try_create_cache_with_action<T: Clone + 'static, K: Eq + Hash + Clone + 'static, A: FnOnce() + 'static>(
        subscribe: impl Fn(SourceCache<T, K>) -> Result<A, ObservableError> + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        Self::try_create_cache(move |cache| subscribe(cache).map(|action| Disposable::create(action)), key_selector)
    }

    /// The asynchronous form whose task gives the disposable released with the subscription
    /// (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, CancellationToken,
    /// Task<IDisposable>>, Func<TObject, TKey>)`): the change sets are delivered from the start,
    /// and an error of the task ends the sequence.
    pub fn create_cache_async_with_disposable_with_token<
        T: Clone + 'static,
        K: Eq + Hash + Clone + 'static,
        Fut: Future<Output = Result<Rc<dyn IDisposable>, ObservableError>> + 'static,
    >(
        subscribe: impl Fn(SourceCache<T, K>, CancellationToken) -> Fut + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let key_selector: Rc<dyn Fn(&T) -> K> = Rc::new(key_selector);
        Observable::create_async_with_disposable_with_token(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>, token: CancellationToken| {
            let key_selector = key_selector.clone();
            let cache = SourceCache::new(move |item: &T| key_selector(item));
            let responder = cache.connect().subscribe_safe(observer.clone());
            let future = subscribe(cache.clone(), token);
            async move {
                let disposable = match future.await {
                    Ok(disposable) => Some(disposable),
                    Err(error) => {
                        observer.on_error(error);
                        None
                    }
                };

                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || {
                    responder.dispose();
                    cache.dispose();
                    release(disposable);
                    observer.on_completed();
                }))
            }
        })
    }

    /// [`create_cache_async_with_disposable_with_token`](Self::create_cache_async_with_disposable_with_token)
    /// without a token (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, Task<IDisposable>>,
    /// Func<TObject, TKey>)`).
    pub fn create_cache_async_with_disposable<
        T: Clone + 'static,
        K: Eq + Hash + Clone + 'static,
        Fut: Future<Output = Result<Rc<dyn IDisposable>, ObservableError>> + 'static,
    >(
        subscribe: impl Fn(SourceCache<T, K>) -> Fut + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        Self::create_cache_async_with_disposable_with_token(move |cache, _token| subscribe(cache), key_selector)
    }

    /// The asynchronous form whose task gives the action run with the release of the subscription
    /// (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, CancellationToken, Task<Action>>,
    /// Func<TObject, TKey>)`): disposing the subscription completes the sequence, then runs the
    /// action.
    pub fn create_cache_async_with_action_with_token<
        T: Clone + 'static,
        K: Eq + Hash + Clone + 'static,
        A: FnOnce() + 'static,
        Fut: Future<Output = Result<A, ObservableError>> + 'static,
    >(
        subscribe: impl Fn(SourceCache<T, K>, CancellationToken) -> Fut + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let key_selector: Rc<dyn Fn(&T) -> K> = Rc::new(key_selector);
        Observable::create_async_with_disposable_with_token(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>, token: CancellationToken| {
            let key_selector = key_selector.clone();
            let cache = SourceCache::new(move |item: &T| key_selector(item));
            let responder = cache.connect().subscribe_safe(observer.clone());
            let future = subscribe(cache.clone(), token);
            async move {
                let dispose_action = match future.await {
                    Ok(action) => Some(action),
                    Err(error) => {
                        observer.on_error(error);
                        None
                    }
                };

                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || {
                    responder.dispose();
                    cache.dispose();
                    observer.on_completed();
                    if let Some(dispose_action) = dispose_action {
                        dispose_action();
                    }
                }))
            }
        })
    }

    /// [`create_cache_async_with_action_with_token`](Self::create_cache_async_with_action_with_token)
    /// without a token (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, Task<Action>>,
    /// Func<TObject, TKey>)`).
    pub fn create_cache_async_with_action<
        T: Clone + 'static,
        K: Eq + Hash + Clone + 'static,
        A: FnOnce() + 'static,
        Fut: Future<Output = Result<A, ObservableError>> + 'static,
    >(
        subscribe: impl Fn(SourceCache<T, K>) -> Fut + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        Self::create_cache_async_with_action_with_token(move |cache, _token| subscribe(cache), key_selector)
    }

    /// The asynchronous form whose task only fills the cache
    /// (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, CancellationToken, Task>,
    /// Func<TObject, TKey>)`): disposing the subscription completes the sequence.
    pub fn create_cache_async_with_token<T: Clone + 'static, K: Eq + Hash + Clone + 'static, Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        subscribe: impl Fn(SourceCache<T, K>, CancellationToken) -> Fut + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let key_selector: Rc<dyn Fn(&T) -> K> = Rc::new(key_selector);
        Observable::create_async_with_disposable_with_token(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>, token: CancellationToken| {
            let key_selector = key_selector.clone();
            let cache = SourceCache::new(move |item: &T| key_selector(item));
            let responder = cache.connect().subscribe_safe(observer.clone());
            let future = subscribe(cache.clone(), token);
            async move {
                if let Err(error) = future.await {
                    observer.on_error(error);
                }

                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || {
                    responder.dispose();
                    cache.dispose();
                    observer.on_completed();
                }))
            }
        })
    }

    /// [`create_cache_async_with_token`](Self::create_cache_async_with_token) without a token
    /// (`Create<TObject, TKey>(Func<ISourceCache<TObject, TKey>, Task>, Func<TObject, TKey>)`).
    pub fn create_cache_async<T: Clone + 'static, K: Eq + Hash + Clone + 'static, Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        subscribe: impl Fn(SourceCache<T, K>) -> Fut + 'static,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        Self::create_cache_async_with_token(move |cache, _token| subscribe(cache), key_selector)
    }

    // The forms of lists.

    /// A sequence of the change sets of a list that `subscribe` fills; the disposable it returns is
    /// disposed with the subscription, after the sequence completes
    /// (`Create<T>(Func<ISourceList<T>, IDisposable>)`).
    pub fn create_list<T: Clone + PartialEq + 'static>(
        subscribe: impl Fn(SourceList<T>) -> Rc<dyn IDisposable> + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Self::try_create_list(move |list| Ok(subscribe(list)))
    }

    /// [`create_list`](Self::create_list) with a function that can fail: its error ends the
    /// sequence.
    pub fn try_create_list<T: Clone + PartialEq + 'static>(
        subscribe: impl Fn(SourceList<T>) -> Result<Rc<dyn IDisposable>, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Observable::create(move |observer: Rc<dyn IObserver<crate::list::ChangeSet<T>>>| {
            let list = SourceList::new();

            let dispose_action = match subscribe(list.clone()) {
                Ok(disposable) => Some(disposable),
                Err(error) => {
                    observer.on_error(error);
                    None
                }
            };

            let completion = observer.clone();
            let connection = list.connect().subscribe_safe(observer);
            Disposable::create(move || {
                connection.dispose();
                list.dispose();
                completion.on_completed();
                release(dispose_action);
            })
        })
    }

    /// A sequence of the change sets of a list that `subscribe` fills; the action it returns runs
    /// when the subscription is disposed (`Create<T>(Func<ISourceList<T>, Action>)`).
    pub fn create_list_with_action<T: Clone + PartialEq + 'static, A: FnOnce() + 'static>(
        subscribe: impl Fn(SourceList<T>) -> A + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Self::try_create_list(move |list| Ok(Disposable::create(subscribe(list))))
    }

    /// [`create_list_with_action`](Self::create_list_with_action) with a function that can fail:
    /// its error ends the sequence.
    pub fn try_create_list_with_action<T: Clone + PartialEq + 'static, A: FnOnce() + 'static>(
        subscribe: impl Fn(SourceList<T>) -> Result<A, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Self::try_create_list(move |list| subscribe(list).map(|action| Disposable::create(action)))
    }

    /// The asynchronous form whose task gives the disposable released with the subscription
    /// (`Create<T>(Func<ISourceList<T>, CancellationToken, Task<IDisposable>>)`).
    pub fn create_list_async_with_disposable_with_token<
        T: Clone + PartialEq + 'static,
        Fut: Future<Output = Result<Rc<dyn IDisposable>, ObservableError>> + 'static,
    >(
        subscribe: impl Fn(SourceList<T>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Observable::create_async_with_disposable_with_token(move |observer: Rc<dyn IObserver<crate::list::ChangeSet<T>>>, token: CancellationToken| {
            let list = SourceList::new();
            let responder = list.connect().subscribe_safe(observer.clone());
            let future = subscribe(list.clone(), token);
            async move {
                let dispose_action = match future.await {
                    Ok(disposable) => Some(disposable),
                    Err(error) => {
                        observer.on_error(error);
                        None
                    }
                };

                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || {
                    responder.dispose();
                    list.dispose();
                    observer.on_completed();
                    release(dispose_action);
                }))
            }
        })
    }

    /// [`create_list_async_with_disposable_with_token`](Self::create_list_async_with_disposable_with_token)
    /// without a token (`Create<T>(Func<ISourceList<T>, Task<IDisposable>>)`).
    pub fn create_list_async_with_disposable<T: Clone + PartialEq + 'static, Fut: Future<Output = Result<Rc<dyn IDisposable>, ObservableError>> + 'static>(
        subscribe: impl Fn(SourceList<T>) -> Fut + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Self::create_list_async_with_disposable_with_token(move |list, _token| subscribe(list))
    }

    /// The asynchronous form whose task gives the action run with the release of the subscription
    /// (`Create<T>(Func<ISourceList<T>, CancellationToken, Task<Action>>)`).
    pub fn create_list_async_with_action_with_token<
        T: Clone + PartialEq + 'static,
        A: FnOnce() + 'static,
        Fut: Future<Output = Result<A, ObservableError>> + 'static,
    >(
        subscribe: impl Fn(SourceList<T>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Observable::create_async_with_disposable_with_token(move |observer: Rc<dyn IObserver<crate::list::ChangeSet<T>>>, token: CancellationToken| {
            let list = SourceList::new();
            let responder = list.connect().subscribe_safe(observer.clone());
            let future = subscribe(list.clone(), token);
            async move {
                let dispose_action = match future.await {
                    Ok(action) => Some(action),
                    Err(error) => {
                        observer.on_error(error);
                        None
                    }
                };

                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || {
                    responder.dispose();
                    list.dispose();
                    observer.on_completed();
                    if let Some(dispose_action) = dispose_action {
                        dispose_action();
                    }
                }))
            }
        })
    }

    /// [`create_list_async_with_action_with_token`](Self::create_list_async_with_action_with_token)
    /// without a token (`Create<T>(Func<ISourceList<T>, Task<Action>>)`).
    pub fn create_list_async_with_action<T: Clone + PartialEq + 'static, A: FnOnce() + 'static, Fut: Future<Output = Result<A, ObservableError>> + 'static>(
        subscribe: impl Fn(SourceList<T>) -> Fut + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Self::create_list_async_with_action_with_token(move |list, _token| subscribe(list))
    }

    /// The asynchronous form whose task only fills the list
    /// (`Create<T>(Func<ISourceList<T>, Task>)`): disposing the subscription completes the
    /// sequence.
    pub fn create_list_async<T: Clone + PartialEq + 'static, Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        subscribe: impl Fn(SourceList<T>) -> Fut + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Observable::create_async_with_disposable(move |observer: Rc<dyn IObserver<crate::list::ChangeSet<T>>>| {
            let list = SourceList::new();
            let responder = list.connect().subscribe_safe(observer.clone());
            let future = subscribe(list.clone());
            async move {
                if let Err(error) = future.await {
                    observer.on_error(error);
                }

                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || {
                    responder.dispose();
                    list.dispose();
                    observer.on_completed();
                }))
            }
        })
    }

    /// The asynchronous form whose task only fills the list, with a token
    /// (`Create<T>(Func<ISourceList<T>, CancellationToken, Task>)`): unlike the other forms, the
    /// change sets are delivered once the task has ended, as upstream subscribes to the list after
    /// awaiting it.
    pub fn create_list_async_with_token<T: Clone + PartialEq + 'static, Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        subscribe: impl Fn(SourceList<T>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        Observable::create_async_with_disposable_with_token(move |observer: Rc<dyn IObserver<crate::list::ChangeSet<T>>>, token: CancellationToken| {
            let list = SourceList::new();
            let future = subscribe(list.clone(), token);
            async move {
                if let Err(error) = future.await {
                    observer.on_error(error);
                }

                let connection = list.connect().subscribe_safe(observer.clone());
                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || {
                    connection.dispose();
                    list.dispose();
                    observer.on_completed();
                }))
            }
        })
    }
}
