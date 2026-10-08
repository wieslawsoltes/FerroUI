//! The reference tests derive a `TestSelector` class from the selecting
//! items control to reach its protected members; those members are public
//! here, so `test_selector()` creates the control itself.
//!
//! Observable collections of the reference tests are notifying lists, plain
//! arrays are non-notifying items sources and view models are model types.
//! The tests that need a list box are in the `list_box_tests` module.

use crate::presenters::ItemsPresenter;
use crate::primitives::{SelectedItemsList, SelectingItemsControl, TemplatedControl, TextSearch};
use crate::selection::{ISelectionModel, SelectionModel};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::{
    items_equal, register_selectable, Control, ControlImpl, ISelectable, IItemsList, ItemsChangedHandler,
    ItemsControl, ItemsSource, ListBoxItem, SelectionChangedEventArgs, SelectionMode,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedEventArgs};
use ferroui_base::data::core::{ModelRef, Untyped, Value, ValueTypes};
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{IndexerBinding, ReflectionBinding, TemplateBinding};
use ferroui_base::input::{InputElement, InputElementImpl, NavigationDirection, TextInputEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Rect, Ref, Size, StyledElementImpl, VisualImpl,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Target = Ref<SelectingItemsControl>;

fn start() -> TestScope {
    test_scope()
}

fn prepare(target: &SelectingItemsControl) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_child(target.to_ref());
    root.set_width(100.0);
    root.set_height(100.0);
    root.styles().add(Style::with_setters(
        Selectors::is::<SelectingItemsControl>(),
        [Setter::new(TemplatedControl::template_property(), template())],
    ));

    root.execute_initial_layout_pass();
    root
}

fn template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<SelectingItemsControl>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("itemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        presenter.register_in_name_scope(&**scope).upcast()
    }))
}

/// A control item that is selectable.
#[repr(C)]
struct Item {
    base: Control,
    value: RefCell<Option<String>>,
}

ferro_class!(Item: Control);
ferro_impl_classes!(Item: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for Item {
    fn constructed(this: &Self) {
        register_selectable::<Item>();
        Self::parent_constructed(this);
    }
}

impl ISelectable for Item {
    fn is_selected(&self) -> bool {
        Item::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        Item::set_is_selected(self, value)
    }
}

impl Item {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), value: RefCell::new(None) })
    }

    fn with_value(value: &str) -> Ref<Self> {
        let item = Self::new();
        *item.value.borrow_mut() = Some(value.to_string());
        item
    }

    fn selected() -> Ref<Self> {
        let item = Self::new();
        item.set_is_selected(true);
        item
    }

    fn is_selected(&self) -> bool {
        SelectingItemsControl::get_is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        SelectingItemsControl::set_is_selected(self, value)
    }
}

/// A list that only ever signals a reset.
struct ResettingCollection {
    items: RefCell<Vec<String>>,
    handlers: RefCell<Vec<(u64, Rc<ItemsChangedHandler>)>>,
    next_token: Cell<u64>,
}

impl ResettingCollection {
    fn new(item_count: usize) -> Rc<Self> {
        Self::with_items((0..item_count).map(|x| format!("Item{x}")).collect())
    }

    fn with_items(items: Vec<String>) -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(items), handlers: RefCell::new(Vec::new()), next_token: Cell::new(1) })
    }

    fn reset(&self, items: &[&str]) {
        *self.items.borrow_mut() = items.iter().map(|x| x.to_string()).collect();
        self.raise_reset();
    }

    fn raise_reset(&self) {
        let handlers: Vec<_> = self.handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(&crate::ItemsChangedEventArgs::RESET);
        }
    }
}

impl IItemsList for ResettingCollection {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        boxed_str(&self.items.borrow()[index])
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.handlers.borrow_mut().push((token, handler));
        Some(token)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.borrow_mut().retain(|(candidate, _)| *candidate != token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn boxed(item: &Ref<Item>) -> Option<BoxedValue> {
    Some(Control::boxed(item.clone()))
}

fn item_of(list: &FerroList<Ref<Control>>, index: usize) -> Ref<Item> {
    list.get(index).cast::<Item>().unwrap()
}

/// A plain array of items.
fn item_source(items: &[Ref<Item>]) -> Option<ItemsSource> {
    Some(ItemsSource::from_items(items.iter().map(boxed)))
}

/// A notifying list of items.
fn item_list(items: &[Ref<Item>]) -> Rc<FerroList<Ref<Control>>> {
    Rc::new(FerroList::from_items(items.iter().map(|item| item.clone().upcast())))
}

/// A notifying list of strings.
fn str_list(items: &[&str]) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items(items.iter().map(|item| item.to_string())))
}

fn strs(values: &[&str]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().copied()))
}

fn source<T: Into<ItemsSource>>(list: &T) -> Option<ItemsSource>
where
    T: Clone,
{
    Some(list.clone().into())
}

fn str_of(item: &Option<BoxedValue>) -> Option<String> {
    item.as_ref().and_then(string_of)
}

fn assert_same(actual: &Option<BoxedValue>, expected: &Option<BoxedValue>) {
    assert!(items_equal(actual, expected), "the items differ");
}

fn assert_items(actual: &[Option<BoxedValue>], expected: &[Option<BoxedValue>]) {
    assert_eq!(actual.len(), expected.len());
    assert!(actual.iter().zip(expected.iter()).all(|(a, b)| items_equal(a, b)), "the items differ");
}

fn contains(items: &[Option<BoxedValue>], item: &Option<BoxedValue>) -> bool {
    items.iter().any(|candidate| items_equal(candidate, item))
}

fn target_with(items_source: Option<ItemsSource>) -> Target {
    let target = SelectingItemsControl::new();
    target.set_items_source(items_source);
    target.set_template(template());
    target
}

fn test_selector() -> Target {
    SelectingItemsControl::new()
}

fn last_selection_changed(target: &SelectingItemsControl) -> Rc<RefCell<Option<SelectionChangedEventArgs>>> {
    let received = Rc::new(RefCell::new(None));
    let result = received.clone();
    target.selection_changed(move |_, args| *received.borrow_mut() = Some(args.clone()));
    result
}

fn all_selection_changed(target: &SelectingItemsControl) -> Rc<RefCell<Vec<SelectionChangedEventArgs>>> {
    let received = Rc::new(RefCell::new(Vec::new()));
    let result = received.clone();
    target.selection_changed(move |_, args| received.borrow_mut().push(args.clone()));
    result
}

fn selected_index_property() -> &'static FerroProperty {
    SelectingItemsControl::selected_index_property().as_property()
}

fn selected_item_property() -> &'static FerroProperty {
    SelectingItemsControl::selected_item_property().as_property()
}

fn selected_items_property() -> &'static FerroProperty {
    SelectingItemsControl::selected_items_property().as_property()
}

fn reflection(path: &str) -> Rc<ReflectionBinding> {
    ReflectionBinding::new(path)
}

/// The binding to a property of another object (the indexer binding of the
/// reference: `other[!Property]`).
fn property_binding(other: &SelectingItemsControl, property: &'static FerroProperty) -> IndexerBinding {
    IndexerBinding::new(other.to_ref().upcast(), property, ferroui_base::data::BindingMode::Default)
}

#[test]
fn selected_index_should_initially_be_minus_1() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    assert_eq!(-1, target.selected_index());
}

#[test]
fn item_is_selected_should_initially_be_false() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    let _root = prepare(&target);

    assert!(!items[0].is_selected());
    assert!(!items[1].is_selected());
}

#[test]
fn setting_selected_item_should_set_item_is_selected_true() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    let _root = prepare(&target);

    target.set_selected_item(boxed(&items[1]));

    assert!(!items[0].is_selected());
    assert!(items[1].is_selected());
}

#[test]
fn setting_selected_item_before_apply_template_should_set_item_is_selected_true() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    target.set_selected_item(boxed(&items[1]));
    let _root = prepare(&target);

    assert!(!items[0].is_selected());
    assert!(items[1].is_selected());
}

#[test]
fn setting_selected_index_before_apply_template_should_set_item_is_selected_true() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    target.set_selected_index(1);
    let _root = prepare(&target);

    assert!(!items[0].is_selected());
    assert!(items[1].is_selected());
}

#[test]
fn setting_selected_item_should_set_selected_index() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    target.apply_template();
    target.set_selected_item(boxed(&items[1]));

    assert_same(&boxed(&items[1]), &target.selected_item());
    assert_eq!(1, target.selected_index());
}

#[test]
fn selected_index_item_is_updated_as_items_removed_when_last_item_is_selected() {
    let _scope = start();
    let items = str_list(&["Foo", "Bar", "FooBar"]);

    let target = target_with(source(&items));

    target.apply_template();
    target.set_selected_item(boxed_str(&items.get(2)));

    assert_eq!(Some(items.get(2)), str_of(&target.selected_item()));
    assert_eq!(2, target.selected_index());

    items.remove_at(0);

    assert_eq!(Some(items.get(1)), str_of(&target.selected_item()));
    assert_eq!(1, target.selected_index());
}

#[test]
fn setting_selected_item_to_not_present_item_should_clear_selection() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    target.apply_template();
    target.set_selected_item(boxed(&items[1]));

    assert_same(&boxed(&items[1]), &target.selected_item());
    assert_eq!(1, target.selected_index());

    target.set_selected_item(boxed(&Item::new()));

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
}

#[test]
fn setting_selected_index_should_set_selected_item() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    target.apply_template();
    target.set_selected_index(1);

    assert_same(&boxed(&items[1]), &target.selected_item());
}

#[test]
fn setting_selected_index_out_of_bounds_with_items_source_should_clear_selection() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    target.apply_template();
    target.set_selected_index(2);

    assert_eq!(-1, target.selected_index());
}

#[test]
fn setting_selected_index_out_of_bounds_without_items_source_should_keep_selection_until_items_source_is_set() {
    let _scope = start();
    let target = SelectingItemsControl::new();
    target.set_template(template());

    target.apply_template();
    target.set_selected_index(2);

    assert_eq!(2, target.selected_index());

    target.set_items_source(item_source(&[]));

    assert_eq!(-1, target.selected_index());
}

#[test]
fn setting_selected_index_without_items_source_should_keep_selection_if_index_exists_when_items_source_is_set() {
    let _scope = start();
    let target = SelectingItemsControl::new();
    target.set_template(template());

    target.apply_template();
    target.set_selected_index(2);

    assert_eq!(2, target.selected_index());

    let items = [Item::new(), Item::new(), Item::new(), Item::new()];
    target.set_items_source(item_source(&items));

    assert_eq!(2, target.selected_index());
    assert_same(&boxed(&items[2]), &target.selected_item());
}

#[test]
fn setting_selected_item_to_non_existent_item_with_items_source_should_clear_selection() {
    let _scope = start();
    let target = SelectingItemsControl::new();
    target.set_items_source(item_source(&[]));
    target.set_template(template());

    target.apply_template();
    target.set_selected_item(boxed(&Item::new()));

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn setting_selected_item_to_non_existent_item_without_items_source_should_keep_selection_until_items_source_is_set() {
    let _scope = start();
    let item = Item::new();

    let target = SelectingItemsControl::new();
    target.set_template(template());

    target.apply_template();
    target.set_selected_item(boxed(&item));

    assert_eq!(-1, target.selected_index());
    assert_same(&boxed(&item), &target.selected_item());

    target.set_items_source(item_source(&[]));

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn setting_selected_item_without_items_source_should_keep_selection_if_item_exists_when_items_source_is_set() {
    let _scope = start();
    let item = Item::new();

    let target = SelectingItemsControl::new();
    target.set_template(template());

    target.apply_template();
    target.set_selected_item(boxed(&item));

    assert_eq!(-1, target.selected_index());
    assert_same(&boxed(&item), &target.selected_item());

    target.set_items_source(item_source(&[Item::new(), Item::new(), item.clone(), Item::new()]));

    assert_eq!(2, target.selected_index());
    assert_same(&boxed(&item), &target.selected_item());
}

#[test]
fn adding_selected_item_should_update_selection() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new()]);

    let target = target_with(source(&items));

    let _root = prepare(&target);
    items.add(Item::selected().upcast());

    assert_eq!(2, target.selected_index());
    assert_same(&boxed(&item_of(&items, 2)), &target.selected_item());
}

#[test]
fn setting_items_to_null_should_clear_selection() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new()]);

    let target = target_with(source(&items));

    target.apply_template();
    target.set_selected_index(1);

    assert_same(&boxed(&item_of(&items, 1)), &target.selected_item());
    assert_eq!(1, target.selected_index());

    target.set_items_source(None);

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
}

#[test]
fn removing_selected_item_should_clear_selection() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new()]);

    let target = target_with(source(&items));

    let _root = prepare(&target);
    target.set_selected_index(1);

    assert_same(&boxed(&item_of(&items, 1)), &target.selected_item());
    assert_eq!(1, target.selected_index());

    let received_args = last_selection_changed(&target);

    let removed = item_of(&items, 1);

    items.remove_at(1);

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
    let received_args = received_args.borrow();
    let received_args = received_args.as_ref().expect("selection changed was not raised");
    assert!(received_args.added_items().is_empty());
    assert_items(received_args.removed_items(), &[boxed(&removed)]);
    assert_eq!(1, items.count());
    assert!(!item_of(&items, 0).is_selected());
}

#[test]
fn removing_selected_item_should_update_selection_with_always_selected() {
    let _scope = start();
    let item0 = Item::new();
    let item1 = Item::new();
    let items = item_list(&[item0.clone(), item1.clone()]);

    let target = test_selector();
    target.set_items_source(source(&items));
    target.set_template(template());
    target.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

    let _root = prepare(&target);
    target.set_selected_index(1);

    assert_same(&boxed(&item_of(&items, 1)), &target.selected_item());
    assert_eq!(1, target.selected_index());

    let received_args = last_selection_changed(&target);

    items.remove_at(1);

    assert_same(&boxed(&item0), &target.selected_item());
    assert_eq!(0, target.selected_index());
    let received_args = received_args.borrow();
    let received_args = received_args.as_ref().expect("selection changed was not raised");
    assert_items(received_args.added_items(), &[boxed(&item0)]);
    assert_items(received_args.removed_items(), &[boxed(&item1)]);
    assert_eq!(1, items.count());
    assert!(item_of(&items, 0).is_selected());
}

#[test]
fn removing_selected_item_should_clear_selection_with_begin_init() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new()]);

    let target = SelectingItemsControl::new();
    target.begin_init();
    target.set_items_source(source(&items));
    target.set_template(template());
    target.end_init();

    let _root = prepare(&target);
    target.set_selected_index(0);

    assert_same(&boxed(&item_of(&items, 0)), &target.selected_item());
    assert_eq!(0, target.selected_index());

    let received_args = last_selection_changed(&target);

    let removed = item_of(&items, 0);

    items.remove_at(0);

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
    let received_args = received_args.borrow();
    let received_args = received_args.as_ref().expect("selection changed was not raised");
    assert!(received_args.added_items().is_empty());
    assert_items(received_args.removed_items(), &[boxed(&removed)]);
    assert_eq!(1, items.count());
    assert!(!item_of(&items, 0).is_selected());
}

#[test]
fn replacing_selected_item_should_clear_selection() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new()]);

    let target = target_with(source(&items));

    let _root = prepare(&target);
    target.set_selected_index(1);

    assert_same(&boxed(&item_of(&items, 1)), &target.selected_item());
    assert_eq!(1, target.selected_index());

    let received_args = last_selection_changed(&target);

    let removed = item_of(&items, 1);
    items.set(1, Item::new().upcast());

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
    let received_args = received_args.borrow();
    let received_args = received_args.as_ref().expect("selection changed was not raised");
    assert!(received_args.added_items().is_empty());
    assert_items(received_args.removed_items(), &[boxed(&removed)]);
    for index in 0..items.count() {
        assert!(!item_of(&items, index).is_selected());
    }
}

#[test]
fn moving_selected_item_should_clear_selection() {
    let _scope = start();
    let items = str_list(&["foo", "bar"]);
    let target = target_with(source(&items));

    let _root = prepare(&target);
    target.set_selected_index(1);

    assert_eq!(Some(items.get(1)), str_of(&target.selected_item()));
    assert_eq!(1, target.selected_index());

    let received_args = last_selection_changed(&target);

    let removed = items.get(1);
    items.move_item(1, 0);

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
    let received_args = received_args.borrow();
    let received_args = received_args.as_ref().expect("selection changed was not raised");
    assert!(received_args.added_items().is_empty());
    assert_items(received_args.removed_items(), &[boxed_str(&removed)]);
}

#[test]
fn moving_selected_container_should_not_clear_selection() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new()]);

    let target = target_with(source(&items));

    let _root = prepare(&target);
    target.set_selected_index(1);

    assert_same(&boxed(&item_of(&items, 1)), &target.selected_item());
    assert_eq!(1, target.selected_index());

    let received_args = all_selection_changed(&target);

    let moved = item_of(&items, 1);
    items.move_item(1, 0);

    // Because the moved container is still marked as selected on the insert
    // part of the move, it will remain selected.
    assert_same(&boxed(&moved), &target.selected_item());
    assert_eq!(0, target.selected_index());
    let received_args = received_args.borrow();
    assert_eq!(2, received_args.len());
    assert_items(received_args[0].removed_items(), &[boxed(&moved)]);
    assert_items(received_args[1].added_items(), &[boxed(&moved)]);
    assert!(item_of(&items, 0).is_selected());
    assert!(!item_of(&items, 1).is_selected());
}

#[test]
fn resetting_items_collection_should_clear_selection() {
    let _scope = start();
    // The list signals a clear as a reset.
    let items = item_list(&[Item::new(), Item::new()]);

    let target = target_with(source(&items));

    target.apply_template();
    target.set_selected_index(1);

    assert_same(&boxed(&item_of(&items, 1)), &target.selected_item());
    assert_eq!(1, target.selected_index());

    items.clear();

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
}

#[test]
fn resetting_items_collection_should_raise_selection_changed() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new(), Item::new()]);

    let target = target_with(source(&items));

    let _root = prepare(&target);
    target.set_selected_index(1);

    let selected_item = item_of(&items, 1);

    let received_args = all_selection_changed(&target);

    items.clear();

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
    let received_args = received_args.borrow();
    assert_eq!(1, received_args.len());
    assert!(received_args[0].added_items().is_empty());
    assert_items(received_args[0].removed_items(), &[boxed(&selected_item)]);
}

// Not a port: the reference keeps the selected value (and its snapshot of the
// selected items) of a selection lost to a reset until the selection changes
// again, which here would keep the items alive (see
// `page::page_lifetime_tests::tab_control_cleared_items_are_freed`).
#[test]
fn resetting_items_collection_should_clear_selected_value() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new(), Item::new()]);

    let target = target_with(source(&items));

    let _root = prepare(&target);
    target.set_selected_index(1);

    let selected_item = item_of(&items, 1);
    assert_same(&boxed(&selected_item), &target.selected_value());

    items.clear();

    assert!(target.selected_value().is_none());
}

#[test]
fn resetting_items_to_empty_with_multiple_selection_should_raise_selection_changed() {
    let _scope = start();
    let items = item_list(&[Item::new(), Item::new(), Item::new()]);

    let target = test_selector();
    target.set_items_source(source(&items));
    target.set_template(template());
    target.set_selection_mode(SelectionMode::MULTIPLE);

    let _root = prepare(&target);
    target.set_selected_index(0);
    target.selection().select(2);

    let selected0 = item_of(&items, 0);
    let selected2 = item_of(&items, 2);

    let received_args = all_selection_changed(&target);

    items.clear();

    assert!(target.selected_item().is_none());
    assert_eq!(-1, target.selected_index());
    let received_args = received_args.borrow();
    assert_eq!(1, received_args.len());
    assert!(received_args[0].added_items().is_empty());
    assert_eq!(2, received_args[0].removed_items().len());
    assert!(contains(received_args[0].removed_items(), &boxed(&selected0)));
    assert!(contains(received_args[0].removed_items(), &boxed(&selected2)));
}

#[test]
fn resetting_items_with_preserved_selection_should_not_report_deselection() {
    let _scope = start();
    let items = ResettingCollection::new(3);

    let target = target_with(Some(ItemsSource::new(items.clone())));

    target.apply_template();
    target.set_selected_index(1);

    let received_args = all_selection_changed(&target);

    items.reset(&["Item2", "Item0", "Item1"]);

    assert_eq!(Some("Item1".to_string()), str_of(&target.selected_item()));
    let received_args = received_args.borrow();
    assert_eq!(1, received_args.len());
    assert!(received_args[0].removed_items().is_empty());
}

#[test]
fn raising_is_selected_changed_on_item_should_update_selection() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    let _root = prepare(&target);
    target.set_selected_item(boxed(&items[1]));

    assert!(!items[0].is_selected());
    assert!(items[1].is_selected());

    items[0].set_is_selected(true);
    items[0].raise_event(&RoutedEventArgs::with_event(SelectingItemsControl::is_selected_changed_event()));

    assert_eq!(0, target.selected_index());
    assert_same(&boxed(&items[0]), &target.selected_item());
    assert!(items[0].is_selected());
    assert!(!items[1].is_selected());
}

#[test]
fn clearing_is_selected_and_raising_is_selected_changed_on_item_should_update_selection() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    let _root = prepare(&target);
    target.set_selected_item(boxed(&items[1]));

    assert!(!items[0].is_selected());
    assert!(items[1].is_selected());

    items[1].set_is_selected(false);
    items[1].raise_event(&RoutedEventArgs::with_event(SelectingItemsControl::is_selected_changed_event()));

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn raising_is_selected_changed_on_someone_elses_item_should_not_update_selection() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    target.apply_template();
    target.set_selected_item(boxed(&items[1]));

    let not_child = Item::selected();

    target.raise_event(&RoutedEventArgs::with_event_and_source(
        SelectingItemsControl::is_selected_changed_event(),
        &not_child,
    ));

    assert_same(&target.selected_item(), &boxed(&items[1]));
}

#[test]
fn setting_selected_index_should_raise_selection_changed_event() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));

    let called = Rc::new(Cell::new(false));

    let flag = called.clone();
    let expected = boxed(&items[1]);
    target.selection_changed(move |_, e| {
        assert_eq!(1, e.added_items().len());
        assert_same(&expected, &e.added_items()[0]);
        assert!(e.removed_items().is_empty());
        flag.set(true);
    });

    target.set_selected_index(1);

    assert!(called.get());
}

#[test]
fn clearing_selected_index_should_raise_selection_changed_event() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let target = target_with(item_source(&items));
    target.set_selected_index(1);

    let _root = prepare(&target);

    let called = Rc::new(Cell::new(false));

    let flag = called.clone();
    let expected = boxed(&items[1]);
    target.selection_changed(move |_, e| {
        assert_eq!(1, e.removed_items().len());
        assert_same(&expected, &e.removed_items()[0]);
        assert!(e.added_items().is_empty());
        flag.set(true);
    });

    target.set_selected_index(-1);

    assert!(called.get());
}

#[test]
fn changing_items_source_during_selection_changed_when_selection_lost_does_not_throw() {
    let _scope = start();
    // Issue #7536.
    let target = SelectingItemsControl::new();
    target.set_items_source(source(&item_list(&[Item::new(), Item::new(), Item::new()])));
    target.set_selected_index(0);
    let raised = Rc::new(Cell::new(0));

    let counter = raised.clone();
    let weak = target.downgrade();
    target.selection_changed(move |_, _| {
        let target = weak.upgrade().unwrap();
        target.set_items_source(source(&item_list(&[Item::new(), Item::new()])));
        counter.set(counter.get() + 1);
    });

    target.set_selected_index(-1);

    assert_eq!(1, raised.get());
}

#[test]
fn setting_selected_index_should_raise_property_changed_events() {
    let _scope = start();
    let items = str_list(&["foo", "bar", "baz"]);

    let target = test_selector();
    target.set_items_source(source(&items));
    target.set_template(template());

    let selected_index_raised = Rc::new(Cell::new(0));
    let selected_item_raised = Rc::new(Cell::new(0));

    let index_counter = selected_index_raised.clone();
    let item_counter = selected_item_raised.clone();
    target.property_changed(move |e| {
        if e.property() == selected_index_property() {
            assert_eq!(Some(-1), e.get_old_value::<i32>());
            assert_eq!(1, e.get_new_value::<i32>());
            index_counter.set(index_counter.get() + 1);
        } else if e.property() == selected_item_property() {
            assert!(e.get_old_value::<Option<BoxedValue>>().unwrap().is_none());
            assert_eq!(Some("bar".to_string()), str_of(&e.get_new_value::<Option<BoxedValue>>()));
            item_counter.set(item_counter.get() + 1);
        }
    });

    target.set_selected_index(1);

    assert_eq!(1, selected_index_raised.get());
    assert_eq!(1, selected_item_raised.get());
}

#[test]
fn removing_selected_item_should_raise_property_changed_events() {
    let _scope = start();
    let items = str_list(&["foo", "bar", "baz"]);

    let target = test_selector();
    target.set_items_source(source(&items));
    target.set_template(template());

    let selected_index_raised = Rc::new(Cell::new(0));
    let selected_item_raised = Rc::new(Cell::new(0));
    target.set_selected_index(1);

    let index_counter = selected_index_raised.clone();
    target.property_changed(move |e| {
        if e.property() == selected_index_property() {
            assert_eq!(Some(1), e.get_old_value::<i32>());
            assert_eq!(-1, e.get_new_value::<i32>());
            index_counter.set(index_counter.get() + 1);
        } else if e.property() == selected_item_property() {
            assert_eq!(Some("bar".to_string()), str_of(&e.get_old_value::<Option<BoxedValue>>().unwrap()));
            assert!(e.get_new_value::<Option<BoxedValue>>().is_none());
        }
    });

    items.remove_at(1);

    assert_eq!(1, selected_index_raised.get());
    assert_eq!(0, selected_item_raised.get());
}

#[test]
fn removing_selected_item0_should_raise_property_changed_events_with_always_selected() {
    let _scope = start();
    let items = str_list(&["foo", "bar", "baz"]);

    let target = test_selector();
    target.set_items_source(source(&items));
    target.set_template(template());
    target.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

    let selected_index_raised = Rc::new(Cell::new(0));
    let selected_item_raised = Rc::new(Cell::new(0));
    target.set_selected_index(0);

    let index_counter = selected_index_raised.clone();
    let item_counter = selected_item_raised.clone();
    target.property_changed(move |e| {
        if e.property() == selected_index_property() {
            index_counter.set(index_counter.get() + 1);
        } else if e.property() == selected_item_property() {
            assert_eq!(Some("foo".to_string()), str_of(&e.get_old_value::<Option<BoxedValue>>().unwrap()));
            assert_eq!(Some("bar".to_string()), str_of(&e.get_new_value::<Option<BoxedValue>>()));
            item_counter.set(item_counter.get() + 1);
        }
    });

    items.remove_at(0);

    assert_eq!(0, selected_index_raised.get());
    assert_eq!(1, selected_item_raised.get());
}

#[test]
fn removing_selected_item1_should_raise_property_changed_events_with_always_selected() {
    let _scope = start();
    let items = str_list(&["foo", "bar", "baz"]);

    let target = test_selector();
    target.set_items_source(source(&items));
    target.set_template(template());
    target.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

    let selected_index_raised = Rc::new(Cell::new(0));
    let selected_item_raised = Rc::new(Cell::new(0));
    target.set_selected_index(1);

    let index_counter = selected_index_raised.clone();
    target.property_changed(move |e| {
        if e.property() == selected_index_property() {
            assert_eq!(Some(1), e.get_old_value::<i32>());
            assert_eq!(0, e.get_new_value::<i32>());
            index_counter.set(index_counter.get() + 1);
        } else if e.property() == selected_item_property() {
            assert_eq!(Some("bar".to_string()), str_of(&e.get_old_value::<Option<BoxedValue>>().unwrap()));
            assert_eq!(Some("foo".to_string()), str_of(&e.get_new_value::<Option<BoxedValue>>()));
        }
    });

    items.remove_at(1);

    assert_eq!(1, selected_index_raised.get());
    assert_eq!(0, selected_item_raised.get());
}

#[test]
fn removing_item_before_selection_should_raise_property_changed_events() {
    let _scope = start();
    let items = str_list(&["foo", "bar", "baz"]);

    let target = target_with(source(&items));

    let selected_index_raised = Rc::new(Cell::new(0));
    let selected_item_raised = Rc::new(Cell::new(0));
    target.set_selected_index(1);

    let index_counter = selected_index_raised.clone();
    let item_counter = selected_item_raised.clone();
    target.property_changed(move |e| {
        if e.property() == selected_index_property() {
            assert_eq!(Some(1), e.get_old_value::<i32>());
            assert_eq!(0, e.get_new_value::<i32>());
            index_counter.set(index_counter.get() + 1);
        } else if e.property() == selected_item_property() {
            item_counter.set(item_counter.get() + 1);
        }
    });

    items.remove_at(0);

    assert_eq!(1, selected_index_raised.get());
    assert_eq!(0, selected_item_raised.get());
}

#[test]
fn order_of_setting_items_and_selected_index_during_initialization_should_not_matter() {
    let _scope = start();
    let items = strs(&["Foo", "Bar"]);
    let target = SelectingItemsControl::new();

    target.begin_init();
    target.set_selected_index(1);
    target.set_items_source(items);
    target.end_init();

    let _root = prepare(&target);

    assert_eq!(1, target.selected_index());
    assert_eq!(Some("Bar".to_string()), str_of(&target.selected_item()));
}

#[test]
fn order_of_setting_items_and_selected_item_during_initialization_should_not_matter() {
    let _scope = start();
    let items = strs(&["Foo", "Bar"]);
    let target = SelectingItemsControl::new();

    target.begin_init();
    target.set_selected_item(boxed_str("Bar"));
    target.set_items_source(items);
    target.end_init();

    let _root = prepare(&target);

    assert_eq!(1, target.selected_index());
    assert_eq!(Some("Bar".to_string()), str_of(&target.selected_item()));
}

struct MasterViewModel {
    child: RefCell<Option<Rc<ChildViewModel>>>,
    property_changed: Event<str>,
}

impl MasterViewModel {
    fn new(child: Rc<ChildViewModel>) -> Rc<Self> {
        Model::new_model(Self { child: RefCell::new(Some(child)), property_changed: Event::new() })
    }

    fn child(&self) -> Option<Rc<ChildViewModel>> {
        self.child.borrow().clone()
    }

    fn set_child(&self, value: Option<Rc<ChildViewModel>>) {
        *self.child.borrow_mut() = value;
        self.property_changed.raise("Child");
    }
}

impl INotifyPropertyChanged for MasterViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(MasterViewModel, |b| b
    .notify_property_changed()
    .property::<ModelRef<ChildViewModel>>("Child", |vm| vm.child(), |vm, v| vm.set_child(v)));

struct ChildViewModel {
    items: ItemsSource,
    selected_item: RefCell<Option<BoxedValue>>,
    selected_index: Cell<i32>,
    property_changed: Event<str>,
}

impl ChildViewModel {
    fn new(items: &[Ref<Item>], selected_item: Option<BoxedValue>, selected_index: i32) -> Rc<Self> {
        ItemsSource::register_binding_conversion::<ItemsSource>();
        Model::new_model(Self {
            items: item_source(items).unwrap(),
            selected_item: RefCell::new(selected_item),
            selected_index: Cell::new(selected_index),
            property_changed: Event::new(),
        })
    }
}

impl INotifyPropertyChanged for ChildViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ChildViewModel, |b| b
    .notify_property_changed()
    .read_only::<Value<ItemsSource>>("Items", |vm| vm.items.clone())
    .property::<Untyped>("SelectedItem", |vm| vm.selected_item.borrow().clone(), |vm, v| {
        *vm.selected_item.borrow_mut() = v
    })
    .property::<Value<i32>>("SelectedIndex", |vm| vm.selected_index.get(), |vm, v| vm.selected_index.set(v)));

struct SelectionViewModel {
    selected_index: Cell<i32>,
    selected_item: RefCell<Option<BoxedValue>>,
    items: Rc<BindableList<String>>,
    selected_items: SelectedItemsList,
    property_changed: Event<str>,
}

impl SelectionViewModel {
    fn new() -> Rc<Self> {
        ItemsSource::register_binding_conversion::<Rc<BindableList<String>>>();
        ValueTypes::register_nullable::<SelectedItemsList>();
        Model::new_model(Self {
            selected_index: Cell::new(-1),
            selected_item: RefCell::new(None),
            items: BindableList::new(Vec::<String>::new()),
            selected_items: SelectedItemsList::new(),
            property_changed: Event::new(),
        })
    }

    fn selected_index(&self) -> i32 {
        self.selected_index.get()
    }

    fn set_selected_index(&self, value: i32) {
        self.selected_index.set(value);
        self.property_changed.raise("SelectedIndex");
    }

    fn selected_item(&self) -> Option<BoxedValue> {
        self.selected_item.borrow().clone()
    }

    fn set_selected_item(&self, value: Option<BoxedValue>) {
        *self.selected_item.borrow_mut() = value;
        self.property_changed.raise("SelectedItem");
    }

    fn items(&self) -> &FerroList<String> {
        self.items.items()
    }
}

impl INotifyPropertyChanged for SelectionViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(SelectionViewModel, |b| b
    .notify_property_changed()
    .property::<Value<i32>>("SelectedIndex", |vm| vm.selected_index(), |vm, v| vm.set_selected_index(v))
    .property::<Untyped>("SelectedItem", |vm| vm.selected_item(), |vm, v| vm.set_selected_item(v))
    .read_only::<Value<Rc<BindableList<String>>>>("Items", |vm| vm.items.clone())
    .read_only::<Value<SelectedItemsList>>("SelectedItems", |vm| vm.selected_items.clone()));

fn boxed_model<T: PartialEq + 'static>(model: &Rc<T>) -> Option<BoxedValue> {
    Some(model.clone())
}

fn items_source_property() -> &'static FerroProperty {
    ItemsControl::items_source_property().as_property()
}

/// Runs a test body on its own thread and fails if it does not finish in
/// time (the reference tests have a timeout of two seconds).
fn run_on_dedicated_thread(body: impl FnOnce() + Send + 'static) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        body();
        let _ = sender.send(());
    });

    match receiver.recv_timeout(std::time::Duration::from_millis(2000)) {
        Ok(()) => thread.join().unwrap(),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            if let Err(error) = thread.join() {
                std::panic::resume_unwind(error);
            }
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => panic!("the test timed out"),
    }
}

fn list_box_item(focusable: bool) -> Option<BoxedValue> {
    let item = ListBoxItem::new();
    if !focusable {
        item.set_focusable(false);
    }
    Some(Control::boxed(item))
}

#[test]
fn changing_data_context_should_not_clear_nested_view_model_selected_item() {
    let _scope = start();
    let items = [Item::new(), Item::new()];

    let vm = MasterViewModel::new(ChildViewModel::new(&items, boxed(&items[1]), 0));

    let target = SelectingItemsControl::new();
    target.set_data_context(boxed_model(&vm));
    let items_binding = reflection("Child.Items");
    let selected_binding = reflection("Child.SelectedItem");

    target.bind_binding(items_source_property(), &items_binding);
    target.bind_binding(selected_item_property(), &selected_binding);

    assert_eq!(1, target.selected_index());
    assert_same(&vm.child().unwrap().selected_item.borrow(), &target.selected_item());

    let items = [Item::with_value("Item1"), Item::with_value("Item2"), Item::with_value("Item3")];

    let vm = MasterViewModel::new(ChildViewModel::new(&items, boxed(&items[2]), 0));

    target.set_data_context(boxed_model(&vm));

    assert_eq!(2, target.selected_index());
    assert_same(&vm.child().unwrap().selected_item.borrow(), &target.selected_item());
}

#[test]
fn resetting_items_collection_should_retain_selection() {
    let _scope = start();
    let items = ResettingCollection::with_items(vec!["Foo".to_string(), "Bar".to_string(), "Baz".to_string()]);
    let target = SelectingItemsControl::new();
    target.set_items_source(Some(ItemsSource::new(items.clone())));

    target.set_selected_index(1);

    items.raise_reset();

    assert!(target.selected_index() == 1);
}

#[test]
fn mode_for_selected_index_is_two_way_by_default() {
    let _scope = start();
    let items = [Item::new(), Item::new(), Item::new()];

    let vm = MasterViewModel::new(ChildViewModel::new(&items, None, 1));

    let target = SelectingItemsControl::new();
    target.set_data_context(boxed_model(&vm));
    let items_binding = reflection("Child.Items");
    let selected_ind_binding = reflection("Child.SelectedIndex");

    target.bind_binding(items_source_property(), &items_binding);
    target.bind_binding(selected_index_property(), &selected_ind_binding);

    assert_eq!(1, target.selected_index());

    target.set_selected_index(2);

    assert_eq!(2, target.selected_index());
    assert_eq!(2, vm.child().unwrap().selected_index.get());
}

#[test]
fn move_selection_wrap_does_not_hang_with_no_focusable_controls() {
    run_on_dedicated_thread(|| {
        let _scope = start();
        // Issue #3094.
        let target = test_selector();
        target.set_template(template());
        target.items().add(list_box_item(false));
        target.items().add(list_box_item(false));
        target.set_selected_index(0);

        target.measure(Size::new(100.0, 100.0));
        target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

        target.move_selection(NavigationDirection::Next, true, false);
    });
}

#[test]
fn move_selection_skips_non_focusable_controls_when_moving_to_last_item() {
    let _scope = start();
    let target = test_selector();
    target.set_template(template());
    target.items().add(list_box_item(true));
    target.items().add(list_box_item(false));

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.move_selection(NavigationDirection::Last, true, false);

    assert_eq!(0, target.selected_index());
}

#[test]
fn move_selection_skips_non_focusable_controls_when_moving_to_first_item() {
    let _scope = start();
    let target = test_selector();
    target.set_template(template());
    target.items().add(list_box_item(false));
    target.items().add(list_box_item(true));

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.move_selection(NavigationDirection::Last, true, false);

    assert_eq!(1, target.selected_index());
}

#[test]
fn move_selection_does_not_hang_when_all_items_are_non_focusable_and_we_move_to_first_item() {
    run_on_dedicated_thread(|| {
        let _scope = start();
        let target = test_selector();
        target.set_template(template());
        target.items().add(list_box_item(false));
        target.items().add(list_box_item(false));

        target.measure(Size::new(100.0, 100.0));
        target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

        target.move_selection(NavigationDirection::First, true, false);

        assert_eq!(-1, target.selected_index());
    });
}

#[test]
fn move_selection_does_not_hang_when_all_items_are_non_focusable_and_we_move_to_last_item() {
    run_on_dedicated_thread(|| {
        let _scope = start();
        let target = test_selector();
        target.set_template(template());
        target.items().add(list_box_item(false));
        target.items().add(list_box_item(false));

        target.measure(Size::new(100.0, 100.0));
        target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

        target.move_selection(NavigationDirection::Last, true, false);

        assert_eq!(-1, target.selected_index());
    });
}

#[test]
fn move_selection_does_select_disabled_controls() {
    let _scope = start();
    // Issue #3426.
    let target = test_selector();
    target.set_template(template());
    target.items().add(list_box_item(true));
    let disabled = ListBoxItem::new();
    disabled.set_is_enabled(false);
    target.items().add(Some(Control::boxed(disabled)));
    target.set_selected_index(0);

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.move_selection(NavigationDirection::Next, true, false);

    assert_eq!(0, target.selected_index());
}

#[test]
fn pre_selecting_item_should_set_selection_after_it_was_added_when_always_selected() {
    let _scope = start();
    let target = test_selector();
    target.set_selection_mode(SelectionMode::ALWAYS_SELECTED);
    target.set_template(template());

    let second = Item::selected();

    let items = item_list(&[Item::new(), second.clone()]);

    target.set_items_source(source(&items));

    let _root = prepare(&target);

    assert_same(&boxed(&second), &target.selected_item());

    assert_eq!(1, target.selected_index());
}

#[test]
fn setting_selection_mode_should_update_selection_model() {
    let _scope = start();
    let target = test_selector();
    let model = target.selection();

    assert!(model.single_select());

    target.set_selection_mode(SelectionMode::MULTIPLE);

    assert!(!model.single_select());
}

#[test]
fn preserves_selected_item_when_items_changed() {
    // Issue #4048
    let _scope = start();
    let target = SelectingItemsControl::new();
    target.set_items_source(strs(&["foo", "bar", "baz"]));
    target.set_selected_item(boxed_str("bar"));

    let _root = prepare(&target);

    assert_eq!(1, target.selected_index());
    assert_eq!(Some("bar".to_string()), str_of(&target.selected_item()));

    target.set_items_source(strs(&["qux", "foo", "bar"]));

    assert_eq!(2, target.selected_index());
    assert_eq!(Some("bar".to_string()), str_of(&target.selected_item()));
}

#[test]
fn setting_selected_items_raises_property_changed() {
    let _scope = start();
    let target = test_selector();
    target.set_items_source(strs(&["foo", "bar", "baz"]));

    let raised = Rc::new(Cell::new(0));
    let new_value = SelectedItemsList::new();

    let _root = prepare(&target);

    let counter = raised.clone();
    let expected = new_value.clone();
    target.property_changed(move |e| {
        if e.property() == selected_items_property() {
            assert!(e.get_old_value::<Option<SelectedItemsList>>().unwrap().is_none());
            assert!(e.get_new_value::<Option<SelectedItemsList>>() == Some(expected.clone()));
            counter.set(counter.get() + 1);
        }
    });

    target.set_selected_items(Some(new_value));

    assert_eq!(1, raised.get());
}

#[test]
fn setting_selection_raises_selected_items_property_changed() {
    let _scope = start();
    let target = test_selector();
    target.set_items_source(strs(&["foo", "bar", "baz"]));

    let raised = Rc::new(Cell::new(0));
    let old_value = target.selected_items();

    let _root = prepare(&target);

    let counter = raised.clone();
    target.property_changed(move |e| {
        if e.property() == selected_items_property() {
            assert!(e.get_old_value::<Option<SelectedItemsList>>().unwrap() == old_value);
            assert!(e.get_new_value::<Option<SelectedItemsList>>().is_none());
            counter.set(counter.get() + 1);
        }
    });

    let selection: Rc<dyn ISelectionModel> = SelectionModel::<i32>::new();
    target.set_selection(Some(selection));

    assert_eq!(1, raised.get());
}

#[test]
fn handles_removing_last_item_in_two_controls_with_bound_selected_index() {
    let _scope = start();
    let items = str_list(&["foo"]);

    // Simulates problem with a tab strip and a carousel with bound selected
    // index.
    let tab_strip = test_selector();
    tab_strip.set_items_source(source(&items));
    tab_strip.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

    let carousel = test_selector();
    carousel.set_items_source(source(&items));
    carousel.bind_binding(selected_index_property(), &property_binding(&tab_strip, selected_index_property()));

    let tab_strip_raised = Rc::new(Cell::new(0));
    let carousel_raised = Rc::new(Cell::new(0));

    let counter = tab_strip_raised.clone();
    tab_strip.selection_changed(move |_, e| {
        assert_items(e.removed_items(), &[boxed_str("foo")]);
        assert!(e.added_items().is_empty());
        counter.set(counter.get() + 1);
    });

    let counter = carousel_raised.clone();
    carousel.selection_changed(move |_, e| {
        assert_items(e.removed_items(), &[boxed_str("foo")]);
        assert!(e.added_items().is_empty());
        counter.set(counter.get() + 1);
    });

    items.remove_at(0);

    assert_eq!(1, tab_strip_raised.get());
    assert_eq!(1, carousel_raised.get());
}

#[test]
fn handles_removing_last_item_in_controls_with_bound_selected_item() {
    let _scope = start();
    let items = str_list(&["foo"]);

    // Simulates problem with a tab strip and a carousel with bound selected
    // item.
    let tab_strip = test_selector();
    tab_strip.set_items_source(source(&items));
    tab_strip.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

    let carousel = test_selector();
    carousel.set_items_source(source(&items));
    carousel.bind_binding(selected_item_property(), &property_binding(&tab_strip, selected_item_property()));

    let tab_strip_raised = Rc::new(Cell::new(0));
    let carousel_raised = Rc::new(Cell::new(0));

    let counter = tab_strip_raised.clone();
    tab_strip.selection_changed(move |_, e| {
        assert_items(e.removed_items(), &[boxed_str("foo")]);
        assert!(e.added_items().is_empty());
        counter.set(counter.get() + 1);
    });

    let counter = carousel_raised.clone();
    carousel.selection_changed(move |_, e| {
        assert_items(e.removed_items(), &[boxed_str("foo")]);
        assert!(e.added_items().is_empty());
        counter.set(counter.get() + 1);
    });

    items.remove_at(0);

    assert_eq!(1, tab_strip_raised.get());
    assert_eq!(1, carousel_raised.get());
}

fn raise_text_input(target: &SelectingItemsControl, text: &str) {
    let mut args = TextInputEventArgs::new();
    args.set_routed_event(Some(InputElement::text_input_event()));
    args.text = Some(text.to_string());
    target.raise_event(&args);
}

#[test]
fn setting_is_text_search_enabled_enables_or_disables_text_search() {
    let _scope = start();
    let items = [Item::new(), Item::new()];
    TextSearch::set_text(&items[0], Some("Foo".to_string()));
    TextSearch::set_text(&items[1], Some("Bar".to_string()));

    let target = SelectingItemsControl::new();
    target.set_items_source(item_source(&items));
    target.set_template(template());
    target.set_is_text_search_enabled(false);

    let _root = prepare(&target);

    raise_text_input(&target, "Foo");

    assert!(target.selected_item().is_none());

    target.set_is_text_search_enabled(true);

    raise_text_input(&target, "Foo");

    assert_same(&boxed(&items[0]), &target.selected_item());
}

#[test]
fn does_not_write_to_bound_selected_item_when_data_context_changes() {
    let _scope = start();
    // Issue #9438.
    let vm1 = SelectionViewModel::new();
    vm1.items().add("foo".to_string());
    vm1.items().add("bar".to_string());
    vm1.set_selected_item(boxed_str("bar"));

    let vm2 = SelectionViewModel::new();
    vm2.items().add("foo".to_string());
    vm2.items().add("bar".to_string());
    vm2.set_selected_item(boxed_str("bar"));

    let target = SelectingItemsControl::new();
    target.set_data_context(boxed_model(&vm1));
    target.bind_binding(items_source_property(), &reflection("Items"));
    target.bind_binding(selected_item_property(), &reflection("SelectedItem"));
    target.set_template(template());

    assert_eq!(Some("bar".to_string()), str_of(&target.selected_item()));
    assert_eq!(1, target.selected_index());

    let selected_item_changed_raised = Rc::new(Cell::new(0));
    let counter = selected_item_changed_raised.clone();
    vm2.property_changed.add(Rc::new(move |name: &str| {
        if name == "SelectedItem" {
            counter.set(counter.get() + 1);
        }
    }));

    target.set_data_context(boxed_model(&vm2));

    assert_eq!(0, selected_item_changed_raised.get());
}

/// The tests that need a list box (and with it the virtualizing stack
/// panel, its default items panel).
mod list_box_tests {
    use super::*;
    use crate::assigned_binding::AssignedBinding;
    use crate::templates::FuncDataTemplate;
    use crate::mouse_test_helper::MouseTestHelper;
    use crate::{unbox_item, Border, Decorator, ListBox, TextBlock};
    use ferroui_base::data::core::Maybe;
    use ferroui_base::input::platform::PlatformHotkeyConfiguration;
    use ferroui_base::input::KeyboardNavigation;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{ferro_property, FerroLocator, FerroObject, StyledProperty};

    fn bind_hotkey_configuration() {
        FerroLocator::current_mutable()
            .bind::<PlatformHotkeyConfiguration>()
            .to_constant(Rc::new(PlatformHotkeyConfiguration::default()));
    }

    fn list_box(items_source: Option<ItemsSource>) -> Ref<ListBox> {
        let target = ListBox::new();
        target.set_template(template());
        target.set_items_source(items_source);
        target
    }

    fn panel_child(target: &SelectingItemsControl, index: usize) -> Ref<Control> {
        target.presenter().unwrap().panel().unwrap().children().get(index)
    }

    fn track_bring_into_view(target: &SelectingItemsControl) -> Rc<Cell<bool>> {
        let raised = Rc::new(Cell::new(false));
        let result = raised.clone();
        target.add_handler(Control::request_bring_into_view_event(), move |_, _| raised.set(true));
        result
    }

    fn run_jobs() {
        Dispatcher::ui_thread().run_jobs(None);
    }

    fn strings(items: &[Option<BoxedValue>]) -> Vec<String> {
        items.iter().map(|item| str_of(item).unwrap()).collect()
    }

    /// A root whose data context is itself. The reference class derives from
    /// the test root; this one is a logical root of its own, which is all
    /// the test needs.
    #[repr(C)]
    struct RootWithItems {
        base: Decorator,
    }

    ferro_class!(RootWithItems: Decorator);
    ferro_impl_classes!(
        RootWithItems: FerroObjectImpl,
        VisualImpl,
        LayoutableImpl,
        InteractiveImpl,
        InputElementImpl,
        ControlImpl
    );

    impl StyledElementImpl for RootWithItems {
        fn is_logical_root(_this: &Self) -> bool {
            true
        }
    }

    impl RootWithItems {
        ferro_property!(
            fn items_property() -> StyledProperty<Option<ItemsSource>> {
                FerroProperty::register::<RootWithItems, _>("Items", strs(&["a", "b", "c", "d", "e"]))
            }
        );

        ferro_property!(
            fn selected_property() -> StyledProperty<String> {
                FerroProperty::register::<RootWithItems, _>("Selected", "b".to_string())
            }
        );

        fn new() -> Ref<Self> {
            Self::items_property();
            Self::selected_property();
            instantiate(Self { base: Decorator::construct() })
        }
    }

    /// Binds now if the target is initialized, otherwise when it is (the
    /// delayed binding of the reference markup support).
    fn delayed_binding_add(target: &Ref<ListBox>, property: &'static FerroProperty, binding: Rc<ReflectionBinding>) {
        if target.is_initialized() {
            target.bind_binding(property, &binding);
        } else {
            let weak = target.downgrade();
            target.initialized(move || {
                if let Some(target) = weak.upgrade() {
                    target.bind_binding(property, &binding);
                }
            });
        }
    }

    struct ItemModel {
        id: Cell<i32>,
        name: RefCell<Option<String>>,
        property_changed: Event<str>,
    }

    impl ItemModel {
        fn new(id: i32, name: &str) -> Rc<Self> {
            Model::new_model(Self {
                id: Cell::new(id),
                name: RefCell::new(Some(name.to_string())),
                property_changed: Event::new(),
            })
        }

        fn set_id(&self, value: i32) {
            if self.id.get() != value {
                self.id.set(value);
                self.property_changed.raise("Id");
            }
        }

        fn set_name(&self, value: Option<String>) {
            if *self.name.borrow() != value {
                *self.name.borrow_mut() = value;
                self.property_changed.raise("Name");
            }
        }
    }

    impl INotifyPropertyChanged for ItemModel {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    ferro_model!(ItemModel, |b| b
        .notify_property_changed()
        .property::<Value<i32>>("Id", |vm| vm.id.get(), |vm, v| vm.set_id(v))
        .property::<Maybe<String>>("Name", |vm| vm.name.borrow().clone(), |vm, v| vm.set_name(v)));

    struct FullSelectionViewModel {
        selected_item: RefCell<Option<Rc<ItemModel>>>,
        selected_index: Cell<i32>,
        selected_value: Cell<Option<i32>>,
        items: Rc<BindableList<BoxedValue>>,
        property_changed: Event<str>,
    }

    impl FullSelectionViewModel {
        fn new(items: &[Rc<ItemModel>]) -> Rc<Self> {
            ItemsSource::register_binding_conversion::<Rc<BindableList<BoxedValue>>>();
            Model::new_model(Self {
                selected_item: RefCell::new(None),
                selected_index: Cell::new(-1),
                selected_value: Cell::new(None),
                items: BindableList::new(items.iter().map(|item| item.clone() as BoxedValue)),
                property_changed: Event::new(),
            })
        }

        fn item(&self, index: usize) -> Rc<ItemModel> {
            let item: Rc<dyn Any> = self.items.items().get(index);
            item.downcast::<ItemModel>().ok().unwrap()
        }

        fn selected_item(&self) -> Option<Rc<ItemModel>> {
            self.selected_item.borrow().clone()
        }

        fn set_selected_item(&self, value: Option<Rc<ItemModel>>) {
            if *self.selected_item.borrow() != value {
                *self.selected_item.borrow_mut() = value;
                self.property_changed.raise("SelectedItem");
            }
        }

        fn selected_index(&self) -> i32 {
            self.selected_index.get()
        }

        fn set_selected_index(&self, value: i32) {
            if self.selected_index.get() != value {
                self.selected_index.set(value);
                self.property_changed.raise("SelectedIndex");
            }
        }

        fn selected_value(&self) -> Option<i32> {
            self.selected_value.get()
        }

        fn set_selected_value(&self, value: Option<i32>) {
            if self.selected_value.get() != value {
                self.selected_value.set(value);
                self.property_changed.raise("SelectedValue");
            }
        }
    }

    impl INotifyPropertyChanged for FullSelectionViewModel {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    ferro_model!(FullSelectionViewModel, |b| b
        .notify_property_changed()
        .read_only::<Value<Rc<BindableList<BoxedValue>>>>("Items", |vm| vm.items.clone())
        .property::<ModelRef<ItemModel>>("SelectedItem", |vm| vm.selected_item(), |vm, v| vm.set_selected_item(v))
        .property::<Value<i32>>("SelectedIndex", |vm| vm.selected_index(), |vm, v| vm.set_selected_index(v))
        .property::<Maybe<i32>>("SelectedValue", |vm| vm.selected_value(), |vm, v| vm.set_selected_value(v)));

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum SelectionField {
        ItemsSource,
        SelectedItem,
        SelectedIndex,
        SelectedValue,
        SelectedValueBinding,
    }

    const SELECTION_FIELDS: [SelectionField; 5] = [
        SelectionField::ItemsSource,
        SelectionField::SelectedItem,
        SelectionField::SelectedIndex,
        SelectionField::SelectedValue,
        SelectionField::SelectedValueBinding,
    ];

    fn permutations(fields: &[SelectionField]) -> Vec<Vec<SelectionField>> {
        if fields.len() <= 1 {
            return vec![fields.to_vec()];
        }

        let mut result = Vec::new();
        for i in 0..fields.len() {
            let mut rest = fields.to_vec();
            let first = rest.remove(i);
            for mut tail in permutations(&rest) {
                tail.insert(0, first);
                result.push(tail);
            }
        }
        result
    }

    fn get_selection_field_permutation_parameters() -> Vec<Vec<SelectionField>> {
        permutations(&SELECTION_FIELDS)
    }

    fn test_selection_fields(set_item2: impl Fn(&FullSelectionViewModel), fields: &[SelectionField]) {
        let vm = FullSelectionViewModel::new(&[
            ItemModel::new(10, "Item0"),
            ItemModel::new(11, "Item1"),
            ItemModel::new(12, "Item2"),
            ItemModel::new(13, "Item3"),
        ]);

        set_item2(&vm);

        let root = TestRoot::new();
        root.set_width(100.0);
        root.set_height(100.0);

        // Match the begin/end init sequence emitted by the markup compiler.
        root.begin_init();
        let target = ListBox::new();
        target.begin_init();
        root.set_child(target.clone());
        target.set_data_context(boxed_model(&vm));

        for field in fields {
            match field {
                SelectionField::ItemsSource => {
                    target.bind_binding(items_source_property(), &reflection("Items"));
                }
                SelectionField::SelectedItem => {
                    target.bind_binding(selected_item_property(), &reflection("SelectedItem"));
                }
                SelectionField::SelectedIndex => {
                    target.bind_binding(selected_index_property(), &reflection("SelectedIndex"));
                }
                SelectionField::SelectedValue => {
                    target.bind_binding(
                        SelectingItemsControl::selected_value_property().as_property(),
                        &reflection("SelectedValue"),
                    );
                }
                SelectionField::SelectedValueBinding => {
                    target.set_selected_value_binding(Some(AssignedBinding::new(Rc::new(reflection("Id")))));
                }
            }
        }

        target.end_init();
        root.end_init();

        let item2 = boxed_model(&vm.item(2));
        assert!(items_equal(&item2, &target.selected_item()), "{fields:?}");
        assert_eq!(2, target.selected_index(), "{fields:?}");
        assert_eq!(Some(12), unbox_item::<i32>(&target.selected_value()), "{fields:?}");

        assert!(items_equal(&item2, &vm.selected_item().as_ref().and_then(boxed_model)), "{fields:?}");
        assert_eq!(2, vm.selected_index(), "{fields:?}");
        assert_eq!(Some(12), vm.selected_value(), "{fields:?}");
    }

    #[test]
    fn selected_index_should_be_minus_1_after_initialize() {
        let _scope = start();
        let items = [Item::new(), Item::new()];

        let target = ListBox::new();
        target.begin_init();
        target.set_items_source(item_source(&items));
        target.set_template(template());
        target.end_init();

        assert_eq!(-1, target.selected_index());
    }

    #[test]
    fn selected_index_should_be_minus_1_without_initialize() {
        let _scope = start();
        let items = [Item::new(), Item::new()];

        let target = ListBox::new();
        target.set_items_source(item_source(&items));
        target.set_template(template());
        target.set_data_context(Some(Rc::new(())));

        assert_eq!(-1, target.selected_index());
    }

    #[test]
    fn selected_index_should_be_0_after_initialize_with_always_selected() {
        let _scope = start();
        let items = [Item::new(), Item::new()];

        let target = ListBox::new();
        target.begin_init();
        target.set_selection_mode(SelectionMode::SINGLE | SelectionMode::ALWAYS_SELECTED);
        target.set_items_source(item_source(&items));
        target.set_template(template());
        target.end_init();

        let _root = prepare(&target);

        assert_eq!(0, target.selected_index());
    }

    #[test]
    fn setting_selected_index_during_initialize_should_select_item_when_always_selected_is_used() {
        let _scope = start();

        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::SINGLE | SelectionMode::ALWAYS_SELECTED);

        list_box.begin_init();

        list_box.set_selected_index(1);
        let items = str_list(&[]);
        list_box.set_items_source(source(&items));
        items.add("A".to_string());
        items.add("B".to_string());
        items.add("C".to_string());

        list_box.end_init();

        let _root = prepare(&list_box);

        assert_eq!(Some("B".to_string()), str_of(&list_box.selected_item()));
    }

    #[test]
    fn setting_selected_index_before_initialize_should_retain_selection() {
        let _scope = start();
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::SINGLE);
        list_box.set_items_source(strs(&["foo", "bar", "baz"]));
        list_box.set_selected_index(1);

        list_box.begin_init();

        list_box.end_init();

        assert_eq!(1, list_box.selected_index());
        assert_eq!(Some("bar".to_string()), str_of(&list_box.selected_item()));
    }

    #[test]
    fn setting_selected_index_during_initialize_should_take_priority_over_previous_value() {
        let _scope = start();
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::SINGLE);
        list_box.set_items_source(strs(&["foo", "bar", "baz"]));
        list_box.set_selected_index(2);

        list_box.begin_init();

        list_box.set_selected_index(1);

        list_box.end_init();

        assert_eq!(1, list_box.selected_index());
        assert_eq!(Some("bar".to_string()), str_of(&list_box.selected_item()));
    }

    #[test]
    fn setting_selected_item_before_initialize_should_retain_selection() {
        let _scope = start();
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::SINGLE);
        list_box.set_items_source(strs(&["foo", "bar", "baz"]));
        list_box.set_selected_item(boxed_str("bar"));

        list_box.begin_init();

        list_box.end_init();

        assert_eq!(1, list_box.selected_index());
        assert_eq!(Some("bar".to_string()), str_of(&list_box.selected_item()));
    }

    #[test]
    fn setting_selected_items_before_initialize_should_retain_selection() {
        let _scope = start();
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::MULTIPLE);
        list_box.set_items_source(strs(&["foo", "bar", "baz"]));

        let selected = ["foo", "bar"];

        for v in selected {
            list_box.selected_items().unwrap().add(boxed_str(v));
        }

        list_box.begin_init();

        list_box.end_init();

        assert_eq!(selected.to_vec(), strings(&list_box.selected_items().unwrap().to_vec()));
    }

    #[test]
    fn setting_selected_items_during_initialize_should_take_priority_over_previous_value() {
        let _scope = start();
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::MULTIPLE);
        list_box.set_items_source(strs(&["foo", "bar", "baz"]));

        let selected = ["foo", "bar"];

        for v in ["bar", "baz"] {
            list_box.selected_items().unwrap().add(boxed_str(v));
        }

        list_box.begin_init();

        list_box.set_selected_items(Some(SelectedItemsList::from_items(selected.iter().map(|v| boxed_str(v)))));

        list_box.end_init();

        assert_eq!(selected.to_vec(), strings(&list_box.selected_items().unwrap().to_vec()));
    }

    #[test]
    fn setting_selected_index_before_initialize_with_always_selected_should_retain_selection() {
        let _scope = start();
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::SINGLE | SelectionMode::ALWAYS_SELECTED);

        list_box.set_items_source(strs(&["foo", "bar", "baz"]));
        list_box.set_selected_index(1);

        list_box.begin_init();

        list_box.end_init();

        assert_eq!(1, list_box.selected_index());
        assert_eq!(Some("bar".to_string()), str_of(&list_box.selected_item()));
    }

    #[test]
    fn nested_list_box_does_not_change_parent_selected_index() {
        let _scope = start();

        let nested = ListBox::new();
        nested.set_template(template());
        nested.set_items_source(strs(&["foo", "bar"]));
        nested.set_selected_index(1);

        let root = SelectingItemsControl::new();
        root.set_template(template());
        root.set_items_source(Some(ItemsSource::from_items([
            Some(Control::boxed(Border::new())),
            Some(Control::boxed(nested.clone())),
        ])));
        root.set_selected_index(0);

        root.apply_template();
        root.presenter().unwrap().apply_template();
        nested.apply_template();
        nested.presenter().unwrap().apply_template();

        assert_eq!(0, root.selected_index());
        assert_eq!(1, nested.selected_index());

        nested.set_selected_index(0);

        assert_eq!(0, root.selected_index());
    }

    #[test]
    fn tab_once_active_element_should_be_initialized_with_selected_item() {
        let _scope = start();
        let target = list_box(strs(&["Foo", "Bar", "Baz "]));
        target.set_selected_index(1);

        let _root = prepare(&target);

        let container = target.container_from_index(1).unwrap();
        assert_eq!(Some(container.upcast()), KeyboardNavigation::get_tab_once_active_element(&target));
    }

    #[test]
    fn setting_selected_item_with_pointer_should_set_tab_once_active_element() {
        let _scope = start();
        let helper = MouseTestHelper::new();
        let target = list_box(strs(&["Foo", "Bar", "Baz "]));
        bind_hotkey_configuration();
        let _root = prepare(&target);

        let container = target.container_from_index(1).unwrap();
        helper.down(&container);

        assert_eq!(Some(container.upcast()), KeyboardNavigation::get_tab_once_active_element(&target));
    }

    #[test]
    fn removing_selected_item_should_clear_tab_once_active_element() {
        let _scope = start();
        let helper = MouseTestHelper::new();
        let items = str_list(&["Foo", "Bar", "Baz "]);

        let target = list_box(source(&items));
        bind_hotkey_configuration();
        let _root = prepare(&target);

        let panel = target.presenter().unwrap().panel().unwrap();
        helper.down(&panel.children().get(1));

        items.remove_at(1);

        assert!(KeyboardNavigation::get_tab_once_active_element(&panel).is_none());
    }

    #[test]
    fn binding_with_delayed_binding_and_initialization_where_data_context_is_root_works() {
        let _scope = start();
        // Test for #1932.
        let root = RootWithItems::new();

        root.begin_init();
        root.set_data_context(Some(Rc::new(root.clone().upcast::<FerroObject>())));

        let target = ListBox::new();
        target.begin_init();
        root.set_child(target.clone());

        delayed_binding_add(&target, items_source_property(), reflection("Items"));
        delayed_binding_add(&target, selected_item_property(), reflection("Selected"));
        target.end_init();
        root.end_init();

        assert_eq!(Some("b".to_string()), str_of(&target.selected_item()));
    }

    #[test]
    fn should_select_correct_item_when_duplicate_items_are_present() {
        let _scope = start();
        let helper = MouseTestHelper::new();
        let target = list_box(strs(&["Foo", "Bar", "Baz", "Foo", "Bar", "Baz"]));
        bind_hotkey_configuration();
        let _root = prepare(&target);
        helper.down(&panel_child(&target, 3));

        assert_eq!(3, target.selected_index());
    }

    #[test]
    fn should_apply_selected_pseudoclass_to_correct_item_when_duplicate_items_are_present() {
        let _scope = start();
        let helper = MouseTestHelper::new();
        let target = list_box(strs(&["Foo", "Bar", "Baz", "Foo", "Bar", "Baz"]));
        bind_hotkey_configuration();
        let _root = prepare(&target);
        helper.down(&panel_child(&target, 3));

        assert_eq!(
            vec![":pressed".to_string(), ":selected".to_string()],
            panel_child(&target, 3).classes().snapshot().to_vec()
        );
    }

    #[test]
    fn adding_item_before_selected_item_should_update_selected_index() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));
        target.set_selected_index(1);

        let _root = prepare(&target);

        items.insert(0, "Qux".to_string());

        assert_eq!(2, target.selected_index());
        assert_eq!(Some("Bar".to_string()), str_of(&target.selected_item()));
    }

    #[test]
    fn removing_item_before_selected_item_should_update_selected_index() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));
        target.set_selected_index(1);

        let _root = prepare(&target);

        items.remove_at(0);

        assert_eq!(0, target.selected_index());
        assert_eq!(Some("Bar".to_string()), str_of(&target.selected_item()));
    }

    #[test]
    fn binding_selected_index_selects_correct_item() {
        let _scope = start();

        // Issue #4496 (part 2)
        let items = str_list(&[]);

        let other = list_box(source(&items));
        other.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

        let target = list_box(source(&items));
        target.bind_binding(selected_index_property(), &property_binding(&other, selected_index_property()));

        let _other_root = prepare(&other);
        let _root = prepare(&target);

        items.add("Foo".to_string());

        assert_eq!(0, other.selected_index());
        assert_eq!(0, target.selected_index());
    }

    #[test]
    fn binding_selected_item_selects_correct_item() {
        let _scope = start();

        // Issue #4496 (part 2)
        let items = str_list(&[]);

        let other = list_box(source(&items));
        other.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

        let target = list_box(source(&items));
        target.bind_binding(selected_item_property(), &property_binding(&other, selected_item_property()));

        let _root = prepare(&target);
        other.apply_template();
        other.presenter().unwrap().apply_template();

        items.add("Foo".to_string());

        assert_eq!(0, other.selected_index());
        assert_eq!(0, target.selected_index());
    }

    #[test]
    fn replacing_selected_item_should_update_selected_item() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));
        target.set_selected_index(1);

        let _root = prepare(&target);

        items.set(1, "Qux".to_string());

        assert_eq!(-1, target.selected_index());
        assert!(target.selected_item().is_none());
    }

    #[test]
    fn auto_scroll_to_selected_item_causes_scroll_to_selected_item() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));

        let _root = prepare(&target);
        let raised = track_bring_into_view(&target);
        target.set_selected_index(2);
        run_jobs();
        assert!(raised.get());
    }

    #[test]
    fn auto_scroll_to_selected_item_causes_scroll_to_initial_selected_item() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));

        let raised = track_bring_into_view(&target);
        target.set_selected_index(2);
        let _root = prepare(&target);
        run_jobs();
        assert!(raised.get());
    }

    #[test]
    fn auto_scroll_to_selected_item_on_reset_works() {
        // Issue #3148
        let _scope = start();
        let items = ResettingCollection::new(100);

        let target = ListBox::new();
        target.set_items_source(Some(ItemsSource::new(items.clone())));
        target.set_item_template(Some(FuncDataTemplate::for_type::<String>(
            |x, _| {
                let text = TextBlock::new();
                text.set_text(Some(x));
                text.set_width(100.0);
                text.set_height(10.0);
                Some(text.upcast())
            },
            false,
        )));
        target.set_auto_scroll_to_selected_item(true);

        let root = prepare(&target);
        root.measure(Size::new(100.0, 100.0));
        root.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

        let panel = target.presenter().unwrap().panel().unwrap();
        assert!(panel.children().count() > 0);
        assert!(panel.children().count() < 100);

        target.set_selected_item(boxed_str("Item99"));

        // #3148 triggered here.
        items.reset(&["Item99"]);
        root.layout_manager().execute_layout_pass();

        assert_eq!(0, target.selected_index());
        assert_eq!(1, panel.children().snapshot().iter().filter(|x| x.is_visible()).count());
    }

    #[test]
    fn auto_scroll_to_selected_item_scrolls_when_reattached_to_visual_tree_if_selection_changed_while_detached_from_visual_tree(
    ) {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));
        target.set_selected_index(2);

        let root = prepare(&target);

        let raised = track_bring_into_view(&target);

        root.set_child(None);
        target.set_selected_index(1);
        root.set_child(target.clone());
        run_jobs();
        assert!(raised.get());
    }

    #[test]
    fn auto_scroll_to_selected_item_doesnt_scroll_if_reattached_to_visual_tree_with_no_selection_change() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));
        target.set_selected_index(2);

        let root = prepare(&target);

        let raised = track_bring_into_view(&target);

        root.set_child(None);
        root.set_child(target.clone());

        assert!(!raised.get());
    }

    #[test]
    fn auto_scroll_to_selected_item_causes_scroll_when_turned_on() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));
        target.set_auto_scroll_to_selected_item(false);

        let _root = prepare(&target);

        let raised = track_bring_into_view(&target);
        target.set_selected_index(2);
        run_jobs();
        assert!(!raised.get());

        target.set_auto_scroll_to_selected_item(true);
        run_jobs();
        assert!(raised.get());
    }

    #[test]
    fn auto_scroll_to_selected_item_scrolls_synchronously_when_laid_out() {
        let _scope = start();

        let items = str_list(&["Foo", "Bar", "Baz"]);

        let target = list_box(source(&items));

        let _root = prepare(&target);
        let raised = track_bring_into_view(&target);
        target.set_selected_index(2);

        assert!(raised.get());
    }

    #[test]
    fn can_set_both_selected_item_and_selected_items_during_initialization() {
        let _scope = start();

        // Issue #2969.
        let target = ListBox::new();
        let selected_items = SelectedItemsList::new();

        target.begin_init();
        target.set_template(template());
        target.set_items_source(strs(&["Foo", "Bar", "Baz"]));
        target.set_selected_items(Some(selected_items.clone()));
        target.set_selected_item(boxed_str("Bar"));
        target.end_init();

        let _root = prepare(&target);

        assert_eq!(Some("Bar".to_string()), str_of(&target.selected_item()));
        assert_eq!(1, target.selected_index());
        assert!(Some(selected_items.clone()) == target.selected_items());
        assert_eq!(vec!["Bar".to_string()], strings(&selected_items.to_vec()));
    }

    #[test]
    fn does_the_best_it_can_with_auto_selecting_view_model() {
        let _scope = start();

        // Tests the following scenario:
        //
        // - Items changes from empty to having 1 item
        // - The view model auto-selects item 0 in its collection changed
        //   handler
        // - The selection model receives the collection change
        // - And so adjusts the selected item from 0 to 1, which is past the
        //   end of the items.
        //
        // There's not much we can do about this situation because the order
        // in which collection changed handlers are called can't be known.
        // The best we can do is not select an invalid index.
        let vm = SelectionViewModel::new();

        let weak = Rc::downgrade(&vm);
        vm.items().add_collection_changed(Rc::new(move |_: &NotifyCollectionChangedEventArgs<'_, String>| {
            let vm = weak.upgrade().unwrap();
            if vm.selected_index() == -1 && vm.items().count() > 0 {
                vm.set_selected_index(0);
            }
        }));

        let target = ListBox::new();
        target.bind_binding(items_source_property(), &reflection("Items"));
        target.bind_binding(selected_index_property(), &reflection("SelectedIndex"));
        target.set_data_context(boxed_model(&vm));

        let _root = prepare(&target);

        vm.items().add("foo".to_string());
        vm.items().add("bar".to_string());

        assert_eq!(0, target.selected_index());
        assert_eq!(vec![0], target.selection().selected_indexes().to_vec());
        assert_eq!(Some("foo".to_string()), str_of(&target.selected_item()));
        assert_eq!(vec!["foo".to_string()], strings(&target.selected_items().unwrap().to_vec()));
    }

    #[test]
    fn preserves_initial_selected_items_when_bound() {
        let _scope = start();

        // Issue #4272 (there are two issues there, this addresses the second
        // one).
        let vm = SelectionViewModel::new();
        vm.items().add_range(["foo".to_string(), "bar".to_string(), "baz".to_string()]);
        vm.selected_items.add(boxed_str("bar"));

        let target = ListBox::new();
        target.bind_binding(items_source_property(), &reflection("Items"));
        target.bind_binding(selected_items_property(), &reflection("SelectedItems"));
        target.set_data_context(boxed_model(&vm));

        let _root = prepare(&target);

        assert_eq!(1, target.selected_index());
        assert_eq!(vec![1], target.selection().selected_indexes().to_vec());
        assert_eq!(Some("bar".to_string()), str_of(&target.selected_item()));
        assert_eq!(vec!["bar".to_string()], strings(&target.selected_items().unwrap().to_vec()));
    }

    #[test]
    fn should_first_raise_property_changed_notification_then_fire_selection_changed_event() {
        let _scope = start();

        // Issue #11006
        let items = str_list(&[]);

        let vm = SelectionViewModel::new();
        vm.set_selected_item(boxed_str(""));

        let the_list_box = ListBox::new();
        the_list_box.set_data_context(boxed_model(&vm));
        the_list_box.set_template(template());
        the_list_box.set_items_source(source(&items));
        the_list_box.set_selection_mode(SelectionMode::ALWAYS_SELECTED);
        the_list_box.bind_binding(selected_item_property(), &reflection("SelectedItem"));

        // Stands in for the text of a text box.
        let target = Rc::new(RefCell::new(String::new()));

        let _root = prepare(&the_list_box);

        items.add("Default".to_string());
        items.add("First".to_string());
        items.add("Second".to_string());
        items.add("Third".to_string());

        let text = target.clone();
        let view_model = vm.clone();
        the_list_box.selection_changed(move |_, _| {
            *text.borrow_mut() = str_of(&view_model.selected_item()).unwrap();
        });

        the_list_box.set_selected_index(1);
        assert_eq!("First", *target.borrow());

        the_list_box.set_selected_index(2);
        assert_eq!("Second", *target.borrow());

        the_list_box.set_selected_index(3);
        assert_eq!("Third", *target.borrow());
    }

    #[test]
    fn changing_data_context_respects_always_selected() {
        let _scope = start();
        ItemsSource::register_binding_conversion::<ItemsSource>();

        // Issue #12733
        let target = ListBox::new();
        target.set_data_context(Some(Rc::new(ItemsSource::from_values(0..10))));
        target.set_selection_mode(SelectionMode::ALWAYS_SELECTED);
        target.set_template(template());
        target.bind_binding(items_source_property(), &reflection(""));

        assert_eq!(0, target.selected_index());

        target.set_data_context(Some(Rc::new(ItemsSource::from_values(10..20))));

        assert_eq!(0, target.selected_index());
    }

    #[test]
    fn selected_item_and_selection_properties_work_in_any_order_when_initializing() {
        let _scope = start();
        for fields in get_selection_field_permutation_parameters() {
            test_selection_fields(|vm| vm.set_selected_item(Some(vm.item(2))), &fields);
        }
    }

    #[test]
    fn selected_index_and_selection_properties_work_in_any_order_when_initializing() {
        let _scope = start();
        for fields in get_selection_field_permutation_parameters() {
            test_selection_fields(|vm| vm.set_selected_index(2), &fields);
        }
    }

    #[test]
    fn selected_value_and_selection_properties_work_in_any_order_when_initializing() {
        let _scope = start();
        for fields in get_selection_field_permutation_parameters() {
            test_selection_fields(|vm| vm.set_selected_value(Some(12)), &fields);
        }
    }

    #[test]
    fn selected_item_can_access_selection_during_init() {
        let _scope = start();

        let target = ListBox::new();
        target.begin_init();

        let item = ItemModel::new(0, "");

        let selection = SelectionModel::<Rc<ItemModel>>::new();
        selection.set_selected_item(Some(item.clone()));
        target.set_selection(Some(selection));

        assert!(Some(item) == unbox_item::<Rc<ItemModel>>(&target.selected_item()));
    }

    #[test]
    fn selected_index_can_access_selection_during_init() {
        let _scope = start();

        let target = ListBox::new();
        target.begin_init();

        let selection = SelectionModel::<Rc<ItemModel>>::new();
        selection.set_selected_index(42);
        target.set_selection(Some(selection));

        assert_eq!(42, target.selected_index());
    }

    #[test]
    fn anchor_index_can_access_selection_during_init() {
        let _scope = start();

        let target = ListBox::new();
        target.begin_init();

        let selection = SelectionModel::<Rc<ItemModel>>::new();
        selection.set_anchor_index(42);
        target.set_selection(Some(selection));

        assert_eq!(42, target.get_anchor_index());
    }
}
