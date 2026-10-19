//! `Using` with asynchronous factories (`Linq/QueryLanguage.Creation.cs`, region `UsingAsync`):
//! `FromAsync(resourceFactoryAsync).SelectMany(resource => Using(() => resource, resource_ =>
//! FromAsync(ct => observableFactoryAsync(resource_, ct)).Merge()))`, the factories returning
//! futures, which run as tasks (docs/porting/reactive.md, section 5.8).

use crate::linq::merge::merge_observables;
use crate::linq::select_many::select_many;
use crate::linq::Observable;
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::future::Future;
use std::rc::Rc;

impl Observable {
    /// A sequence that, for each subscription, runs the future of `resource_factory_async` as a
    /// task, then the future `observable_factory_async` gives for the resource, and subscribes to
    /// the sequence it gives; the resource is disposed when that subscription ends. Each factory
    /// is given a token that disposing the subscription cancels
    /// (`Using<TResult, TResource>(Func<CancellationToken, Task<TResource>>, Func<TResource,
    /// CancellationToken, Task<IObservable<TResult>>>)`).
    pub fn using_async<T: 'static, R: IDisposable + ?Sized + 'static, FutR, FutO>(
        resource_factory_async: impl Fn(CancellationToken) -> FutR + 'static,
        observable_factory_async: impl Fn(Rc<R>, CancellationToken) -> FutO + 'static,
    ) -> Rc<dyn IObservable<T>>
    where
        FutR: Future<Output = Result<Rc<R>, ObservableError>> + 'static,
        FutO: Future<Output = Result<Rc<dyn IObservable<T>>, ObservableError>> + 'static,
    {
        let observable_factory_async = Rc::new(observable_factory_async);
        let resources = Observable::from_async_with_token(resource_factory_async);
        select_many(resources, move |resource: Rc<R>, _| -> Result<Rc<dyn IObservable<T>>, ObservableError> {
            let observable_factory_async = observable_factory_async.clone();
            Ok(Observable::using(
                move || resource.clone(),
                move |resource_: Rc<R>| {
                    let observable_factory_async = observable_factory_async.clone();
                    merge_observables(
                        Observable::from_async_with_token(move |token| observable_factory_async(resource_.clone(), token)),
                        None,
                    )
                },
            ))
        })
    }
}
