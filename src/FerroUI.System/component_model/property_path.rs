//! Property paths: the counterpart of the expressions `x => x.Name` and `x => x.Address.Street`
//! that upstream libraries take (`Expression<Func<TObject, TValue>>`), without reflection
//! (docs/porting/reactive.md, section 5.5).
//!
//! A [`PropertyPath`] holds a getter of the whole path, the names of its properties, and a way to
//! observe the changes of every step on an object: a change of an intermediate object
//! subscribes to the new one. [`prop!`](crate::prop) writes paths from getters.
//!
//! A path also gives a type-erased view of its steps ([`PropertyPath::steps`], [`PathStep`]): the
//! name of each property, how to read it from the value of the step before, and the object that
//! owns it ([`PropertyOwner`]). That is what an observer chosen at run time (ReactiveUI's
//! `ICreatesObservableForProperty`) works from, where upstream walks the member expressions of
//! the expression tree.

use super::INotifyPropertyChanged;
use crate::IDisposable;
use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Observes the steps of a path on an object; the callback runs when the value of the path may
/// have changed.
type Observe<S> = Rc<dyn Fn(&S, Rc<dyn Fn()>) -> Rc<dyn IDisposable>>;

/// A typed path from an object to a value through its properties (`Expression<Func<S, V>>`).
pub struct PropertyPath<S, V> {
    names: Rc<[String]>,
    get: Rc<dyn Fn(&S) -> V>,
    observe: Observe<S>,
    steps: Rc<[PathStep]>,
}

impl<S, V> Clone for PropertyPath<S, V> {
    fn clone(&self) -> Self {
        Self { names: self.names.clone(), get: self.get.clone(), observe: self.observe.clone(), steps: self.steps.clone() }
    }
}

/// The object that owns the property of one step of a path, as an observer chosen at run time
/// sees it (the `sender` of upstream's `ICreatesObservableForProperty`).
///
/// `object` is the object itself, which an observer downcasts to the types it knows; `notify` is
/// its change notification, when it raises `PropertyChanged`. A step that cannot give its object
/// (a [`PropertyPath::value`] step, or an [`optional_property`](PropertyPath::optional_property)
/// step whose object is `None`) leaves both `None`.
#[derive(Clone, Default)]
pub struct PropertyOwner {
    /// The object that owns the property.
    pub object: Option<Rc<dyn Any>>,
    /// The change notification of the object, when it raises `PropertyChanged`.
    pub notify: Option<Rc<dyn INotifyPropertyChanged>>,
}

impl PropertyOwner {
    /// Whether the two owners are the same object (reference equality, as .NET compares senders).
    pub fn same_object(&self, other: &PropertyOwner) -> bool {
        match (&self.object, &other.object) {
            (Some(first), Some(second)) => Rc::ptr_eq(first, second),
            (None, None) => match (&self.notify, &other.notify) {
                (Some(first), Some(second)) => Rc::ptr_eq(first, second),
                (None, None) => true,
                _ => false,
            },
            _ => false,
        }
    }
}

impl fmt::Debug for PropertyOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyOwner")
            .field("object", &self.object.as_ref().map(|object| Rc::as_ptr(object) as *const ()))
            .field("notify", &self.notify.is_some())
            .finish()
    }
}

/// One step of a path, type-erased (one member access of upstream's expression chain).
///
/// The steps of a path run in order: the first reads from the source of the path, each next one
/// from the value the step before produced. `get` and `owner` take that value as `&dyn Any`; a
/// value of another type is a programming error, and they panic.
#[derive(Clone)]
pub struct PathStep {
    /// The name of the property (`SearchTerm`).
    pub name: String,
    /// The value of the step, from the value of the previous one (or the source of the path).
    pub get: Rc<dyn Fn(&dyn Any) -> Box<dyn Any>>,
    /// The object that owns the property, from the value of the previous step.
    pub owner: Rc<dyn Fn(&dyn Any) -> PropertyOwner>,
}

impl fmt::Debug for PathStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathStep").field("name", &self.name).finish_non_exhaustive()
    }
}

/// The value given to the step `name` as the type it reads from; any other type is a programming
/// error of the caller of the type-erased steps.
fn step_input<'a, T: 'static>(value: &'a dyn Any, name: &str) -> &'a T {
    match value.downcast_ref::<T>() {
        Some(value) => value,
        None => panic!(
            "the step {name} of a property path was given a value of another type than {}",
            std::any::type_name::<T>()
        ),
    }
}

/// The owner of a property of an object that raises `PropertyChanged`.
fn owner_of<M: INotifyPropertyChanged + 'static>(object: &Rc<M>) -> PropertyOwner {
    PropertyOwner { object: Some(object.clone() as Rc<dyn Any>), notify: Some(object.clone() as Rc<dyn INotifyPropertyChanged>) }
}

/// Runs `changed` when `source` raises `PropertyChanged` for `watched`, or for every property (an
/// empty name).
fn observe_property<M: INotifyPropertyChanged + 'static>(source: &Rc<M>, watched: String, changed: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
    let token = source.property_changed().add(Rc::new(move |property: &str| {
        if property.is_empty() || property == watched {
            changed();
        }
    }));
    let source = source.clone();
    on_dispose(move || {
        source.property_changed().remove(token);
    })
}

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

/// The name of a property from the name of its Rust getter: `favorite_color` is `FavoriteColor`,
/// the names that FerroUI's objects raise and its markup binds.
pub fn property_name_of(getter: &str) -> String {
    getter
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

impl<M: INotifyPropertyChanged + 'static, V: 'static> PropertyPath<Rc<M>, V> {
    /// A path of one property of an object that raises `PropertyChanged`: a change of `name`, or
    /// of every property (an empty name), changes the value.
    ///
    /// Its step gives the object as its owner, with its change notification.
    pub fn property(name: impl Into<String>, get: impl Fn(&Rc<M>) -> V + 'static) -> Self {
        let name: String = name.into();
        let watched = name.clone();
        let get: Rc<dyn Fn(&Rc<M>) -> V> = Rc::new(get);
        let step_get = get.clone();
        let step_name = name.clone();
        let owner_name = name.clone();
        let step = PathStep {
            name: name.clone(),
            get: Rc::new(move |source: &dyn Any| Box::new(step_get(step_input::<Rc<M>>(source, &step_name))) as Box<dyn Any>),
            owner: Rc::new(move |source: &dyn Any| owner_of(step_input::<Rc<M>>(source, &owner_name))),
        };
        Self {
            names: Rc::from(vec![name]),
            get,
            observe: Rc::new(move |source: &Rc<M>, changed: Rc<dyn Fn()>| observe_property(source, watched.clone(), changed)),
            steps: Rc::from(vec![step]),
        }
    }
}

impl<M: INotifyPropertyChanged + 'static, V: 'static> PropertyPath<Option<Rc<M>>, Option<V>> {
    /// A path of one property of an object that may be absent (`x => x.Child.Name` where `Child`
    /// may be null): `None` while there is no object, otherwise the property, observed as
    /// [`property`](Self::property) observes it.
    ///
    /// Its step gives the object as its owner when there is one, and an empty owner otherwise.
    pub fn optional_property(name: impl Into<String>, get: impl Fn(&Rc<M>) -> V + 'static) -> Self {
        let name: String = name.into();
        let watched = name.clone();
        let get: Rc<dyn Fn(&Rc<M>) -> V> = Rc::new(get);
        let path_get = get.clone();
        let step_name = name.clone();
        let owner_name = name.clone();
        let step = PathStep {
            name: name.clone(),
            get: Rc::new(move |source: &dyn Any| {
                Box::new(step_input::<Option<Rc<M>>>(source, &step_name).as_ref().map(|object| get(object))) as Box<dyn Any>
            }),
            owner: Rc::new(move |source: &dyn Any| match step_input::<Option<Rc<M>>>(source, &owner_name) {
                Some(object) => owner_of(object),
                None => PropertyOwner::default(),
            }),
        };
        Self {
            names: Rc::from(vec![name]),
            get: Rc::new(move |source: &Option<Rc<M>>| source.as_ref().map(|object| path_get(object))),
            observe: Rc::new(move |source: &Option<Rc<M>>, changed: Rc<dyn Fn()>| match source {
                Some(object) => observe_property(object, watched.clone(), changed),
                None => on_dispose(|| {}),
            }),
            steps: Rc::from(vec![step]),
        }
    }
}

impl<M: INotifyPropertyChanged + 'static, V: 'static> PropertyPath<Option<Rc<M>>, Option<V>> {
    /// [`optional_property`](Self::optional_property) of a property that may itself be absent
    /// (`x => x.A.B.C` where `A` and `B` may be null): the value is `None` while there is no object
    /// or while the property is `None`, so the steps of a path of absent objects do not nest
    /// their `Option`s.
    pub fn optional_property_and_then(name: impl Into<String>, get: impl Fn(&Rc<M>) -> Option<V> + 'static) -> Self {
        let name: String = name.into();
        let watched = name.clone();
        let get: Rc<dyn Fn(&Rc<M>) -> Option<V>> = Rc::new(get);
        let path_get = get.clone();
        let step_name = name.clone();
        let owner_name = name.clone();
        let step = PathStep {
            name: name.clone(),
            get: Rc::new(move |source: &dyn Any| {
                Box::new(step_input::<Option<Rc<M>>>(source, &step_name).as_ref().and_then(|object| get(object))) as Box<dyn Any>
            }),
            owner: Rc::new(move |source: &dyn Any| match step_input::<Option<Rc<M>>>(source, &owner_name) {
                Some(object) => owner_of(object),
                None => PropertyOwner::default(),
            }),
        };
        Self {
            names: Rc::from(vec![name]),
            get: Rc::new(move |source: &Option<Rc<M>>| source.as_ref().and_then(|object| path_get(object))),
            observe: Rc::new(move |source: &Option<Rc<M>>, changed: Rc<dyn Fn()>| match source {
                Some(object) => observe_property(object, watched.clone(), changed),
                None => on_dispose(|| {}),
            }),
            steps: Rc::from(vec![step]),
        }
    }
}

impl<M: 'static, V: 'static> PropertyPath<Rc<M>, V> {
    /// A path of one property of an object that does not raise `PropertyChanged` itself, but whose
    /// changes an observer chosen at run time may see (an object of a user-interface framework,
    /// through its own property system).
    ///
    /// Its step gives the object (the `Rc` of the source) as its owner, without a change
    /// notification, so that the observer with the highest affinity for that object observes the
    /// property. The path itself observes nothing, as [`value`](PropertyPath::value) does.
    pub fn object_property(name: impl Into<String>, get: impl Fn(&Rc<M>) -> V + 'static) -> Self {
        Self::owned_property(
            name,
            get,
            |source: &Rc<M>| PropertyOwner { object: Some(source.clone() as Rc<dyn Any>), notify: None },
            |_: &Rc<M>, _: Rc<dyn Fn()>| on_dispose(|| {}),
        )
    }
}

impl<S: 'static, V: 'static> PropertyPath<S, V> {
    /// A path of one property whose owner and observation the caller gives: `owner` gives the
    /// object that owns the property from the source of the step (what an observer chosen at run
    /// time sees), and `observe` runs its callback when the value may have changed, until the
    /// result is disposed.
    ///
    /// It is how a framework whose objects are not `Rc`s of a type that raises `PropertyChanged`
    /// (handles of the objects of a user interface) writes the steps of its properties.
    pub fn owned_property(
        name: impl Into<String>,
        get: impl Fn(&S) -> V + 'static,
        owner: impl Fn(&S) -> PropertyOwner + 'static,
        observe: impl Fn(&S, Rc<dyn Fn()>) -> Rc<dyn IDisposable> + 'static,
    ) -> Self {
        let name: String = name.into();
        let get: Rc<dyn Fn(&S) -> V> = Rc::new(get);
        let owner: Rc<dyn Fn(&S) -> PropertyOwner> = Rc::new(owner);
        let step_get = get.clone();
        let step_name = name.clone();
        let owner_name = name.clone();
        let step = PathStep {
            name: name.clone(),
            get: Rc::new(move |source: &dyn Any| Box::new(step_get(step_input::<S>(source, &step_name))) as Box<dyn Any>),
            owner: Rc::new(move |source: &dyn Any| owner(step_input::<S>(source, &owner_name))),
        };
        Self { names: Rc::from(vec![name]), get, observe: Rc::new(observe), steps: Rc::from(vec![step]) }
    }

    /// A path of one value that does not notify its changes (a plain field, or an object that is
    /// not `INotifyPropertyChanged`): it is read again only when an earlier step changes.
    ///
    /// Its step cannot give the object that owns the value (the source is not cloned), so its
    /// owner is empty: an observer chosen at run time sees a value that does not notify.
    pub fn value(name: impl Into<String>, get: impl Fn(&S) -> V + 'static) -> Self {
        let name: String = name.into();
        let get: Rc<dyn Fn(&S) -> V> = Rc::new(get);
        let step_get = get.clone();
        let step_name = name.clone();
        let step = PathStep {
            name: name.clone(),
            get: Rc::new(move |source: &dyn Any| Box::new(step_get(step_input::<S>(source, &step_name))) as Box<dyn Any>),
            owner: Rc::new(|_: &dyn Any| PropertyOwner::default()),
        };
        Self {
            names: Rc::from(vec![name]),
            get,
            observe: Rc::new(|_: &S, _: Rc<dyn Fn()>| on_dispose(|| {})),
            steps: Rc::from(vec![step]),
        }
    }

    /// The steps of the path, type-erased, in order (the member accesses of the expression).
    pub fn steps(&self) -> &[PathStep] {
        &self.steps
    }

    /// The names of the properties along the path (the member names of the expression).
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// The path as text, `Address.Street`.
    pub fn path(&self) -> String {
        self.names.join(".")
    }

    /// The value of the path on `source` (the compiled expression).
    pub fn get(&self, source: &S) -> V {
        (self.get)(source)
    }

    /// The getter of the path.
    pub fn getter(&self) -> Rc<dyn Fn(&S) -> V> {
        self.get.clone()
    }

    /// Runs `changed` whenever the value of the path on `source` may have changed, until the
    /// result is disposed.
    pub fn observe(&self, source: &S, changed: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        (self.observe)(source, changed)
    }

    /// The path that continues with `next` from the value of this one (`x => x.A.B`): when the
    /// object of this step changes, the steps of `next` are observed on the new object.
    pub fn then<W: 'static>(self, next: PropertyPath<V, W>) -> PropertyPath<S, W>
    where
        S: Clone,
        V: Clone,
    {
        let names: Vec<String> = self.names.iter().chain(next.names.iter()).cloned().collect();
        let steps: Vec<PathStep> = self.steps.iter().chain(next.steps.iter()).cloned().collect();
        let (first_get, next_get) = (self.get.clone(), next.get.clone());
        let (first, second) = (self, next);
        PropertyPath {
            names: Rc::from(names),
            steps: Rc::from(steps),
            get: Rc::new(move |source: &S| next_get(&first_get(source))),
            observe: Rc::new(move |source: &S, changed: Rc<dyn Fn()>| {
                let inner: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(Some(second.observe(&first.get(source), changed.clone()))));
                let (source_for_change, inner_for_change, second_for_change, first_get) = (source.clone(), inner.clone(), second.clone(), first.get.clone());
                let outer = first.observe(
                    source,
                    Rc::new(move || {
                        let next = second_for_change.observe(&first_get(&source_for_change), changed.clone());
                        let previous = inner_for_change.borrow_mut().replace(next);
                        if let Some(previous) = previous {
                            previous.dispose();
                        }
                        changed();
                    }),
                );
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
}

impl<S: Clone + 'static, M: INotifyPropertyChanged + 'static> PropertyPath<S, Rc<M>> {
    /// [`then`](PropertyPath::then) with one property of the object of this path
    /// ([`PropertyPath::property`]); the object's type comes from this path.
    pub fn then_property<W: 'static>(self, name: impl Into<String>, get: impl Fn(&Rc<M>) -> W + 'static) -> PropertyPath<S, W> {
        self.then(PropertyPath::property(name, get))
    }
}

impl<S: Clone + 'static, M: INotifyPropertyChanged + 'static> PropertyPath<S, Option<Rc<M>>> {
    /// [`then`](PropertyPath::then) with one property of the object of this path, which may be
    /// absent ([`PropertyPath::optional_property`]): the value is `None` while there is no object.
    pub fn then_optional_property<W: 'static>(self, name: impl Into<String>, get: impl Fn(&Rc<M>) -> W + 'static) -> PropertyPath<S, Option<W>> {
        self.then(PropertyPath::optional_property(name, get))
    }
}

impl<S: Clone + 'static, M: INotifyPropertyChanged + 'static> PropertyPath<S, Option<Rc<M>>> {
    /// [`then`](PropertyPath::then) with a property of the object of this path that may itself be
    /// absent ([`PropertyPath::optional_property_and_then`]): `x => x.A.B.C` where each object may
    /// be null is `prop!(X => a).then_optional_property_and_then("B", |a| a.b()).then_optional_property("C", |b| b.c())`.
    pub fn then_optional_property_and_then<W: 'static>(self, name: impl Into<String>, get: impl Fn(&Rc<M>) -> Option<W> + 'static) -> PropertyPath<S, Option<W>> {
        self.then(PropertyPath::optional_property_and_then(name, get))
    }
}

/// A [`PropertyPath`] from getters (`x => x.Name`, `x => x.Address.Street`).
///
/// ```
/// use ferroui_system::component_model::INotifyPropertyChanged;
/// use ferroui_system::events::Event;
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// struct Person { age: Cell<i32>, changed: Event<str> }
/// impl Person {
///     fn age(&self) -> i32 { self.age.get() }
/// }
/// impl INotifyPropertyChanged for Person {
///     fn property_changed(&self) -> &Event<str> { &self.changed }
/// }
///
/// let path = ferroui_system::prop!(Person => age);
/// let person = Rc::new(Person { age: Cell::new(3), changed: Event::new() });
/// assert_eq!(path.get(&person), 3);
/// assert_eq!(path.path(), "Age");
/// ```
///
/// Each step is a getter method of an object that raises `PropertyChanged`; its property name is
/// the getter's in PascalCase (`favorite_color` is `FavoriteColor`), or the one given with
/// `as "Name"`. The steps after the first are objects held in `Rc`.
///
/// `prop!(object Type => getter)` is the path of one property of an object that does not raise
/// `PropertyChanged` itself ([`PropertyPath::object_property`]): its step gives the object as its
/// owner, for an observer chosen at run time.
#[macro_export]
macro_rules! prop {
    (object $type_:ty => $first:ident $(as $name:literal)?) => {
        $crate::component_model::PropertyPath::<::std::rc::Rc<$type_>, _>::object_property(
            $crate::prop!(@name $first $($name)?),
            |source: &::std::rc::Rc<$type_>| source.$first(),
        )
    };
    ($type_:ty => $first:ident $(as $name:literal)? $(. $rest:ident $(as $rest_name:literal)?)*) => {
        $crate::component_model::PropertyPath::<::std::rc::Rc<$type_>, _>::property(
            $crate::prop!(@name $first $($name)?),
            |source: &::std::rc::Rc<$type_>| source.$first(),
        )
        $(.then_property(
            $crate::prop!(@name $rest $($rest_name)?),
            |source| source.$rest(),
        ))*
    };
    (@name $getter:ident $name:literal) => { $name };
    (@name $getter:ident) => { $crate::component_model::property_name_of(::std::stringify!($getter)) };
}
