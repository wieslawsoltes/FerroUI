//! Property paths: the counterpart of the expressions `x => x.Name` and `x => x.Address.Street`
//! that upstream libraries take (`Expression<Func<TObject, TValue>>`), without reflection
//! (docs/porting/reactive.md, section 5.5).
//!
//! A [`PropertyPath`] holds a getter of the whole path, the names of its properties, and a way to
//! observe the changes of every step on an object: a change of an intermediate object
//! subscribes to the new one. [`prop!`](crate::prop) writes paths from getters.

use super::INotifyPropertyChanged;
use crate::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// Observes the steps of a path on an object; the callback runs when the value of the path may
/// have changed.
type Observe<S> = Rc<dyn Fn(&S, Rc<dyn Fn()>) -> Rc<dyn IDisposable>>;

/// A typed path from an object to a value through its properties (`Expression<Func<S, V>>`).
pub struct PropertyPath<S, V> {
    names: Rc<[String]>,
    get: Rc<dyn Fn(&S) -> V>,
    observe: Observe<S>,
}

impl<S, V> Clone for PropertyPath<S, V> {
    fn clone(&self) -> Self {
        Self { names: self.names.clone(), get: self.get.clone(), observe: self.observe.clone() }
    }
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
    pub fn property(name: impl Into<String>, get: impl Fn(&Rc<M>) -> V + 'static) -> Self {
        let name: String = name.into();
        let watched = name.clone();
        Self {
            names: Rc::from(vec![name]),
            get: Rc::new(get),
            observe: Rc::new(move |source: &Rc<M>, changed: Rc<dyn Fn()>| {
                let watched = watched.clone();
                let token = source.property_changed().add(Rc::new(move |property: &str| {
                    if property.is_empty() || property == watched {
                        changed();
                    }
                }));
                let source = source.clone();
                on_dispose(move || {
                    source.property_changed().remove(token);
                })
            }),
        }
    }
}

impl<S: 'static, V: 'static> PropertyPath<S, V> {
    /// A path of one value that does not notify its changes (a plain field, or an object that is
    /// not `INotifyPropertyChanged`): it is read again only when an earlier step changes.
    pub fn value(name: impl Into<String>, get: impl Fn(&S) -> V + 'static) -> Self {
        Self {
            names: Rc::from(vec![name.into()]),
            get: Rc::new(get),
            observe: Rc::new(|_: &S, _: Rc<dyn Fn()>| on_dispose(|| {})),
        }
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
        let (first_get, next_get) = (self.get.clone(), next.get.clone());
        let (first, second) = (self, next);
        PropertyPath {
            names: Rc::from(names),
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
#[macro_export]
macro_rules! prop {
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
