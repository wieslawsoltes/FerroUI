//! The list of the runtime library that markup creates
//! (`<generic:List x:TypeArguments="T">`, `<collections:ArrayList>`).
//!
//! Not a port of an upstream file: upstream creates the `List<T>` and the
//! `ArrayList` of the runtime library. Rust cannot instantiate a generic type
//! for an element type known only when a document is loaded, so both are one
//! list of untyped items (DEVIATIONS.md, Run-time type system of markup). The
//! run-time loader and generated code create the same type, so that a value a
//! document hands on is the same whichever of the two built it.

use std::any::Any;
use std::fmt;
use std::rc::Rc;

use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::MarkupValue;
use ferroui_base::BoxedValue;

use super::compiled::untyped_object_form;

/// The element type of a `List<T>` created in markup, as the list names it.
///
/// The run-time loader states the type of its type system, generated code the
/// full name the compiler resolved ([`RuntimeList::of`]).
pub trait RuntimeListElement {
    /// The full name of the type (`FerroUI.Media.Stretch`).
    fn full_name(&self) -> String;
    fn as_any(&self) -> &dyn Any;
}

/// An element type known by its full name: what generated code states.
struct NamedElement(&'static str);

impl RuntimeListElement for NamedElement {
    fn full_name(&self) -> String {
        self.0.to_string()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A value of a list of the runtime library created in markup:
/// `System.Collections.Generic.List<T>` (with the element type `T`) or
/// `System.Collections.ArrayList` (without an element type).
///
/// The items are held untyped in the list the items controls read
/// (`FerroList<Option<BoxedValue>>`), shared: the list a collection handle
/// receives is the list markup filled. A member that declares a collection
/// handle (the items source of an items control) receives the shared list of
/// the items through the cast the crate of the handle registers for it
/// ([`to_declared`](Self::to_declared)); an untyped target receives the
/// `RuntimeList` itself.
// Deviation (DEVIATIONS.md, Run-time type system of markup): upstream creates the
// `List<T>` and the `ArrayList` of the runtime library; here both are this list of
// untyped items.
#[derive(Clone)]
pub struct RuntimeList {
    element_type: Option<Rc<dyn RuntimeListElement>>,
    items: Rc<FerroList<MarkupValue>>,
}

impl RuntimeList {
    pub fn new(element_type: Option<Rc<dyn RuntimeListElement>>) -> Self {
        Self { element_type, items: Rc::new(FerroList::new()) }
    }

    /// `new List<T>()` for the element type with the full name `element_type`.
    pub fn of(element_type: &'static str) -> Self {
        Self::new(Some(Rc::new(NamedElement(element_type))))
    }

    /// `new ArrayList()`.
    pub fn untyped() -> Self {
        Self::new(None)
    }

    /// The element type of a `List<T>`; `None` for an `ArrayList`.
    pub fn element_type(&self) -> Option<&Rc<dyn RuntimeListElement>> {
        self.element_type.as_ref()
    }

    /// The shared list of the items.
    pub fn items(&self) -> &Rc<FerroList<MarkupValue>> {
        &self.items
    }

    pub fn count(&self) -> usize {
        self.items.count()
    }

    /// The item at `index`. Panics if out of range.
    pub fn get(&self, index: usize) -> MarkupValue {
        self.items.get(index)
    }

    /// Replaces the item at `index`. Panics if out of range.
    pub fn set(&self, index: usize, item: MarkupValue) {
        self.items.set(index, Self::item(item));
    }

    /// Adds an item and returns its index.
    pub fn add(&self, item: MarkupValue) -> usize {
        self.items.add(Self::item(item));
        self.items.count() - 1
    }

    /// An item as the list holds it: an object of the object model in its
    /// untyped form.
    fn item(item: MarkupValue) -> MarkupValue {
        item.map(|item| untyped_object_form(&item).unwrap_or(item))
    }

    /// The list in the Rust type `target` a member declares for a collection:
    /// the shared list of the items, cast to `target` with the assignability
    /// casts of the untyped value conversions. `None` if `target` takes the
    /// list as it is (an untyped target, a list) or no cast to `target` is
    /// registered.
    pub fn to_declared(&self, target: ValueType) -> Option<BoxedValue> {
        if target.is_object() || target.is::<RuntimeList>() {
            return None;
        }
        let items: BoxedValue = Rc::new(self.items.clone());
        ValueTypes::try_cast(&items, target)
    }
}

/// Lists compare by identity of their storage: a list is a reference.
impl PartialEq for RuntimeList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.items, &other.items)
    }
}

impl fmt::Debug for RuntimeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.element_type {
            Some(element_type) => write!(f, "List<{}>[{}]", element_type.full_name(), self.items.count()),
            None => write!(f, "ArrayList[{}]", self.items.count()),
        }
    }
}
