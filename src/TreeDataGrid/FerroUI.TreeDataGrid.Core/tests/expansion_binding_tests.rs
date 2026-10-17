//! `ExpansionBindingTests.cs`: an expansion state bound below the model, observed along its path.

use super::support::{array, header};
use crate::accessor::Accessor;
use crate::model_collection::{observable, Models};
use crate::models::{ColumnRef, HierarchicalExpanderColumn, TextColumn};
use crate::{HierarchicalTreeDataGridSource, ITreeDataGridSource, IndexPath};
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model, ModelTypeBuilder};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn nested_expansion_updates_without_a_view_and_rebinds_replaced_owners() {
    let root = Node::new(vec![Node::new(Vec::new())]);
    let source = create(&root, None);
    assert_eq!(1, source.rows().count());
    let original = root.state();
    original.set_is_expanded(true);
    assert_eq!(2, source.rows().count());
    root.set_state(Some(State::new(false)));
    assert_eq!(1, source.rows().count());
    assert_eq!(0, original.subscriber_count());
    original.set_is_expanded(false);
    original.set_is_expanded(true);
    assert_eq!(1, source.rows().count());
    root.state().set_is_expanded(true);
    assert_eq!(2, source.rows().count());
    source.collapse(&IndexPath::new(0));
    assert!(!root.state().is_expanded());
    source.expand(&IndexPath::new(0));
    assert!(root.state().is_expanded());
}

#[test]
fn source_disposal_or_row_removal_releases_entire_property_path() {
    // [InlineData(false)], [InlineData(true)]
    for remove in [false, true] {
        let root = Node::new(vec![Node::new(Vec::new())]);
        let items = observable(vec![root.clone()]);
        let source = create(&root, Some(items.clone()));
        assert_eq!(1, source.rows().count());
        assert!(root.subscriber_count() > 0, "remove: {remove}");
        assert!(root.state().subscriber_count() > 0, "remove: {remove}");
        if remove {
            items.clear();
        } else {
            source.dispose();
        }
        assert_eq!(0, root.subscriber_count(), "remove: {remove}");
        assert_eq!(0, root.state().subscriber_count(), "remove: {remove}");
    }
}

#[test]
fn null_safe_selector_reconnects_a_missing_intermediate_object() {
    let root = Node::new(vec![Node::new(Vec::new())]);
    let source = HierarchicalTreeDataGridSource::new(array(vec![root.clone()]));
    source.columns().add(HierarchicalExpanderColumn::new(
        TextColumn::new(header("Name"), Accessor::new(|_x: &Rc<Node>| "Node".to_string()), None, None, None) as ColumnRef<Node>,
        |x: &Rc<Node>| Some(x.children.clone() as Models<Node>),
        None,
        Some(
            Accessor::member("State", |x: &Rc<Node>| x.state.borrow().as_ref().is_some_and(|state| state.is_expanded()))
                .through(|x: &Rc<Node>| x.state.borrow().clone().map(|state| state as BoxedValue), "IsExpanded"),
        ),
        Some(Rc::new(|x: &Rc<Node>, value: bool| {
            let state = x.state.borrow().clone();
            if let Some(state) = state {
                state.set_is_expanded(value);
            }
        })),
    ) as ColumnRef<Node>);
    assert_eq!(1, source.rows().count());
    let original = root.state();
    // The reference assigns `null!`: the state is an optional value here.
    root.set_state(None);
    assert_eq!(0, original.subscriber_count());
    root.set_state(Some(State::new(true)));
    assert_eq!(2, source.rows().count());
}

fn create(root: &Rc<Node>, items: Option<Rc<FerroList<Rc<Node>>>>) -> Rc<HierarchicalTreeDataGridSource<Node>> {
    let items = items.unwrap_or_else(|| observable(vec![root.clone()]));
    let source = HierarchicalTreeDataGridSource::new(items as Models<Node>);
    source.columns().add(HierarchicalExpanderColumn::new(
        TextColumn::new(header("Name"), Accessor::new(|_x: &Rc<Node>| "Node".to_string()), None, None, None) as ColumnRef<Node>,
        |x: &Rc<Node>| Some(x.children.clone() as Models<Node>),
        None,
        // `isExpandedSelector: x => x.State.IsExpanded`, with the setter the reference synthesizes from it.
        Some(
            Accessor::member("State", |x: &Rc<Node>| x.state().is_expanded())
                .with_setter(|x: &Rc<Node>, value: bool| x.state().set_is_expanded(value))
                .through(|x: &Rc<Node>| x.state.borrow().clone().map(|state| state as BoxedValue), "IsExpanded"),
        ),
        None,
    ) as ColumnRef<Node>);
    source
}

/// `Observable`'s `SubscriberCount`, and `Node` with its `State` (`null` when `None`).
struct Node {
    state: RefCell<Option<Rc<State>>>,
    children: Rc<FerroList<Rc<Node>>>,
    changed: Event<str>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Node {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model for Node {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

impl Node {
    /// `new Node { Children = { children } }`.
    fn new(children: Vec<Rc<Node>>) -> Rc<Self> {
        Self::register();
        Rc::new(Self { state: RefCell::new(Some(State::new(false))), children: observable(children), changed: Event::new() })
    }

    /// `State` (not null).
    fn state(&self) -> Rc<State> {
        self.state.borrow().clone().expect("the state of the node")
    }

    fn set_state(&self, value: Option<Rc<State>>) {
        *self.state.borrow_mut() = value;
        self.changed.raise("State");
    }

    fn subscriber_count(&self) -> usize {
        self.changed.handler_count()
    }
}

struct State {
    expanded: Cell<bool>,
    changed: Event<str>,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for State {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model for State {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

impl State {
    /// `new State { IsExpanded = expanded }`.
    fn new(expanded: bool) -> Rc<Self> {
        Self::register();
        Rc::new(Self { expanded: Cell::new(expanded), changed: Event::new() })
    }

    fn is_expanded(&self) -> bool {
        self.expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        if self.expanded.get() == value {
            return;
        }
        self.expanded.set(value);
        self.changed.raise("IsExpanded");
    }

    fn subscriber_count(&self) -> usize {
        self.changed.handler_count()
    }
}
