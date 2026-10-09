//! Port of `PropertyChangedExtensions.cs`.

use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::reactive::{AnonymousObserver, Disposable, IObservable, IObserver, Observable};
use std::cell::RefCell;
use std::rc::Rc;

/// Observables of the properties of a view model.
///
/// The managed original takes the property as an expression
/// (`x => x.Name`) and reads its name and its getter from it; here the name
/// and the getter are passed.
pub struct PropertyChangedExtensions;

impl PropertyChangedExtensions {
    /// `model.WhenAnyValue(x => x.Property)`: the value of the property now
    /// and after every change notification that names it.
    ///
    /// Deviation (DEVIATIONS.md, ControlCatalog sample): the managed
    /// original captures the model in the observable. A view
    /// model that keeps an observable of its own properties (a property that
    /// is `this.WhenAnyValue(..)`) holds itself that way, and its collector
    /// frees it; here the observable holds the model weakly, and a
    /// subscription to the observable of a model that is gone produces
    /// nothing.
    pub fn when_any_value<TModel, TRes>(
        model: &Rc<TModel>,
        property_name: &'static str,
        getter: impl Fn(&TModel) -> TRes + 'static,
    ) -> Rc<dyn IObservable<TRes>>
    where
        TModel: INotifyPropertyChanged + 'static,
        TRes: 'static,
    {
        let model = Rc::downgrade(model);
        let getter = Rc::new(getter);
        Observable::create(move |observer: Rc<dyn IObserver<TRes>>| {
            let Some(source) = model.upgrade() else { return Disposable::empty() };
            let token = {
                let (target, getter, observer) = (model.clone(), getter.clone(), observer.clone());
                source.property_changed().add(Rc::new(move |name: &str| {
                    if name == property_name {
                        if let Some(target) = target.upgrade() {
                            observer.on_next(getter(&target));
                        }
                    }
                }))
            };
            observer.on_next(getter(&source));
            let (model, observer) = (model.clone(), observer.clone());
            Disposable::create(move || {
                if let Some(model) = model.upgrade() {
                    model.property_changed().remove(token);
                }
                observer.on_completed();
            })
        })
    }

    /// `model.WhenAnyValue(v1, cb)`.
    pub fn when_any_value_select<TModel, T1, TRes>(
        model: &Rc<TModel>,
        v1: (&'static str, impl Fn(&TModel) -> T1 + 'static),
        cb: impl Fn(T1) -> TRes + 'static,
    ) -> Rc<dyn IObservable<TRes>>
    where
        TModel: INotifyPropertyChanged + 'static,
        T1: 'static,
        TRes: 'static,
    {
        let source = Self::when_any_value(model, v1.0, v1.1);
        let cb = Rc::new(cb);
        Observable::create(move |observer: Rc<dyn IObserver<TRes>>| {
            let cb = cb.clone();
            source.subscribe(Rc::new(AnonymousObserver::new(move |value| observer.on_next(cb(value)))))
        })
    }

    /// `model.WhenAnyValue(v1, v2, cb)`.
    pub fn when_any_value2<TModel, T1, T2, TRes>(
        model: &Rc<TModel>,
        v1: (&'static str, impl Fn(&TModel) -> T1 + 'static),
        v2: (&'static str, impl Fn(&TModel) -> T2 + 'static),
        cb: impl Fn(T1, T2) -> TRes + 'static,
    ) -> Rc<dyn IObservable<TRes>>
    where
        TModel: INotifyPropertyChanged + 'static,
        T1: Clone + 'static,
        T2: Clone + 'static,
        TRes: 'static,
    {
        combine_latest(Self::when_any_value(model, v1.0, v1.1), Self::when_any_value(model, v2.0, v2.1), cb)
    }

    /// `model.WhenAnyValue(v1, v2)`: the pair of the values.
    pub fn when_any_value2_tuple<TModel, T1, T2>(
        model: &Rc<TModel>,
        v1: (&'static str, impl Fn(&TModel) -> T1 + 'static),
        v2: (&'static str, impl Fn(&TModel) -> T2 + 'static),
    ) -> Rc<dyn IObservable<(T1, T2)>>
    where
        TModel: INotifyPropertyChanged + 'static,
        T1: Clone + 'static,
        T2: Clone + 'static,
    {
        Self::when_any_value2(model, v1, v2, |a1, a2| (a1, a2))
    }

    /// `model.WhenAnyValue(v1, v2, v3, cb)`.
    pub fn when_any_value3<TModel, T1, T2, T3, TRes>(
        model: &Rc<TModel>,
        v1: (&'static str, impl Fn(&TModel) -> T1 + 'static),
        v2: (&'static str, impl Fn(&TModel) -> T2 + 'static),
        v3: (&'static str, impl Fn(&TModel) -> T3 + 'static),
        cb: impl Fn(T1, T2, T3) -> TRes + 'static,
    ) -> Rc<dyn IObservable<TRes>>
    where
        TModel: INotifyPropertyChanged + 'static,
        T1: Clone + 'static,
        T2: Clone + 'static,
        T3: Clone + 'static,
        TRes: 'static,
    {
        let pair = combine_latest(
            Self::when_any_value(model, v1.0, v1.1),
            Self::when_any_value(model, v2.0, v2.1),
            |l, r| (l, r),
        );
        combine_latest(pair, Self::when_any_value(model, v3.0, v3.1), move |t: (T1, T2), r| cb(t.0, t.1, r))
    }

    /// `model.WhenAnyValue(v1, v2, v3)`: the triple of the values.
    pub fn when_any_value3_tuple<TModel, T1, T2, T3>(
        model: &Rc<TModel>,
        v1: (&'static str, impl Fn(&TModel) -> T1 + 'static),
        v2: (&'static str, impl Fn(&TModel) -> T2 + 'static),
        v3: (&'static str, impl Fn(&TModel) -> T3 + 'static),
    ) -> Rc<dyn IObservable<(T1, T2, T3)>>
    where
        TModel: INotifyPropertyChanged + 'static,
        T1: Clone + 'static,
        T2: Clone + 'static,
        T3: Clone + 'static,
    {
        Self::when_any_value3(model, v1, v2, v3, |a1, a2, a3| (a1, a2, a3))
    }
}

/// `Observable.CombineLatest(first, second, resultSelector)`: a value for
/// every value of either source once both have produced one.
///
/// The managed library compiles the operator from the reactive sources of
/// the base assembly, which the base crate does not have; it is local to
/// this crate.
fn combine_latest<T1, T2, TRes>(
    first: Rc<dyn IObservable<T1>>,
    second: Rc<dyn IObservable<T2>>,
    result_selector: impl Fn(T1, T2) -> TRes + 'static,
) -> Rc<dyn IObservable<TRes>>
where
    T1: Clone + 'static,
    T2: Clone + 'static,
    TRes: 'static,
{
    let result_selector = Rc::new(result_selector);
    Observable::create(move |observer: Rc<dyn IObserver<TRes>>| {
        let latest: Rc<(RefCell<Option<T1>>, RefCell<Option<T2>>)> = Rc::new((RefCell::new(None), RefCell::new(None)));
        let publish = {
            let (latest, result_selector) = (latest.clone(), result_selector.clone());
            Rc::new(move || {
                let values = (latest.0.borrow().clone(), latest.1.borrow().clone());
                if let (Some(a), Some(b)) = values {
                    observer.on_next(result_selector(a, b));
                }
            })
        };
        let first_subscription = {
            let (latest, publish) = (latest.clone(), publish.clone());
            first.subscribe(Rc::new(AnonymousObserver::new(move |value| {
                *latest.0.borrow_mut() = Some(value);
                publish();
            })))
        };
        let second_subscription = {
            let (latest, publish) = (latest.clone(), publish.clone());
            second.subscribe(Rc::new(AnonymousObserver::new(move |value| {
                *latest.1.borrow_mut() = Some(value);
                publish();
            })))
        };
        Disposable::create(move || {
            first_subscription.dispose();
            second_subscription.dispose();
        })
    })
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream library has no tests.
    use super::*;
    use crate::ViewModelBase;
    use ferroui_base::data::model::Event;
    use std::cell::Cell;

    struct Model {
        base: ViewModelBase,
        a: Cell<i32>,
        b: Cell<i32>,
    }

    impl INotifyPropertyChanged for Model {
        fn property_changed(&self) -> &Event<str> {
            self.base.property_changed()
        }
    }

    fn model() -> Rc<Model> {
        Rc::new(Model { base: ViewModelBase::new(), a: Cell::new(1), b: Cell::new(10) })
    }

    #[test]
    fn when_any_value_produces_the_current_value_and_the_changes() {
        let model = model();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        let observable = PropertyChangedExtensions::when_any_value(&model, "A", |m: &Model| m.a.get());
        let subscription = observable.subscribe(Rc::new(AnonymousObserver::new(move |v| sink.borrow_mut().push(v))));

        model.base.raise_and_set_if_changed_cell(&model.a, 2, "A");
        model.base.raise_and_set_if_changed_cell(&model.b, 20, "B");
        assert_eq!(vec![1, 2], *seen.borrow());

        subscription.dispose();
        model.base.raise_and_set_if_changed_cell(&model.a, 3, "A");
        assert_eq!(vec![1, 2], *seen.borrow());
        assert!(!model.property_changed().has_handlers());
    }

    #[test]
    fn when_any_value2_combines_the_latest_values() {
        let model = model();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        let observable = PropertyChangedExtensions::when_any_value2(
            &model,
            ("A", |m: &Model| m.a.get()),
            ("B", |m: &Model| m.b.get()),
            |a, b| a + b,
        );
        let _subscription = observable.subscribe(Rc::new(AnonymousObserver::new(move |v| sink.borrow_mut().push(v))));

        model.base.raise_and_set_if_changed_cell(&model.a, 2, "A");
        model.base.raise_and_set_if_changed_cell(&model.b, 20, "B");
        assert_eq!(vec![11, 12, 22], *seen.borrow());
    }
}
