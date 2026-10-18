//! `Bindings/TypedBindingTests.cs`: the bindings of cells to the values of their models read,
//! write, follow the changes of nested paths, and release a replaced root.
//!
//! The reference tests its vendored typed binding engine (`TypedBinding<TIn>.OneWay/TwoWay`,
//! `Instance(source)`, `IRetargetableTypedBindingExpression`), which the port replaced by the
//! cell bindings of accessors. The tests map as follows:
//!
//! - `TypedBinding<TIn>.OneWay(vm => vm.Child!.Name)` is an [`Accessor`] with no setter whose
//!   getter walks the path and whose links (`through`) name the objects below the model that are
//!   observed; `TwoWay` adds the setter (the assignment the reference synthesizes).
//! - `binding.Instance(source)` is [`AccessorBinding::new`] of the accessor on the source.
//! - `target.Bind(TestTarget.TextProperty, expression)` subscribes a [`TestTarget`] to the
//!   binding: the target holds the last value the binding published.
//! - `binding.Write!.Invoke(source, value)` is the accessor's setter; `expression.OnNext(value)` is
//!   [`ICellBinding::on_next`].
//! - `IRetargetableTypedBindingExpression.TrySetRoot` is [`IRetargetableCellBinding::try_set_root`].

use super::support::assert_panics;
use crate::models::{AccessorBinding, ICellBinding, IRetargetableCellBinding};
use crate::presentation::{untyped_conversion, CoreCellBinding};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::models::{
    ColumnRef as CoreColumnRef, HierarchicalExpanderColumn as NeutralExpanderColumn, IColumn as CoreColumn,
    TextColumn as NeutralTextColumn,
};
use ferroui_treedatagrid_core::{Accessor, PropertyPathObserver};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

// ---------------------------------------------------------------------------------------------
// Models and targets
// ---------------------------------------------------------------------------------------------

/// `TestViewModel`: a name and a child, with property change notifications.
struct TestViewModel {
    name: RefCell<String>,
    child: RefCell<Option<Rc<TestViewModel>>>,
    changed: Event<str>,
}

ferroui_base::ferro_model!(TestViewModel, |b| b.notify_property_changed());

impl INotifyPropertyChanged for TestViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl TestViewModel {
    /// `new TestViewModel { Name = name, Child = child }`.
    fn new(name: &str, child: Option<Rc<TestViewModel>>) -> Rc<Self> {
        Model::new_model(Self { name: RefCell::new(name.to_string()), child: RefCell::new(child), changed: Event::new() })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    /// `Name = value` (`SetProperty`: raises `Name` when the value changed).
    fn set_name(&self, value: &str) {
        if *self.name.borrow() == value {
            return;
        }
        *self.name.borrow_mut() = value.to_string();
        self.changed.raise("Name");
    }

    fn child(&self) -> Option<Rc<TestViewModel>> {
        self.child.borrow().clone()
    }

    /// `Child = value` (`SetProperty` with the default comparer: the identity of the child).
    fn set_child(&self, value: Option<Rc<TestViewModel>>) {
        let same = match (&*self.child.borrow(), &value) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        *self.child.borrow_mut() = value;
        self.changed.raise("Child");
    }

    /// The number of subscribers to the model's `PropertyChanged`.
    fn subscriber_count(&self) -> usize {
        self.changed.handler_count()
    }
}

/// `TestExpandableNode`: an expansion state that counts its expansions and collapses.
struct TestExpandableNode {
    is_expanded: Cell<bool>,
    expand_call_count: Cell<i32>,
    collapse_call_count: Cell<i32>,
    changed: Event<str>,
}

ferroui_base::ferro_model!(TestExpandableNode, |b| b.notify_property_changed());

impl INotifyPropertyChanged for TestExpandableNode {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl TestExpandableNode {
    /// `new TestExpandableNode { IsExpanded = is_expanded }`.
    fn new(is_expanded: bool) -> Rc<Self> {
        let node = Model::new_model(Self {
            is_expanded: Cell::new(false),
            expand_call_count: Cell::new(0),
            collapse_call_count: Cell::new(0),
            changed: Event::new(),
        });
        node.set_is_expanded(is_expanded);
        node
    }

    fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    /// `IsExpanded = value`: the expand or collapse logic runs, then the change is raised.
    fn set_is_expanded(&self, value: bool) {
        if self.is_expanded.get() != value {
            self.is_expanded.set(value);

            // Simulate expand/collapse logic
            if value {
                self.expand_call_count.set(self.expand_call_count.get() + 1);
            } else {
                self.collapse_call_count.set(self.collapse_call_count.get() + 1);
            }

            self.changed.raise("IsExpanded");
        }
    }

    fn expand_call_count(&self) -> i32 {
        self.expand_call_count.get()
    }

    fn collapse_call_count(&self) -> i32 {
        self.collapse_call_count.get()
    }
}

/// `TestTarget`: the target of a binding of text, holding the last value it received
/// (`TextProperty`). Setting the text does not write it back: the tests write through the
/// binding's setter, as the reference's do.
struct TestTarget {
    text: RefCell<Option<String>>,
    subscription: RefCell<Option<(Rc<dyn ICellBinding<Option<String>>>, u64)>>,
}

impl TestTarget {
    fn new() -> Rc<Self> {
        Rc::new(Self { text: RefCell::new(None), subscription: RefCell::new(None) })
    }

    /// `target.Bind(TestTarget.TextProperty, expression)`.
    fn bind(self: &Rc<Self>, expression: Rc<dyn ICellBinding<Option<String>>>) {
        let weak: Weak<Self> = Rc::downgrade(self);
        let token = expression.subscribe(Rc::new(move |value: &Option<String>| {
            if let Some(this) = weak.upgrade() {
                *this.text.borrow_mut() = value.clone();
            }
        }));
        *self.subscription.borrow_mut() = Some((expression, token));
    }

    fn text(&self) -> Option<String> {
        self.text.borrow().clone()
    }

    fn set_text(&self, value: &str) {
        *self.text.borrow_mut() = Some(value.to_string());
    }
}

impl Drop for TestTarget {
    fn drop(&mut self) {
        if let Some((expression, token)) = self.subscription.borrow_mut().take() {
            expression.unsubscribe(token);
        }
    }
}

/// `TestExpandableTarget`: the target of a binding of the expansion state; setting its state
/// writes it to the source node through the binding's setter (`_typedBinding?.Write?.Invoke`).
struct TestExpandableTarget {
    is_expanded: Cell<bool>,
    typed_binding: RefCell<Option<Accessor<TestExpandableNode, bool>>>,
    source_node: RefCell<Option<Rc<TestExpandableNode>>>,
    subscription: RefCell<Option<(Rc<AccessorBinding<TestExpandableNode, bool>>, u64)>>,
}

impl TestExpandableTarget {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            is_expanded: Cell::new(false),
            typed_binding: RefCell::new(None),
            source_node: RefCell::new(None),
            subscription: RefCell::new(None),
        })
    }

    /// `Bind(IsExpandedProperty, expression, typedBinding, sourceNode)`.
    fn bind(
        self: &Rc<Self>,
        expression: &Rc<AccessorBinding<TestExpandableNode, bool>>,
        typed_binding: &Accessor<TestExpandableNode, bool>,
        source_node: &Rc<TestExpandableNode>,
    ) {
        let weak: Weak<Self> = Rc::downgrade(self);
        let token = expression.subscribe(Rc::new(move |value: &bool| {
            if let Some(this) = weak.upgrade() {
                this.is_expanded.set(*value);
            }
        }));
        *self.subscription.borrow_mut() = Some((expression.clone(), token));
        *self.source_node.borrow_mut() = Some(source_node.clone());
        *self.typed_binding.borrow_mut() = Some(typed_binding.clone());
    }

    fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        self.is_expanded.set(value);
        let typed_binding = self.typed_binding.borrow().clone();
        let source_node = self.source_node.borrow().clone();
        if let (Some(typed_binding), Some(source_node)) = (typed_binding, source_node) {
            typed_binding.set(&source_node, value);
        }
    }
}

impl Drop for TestExpandableTarget {
    fn drop(&mut self) {
        if let Some((expression, token)) = self.subscription.borrow_mut().take() {
            expression.unsubscribe(token);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Bindings
// ---------------------------------------------------------------------------------------------

/// The view model `depth` children below `vm` (`vm.Child!.Child!...`), or `None` when one on the
/// way is null.
fn descend(vm: &Rc<TestViewModel>, depth: usize) -> Option<Rc<TestViewModel>> {
    let mut current = vm.clone();
    for _ in 0..depth {
        current = current.child()?;
    }
    Some(current)
}

/// `TypedBinding<TestViewModel>.OneWay(vm => vm.Child!...Child!.Name)` with `depth` children on
/// the path: the getter walks the path (null when a child on it is null), and each object below
/// the model is observed for the member read from it (`Child`, and `Name` on the last).
fn one_way(depth: usize) -> Accessor<TestViewModel, Option<String>> {
    let member = if depth == 0 { "Name" } else { "Child" };
    let mut accessor = Accessor::member(member, move |vm: &Rc<TestViewModel>| descend(vm, depth).map(|owner| owner.name()));
    for level in 1..=depth {
        let property = if level == depth { "Name" } else { "Child" };
        accessor = accessor.through(move |vm: &Rc<TestViewModel>| descend(vm, level).map(|owner| owner as BoxedValue), property);
    }
    accessor
}

/// `TypedBinding<TestViewModel>.TwoWay(vm => vm.Child!...Child!.Name)`: [`one_way`] with the
/// assignment of the last member (`Write`).
fn two_way(depth: usize) -> Accessor<TestViewModel, Option<String>> {
    one_way(depth).with_setter(move |vm: &Rc<TestViewModel>, value: Option<String>| {
        if let Some(owner) = descend(vm, depth) {
            owner.set_name(&value.unwrap_or_default());
        }
    })
}

/// `binding.Instance(source)`.
fn instance(
    binding: &Accessor<TestViewModel, Option<String>>,
    source: &Rc<TestViewModel>,
) -> Rc<AccessorBinding<TestViewModel, Option<String>>> {
    AccessorBinding::new(binding.clone(), Some(source.clone()))
}

/// `binding.Write!.Invoke(source, value)`.
fn write(binding: &Accessor<TestViewModel, Option<String>>, source: &Rc<TestViewModel>, value: Option<String>) {
    let setter = binding.setter().expect("the setter of a two-way binding");
    setter(source, value);
}

fn bind(target: &Rc<TestTarget>, expression: &Rc<AccessorBinding<TestViewModel, Option<String>>>) {
    target.bind(expression.clone() as Rc<dyn ICellBinding<Option<String>>>);
}

fn text(value: &str) -> Option<String> {
    Some(value.to_string())
}

/// `TypedBinding<TestExpandableNode>.TwoWay(vm => vm.IsExpanded)`.
fn is_expanded_binding() -> Accessor<TestExpandableNode, bool> {
    Accessor::member("IsExpanded", |node: &Rc<TestExpandableNode>| node.is_expanded())
        .with_setter(|node: &Rc<TestExpandableNode>, value: bool| node.set_is_expanded(value))
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn retargeted_binding_unsubscribes_old_root_and_tracks_new_root() {
    let first = TestViewModel::new("First", None);
    let second = TestViewModel::new("Second", None);
    let target = TestTarget::new();
    let expression = instance(&one_way(0), &first);

    bind(&target, &expression);
    let retargetable: &dyn IRetargetableCellBinding =
        expression.as_retargetable().expect("a binding whose root can be replaced");

    assert!(retargetable.try_set_root(Some(&(second.clone() as BoxedValue))));
    assert_eq!(text("Second"), target.text());
    assert_eq!(0, first.subscriber_count());

    first.set_name("Old root changed");
    assert_eq!(text("Second"), target.text());

    second.set_name("New root changed");
    assert_eq!(text("New root changed"), target.text());

    assert!(retargetable.try_set_root(None));
    assert_eq!(0, second.subscriber_count());
    second.set_name("Detached root changed");
    // The reference publishes null when the root is cleared; the port's binding has no value
    // without a root and publishes nothing, so the target keeps the last value of the old root
    // and the change of the detached root does not reach it.
    assert!(expression.value().is_none());
    assert_eq!(text("New root changed"), target.text());
}

#[test]
fn one_way_delegate_overload_uses_precompiled_delegates() {
    let source = TestViewModel::new("Test", None);
    let target = TestTarget::new();
    // `OneWay(vm => vm.Name, new Func<TestViewModel, object>[] { vm => vm })`: a getter delegate
    // whose only link is the model itself, an accessor without a member name, which follows every
    // change of the model.
    let binding = Accessor::new(|vm: &Rc<TestViewModel>| Some(vm.name()));

    target.bind(AccessorBinding::new(binding, Some(source.clone())) as Rc<dyn ICellBinding<Option<String>>>);
    source.set_name("Updated");

    assert_eq!(text("Updated"), target.text());
}

#[test]
fn one_way_delegate_overload_rejects_null_delegates() {
    // A null getter: the port's accessors always have a getter, so the binding that can be
    // given a column without one, the binding of an untyped cell of a neutral column, rejects a
    // column that reads no value (an expander column).
    let inner = NeutralTextColumn::new(
        Some(Rc::new("Name".to_string()) as BoxedValue),
        Accessor::new(|vm: &Rc<TestViewModel>| Some(vm.name())),
        None,
        None,
        None,
    ) as CoreColumnRef<TestViewModel>;
    let expander = NeutralExpanderColumn::new(inner, |_: &Rc<TestViewModel>| None, None, None, None);
    assert_panics(|| {
        CoreCellBinding::new(expander.clone() as Rc<dyn CoreColumn>, untyped_conversion(), None);
    });

    // A null array of links cannot be expressed: an accessor without links is the delegate
    // form, whose path has nothing to observe below the model.
    assert!(PropertyPathObserver::create(&Accessor::new(|vm: &Rc<TestViewModel>| Some(vm.name()))).is_none());
}

#[test]
fn one_way_binding_single_link_should_get_value() {
    // Arrange
    let source = TestViewModel::new("Test", None);
    let target = TestTarget::new();

    // Create a binding with a single link (source.Name)
    let binding = one_way(0);
    let expression = instance(&binding, &source);

    // Act
    bind(&target, &expression);

    // Assert
    assert_eq!(text("Test"), target.text());
}

#[test]
fn one_way_binding_single_link_should_listen_to_changes() {
    // Arrange
    let source = TestViewModel::new("Test", None);
    let target = TestTarget::new();

    // Create a binding with a single link (source.Name)
    let binding = one_way(0);
    let expression = instance(&binding, &source);

    // Act
    bind(&target, &expression);
    source.set_name("Updated");

    // Assert
    assert_eq!(text("Updated"), target.text());
}

#[test]
fn two_way_binding_single_link_should_listen_and_write() {
    // Arrange
    let source = TestViewModel::new("Test", None);
    let target = TestTarget::new();

    // Create a binding with a single link (source.Name)
    let binding = two_way(0);
    let expression = instance(&binding, &source);

    // Act - set up binding
    bind(&target, &expression);

    // Act - change source
    source.set_name("Updated");

    // Assert
    assert_eq!(text("Updated"), target.text());

    // Act - change target
    target.set_text("UpdatedFromTarget");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("UpdatedFromTarget", source.name());
}

#[test]
fn one_way_binding_two_links_should_get_value() {
    // Arrange
    let child = TestViewModel::new("Child", None);
    let source = TestViewModel::new("Parent", Some(child));
    let target = TestTarget::new();

    // Create a binding with two links (source.Child.Name)
    let binding = one_way(1);
    let expression = instance(&binding, &source);

    // Act
    bind(&target, &expression);

    // Assert
    assert_eq!(text("Child"), target.text());
}

#[test]
fn one_way_binding_two_links_should_listen_to_changes() {
    // Arrange
    let child = TestViewModel::new("Child", None);
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with two links (source.Child.Name)
    let binding = one_way(1);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - update leaf property
    child.set_name("UpdatedChild");

    // Assert
    assert_eq!(text("UpdatedChild"), target.text());

    // Act - update intermediate property
    let new_child = TestViewModel::new("NewChild", None);
    source.set_child(Some(new_child));

    // Assert
    assert_eq!(text("NewChild"), target.text());
}

#[test]
fn two_way_binding_two_links_should_listen_and_write() {
    // Arrange
    let child = TestViewModel::new("Child", None);
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with two links (source.Child.Name)
    let binding = two_way(1);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - update from source
    child.set_name("UpdatedChild");

    // Assert
    assert_eq!(text("UpdatedChild"), target.text());

    // Act - update from target
    target.set_text("UpdatedFromTarget");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("UpdatedFromTarget", child.name());

    // Act - change intermediate link
    let new_child = TestViewModel::new("NewChild", None);
    source.set_child(Some(new_child.clone()));

    // Assert
    assert_eq!(text("NewChild"), target.text());

    // Act - update from target after changing intermediate
    target.set_text("FinalUpdate");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("FinalUpdate", new_child.name());
}

#[test]
fn one_way_binding_three_links_should_get_value() {
    // Arrange
    let grand_child = TestViewModel::new("GrandChild", None);
    let child = TestViewModel::new("Child", Some(grand_child));
    let source = TestViewModel::new("Parent", Some(child));
    let target = TestTarget::new();

    // Create a binding with three links (source.Child.Child.Name)
    let binding = one_way(2);
    let expression = instance(&binding, &source);

    // Act
    bind(&target, &expression);

    // Assert
    assert_eq!(text("GrandChild"), target.text());
}

#[test]
fn one_way_binding_three_links_should_listen_to_changes() {
    // Arrange
    let grand_child = TestViewModel::new("GrandChild", None);
    let child = TestViewModel::new("Child", Some(grand_child.clone()));
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with three links (source.Child.Child.Name)
    let binding = one_way(2);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - update leaf property
    grand_child.set_name("UpdatedGrandChild");

    // Assert
    assert_eq!(text("UpdatedGrandChild"), target.text());

    // Act - update middle property
    let new_grand_child = TestViewModel::new("NewGrandChild", None);
    child.set_child(Some(new_grand_child));

    // Assert
    assert_eq!(text("NewGrandChild"), target.text());

    // Act - update root property
    let new_child_with_grand_child =
        TestViewModel::new("NewChild", Some(TestViewModel::new("NewestGrandChild", None)));
    source.set_child(Some(new_child_with_grand_child));

    // Assert
    assert_eq!(text("NewestGrandChild"), target.text());
}

#[test]
fn two_way_binding_three_links_should_listen_and_write() {
    // Arrange
    let grand_child = TestViewModel::new("GrandChild", None);
    let child = TestViewModel::new("Child", Some(grand_child.clone()));
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with three links (source.Child.Child.Name)
    let binding = two_way(2);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - set from target
    target.set_text("UpdatedFromTarget");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("UpdatedFromTarget", grand_child.name());

    // Act - change intermediate node and update from target
    let new_grand_child = TestViewModel::new("NewGrandChild", None);
    child.set_child(Some(new_grand_child.clone()));

    // Assert initial update
    assert_eq!(text("NewGrandChild"), target.text());

    // Act - update again from target
    target.set_text("AfterIntermediateChange");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("AfterIntermediateChange", new_grand_child.name());
}

#[test]
fn one_way_binding_four_links_should_get_value() {
    // Arrange
    let great_grand_child = TestViewModel::new("GreatGrandChild", None);
    let grand_child = TestViewModel::new("GrandChild", Some(great_grand_child));
    let child = TestViewModel::new("Child", Some(grand_child));
    let source = TestViewModel::new("Parent", Some(child));
    let target = TestTarget::new();

    // Create a binding with four links (source.Child.Child.Child.Name)
    let binding = one_way(3);
    let expression = instance(&binding, &source);

    // Act
    bind(&target, &expression);

    // Assert
    assert_eq!(text("GreatGrandChild"), target.text());
}

#[test]
fn one_way_binding_four_links_should_listen_to_changes() {
    // Arrange
    let great_grand_child = TestViewModel::new("GreatGrandChild", None);
    let grand_child = TestViewModel::new("GrandChild", Some(great_grand_child.clone()));
    let child = TestViewModel::new("Child", Some(grand_child.clone()));
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with four links (source.Child.Child.Child.Name)
    let binding = one_way(3);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - update leaf node
    great_grand_child.set_name("UpdatedGreatGrandChild");

    // Assert
    assert_eq!(text("UpdatedGreatGrandChild"), target.text());

    // Act - update middle node
    let new_great_grand_child = TestViewModel::new("NewGreatGrandChild", None);
    grand_child.set_child(Some(new_great_grand_child));

    // Assert
    assert_eq!(text("NewGreatGrandChild"), target.text());

    // Act - update higher node
    let new_grand_child_with_child =
        TestViewModel::new("NewGrandChild", Some(TestViewModel::new("BrandNewGreatGrandChild", None)));
    child.set_child(Some(new_grand_child_with_child));

    // Assert
    assert_eq!(text("BrandNewGreatGrandChild"), target.text());
}

#[test]
fn two_way_binding_four_links_should_listen_and_write() {
    // Arrange
    let great_grand_child = TestViewModel::new("GreatGrandChild", None);
    let grand_child = TestViewModel::new("GrandChild", Some(great_grand_child.clone()));
    let child = TestViewModel::new("Child", Some(grand_child.clone()));
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with four links (source.Child.Child.Child.Name)
    let binding = two_way(3);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - set from target
    target.set_text("UpdatedFromTarget");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("UpdatedFromTarget", great_grand_child.name());

    // Act - change leaf node
    let new_great_grand_child = TestViewModel::new("NewGreatGrandChild", None);
    grand_child.set_child(Some(new_great_grand_child.clone()));

    // Assert
    assert_eq!(text("NewGreatGrandChild"), target.text());

    // Act - update from target
    target.set_text("AfterLeafChange");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("AfterLeafChange", new_great_grand_child.name());

    // Act - update middle node
    let new_grand_child_with_child =
        TestViewModel::new("NewGrandChild", Some(TestViewModel::new("BrandNewGreatGrandChild", None)));
    child.set_child(Some(new_grand_child_with_child.clone()));

    // Assert
    assert_eq!(text("BrandNewGreatGrandChild"), target.text());

    // Act - update from target
    target.set_text("AfterMiddleChange");
    write(&binding, &source, target.text());

    // Assert
    assert_eq!("AfterMiddleChange", new_grand_child_with_child.child().expect("the great grandchild").name());
}

#[test]
fn binding_should_handle_null_intermediate_values() {
    // Arrange
    let source = TestViewModel::new("Parent", None);
    let target = TestTarget::new();

    // Create a binding that will encounter a null (source.Child.Name)
    let binding = one_way(1);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Assert - the value is null while the path is broken
    assert_eq!(None, target.text());

    // Act - fix the null reference
    let new_child = TestViewModel::new("NewChild", None);
    source.set_child(Some(new_child));

    // Assert - binding should work now
    assert_eq!(text("NewChild"), target.text());
}

#[test]
fn two_way_binding_single_link_should_update_via_on_next() {
    // Arrange
    let source = TestViewModel::new("Test", None);
    let target = TestTarget::new();

    // Create a binding with a single link (source.Name)
    let binding = two_way(0);
    let expression = instance(&binding, &source);

    // Act - set up binding
    bind(&target, &expression);

    // Act - update via OnNext
    expression.on_next(text("UpdatedViaOnNext"));

    // Assert
    assert_eq!("UpdatedViaOnNext", source.name());
    assert_eq!(text("UpdatedViaOnNext"), target.text());
}

#[test]
fn two_way_binding_two_links_should_update_via_on_next() {
    // Arrange
    let child = TestViewModel::new("Child", None);
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with two links (source.Child.Name)
    let binding = two_way(1);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - update via OnNext
    expression.on_next(text("UpdatedViaOnNext"));

    // Assert
    assert_eq!("UpdatedViaOnNext", child.name());
    assert_eq!(text("UpdatedViaOnNext"), target.text());

    // Act - change intermediate link
    let new_child = TestViewModel::new("NewChild", None);
    source.set_child(Some(new_child.clone()));

    // Assert
    assert_eq!(text("NewChild"), target.text());

    // Act - update via OnNext after changing intermediate
    expression.on_next(text("AfterIntermediateChange"));

    // Assert
    assert_eq!("AfterIntermediateChange", new_child.name());
}

#[test]
fn two_way_binding_three_links_should_update_via_on_next() {
    // Arrange
    let grand_child = TestViewModel::new("GrandChild", None);
    let child = TestViewModel::new("Child", Some(grand_child.clone()));
    let source = TestViewModel::new("Parent", Some(child.clone()));
    let target = TestTarget::new();

    // Create a binding with three links (source.Child.Child.Name)
    let binding = two_way(2);
    let expression = instance(&binding, &source);

    // Act - initialize binding
    bind(&target, &expression);

    // Act - update via OnNext
    expression.on_next(text("UpdatedViaOnNext"));

    // Assert
    assert_eq!("UpdatedViaOnNext", grand_child.name());

    // Act - change intermediate node
    let new_grand_child = TestViewModel::new("NewGrandChild", None);
    child.set_child(Some(new_grand_child.clone()));

    // Assert initial update
    assert_eq!(text("NewGrandChild"), target.text());

    // Act - update via OnNext after changing intermediate
    expression.on_next(text("AfterIntermediateChange"));

    // Assert
    assert_eq!("AfterIntermediateChange", new_grand_child.name());
}

#[test]
fn two_way_binding_should_propagate_is_expanded_toggle() {
    // Arrange
    let node = TestExpandableNode::new(false);
    let target = TestExpandableTarget::new();

    // Create a binding for IsExpanded property (like in FileTreeNodeModel)
    let binding = is_expanded_binding();
    let expression = AccessorBinding::new(binding.clone(), Some(node.clone()));

    // Act - initialize binding
    target.bind(&expression, &binding, &node);

    // Assert initial state
    assert!(!target.is_expanded());
    assert!(!node.is_expanded());
    assert_eq!(0, node.expand_call_count());
    assert_eq!(0, node.collapse_call_count());

    // Act - simulate ToggleExpandedCommand by directly toggling the property
    node.set_is_expanded(true);

    // Assert expanded state propagated to target
    assert!(target.is_expanded());
    assert!(node.is_expanded());
    assert_eq!(1, node.expand_call_count());
    assert_eq!(0, node.collapse_call_count());

    // Act - toggle back to collapsed
    node.set_is_expanded(false);

    // Assert collapsed state propagated to target
    assert!(!target.is_expanded());
    assert!(!node.is_expanded());
    assert_eq!(1, node.expand_call_count());
    assert_eq!(1, node.collapse_call_count());

    // Act - toggle again
    node.set_is_expanded(true);

    // Assert expanded state propagated to target
    assert!(target.is_expanded());
    assert!(node.is_expanded());
    assert_eq!(2, node.expand_call_count());
    assert_eq!(1, node.collapse_call_count());
}

#[test]
fn two_way_binding_should_work_with_multiple_binding_expressions() {
    // Arrange
    let node = TestExpandableNode::new(false);
    let target1 = TestExpandableTarget::new();
    let target2 = TestExpandableTarget::new();

    // Create a binding for IsExpanded property
    let binding = is_expanded_binding();

    // Create two separate binding expressions from the same binding
    let expression1 = AccessorBinding::new(binding.clone(), Some(node.clone()));
    let expression2 = AccessorBinding::new(binding.clone(), Some(node.clone()));

    // Initialize bindings
    target1.bind(&expression1, &binding, &node);
    target2.bind(&expression2, &binding, &node);

    // Act - change from model
    node.set_is_expanded(true);

    // Assert both targets updated
    assert!(target1.is_expanded());
    assert!(target2.is_expanded());

    // Act - change from one target
    target1.set_is_expanded(false);

    // Assert all synchronized
    assert!(!node.is_expanded());
    assert!(!target2.is_expanded());
}

#[test]
fn two_way_binding_should_work_with_multiple_binding_expressions_and_long_chain() {
    // Create a deep chain of 5 TestViewModels
    let level5 = TestViewModel::new("Level5", None);
    let level4 = TestViewModel::new("Level4", Some(level5.clone()));
    let level3 = TestViewModel::new("Level3", Some(level4));
    let level2 = TestViewModel::new("Level2", Some(level3.clone()));
    let level1 = TestViewModel::new("Level1", Some(level2));
    let root = TestViewModel::new("Root", Some(level1.clone()));

    // Create simple targets with a Text property
    let target1 = TestTarget::new();
    let target2 = TestTarget::new();

    // Create a binding for the deep Name property (5 levels deep)
    let binding = two_way(5);

    // Create two separate binding expressions from the same binding
    let expression1 = instance(&binding, &root);
    let expression2 = instance(&binding, &root);

    // Initialize bindings
    bind(&target1, &expression1);
    bind(&target2, &expression2);

    // Assert initial binding
    assert_eq!(text("Level5"), target1.text());
    assert_eq!(text("Level5"), target2.text());

    // Act - change leaf node property
    level5.set_name("Updated Level5");

    // Assert both targets updated
    assert_eq!(text("Updated Level5"), target1.text());
    assert_eq!(text("Updated Level5"), target2.text());

    // Act - replace a middle node in the chain
    let new_level4 = TestViewModel::new("New Level4", Some(TestViewModel::new("New Level5", None)));
    level3.set_child(Some(new_level4.clone()));

    // Assert both targets updated to the new path
    assert_eq!(text("New Level5"), target1.text());
    assert_eq!(text("New Level5"), target2.text());

    // Act - change leaf node property on the old path
    level5.set_name("Changed from Old Path");

    // Assert both targets still point to the new path
    assert_eq!(text("New Level5"), target1.text());
    assert_eq!(text("New Level5"), target2.text());

    // Act - update from target after chain replacement
    target1.set_text("Changed from Target1");
    write(&binding, &root, target1.text());

    // Assert the new leaf node and other target got updated
    assert_eq!("Changed from Target1", new_level4.child().expect("the new fifth level").name());
    assert_eq!(text("Changed from Target1"), target2.text());

    // Act - replace another node higher in the chain
    let new_level2 = TestViewModel::new(
        "New Level2",
        Some(TestViewModel::new(
            "Newest Level3",
            Some(TestViewModel::new("Newest Level4", Some(TestViewModel::new("Newest Level5", None)))),
        )),
    );
    level1.set_child(Some(new_level2.clone()));

    // Assert both targets updated to the new deeply nested chain
    assert_eq!(text("Newest Level5"), target1.text());
    assert_eq!(text("Newest Level5"), target2.text());

    // Act - update from target after major chain replacement
    target2.set_text("Final value");
    write(&binding, &root, target2.text());

    // Assert the new deep leaf node and other target got updated
    let newest_level5 = descend(&new_level2, 3).expect("the newest fifth level");
    assert_eq!("Final value", newest_level5.name());
    assert_eq!(text("Final value"), target1.text());
}
