//! `Models/TreeDataGrid/ObjectHierarchicalExpanderColumn.cs`: the expander column of the
//! declarative columns of markup, over models that are untyped values.
//!
//! The reference's separate class differs from `HierarchicalExpanderColumn<object>` only in taking
//! delegates and the objects their paths observe in place of expressions; with accessors (which
//! carry the getter, the setter and the observed objects) it is that column, built from the
//! delegates.

use super::{HierarchicalExpanderColumn, IViewColumn};
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::{Accessor, Models};
use std::rc::Rc;

/// An object below a model whose changes a binding observes (`Func<object, object>`).
pub type ObjectLink = Rc<dyn Fn(&BoxedValue) -> Option<BoxedValue>>;

/// `ObjectHierarchicalExpanderColumn`.
pub struct ObjectHierarchicalExpanderColumn;

impl ObjectHierarchicalExpanderColumn {
    /// An expander column of `inner` over untyped models. `child_selector` gives the children of
    /// a model; `has_children_selector` whether it has children, re-read when one of
    /// `has_children_links` changes; `is_expanded_getter` and `is_expanded_setter` the model's
    /// expansion state, re-read when one of `is_expanded_links` changes. Without links the model
    /// itself is observed.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inner: Rc<dyn IViewColumn<BoxedValue>>,
        child_selector: impl Fn(&BoxedValue) -> Option<Models<BoxedValue>> + 'static,
        has_children_selector: Option<Rc<dyn Fn(&BoxedValue) -> bool>>,
        is_expanded_getter: Option<Rc<dyn Fn(&BoxedValue) -> bool>>,
        is_expanded_setter: Option<Rc<dyn Fn(&BoxedValue, bool)>>,
        has_children_links: Option<Vec<ObjectLink>>,
        is_expanded_links: Option<Vec<ObjectLink>>,
    ) -> Rc<HierarchicalExpanderColumn<BoxedValue>> {
        let has_children =
            has_children_selector.map(|read| create_binding(read, None, has_children_links.unwrap_or_default()));
        let is_expanded = is_expanded_getter
            .map(|read| create_binding(read, is_expanded_setter, is_expanded_links.unwrap_or_default()));
        HierarchicalExpanderColumn::new(
            inner,
            move |model: &Rc<BoxedValue>| child_selector(&**model),
            has_children,
            is_expanded,
        )
    }
}

// `CreateBinding`: the accessor of a delegate and the objects it observes (the model itself when
// there are no links).
fn create_binding(
    read: Rc<dyn Fn(&BoxedValue) -> bool>,
    write: Option<Rc<dyn Fn(&BoxedValue, bool)>>,
    links: Vec<ObjectLink>,
) -> Accessor<BoxedValue, bool> {
    let mut accessor = Accessor::new(move |model: &Rc<BoxedValue>| read(&**model));
    if let Some(write) = write {
        accessor = accessor.with_setter(move |model: &Rc<BoxedValue>, value: bool| write(&**model, value));
    }
    if links.is_empty() {
        accessor = accessor.through(|model: &Rc<BoxedValue>| Some((**model).clone()), "");
    } else {
        for link in links {
            accessor = accessor.through(move |model: &Rc<BoxedValue>| link(&**model), "");
        }
    }
    accessor
}
