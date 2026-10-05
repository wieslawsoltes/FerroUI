use crate::presenters::ItemsPresenter;
use crate::primitives::{SelectingItemsControl, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::{items_equal, AssignedBinding, ItemsControl, ItemsSource};
use ferroui_base::data::core::Maybe;
use ferroui_base::data::model::Model;
use ferroui_base::data::{BindingBase, ReflectionBinding, TemplateBinding};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{ferro_model, BoxedValue, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) struct TestClass {
    name: RefCell<Option<String>>,
    alt_property: RefCell<Option<String>>,
}

ferro_model!(TestClass, |b| b
    .property::<Maybe<String>>("Name", |x| x.name(), |x, v| x.set_name(v))
    .property::<Maybe<String>>("AltProperty", |x| x.alt_property(), |x, v| x.set_alt_property(v)));

impl TestClass {
    pub(crate) fn new(name: Option<&str>, alt: Option<&str>) -> Rc<Self> {
        Model::new_model(Self {
            name: RefCell::new(name.map(str::to_string)),
            alt_property: RefCell::new(alt.map(str::to_string)),
        })
    }

    pub(crate) fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub(crate) fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
    }

    pub(crate) fn alt_property(&self) -> Option<String> {
        self.alt_property.borrow().clone()
    }

    pub(crate) fn set_alt_property(&self, value: Option<String>) {
        *self.alt_property.borrow_mut() = value;
    }

    pub(crate) fn get_items() -> Vec<Rc<TestClass>> {
        vec![
            TestClass::new(None, None),
            TestClass::new(Some("Item1"), Some("Alt1")),
            TestClass::new(Some("Item2"), Some("Alt2")),
            TestClass::new(Some("Item3"), Some("Alt3")),
            TestClass::new(Some("Item4"), Some("Alt4")),
            TestClass::new(Some("Item5"), Some("Alt5")),
        ]
    }
}

fn item(value: &Rc<TestClass>) -> Option<BoxedValue> {
    Some(value.clone() as BoxedValue)
}

fn source(items: &[Rc<TestClass>]) -> Option<ItemsSource> {
    Some(ItemsSource::from_items(items.iter().map(item)))
}

fn binding(path: &str) -> Option<AssignedBinding> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new(path);
    Some(AssignedBinding::new(binding))
}

fn name_of(value: &Rc<TestClass>) -> Option<BoxedValue> {
    value.name().map(|name| Rc::new(name) as BoxedValue)
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

fn prepare(target: &Ref<SelectingItemsControl>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_child(target.clone());
    root.set_width(100.0);
    root.set_height(100.0);
    root.styles().add(Style::with_setters(
        Selectors::is::<SelectingItemsControl>(),
        [Setter::new(TemplatedControl::template_property(), template())],
    ));

    root.execute_initial_layout_pass();
    root
}

#[test]
fn setting_selected_item_sets_selected_value() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_selected_value_binding(binding("Name"));
    sic.set_template(template());

    sic.set_selected_item(item(&items[1]));

    assert!(items_equal(&name_of(&items[1]), &sic.selected_value()));
}

#[test]
fn setting_selected_index_sets_selected_value() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_selected_value_binding(binding("Name"));
    sic.set_template(template());

    sic.set_selected_index(1);

    assert!(items_equal(&name_of(&items[1]), &sic.selected_value()));
}


#[test]
fn setting_selected_items_sets_selected_value() {
    use crate::primitives::SelectedItemsList;
    use crate::ListBox;

    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = ListBox::new();
    sic.set_items_source(source(&items));
    sic.set_selected_value_binding(binding("Name"));
    sic.set_template(template());

    sic.set_selected_items(Some(SelectedItemsList::from_items([item(&items[2]), item(&items[4]), item(&items[5])])));

    // When interacting, the selected item is the first item in the selected
    // items collection. But when set here, it's the last.
    assert!(items_equal(&name_of(&items[5]), &sic.selected_value()));
}

#[test]
fn setting_selected_value_sets_selected_index() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_selected_value_binding(binding("Name"));
    sic.set_template(template());

    let _root = prepare(&sic);

    sic.set_selected_value(name_of(&items[2]));

    assert_eq!(2, sic.selected_index());
}

#[test]
fn setting_selected_value_sets_selected_item() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_selected_value_binding(binding("Name"));
    sic.set_template(template());

    let _root = prepare(&sic);

    sic.set_selected_value(boxed_str("Item2"));

    assert!(items_equal(&item(&items[2]), &sic.selected_item()));
}

#[test]
fn changing_selected_value_binding_updates_selected_value() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_selected_value_binding(binding("Name"));
    sic.set_template(template());

    sic.set_selected_value(boxed_str("Item2"));

    sic.set_selected_value_binding(binding("AltProperty"));

    // Ensure the selected item didn't change.
    assert!(items_equal(&item(&items[2]), &sic.selected_item()));

    assert!(items_equal(&boxed_str("Alt2"), &sic.selected_value()));
}

#[test]
fn selected_value_with_null_selected_value_binding_is_item() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_template(template());

    sic.set_selected_index(1);

    assert!(items_equal(&item(&items[1]), &sic.selected_value()));
}

#[test]
fn setting_selected_value_before_initialize_should_retain_selection() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_template(template());
    sic.set_selected_value_binding(binding("Name"));
    sic.set_selected_value(boxed_str("Item2"));

    sic.begin_init();
    sic.end_init();

    assert!(items_equal(&name_of(&items[2]), &sic.selected_value()));
}

#[test]
fn setting_selected_value_to_non_existent_item_without_items_source_should_keep_selection_until_items_source_is_set() {
    let _scope = test_scope();
    let target = SelectingItemsControl::new();
    target.set_template(template());
    target.set_selected_value_binding(binding("Name"));

    target.apply_template();
    let value = boxed_str("Item2");
    target.set_selected_value(value.clone());

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
    assert!(Rc::ptr_eq(value.as_ref().unwrap(), target.selected_value().as_ref().unwrap()));

    target.set_items_source(source(&[]));

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
    assert!(target.selected_value().is_none());
}

#[test]
fn setting_selected_value_without_items_source_should_keep_selection_if_item_exists_when_items_source_is_set() {
    let _scope = test_scope();
    let target = SelectingItemsControl::new();
    target.set_template(template());
    target.set_selected_value_binding(binding("Name"));

    target.apply_template();
    let value = boxed_str("Item2");
    target.set_selected_value(value.clone());

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
    assert!(Rc::ptr_eq(value.as_ref().unwrap(), target.selected_value().as_ref().unwrap()));

    let items = TestClass::get_items();
    target.set_items_source(source(&items));

    assert_eq!(2, target.selected_index());
    assert!(Rc::ptr_eq(item(&items[2]).as_ref().unwrap(), target.selected_item().as_ref().unwrap()));
    assert!(items_equal(&boxed_str("Item2"), &target.selected_value()));
}

#[test]
fn setting_selected_value_during_initialize_should_take_priority_over_previous_value() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_template(template());
    sic.set_selected_value_binding(binding("Name"));
    sic.set_selected_value(boxed_str("Item2"));

    sic.begin_init();
    sic.set_selected_value(boxed_str("Item1"));
    sic.end_init();

    assert!(items_equal(&name_of(&items[1]), &sic.selected_value()));
}

#[test]
fn changing_items_should_clear_selected_value() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_template(template());
    sic.set_selected_value_binding(binding("Name"));
    sic.set_selected_value(boxed_str("Item2"));

    let _root = prepare(&sic);

    sic.set_items_source(source(&[TestClass::new(Some("NewItem"), Some(""))]));

    assert!(sic.selected_value().is_none());
}

#[test]
fn setting_selected_value_should_raise_selection_changed_event() {
    // Unlike the selected index/selected item tests, we need the items
    // control to initialize so that the selected value can actually be
    // looked up.
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_template(template());
    sic.set_selected_value_binding(binding("Name"));

    let _root = prepare(&sic);

    let called = Rc::new(Cell::new(false));
    let (flag, expected) = (called.clone(), item(&items[2]));
    sic.selection_changed(move |_, e| {
        assert_eq!(1, e.added_items().len());
        assert!(Rc::ptr_eq(expected.as_ref().unwrap(), e.added_items()[0].as_ref().unwrap()));
        assert!(e.removed_items().is_empty());
        flag.set(true);
    });

    sic.set_selected_value(boxed_str("Item2"));
    assert!(called.get());
}

#[test]
fn clearing_selected_value_should_raise_selection_changed_event() {
    let _scope = test_scope();
    let items = TestClass::get_items();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(source(&items));
    sic.set_template(template());
    sic.set_selected_value_binding(binding("Name"));
    sic.set_selected_value(boxed_str("Item2"));

    let called = Rc::new(Cell::new(false));
    let (flag, expected) = (called.clone(), item(&items[2]));
    sic.selection_changed(move |_, e| {
        assert_eq!(1, e.removed_items().len());
        assert!(Rc::ptr_eq(expected.as_ref().unwrap(), e.removed_items()[0].as_ref().unwrap()));
        assert!(e.added_items().is_empty());
        flag.set(true);
    });

    sic.set_selected_value(None);
    assert!(called.get());
}

#[test]
fn handles_null_selected_item_when_selected_value_binding_assigned() {
    // Issue #11220
    let _scope = test_scope();
    let sic = SelectingItemsControl::new();
    sic.set_items_source(Some(ItemsSource::from_items([None])));
    sic.set_selected_index(1);
    sic.set_selected_value_binding(binding("Name"));
    sic.set_template(template());

    assert!(sic.selected_value().is_none());
}
