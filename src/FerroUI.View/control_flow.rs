//! Children that follow state: a keyed list ([`for_each`]), a condition
//! ([`show`]), anything computed ([`dynamic`]), and the reference to a
//! control an element made ([`NodeRef`]).
//!
//! Each is an element that a parent mounts in place: its children are made
//! when it is mounted and made again (or kept, by key) when what they read
//! changes. Each child has an owner of its own, disposed when the child is
//! taken out, so the signals and effects a child made end with it.

use crate::element::{AnyValueEq, DynamicChildren, Element, IntoElement, Key};
use crate::reactive::{memo, signal, Signal};
use ferroui_base::{ObjectType, Ref};
use std::rc::Rc;

/// A list of children made from items, one per item, kept by key: an item
/// whose key and value stay keeps its control; an item that is new or whose
/// value changed gets a new one; the controls of items that left are taken
/// out. `each` is reactive: the list follows the signals it reads.
///
/// ```ignore
/// for_each(move || todos.get(), |todo| todo.id, move |todo| text_block().text(todo.title))
/// ```
pub fn for_each<T, I, K, E>(
    each: impl Fn() -> I + 'static,
    key: impl Fn(&T) -> K + 'static,
    view: impl Fn(T) -> E + 'static,
) -> Element
where
    I: IntoIterator<Item = T>,
    T: Clone + PartialEq + 'static,
    K: Into<Key>,
    E: IntoElement,
{
    let view = Rc::new(view);
    Element::Dynamic(DynamicChildren {
        render: Rc::new(move || {
            each()
                .into_iter()
                .map(|item| {
                    let key = key(&item).into();
                    let value: Rc<dyn AnyValueEq> = Rc::new(item.clone());
                    let view = view.clone();
                    let render: Rc<dyn Fn() -> Element> = Rc::new(move || view(item.clone()).into_element());
                    (Some(key), Some(value), render)
                })
                .collect()
        }),
    })
}

/// The children of `then` while `when` is true, and nothing otherwise. The
/// children are made again only when the condition changes.
pub fn show<E: IntoElement>(when: impl Fn() -> bool + 'static, then: impl Fn() -> E + 'static) -> Element {
    show_else(when, then, || Element::Empty)
}

/// The children of `then` while `when` is true, those of `otherwise`
/// otherwise.
pub fn show_else<E: IntoElement, O: IntoElement>(
    when: impl Fn() -> bool + 'static,
    then: impl Fn() -> E + 'static,
    otherwise: impl Fn() -> O + 'static,
) -> Element {
    let condition = memo(move |_| when());
    let then: Rc<dyn Fn() -> Element> = Rc::new(move || then().into_element());
    let otherwise: Rc<dyn Fn() -> Element> = Rc::new(move || otherwise().into_element());
    Element::Dynamic(DynamicChildren {
        render: Rc::new(move || {
            let render = if condition.get() { then.clone() } else { otherwise.clone() };
            vec![(None, None, render)]
        }),
    })
}

/// Children computed by a reactive closure: made again whenever what it
/// read changes. An element with a key keeps its control while an element
/// with the same key comes back (its values are not compared: give the key
/// what identifies the values, or use [`for_each`]).
pub fn dynamic<E: IntoElement>(render: impl Fn() -> E + 'static) -> Element {
    Element::Dynamic(DynamicChildren {
        render: Rc::new(move || {
            let element = render().into_element();
            let children = match element {
                Element::Fragment(items) | Element::List(items) => items,
                Element::Empty => Vec::new(),
                element => vec![element],
            };
            children
                .into_iter()
                .map(|child| {
                    let key = child.key().cloned();
                    let render: Rc<dyn Fn() -> Element> = Rc::new(move || child.clone());
                    (key, None, render)
                })
                .collect()
        }),
    })
}

/// The control an element made, once it is mounted (see
/// [`El::node_ref`](crate::El::node_ref)). `Copy`; reading it is reactive.
pub struct NodeRef<T: ObjectType> {
    control: Signal<Option<Ref<T>>>,
}

impl<T: ObjectType> Clone for NodeRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ObjectType> Copy for NodeRef<T> {}

impl<T: ObjectType> NodeRef<T> {
    /// A reference with no control yet, owned by the current owner.
    pub fn new() -> Self {
        Self { control: signal(None) }
    }

    /// The control, once mounted.
    pub fn get(&self) -> Option<Ref<T>> {
        self.control.get()
    }

    /// The control without recording the read.
    pub fn get_untracked(&self) -> Option<Ref<T>> {
        self.control.get_untracked()
    }

    pub(crate) fn set(&self, control: Ref<T>) {
        self.control.set(Some(control));
    }
}

impl<T: ObjectType> Default for NodeRef<T> {
    fn default() -> Self {
        Self::new()
    }
}
