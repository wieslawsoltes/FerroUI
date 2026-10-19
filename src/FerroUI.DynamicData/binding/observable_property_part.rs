//! `ObservablePropertyPart` (`Binding/ObservablePropertyPart.cs`) and the parsing of expressions
//! into their steps (`Binding/ExpressionBuilder.cs`): [`PropertyChain`], a path whose steps may be
//! null.
//!
//! Upstream splits an expression `x => x.Child!.Age` into its steps by reflection, each with an
//! invoker that reads it and a factory of the notifications of its property, and walks the steps
//! from the root: a null step leaves the value "unobtainable". A [`PropertyPath`] of
//! `ferroui-system` observes every step of a path whose objects are never null; a
//! [`PropertyChain`] adds the steps that may be null, as [`PropertyChain::nullable`] (an
//! `Option<..>` property), and the conversions of a step ([`PropertyChain::map`], upstream's
//! `Convert` nodes).

use ferroui_system::component_model::PropertyPath;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// Observes the steps of a chain on an object; the callback runs when the value may have changed.
type Observe<S> = Rc<dyn Fn(&S, Rc<dyn Fn()>) -> Rc<dyn IDisposable>>;

/// A subscription made of a closure that ends it.
struct OnDispose(RefCell<Option<Box<dyn FnOnce()>>>);

impl IDisposable for OnDispose {
    fn dispose(&self) {
        let end = self.0.borrow_mut().take();
        if let Some(end) = end {
            end();
        }
    }
}

fn on_dispose(end: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
    Rc::new(OnDispose(RefCell::new(Some(Box::new(end)))))
}

/// A path from an object to a value through properties, any step of which may be null (the
/// expression of `WhenPropertyChanged` and `WhenValueChanged`, split into its
/// `ObservablePropertyPart`s).
///
/// Its value is `None` when a step is null (upstream's `UnobtainableValue`). Each step that
/// notifies its changes is observed on the object it is read from; when a step changes, the
/// steps after it are observed again on the new objects, and none is observed past a null step.
///
/// ```
/// use ferroui_dynamic_data::binding::PropertyChain;
/// use ferroui_system::component_model::{INotifyPropertyChanged, PropertyPath};
/// use ferroui_system::events::Event;
/// use std::cell::{Cell, RefCell};
/// use std::rc::Rc;
///
/// struct Child { age: Cell<i32>, changed: Event<str> }
/// impl Child { fn age(&self) -> i32 { self.age.get() } }
/// impl INotifyPropertyChanged for Child { fn property_changed(&self) -> &Event<str> { &self.changed } }
///
/// struct Parent { child: RefCell<Option<Rc<Child>>>, changed: Event<str> }
/// impl Parent { fn child(&self) -> Option<Rc<Child>> { self.child.borrow().clone() } }
/// impl INotifyPropertyChanged for Parent { fn property_changed(&self) -> &Event<str> { &self.changed } }
///
/// // x => x.Child!.Age
/// let chain = PropertyChain::nullable(ferroui_system::prop!(Parent => child)).then(ferroui_system::prop!(Child => age));
/// let parent = Rc::new(Parent { child: RefCell::new(None), changed: Event::new() });
/// assert_eq!(chain.get(&parent), None);
/// *parent.child.borrow_mut() = Some(Rc::new(Child { age: Cell::new(3), changed: Event::new() }));
/// assert_eq!(chain.get(&parent), Some(3));
/// assert_eq!(chain.path(), "Child.Age");
/// ```
pub struct PropertyChain<S, V> {
    names: Rc<[String]>,
    get: Rc<dyn Fn(&S) -> Option<V>>,
    observe: Observe<S>,
}

impl<S, V> Clone for PropertyChain<S, V> {
    fn clone(&self) -> Self {
        Self { names: self.names.clone(), get: self.get.clone(), observe: self.observe.clone() }
    }
}

impl<S: 'static, V: 'static> PropertyChain<S, V> {
    /// The chain of a path whose steps are never null.
    pub fn new(path: PropertyPath<S, V>) -> Self {
        let names: Rc<[String]> = path.names().iter().cloned().collect();
        let getter = path.getter();
        Self { names, get: Rc::new(move |source: &S| Some(getter(source))), observe: Rc::new(move |source: &S, changed: Rc<dyn Fn()>| path.observe(source, changed)) }
    }

    /// The chain of a path whose last step may be null (`x => x.Child!`): its value is `None`
    /// when the step is.
    pub fn nullable(path: PropertyPath<S, Option<V>>) -> Self {
        let names: Rc<[String]> = path.names().iter().cloned().collect();
        let getter = path.getter();
        Self { names, get: getter, observe: Rc::new(move |source: &S, changed: Rc<dyn Fn()>| path.observe(source, changed)) }
    }

    /// The names of the properties along the chain.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// The chain as text, `Child.Age`.
    pub fn path(&self) -> String {
        self.names.join(".")
    }

    /// The value of the chain on `source`; `None` when a step is null (the walk of
    /// `ReadCurrent`).
    pub fn get(&self, source: &S) -> Option<V> {
        (self.get)(source)
    }

    /// The getter of the chain.
    pub fn getter(&self) -> Rc<dyn Fn(&S) -> Option<V>> {
        self.get.clone()
    }

    /// Runs `changed` whenever the value of the chain on `source` may have changed, until the
    /// result is disposed (the subscriptions of the levels of `DeepChainSubscription`).
    pub fn observe(&self, source: &S, changed: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        (self.observe)(source, changed)
    }

    /// The chain that continues with `next` from the value of this one (`x => x.A.B`): when the
    /// object of this step changes, the steps of `next` are observed on the new object, and on
    /// none when it is null.
    pub fn then<P: IntoPropertyChain<V>>(self, next: P) -> PropertyChain<S, P::Value>
    where
        S: Clone,
        V: Clone,
        P::Value: 'static,
    {
        let next = next.into_property_chain();
        let names: Rc<[String]> = self.names.iter().chain(next.names.iter()).cloned().collect();
        let (first_get, next_get) = (self.get.clone(), next.get.clone());
        let (first, second) = (self, next);
        PropertyChain {
            names,
            get: Rc::new(move |source: &S| first_get(source).and_then(|value| next_get(&value))),
            observe: Rc::new(move |source: &S, changed: Rc<dyn Fn()>| {
                let inner: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
                let (source_for_change, inner_for_change, second_for_change, first_get) = (source.clone(), inner.clone(), second.clone(), first.get.clone());
                let changed_for_change = changed.clone();
                // The levels are subscribed from the root to the leaf (`ResubscribeFrom`).
                let outer = first.observe(
                    source,
                    Rc::new(move || {
                        let changed = changed_for_change.clone();
                        // The steps after a null one are not observed (`ResubscribeFrom`).
                        let next = first_get(&source_for_change).map(|value| second_for_change.observe(&value, changed.clone()));
                        let previous = std::mem::replace(&mut *inner_for_change.borrow_mut(), next);
                        if let Some(previous) = previous {
                            previous.dispose();
                        }
                        changed();
                    }),
                );
                let initial = first.get(source).map(|value| second.observe(&value, changed.clone()));
                let replaced = std::mem::replace(&mut *inner.borrow_mut(), initial);
                if let Some(replaced) = replaced {
                    replaced.dispose();
                }
                on_dispose(move || {
                    outer.dispose();
                    let inner = inner.borrow_mut().take();
                    if let Some(inner) = inner {
                        inner.dispose();
                    }
                })
            }),
        }
    }

    /// [`then`](Self::then) with a step that may be null (`x => x.A!.B`).
    pub fn then_nullable<W: 'static>(self, next: PropertyPath<V, Option<W>>) -> PropertyChain<S, W>
    where
        S: Clone,
        V: Clone,
    {
        self.then(PropertyChain::nullable(next))
    }

    /// The chain whose value is converted by `convert` (an expression's `Convert` node, such as
    /// `x => (decimal)x.Amount` or `x => ((IHasAge)x.Child).Age`); the steps observed are the
    /// same.
    pub fn map<W: 'static>(self, convert: impl Fn(V) -> W + 'static) -> PropertyChain<S, W> {
        let get = self.get.clone();
        PropertyChain { names: self.names.clone(), get: Rc::new(move |source: &S| get(source).map(&convert)), observe: self.observe.clone() }
    }
}

/// What the operators that observe properties take: a [`PropertyPath`] (from `prop!`), or a
/// [`PropertyChain`] with steps that may be null.
pub trait IntoPropertyChain<S> {
    /// The type of the value at the end of the path.
    type Value;

    /// The path as a chain.
    fn into_property_chain(self) -> PropertyChain<S, Self::Value>;
}

impl<S: 'static, V: 'static> IntoPropertyChain<S> for PropertyPath<S, V> {
    type Value = V;

    fn into_property_chain(self) -> PropertyChain<S, V> {
        PropertyChain::new(self)
    }
}

impl<S, V> IntoPropertyChain<S> for PropertyChain<S, V> {
    type Value = V;

    fn into_property_chain(self) -> PropertyChain<S, V> {
        self
    }
}
