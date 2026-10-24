//! The element tree: a description of controls, their properties, handlers
//! and children, and how it becomes controls ([`Element::mount`]).
//!
//! An element is data: the class to create, a list of property values (the
//! property and its value, compared by value), handlers, children and a key.
//! Mounting creates the controls (`mount.rs`). What changes afterwards
//! changes through the reactive values the element holds (a property set from
//! a signal or a closure, a list of children that follows a signal), or by a
//! component that runs again ([`rerender`]): the reconciler compares the tree
//! it returns with the one before and patches the controls.

use crate::reactive::Owner;
use ferroui_base::data::BindingBase;
use ferroui_base::{BoxedValue, FerroProperty, Ref};
use ferroui_controls::Control;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

/// The key of an element among its siblings: an element of a dynamic list
/// keeps its control while its key stays.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    /// A number.
    Int(i64),
    /// A string.
    Str(Rc<str>),
}

impl From<i64> for Key {
    fn from(value: i64) -> Self {
        Key::Int(value)
    }
}

impl From<i32> for Key {
    fn from(value: i32) -> Self {
        Key::Int(i64::from(value))
    }
}

impl From<u32> for Key {
    fn from(value: u32) -> Self {
        Key::Int(i64::from(value))
    }
}

impl From<usize> for Key {
    fn from(value: usize) -> Self {
        Key::Int(value as i64)
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::Str(Rc::from(value))
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::Str(Rc::from(value))
    }
}

/// The value an element gives a property.
#[derive(Clone)]
pub enum PropValue {
    /// A value, set once. Values compare by value.
    Value(BoxedValue),
    /// A value computed by a reactive closure: the property is set again
    /// whenever what the closure read changes.
    Dynamic(Rc<dyn Fn() -> BoxedValue>),
    /// A value that follows a signal or a memo: as `Dynamic`, and equal to
    /// another that follows the same source, so that a component that runs
    /// again does not start it again.
    Follows {
        /// The identity of the signal or memo.
        source: u64,
        /// The value of the property from the source.
        compute: Rc<dyn Fn() -> BoxedValue>,
    },
    /// A value that goes both ways: the property follows `get`, and a change
    /// of the property is given to `set`.
    TwoWay {
        /// The value of the source, read reactively.
        get: Rc<dyn Fn() -> BoxedValue>,
        /// Takes a new value of the property.
        set: Rc<dyn Fn(&dyn Any)>,
        /// The identity of the signal, when it is one: two-way values of the
        /// same signal are equal.
        source: Option<u64>,
    },
    /// An element, mounted with its parent: the property holds its control
    /// (`Content`, `Tag`, a header).
    Element(Rc<RefCell<Option<Element>>>),
    /// A binding of the property system (`Binding`, a path on the data
    /// context, a compiled binding).
    Binding(Rc<dyn BindingBase>),
    /// An element, mounted with its parent, for a property that holds a
    /// control of a class (`context_menu`): `wrap` makes the value of the
    /// property from the control.
    TypedElement {
        /// The element.
        element: Rc<RefCell<Option<Element>>>,
        /// The value of the property for the control (`None` when it is not
        /// of the class of the property).
        wrap: fn(Option<Ref<Control>>) -> BoxedValue,
    },
}

impl PartialEq for PropValue {
    /// Values compare by value; anything else is never equal (a reconciler
    /// sets it again).
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PropValue::Value(a), PropValue::Value(b)) => a == b,
            (PropValue::Follows { source: a, .. }, PropValue::Follows { source: b, .. }) => a == b,
            (PropValue::TwoWay { source: Some(a), .. }, PropValue::TwoWay { source: Some(b), .. }) => a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for PropValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropValue::Value(_) => f.write_str("Value"),
            PropValue::Dynamic(_) => f.write_str("Dynamic"),
            PropValue::Follows { source, .. } => write!(f, "Follows({source})"),
            PropValue::TwoWay { .. } => f.write_str("TwoWay"),
            PropValue::Element(_) => f.write_str("Element"),
            PropValue::Binding(_) => f.write_str("Binding"),
            PropValue::TypedElement { .. } => f.write_str("TypedElement"),
        }
    }
}

/// A property and the value an element gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct PropEntry {
    /// The property.
    pub property: &'static FerroProperty,
    /// Its value.
    pub value: PropValue,
}

/// What attaches a handler (a routed event, a change of a property) to the
/// control of an element.
pub type Attach = Rc<dyn Fn(&Ref<Control>)>;

/// A handler of an element. Its closure is in a cell that the attached
/// trampoline reads, so that a component that runs again with handlers of
/// the same identities in the same order swaps the closures into the cells
/// of the attached handlers instead of detaching and attaching them.
#[derive(Clone)]
pub struct Handler {
    /// What it handles: the address of the routed event or of the property;
    /// 0 for a handler that is attached again on every run.
    pub identity: usize,
    /// Attaches the trampoline of `cell` to a control.
    pub attach: Attach,
    /// The closure: a `RefCell<Rc<dyn Fn(..)>>` of the type of the handler.
    pub cell: Rc<dyn Any>,
    /// Puts the closure of the second cell into the first (cells of this
    /// handler's type).
    pub swap: fn(&dyn Any, &dyn Any),
}

impl Handler {
    /// A handler that is attached again on every run (no cell to swap).
    pub fn attached_again(attach: Attach) -> Self {
        fn no_swap(_: &dyn Any, _: &dyn Any) {}
        Handler { identity: 0, attach, cell: Rc::new(()), swap: no_swap }
    }
}

/// The class of an element: how its control is made.
#[derive(Clone, Copy)]
pub struct ElementClass {
    /// The name of the class, for messages.
    pub name: &'static str,
    /// The identity of the class.
    pub type_id: fn() -> std::any::TypeId,
    /// Creates a control of the class.
    pub create: fn() -> Ref<Control>,
}

impl PartialEq for ElementClass {
    fn eq(&self, other: &Self) -> bool {
        (self.type_id)() == (other.type_id)()
    }
}

impl fmt::Debug for ElementClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// An element of a control class.
#[derive(Clone)]
pub struct ControlElement {
    /// The class of the control.
    pub class: ElementClass,
    /// The key among its siblings.
    pub key: Option<Key>,
    /// `Name` of the control.
    pub name: Option<String>,
    /// Style classes of the control.
    pub classes: Vec<String>,
    /// Values of properties, in the order they were given.
    pub props: Vec<PropEntry>,
    /// Handlers.
    pub handlers: Vec<Handler>,
    /// Children, placed into the content of the control (see
    /// [`Element::mount`]).
    pub children: Vec<Element>,
    /// Called with the control once it was made and given its values.
    pub on_mount: Vec<Attach>,
}

impl fmt::Debug for ControlElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlElement")
            .field("class", &self.class)
            .field("key", &self.key)
            .field("name", &self.name)
            .field("classes", &self.classes)
            .field("props", &self.props)
            .field("children", &self.children)
            .finish()
    }
}

impl ControlElement {
    /// An element of `class` with nothing set.
    pub fn new(class: ElementClass) -> Self {
        Self {
            class,
            key: None,
            name: None,
            classes: Vec::new(),
            props: Vec::new(),
            handlers: Vec::new(),
            children: Vec::new(),
            on_mount: Vec::new(),
        }
    }
}

/// A list of children that follows reactive state: the closure is run in an
/// effect, and its elements replace the ones it gave before; elements with a
/// key that stays keep their control (see [`for_each`](crate::for_each)).
#[derive(Clone)]
pub struct DynamicChildren {
    pub(crate) render: Rc<dyn Fn() -> Vec<(Option<Key>, Option<Rc<dyn AnyValueEq>>, Rc<dyn Fn() -> Element>)>>,
}

/// A value an item of a dynamic list is compared by, to decide whether its
/// element is made again.
pub trait AnyValueEq: Any {
    /// Whether `other` holds an equal value.
    fn item_equals(&self, other: &dyn AnyValueEq) -> bool;
    /// The value as `Any`.
    fn item_as_any(&self) -> &dyn Any;
}

impl<T: PartialEq + 'static> AnyValueEq for T {
    fn item_equals(&self, other: &dyn AnyValueEq) -> bool {
        other.item_as_any().downcast_ref::<T>().is_some_and(|other| other == self)
    }

    fn item_as_any(&self) -> &dyn Any {
        self
    }
}

/// A component that runs again: its function runs when it is mounted, again
/// whenever a signal it read changes or its parent runs again, and what it
/// returns is patched into the controls it made before (see [`rerender`] and
/// the [hooks](crate::hooks)).
#[derive(Clone)]
pub struct ComponentElement {
    /// The name of the component, for messages.
    pub name: &'static str,
    /// The identity of the component: a component of another identity at
    /// the same place is mounted anew.
    pub type_id: TypeId,
    /// The key among its siblings.
    pub key: Option<Key>,
    /// Runs the component with what its parent gave it.
    pub render: Rc<dyn Fn() -> Element>,
    /// What its parent gave it, when the component is memoized: a parent
    /// that runs again with equal values does not run it again.
    pub props: Option<Rc<dyn AnyValueEq>>,
}

/// An element of a component that runs again: `render` runs when the
/// element is mounted and again whenever a signal it read changes or the
/// component that holds the element runs again; the elements it returns are
/// compared with those of the run before and the controls are patched
/// (`docs/declarative-ui.md`, section 8). State that outlives one run is kept
/// in [hooks](crate::hooks).
///
/// ```ignore
/// fn counter() -> Element {
///     rerender(|| {
///         let count = use_signal(|| 0);
///         button()
///             .content(format!("Clicked {} times", count.get()))
///             .on_click(move |_| count.update(|c| *c += 1))
///             .into()
///     })
/// }
/// ```
///
/// Two elements of `rerender` are the same component when their closures are
/// of the same type: the same place in the code.
pub fn rerender<F, E>(render: F) -> Element
where
    F: Fn() -> E + 'static,
    E: IntoElement,
{
    Element::Component(ComponentElement {
        name: std::any::type_name::<F>(),
        type_id: TypeId::of::<F>(),
        key: None,
        render: Rc::new(move || render().into_element()),
        props: None,
    })
}

/// What `#[component(rerender)]` uses to memoize a component whose
/// parameters can be compared: `(&MemoProbe(parameters)).memo_props()` is
/// the parameters when they are `PartialEq + Clone` ([`MemoWhenComparable`])
/// and `None` otherwise ([`MemoNever`]), chosen when the expansion is
/// compiled, where the types of the parameters are known.
#[doc(hidden)]
pub struct MemoProbe<T>(pub T);

#[doc(hidden)]
pub trait MemoWhenComparable {
    fn memo_props(&self) -> Option<Rc<dyn AnyValueEq>>;
}

impl<T: PartialEq + Clone + 'static> MemoWhenComparable for MemoProbe<T> {
    fn memo_props(&self) -> Option<Rc<dyn AnyValueEq>> {
        Some(Rc::new(self.0.clone()))
    }
}

#[doc(hidden)]
pub trait MemoNever {
    fn memo_props(&self) -> Option<Rc<dyn AnyValueEq>>;
}

impl<T> MemoNever for &MemoProbe<T> {
    fn memo_props(&self) -> Option<Rc<dyn AnyValueEq>> {
        None
    }
}

/// An element of the component `C` (the type `#[component(rerender)]`
/// declares), for the expansion of the macro.
#[doc(hidden)]
pub fn component_element<C: 'static>(
    name: &'static str,
    key: Option<Key>,
    render: impl Fn() -> Element + 'static,
    props: Option<Rc<dyn AnyValueEq>>,
) -> Element {
    Element::Component(ComponentElement { name, type_id: TypeId::of::<C>(), key, render: Rc::new(render), props })
}

/// A description of user interface: a control with its properties and
/// children, text, several elements, a list that follows state, or a control
/// that exists already.
#[derive(Clone, Default)]
pub enum Element {
    /// Nothing.
    #[default]
    Empty,
    /// A control.
    Control(Box<ControlElement>),
    /// Text: the text of the content of its parent, or a `TextBlock` where a
    /// control is needed.
    Text(Rc<str>),
    /// Several elements in place of one.
    Fragment(Vec<Element>),
    /// The elements of an iteration (`for` in `rsx!`, a `Vec`): a fragment
    /// whose items are checked for keys in debug builds.
    List(Vec<Element>),
    /// Children that follow reactive state.
    Dynamic(DynamicChildren),
    /// A control made elsewhere (a XAML user control, a control kept by the
    /// application).
    Existing(Ref<Control>),
    /// A component that runs again (see [`rerender`]).
    Component(ComponentElement),
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Element::Empty => f.write_str("Empty"),
            Element::Control(control) => control.fmt(f),
            Element::Text(text) => write!(f, "Text({text:?})"),
            Element::Fragment(items) => f.debug_tuple("Fragment").field(items).finish(),
            Element::List(items) => f.debug_tuple("List").field(items).finish(),
            Element::Dynamic(_) => f.write_str("Dynamic"),
            Element::Existing(_) => f.write_str("Existing"),
            Element::Component(component) => write!(f, "Component({})", component.name),
        }
    }
}

/// What turns into an element: an element, a builder of one, a control.
pub trait IntoElement {
    /// The element.
    fn into_element(self) -> Element;
}

impl IntoElement for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl<T: ferroui_base::ObjectType + ferroui_base::Upcast<Control>> IntoElement for Ref<T> {
    fn into_element(self) -> Element {
        Element::Existing(self.upcast())
    }
}

impl<T: IntoElement> IntoElement for Option<T> {
    fn into_element(self) -> Element {
        self.map_or(Element::Empty, IntoElement::into_element)
    }
}

impl<T: IntoElement> IntoElement for Vec<T> {
    fn into_element(self) -> Element {
        Element::List(self.into_iter().map(IntoElement::into_element).collect())
    }
}

/// What can be a child of an element: an element, a builder, text, a list of
/// them.
pub trait IntoChild {
    /// The child as an element.
    fn into_child(self) -> Element;
}

impl<T: IntoElement> IntoChild for T {
    fn into_child(self) -> Element {
        self.into_element()
    }
}

impl IntoChild for &str {
    fn into_child(self) -> Element {
        Element::Text(Rc::from(self))
    }
}

impl IntoChild for String {
    fn into_child(self) -> Element {
        Element::Text(Rc::from(self))
    }
}

/// What can be several children: a vector, an iterator collected, a tuple.
pub trait IntoChildren {
    /// The children.
    fn into_children(self) -> Vec<Element>;
}

impl<T: IntoChild> IntoChildren for Vec<T> {
    fn into_children(self) -> Vec<Element> {
        self.into_iter().map(IntoChild::into_child).collect()
    }
}

impl<T: IntoChild, const N: usize> IntoChildren for [T; N] {
    fn into_children(self) -> Vec<Element> {
        self.into_iter().map(IntoChild::into_child).collect()
    }
}

macro_rules! tuple_children {
    ($($name:ident),+) => {
        impl<$($name: IntoChild),+> IntoChildren for ($($name,)+) {
            #[allow(non_snake_case)]
            fn into_children(self) -> Vec<Element> {
                let ($($name,)+) = self;
                vec![$($name.into_child()),+]
            }
        }
    };
}

tuple_children!(A);
tuple_children!(A, B);
tuple_children!(A, B, C);
tuple_children!(A, B, C, D);
tuple_children!(A, B, C, D, E);
tuple_children!(A, B, C, D, E, F);
tuple_children!(A, B, C, D, E, F, G);
tuple_children!(A, B, C, D, E, F, G, H);
tuple_children!(A, B, C, D, E, F, G, H, I);
tuple_children!(A, B, C, D, E, F, G, H, I, J);
tuple_children!(A, B, C, D, E, F, G, H, I, J, K);
tuple_children!(A, B, C, D, E, F, G, H, I, J, K, L);

impl Element {
    /// Mounts the element as a control: creates the controls of the tree,
    /// gives them their values and children, attaches their handlers and
    /// starts the effects of their reactive values and the components that
    /// run again. The effects belong to the current owner (see [`Owner`]):
    /// disposing it stops them.
    ///
    /// Text becomes a `TextBlock`, and several elements a `StackPanel`.
    ///
    /// # Panics
    /// Panics for an empty element or a dynamic list, which have no control
    /// of their own.
    pub fn mount(self) -> Ref<Control> {
        crate::mount::mount_root(self)
    }

    /// Mounts the element under a new owner of its own, which is disposed
    /// when the returned owner is: the root of a window or of a piece of user
    /// interface that ends on its own.
    pub fn mount_root(self) -> (Ref<Control>, Owner) {
        let owner = Owner::new_root();
        let control = owner.with(|| self.mount());
        (control, owner)
    }

    /// The key of the element, if it has one.
    pub fn key(&self) -> Option<&Key> {
        match self {
            Element::Control(element) => element.key.as_ref(),
            Element::Component(component) => component.key.as_ref(),
            _ => None,
        }
    }
}

/// The element with `key` (an element of a control keeps it; any other
/// element has no key and is returned as it is).
pub fn with_key(element: Element, key: Key) -> Element {
    match element {
        Element::Control(mut control) => {
            control.key = Some(key);
            Element::Control(control)
        }
        Element::Component(mut component) => {
            component.key = Some(key);
            Element::Component(component)
        }
        element => element,
    }
}
