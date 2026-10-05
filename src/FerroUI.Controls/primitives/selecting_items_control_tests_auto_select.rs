use crate::items_source::{IItemsList, ItemsChangedHandler};
use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{SelectingItemsControl, SelectingItemsControlImpl, TemplatedControlImpl};
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
    ITemplateOf,
};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::{
    items_equal, ContentControl, Control, ControlImpl, ItemsControl, ItemsControlImpl, ItemsControlImplExt,
    ItemsSource, ListBox, ListBoxItem, Panel, ScrollViewer, SelectionMode, StackPanel, TextBlock,
    VirtualizingStackPanel,
};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::BindingPriority;
use ferroui_base::threading::Dispatcher;
use ferroui_base::collections::FerroList;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroObjectImplExt, Ref, StyledElementImpl, VisualImpl,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<SelectingItemsControl>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("itemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        presenter.register_in_name_scope(&**scope).upcast()
    }))
}

fn strs(values: &[&str]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().copied()))
}

fn string_list(values: &[&str]) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items(values.iter().map(|value| value.to_string())))
}

/// Initializes and lays out `target` the way it's done in markup. This is
/// important for some tests, as the selection model isn't committed yet
/// while doing so.
fn init_with_begin_end_init(target: &SelectingItemsControl, items: ItemsSource) -> Ref<TestRoot> {
    target.begin_init();
    target.set_template(template());
    target.set_items_source(Some(items));
    target.end_init();

    let root = TestRoot::with_child(target.to_ref());
    root.execute_initial_layout_pass();
    root
}

/// Overrides the default selection mode of the class `T` on the current
/// thread, once.
fn override_selection_mode<T: ferroui_base::ObjectType>(done: &'static std::thread::LocalKey<Cell<bool>>) {
    if !done.replace(true) {
        SelectingItemsControl::selection_mode_property().override_default_value::<T>(SelectionMode::ALWAYS_SELECTED);
    }
}

#[repr(C)]
struct TestSelector {
    base: SelectingItemsControl,
}

ferro_class!(TestSelector: SelectingItemsControl);
ferro_impl_classes!(
    TestSelector: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for TestSelector {
    fn constructed(this: &Self) {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        override_selection_mode::<TestSelector>(&DONE);
        Self::parent_constructed(this);
    }
}

impl TestSelector {
    fn new() -> Ref<Self> {
        instantiate(Self { base: SelectingItemsControl::construct() })
    }
}

/// A list that notifies of a reset when an item is added.
struct ResetOnAdd {
    items: RefCell<Vec<String>>,
    handlers: RefCell<Vec<(u64, Rc<ItemsChangedHandler>)>>,
    next_token: Cell<u64>,
}

impl ResetOnAdd {
    fn new() -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(Vec::new()), handlers: RefCell::new(Vec::new()), next_token: Cell::new(1) })
    }

    fn add(&self, item: &str) {
        self.items.borrow_mut().push(item.to_string());
        let handlers: Vec<Rc<ItemsChangedHandler>> =
            self.handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(&crate::ItemsChangedEventArgs::RESET);
        }
    }
}

impl IItemsList for ResetOnAdd {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Rc::new(self.items.borrow()[index].clone()))
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

/// A selector with `ALWAYS_SELECTED` that hides the first N containers
/// during preparation. This simulates a view model scenario where an item
/// container theme binding makes some containers invisible.
#[repr(C)]
struct AlwaysSelectedTestSelectorHidingFirstContainers {
    base: SelectingItemsControl,
    hidden_count: i32,
}

ferro_class!(AlwaysSelectedTestSelectorHidingFirstContainers: SelectingItemsControl);
ferro_impl_classes!(
    AlwaysSelectedTestSelectorHidingFirstContainers: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for AlwaysSelectedTestSelectorHidingFirstContainers {
    fn constructed(this: &Self) {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        override_selection_mode::<AlwaysSelectedTestSelectorHidingFirstContainers>(&DONE);
        Self::parent_constructed(this);
    }
}

impl ItemsControlImpl for AlwaysSelectedTestSelectorHidingFirstContainers {
    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);
        if index < this.hidden_count {
            container.set_is_visible(false);
        }
    }
}

impl AlwaysSelectedTestSelectorHidingFirstContainers {
    fn new(hidden_count: i32) -> Ref<Self> {
        instantiate(Self { base: SelectingItemsControl::construct(), hidden_count })
    }
}

/// A selector with `ALWAYS_SELECTED` that disables the first N containers
/// during preparation. This simulates a view model scenario where an item
/// container theme binding disables some containers.
#[repr(C)]
struct AlwaysSelectedTestSelectorDisablingFirstContainers {
    base: SelectingItemsControl,
    disabled_count: i32,
}

ferro_class!(AlwaysSelectedTestSelectorDisablingFirstContainers: SelectingItemsControl);
ferro_impl_classes!(
    AlwaysSelectedTestSelectorDisablingFirstContainers: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for AlwaysSelectedTestSelectorDisablingFirstContainers {
    fn constructed(this: &Self) {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        override_selection_mode::<AlwaysSelectedTestSelectorDisablingFirstContainers>(&DONE);
        Self::parent_constructed(this);
    }
}

impl ItemsControlImpl for AlwaysSelectedTestSelectorDisablingFirstContainers {
    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);
        if index < this.disabled_count {
            container.set_is_enabled(false);
        }
    }
}

impl AlwaysSelectedTestSelectorDisablingFirstContainers {
    fn new(disabled_count: i32) -> Ref<Self> {
        instantiate(Self { base: SelectingItemsControl::construct(), disabled_count })
    }
}

#[test]
fn first_item_should_be_selected() {
    let _scope = test_scope();
    let target = TestSelector::new();
    target.set_items_source(strs(&["foo", "bar"]));
    target.set_template(template());

    target.apply_template();

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("foo"), &target.selected_item()));
}

#[test]
fn first_item_should_be_selected_when_added() {
    let _scope = test_scope();
    let items = string_list(&[]);
    let target = TestSelector::new();
    target.set_items_source(Some(items.clone().into()));
    target.set_template(template());

    target.apply_template();
    items.add("foo".to_string());

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("foo"), &target.selected_item()));
}

#[test]
fn first_item_should_be_selected_when_reset() {
    let _scope = test_scope();
    let items = ResetOnAdd::new();
    let target = TestSelector::new();
    target.set_items_source(Some(ItemsSource::new(items.clone())));
    target.set_template(template());

    target.apply_template();
    items.add("foo");

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("foo"), &target.selected_item()));
}

#[test]
fn item_should_be_selected_when_selection_removed() {
    let _scope = test_scope();
    let items = string_list(&["foo", "bar", "baz", "qux"]);

    let target = TestSelector::new();
    target.set_items_source(Some(items.clone().into()));
    target.set_template(template());

    target.apply_template();
    target.set_selected_index(2);
    items.remove_at(2);

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("foo"), &target.selected_item()));
}

#[test]
fn can_change_selection_from_selection_changed_when_always_selected_reselects() {
    // Regression test for the #7536 fix: clearing the selection makes
    // `ALWAYS_SELECTED` reselect the first item (via the lost selection
    // event), which raises the selection changed event. A handler that
    // changes the selection from there must be honoured rather than
    // swallowed by the batch update that wraps the lost selection handler.
    let _scope = test_scope();
    let target = TestSelector::new();
    target.set_items_source(strs(&["foo", "bar", "baz"]));
    target.set_template(template());

    target.apply_template();
    target.set_selected_index(1);

    let raised = Cell::new(0);
    let weak = target.downgrade();
    target.selection_changed(move |_, _| {
        raised.set(raised.get() + 1);
        if raised.get() == 1 {
            weak.upgrade().unwrap().set_selected_index(2);
        }
    });

    target.set_selected_index(-1);

    assert_eq!(2, target.selected_index());
    assert!(items_equal(&boxed_str("baz"), &target.selected_item()));
}

#[test]
fn selection_should_be_cleared_when_no_items_left() {
    let _scope = test_scope();
    let items = string_list(&["foo", "bar"]);

    let target = TestSelector::new();
    target.set_items_source(Some(items.clone().into()));
    target.set_template(template());

    target.apply_template();
    target.set_selected_index(1);
    items.remove_at(1);
    items.remove_at(0);

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn removing_selected_first_item_should_select_next_item() {
    let _scope = test_scope();
    let items = string_list(&["foo", "bar"]);
    let target = TestSelector::new();
    target.set_items_source(Some(items.clone().into()));
    target.set_template(template());

    target.apply_template();
    target.presenter().unwrap().apply_template();
    items.remove_at(0);

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("bar"), &target.selected_item()));
}

fn list_box_item(content: &str) -> Ref<ListBoxItem> {
    let item = ListBoxItem::new();
    item.set_content(boxed_str(content));
    item
}

#[test]
fn first_visible_item_should_be_selected_when_first_container_is_hidden() {
    // Uses own-container items (controls) so that the visibility can be set
    // before preparation.
    let _scope = test_scope();
    let hidden = list_box_item("hidden");
    hidden.set_is_visible(false);
    let visible = list_box_item("visible");

    let target = TestSelector::new();
    target.set_items_source(Some(ItemsSource::from_items([
        Some(Control::boxed(hidden)),
        Some(Control::boxed(visible)),
    ])));
    target.set_template(template());

    target.apply_template();
    target.presenter().unwrap().apply_template();

    assert_eq!(1, target.selected_index());
}

#[test]
fn first_enabled_item_should_be_selected_when_first_container_is_disabled() {
    let _scope = test_scope();
    let disabled = list_box_item("disabled");
    disabled.set_is_enabled(false);
    let enabled = list_box_item("enabled");

    let target = TestSelector::new();
    target.set_items_source(Some(ItemsSource::from_items([
        Some(Control::boxed(disabled)),
        Some(Control::boxed(enabled)),
    ])));
    target.set_template(template());

    target.apply_template();
    target.presenter().unwrap().apply_template();

    assert_eq!(1, target.selected_index());
}

#[test]
fn first_visible_item_should_be_selected_when_container_becomes_hidden_during_preparation() {
    // Regression test for pull request 20798 of the reference project.
    // Simulates a view model scenario where container visibility is set by
    // a binding applied during the preparation of the container (e.g. an
    // item container theme). Verifies that selection lands on the first
    // truly visible container rather than an unrealized item.
    let _scope = test_scope();
    let target = AlwaysSelectedTestSelectorHidingFirstContainers::new(2);
    target.set_items_source(strs(&["item-0", "item-1", "item-2"]));
    target.set_template(template());

    target.apply_template();
    target.presenter().unwrap().apply_template();

    assert_eq!(2, target.selected_index());
}

#[test]
fn selection_should_be_cleared_when_all_containers_are_hidden_during_preparation() {
    // When all containers are made invisible during preparation (view model
    // binding scenario), the selected index must be -1 rather than the last
    // container's index. Previously the selection would cascade to the last
    // unrealized item and land on an invisible one.
    let _scope = test_scope();
    let target = AlwaysSelectedTestSelectorHidingFirstContainers::new(3);
    target.set_items_source(strs(&["item-0", "item-1", "item-2"]));
    target.set_template(template());

    target.apply_template();
    target.presenter().unwrap().apply_template();

    assert_eq!(-1, target.selected_index());
}

#[test]
fn first_visible_item_should_be_selected_when_items_are_added_to_a_laid_out_control() {
    let _scope = test_scope();

    let items = string_list(&[]);
    let target = AlwaysSelectedTestSelectorHidingFirstContainers::new(1);
    let _root = init_with_begin_end_init(&target, items.clone().into());

    items.add("item-0".to_string());
    items.add("item-1".to_string());

    assert_eq!(1, target.selected_index());
    assert!(items_equal(&boxed_str("item-1"), &target.selected_item()));
}

#[test]
fn selection_should_be_cleared_when_a_hidden_item_is_added_to_a_laid_out_control() {
    let _scope = test_scope();

    let items = string_list(&[]);
    let target = AlwaysSelectedTestSelectorHidingFirstContainers::new(1);
    let _root = init_with_begin_end_init(&target, items.clone().into());

    items.add("item-0".to_string());

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn first_enabled_item_should_be_selected_when_items_are_added_to_a_laid_out_control() {
    let _scope = test_scope();

    let items = string_list(&[]);
    let target = AlwaysSelectedTestSelectorDisablingFirstContainers::new(1);
    let _root = init_with_begin_end_init(&target, items.clone().into());

    items.add("item-0".to_string());
    items.add("item-1".to_string());

    assert_eq!(1, target.selected_index());
    assert!(items_equal(&boxed_str("item-1"), &target.selected_item()));
}

#[test]
fn selection_should_settle_on_the_visible_item_when_items_are_added_to_a_laid_out_control() {
    let _scope = test_scope();

    let items = string_list(&[]);
    let target = AlwaysSelectedTestSelectorHidingFirstContainers::new(1);
    let _root = init_with_begin_end_init(&target, items.clone().into());

    let selection_changes = Rc::new(RefCell::new(Vec::new()));
    let (sink, weak) = (selection_changes.clone(), target.downgrade());
    target.selection_changed(move |_, _| sink.borrow_mut().push(weak.upgrade().unwrap().selected_item()));

    items.add("item-0".to_string());
    items.add("item-1".to_string());

    assert!(items_equal(&boxed_str("item-1"), selection_changes.borrow().last().unwrap()));
    assert!(items_equal(&boxed_str("item-1"), &target.selected_item()));
}

fn create_scroll_viewer_template(parent: &Ref<ScrollViewer>, scope: &NameScopeRef) -> Ref<Control> {
    let presenter = ScrollContentPresenter::new();
    presenter.set_name(Some("PART_ContentPresenter".to_string()));
    let parent: &FerroObject = parent;
    presenter.bind(
        ContentPresenter::content_property(),
        FerroObjectExtensions::get_observable(parent, ContentControl::content_property()),
        BindingPriority::LocalValue,
    );
    presenter.register_in_name_scope(&**scope).upcast()
}

fn create_list_box_template(parent: &Ref<ListBox>, scope: &NameScopeRef) -> Ref<Control> {
    let items_presenter = ItemsPresenter::new();
    items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
    let parent: &FerroObject = parent;
    items_presenter.bind(
        ItemsPresenter::items_panel_property(),
        FerroObjectExtensions::get_observable(parent, ItemsControl::items_panel_property()),
        BindingPriority::LocalValue,
    );

    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
    scroll_viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_scroll_viewer_template)));
    scroll_viewer.set_content(Some(Control::boxed(items_presenter.register_in_name_scope(&**scope))));
    scroll_viewer.register_in_name_scope(&**scope).upcast()
}

/// A list box of 100 items of height 50 in a viewport of height 100, with a
/// virtualizing panel that realizes nothing beyond the viewport.
fn auto_scroll_list_box() -> Ref<ListBox> {
    let items: Vec<String> = (0..100).map(|i| format!("Item {i}")).collect();

    let target = ListBox::new();
    target.set_template(Some(FuncControlTemplate::for_type::<ListBox>(create_list_box_template)));
    target.set_items_source(Some(ItemsSource::from_strs(items.iter().map(String::as_str))));
    target.set_item_template(Some(FuncDataTemplate::for_type::<String>(
        |_, _| {
            let text_block = TextBlock::new();
            text_block.set_height(50.0);
            Some(text_block.upcast())
        },
        false,
    )));
    target.set_height(100.0);
    let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
        let panel = VirtualizingStackPanel::new();
        panel.set_cache_length(0.0);
        Some(panel.upcast::<Panel>())
    });
    target.set_items_panel(items_panel);
    target.set_auto_scroll_to_selected_item(true);
    target
}

fn scroll_viewer_of(target: &ListBox) -> Ref<ScrollViewer> {
    target.visual_children().get(0).cast::<ScrollViewer>().unwrap()
}

#[test]
fn auto_scroll_to_selected_item_should_work_when_becoming_visible() {
    let _scope = test_scope();

    let target = auto_scroll_list_box();
    target.set_is_visible(false);

    target.set_height(100.0);
    target.set_width(100.0);
    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();

    // Select item 50
    target.set_selected_index(50);

    // Make visible
    target.set_is_visible(true);
    target.update_layout();

    // Wait for dispatcher
    Dispatcher::ui_thread().run_jobs(None);
    target.update_layout();

    let scroll_viewer = scroll_viewer_of(&target);
    let offset = scroll_viewer.offset().y;

    // Item 50 is at 50 * 50 = 2500.
    // The list box height is 100, so it should be visible if the offset is
    // between 2400 and 2500.
    assert!((2400.0..=2500.0).contains(&offset), "offset: {offset}");
}

#[test]
fn auto_scroll_to_selected_item_should_work_when_ancestor_becomes_visible() {
    let _scope = test_scope();

    let target = auto_scroll_list_box();

    target.set_height(100.0);
    target.set_width(100.0);

    let host = StackPanel::new();
    host.set_is_visible(false);
    host.children().add(target.clone());

    let root = TestRoot::with_child(&host);
    root.execute_initial_layout_pass();

    target.set_selected_index(50);
    assert!(!target.is_effectively_visible());

    host.set_is_visible(true);
    root.layout_manager().execute_layout_pass();
    Dispatcher::ui_thread().run_jobs(None);
    root.layout_manager().execute_layout_pass();

    let scroll_viewer = scroll_viewer_of(&target);
    let offset = scroll_viewer.offset().y;
    assert!((2400.0..=2500.0).contains(&offset), "offset: {offset}");
}

#[test]
fn auto_scroll_to_selected_item_executes_within_layout_pass_when_becoming_visible() {
    let _scope = test_scope();

    let target = auto_scroll_list_box();
    target.set_is_visible(false);

    target.set_height(100.0);
    target.set_width(100.0);
    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();

    target.set_selected_index(50);
    target.set_is_visible(true);

    target.update_layout();

    let scroll_viewer = scroll_viewer_of(&target);
    let offset = scroll_viewer.offset().y;
    assert!((2400.0..=2500.0).contains(&offset), "offset: {offset}");
}
