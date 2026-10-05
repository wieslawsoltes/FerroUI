use crate::generators::RecycleKey;
use crate::i_selectable::{register_selectable, ISelectable};
use crate::mixins::SelectableMixin;
use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{
    ScrollBar, SelectedItemsList, SelectingItemsControl, SelectingItemsControlImpl, TemplatedControl, TemplatedControlImpl,
};
use crate::selection::{ISelectionModel, SelectionModel};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::{
    items_equal, Border, ContentControl, ContentControlImpl, Control, ControlImpl, ItemsControl, ItemsControlImpl,
    ItemsSource, ListBoxItem, Panel, ScrollViewer, SelectionChangedEventArgs, SelectionMode,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{Value, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingBase, BindingMode, ReflectionBinding, TemplateBinding};
use ferroui_base::input::{IKeyboardDevice, InputElementImpl, KeyboardDevice};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::SolidColorBrush;
use ferroui_base::reactive::ObservableExt;
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, BoxedValue, FerroLocator, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, StyledProperty,
    TypeInfo, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

// --- test classes ---------------------------------------------------------

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
        Self::parent_constructed(this);
        this.set_selection_mode(SelectionMode::MULTIPLE);
    }
}

impl TestSelector {
    fn construct() -> Self {
        Self { base: SelectingItemsControl::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn selected_items(&self) -> SelectedItemsList {
        self.base.selected_items().unwrap()
    }

    fn select_all(&self) {
        self.selection().select_all()
    }

    fn unselect_all(&self) {
        self.selection().clear()
    }

    fn select_range(&self, index: i32) {
        self.update_selection(index, true, true, false, false, false)
    }

    fn toggle(&self, index: i32) {
        self.update_selection(index, true, false, true, false, false)
    }
}

#[repr(C)]
struct TestSelectorWithContainers {
    base: TestSelector,
}

ferro_class!(TestSelectorWithContainers: TestSelector);
ferro_impl_classes!(
    TestSelectorWithContainers: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl StyledElementImpl for TestSelectorWithContainers {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        TestSelector::TYPE
    }
}

impl ItemsControlImpl for TestSelectorWithContainers {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        TestContainer::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<TestContainer>(item)
    }
}

impl TestSelectorWithContainers {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TestSelector::construct() })
    }
}

#[repr(C)]
struct TestContainer {
    base: ContentControl,
}

ferro_class!(TestContainer: ContentControl);
ferro_impl_classes!(
    TestContainer: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for TestContainer {
    fn constructed(this: &Self) {
        Self::class_init();
        Self::parent_constructed(this);
    }
}

impl ISelectable for TestContainer {
    fn is_selected(&self) -> bool {
        TestContainer::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        TestContainer::set_is_selected(self, value)
    }
}

impl TestContainer {
    ferro_property!(
        fn is_selected_property() -> StyledProperty<bool> {
            SelectingItemsControl::is_selected_property().add_owner::<TestContainer>()
        }
    );

    fn class_init() {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        if DONE.replace(true) {
            return;
        }

        register_selectable::<TestContainer>();
        SelectableMixin::attach::<TestContainer>(Self::is_selected_property());
    }

    fn new() -> Ref<Self> {
        instantiate(Self { base: ContentControl::construct() })
    }

    fn selected() -> Ref<Self> {
        let result = Self::new();
        result.set_is_selected(true);
        result
    }

    fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value)
    }
}

struct ItemViewModel {
    value: String,
    is_selected: Cell<bool>,
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for ItemViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ItemViewModel, |b| b
    .notify_property_changed()
    .read_only::<Value<String>>("Value", |vm| vm.value.clone())
    .property::<Value<bool>>("IsSelected", |vm| vm.is_selected(), |vm, v| vm.set_is_selected(v)));

impl ItemViewModel {
    fn new(value: &str, is_selected: bool) -> Rc<Self> {
        Model::new_model(Self {
            value: value.to_string(),
            is_selected: Cell::new(is_selected),
            property_changed: Event::new(),
        })
    }

    fn is_selected(&self) -> bool {
        self.is_selected.get()
    }

    fn set_is_selected(&self, value: bool) {
        if self.is_selected.get() != value {
            self.is_selected.set(value);
            self.property_changed.raise("IsSelected");
        }
    }
}

struct OldDataContextViewModel {
    items: ItemsSource,
    selected_items: SelectedItemsList,
    selection: Rc<SelectionModel<String>>,
}

ferro_model!(OldDataContextViewModel, |b| b
    .read_only::<Value<ItemsSource>>("Items", |vm| vm.items.clone())
    .read_only::<Value<SelectedItemsList>>("SelectedItems", |vm| vm.selected_items.clone())
    .read_only::<Value<Rc<dyn ISelectionModel>>>("Selection", |vm| vm.selection.clone()));

impl OldDataContextViewModel {
    fn new() -> Rc<Self> {
        register_binding_conversions();
        Model::new_model(Self {
            items: ItemsSource::from_strs(["foo", "bar"]),
            selected_items: SelectedItemsList::new(),
            selection: SelectionModel::new(),
        })
    }
}

/// The data context of
/// `unbound_selected_items_should_be_cleared_when_data_context_cleared`.
struct ItemsViewModel {
    items: ItemsSource,
}

ferro_model!(ItemsViewModel, |b| b.read_only::<Value<ItemsSource>>("Items", |vm| vm.items.clone()));

/// Lets bindings deliver the collections and the selection model of the
/// view models to the (nullable) properties of the control.
fn register_binding_conversions() {
    ItemsSource::register_binding_conversion::<ItemsSource>();
    ValueTypes::register_conversion::<SelectedItemsList, Option<SelectedItemsList>>(|x| Some(Some(x.clone())));
    ValueTypes::register_conversion::<Rc<dyn ISelectionModel>, Option<Rc<dyn ISelectionModel>>>(|x| {
        Some(Some(x.clone()))
    });
}

// --- helpers --------------------------------------------------------------

#[derive(Default)]
struct Options {
    data_context: Option<BoxedValue>,
    items: Option<Vec<Option<BoxedValue>>>,
    items_source: Option<ItemsSource>,
    item_container_theme: Option<Ref<ControlTheme>>,
    styles: Vec<Ref<Style>>,
    no_layout: bool,
    virtualizing: bool,
}

struct Target {
    root: Ref<TestRoot>,
    target: Ref<TestSelector>,
    _scope: TestScope,
}

impl Deref for Target {
    type Target = Ref<TestSelector>;

    fn deref(&self) -> &Ref<TestSelector> {
        &self.target
    }
}

fn start() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
    scope
}

fn create_target(configure: impl FnOnce(&mut Options)) -> Target {
    create_target_of(TestSelector::new(), configure)
}

fn create_target_of(target: Ref<TestSelector>, configure: impl FnOnce(&mut Options)) -> Target {
    let scope = start();
    let mut options = Options::default();
    configure(&mut options);

    target.set_data_context(options.data_context);
    target.set_item_container_theme(options.item_container_theme);
    target.set_item_template(None);
    target.set_items_source(options.items_source);
    target.set_selection_mode(SelectionMode::MULTIPLE);

    if let Some(items) = options.items {
        for item in items {
            target.items().add(item);
        }
    }

    if options.virtualizing {
        use crate::templates::{FuncTemplate, ITemplateOf};
        let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(crate::VirtualizingStackPanel::new().upcast::<Panel>()));
        target.set_items_panel(items_panel);
    }

    let root = create_root(&target);

    for style in options.styles {
        root.styles().add(style);
    }

    if !options.no_layout {
        root.execute_initial_layout_pass();
    }

    Target { root, target, _scope: scope }
}

fn create_root(child: &Ref<TestSelector>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.resources().add(TestSelector::TYPE, theme_resource(create_test_selector_control_theme()));
    root.resources().add(TestContainer::TYPE, theme_resource(create_test_container_theme()));
    root.resources().add(ScrollViewer::TYPE, theme_resource(create_scroll_viewer_theme()));
    root.set_child(child.clone());
    root
}

fn theme_resource(theme: Ref<ControlTheme>) -> Option<BoxedValue> {
    Some(Rc::new(theme))
}

fn create_test_selector_control_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        TestSelector::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(create_test_selector_template()))],
    )
}

fn create_test_selector_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ItemsControl>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        let border = Border::new();
        border.set_background(Some(SolidColorBrush::from_uint32(0xffffffff).into()));
        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));
        border.set_child(scroll_viewer.register_in_name_scope(&**scope));
        border.upcast()
    })
}

fn create_scroll_viewer_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        ScrollViewer::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(create_scroll_viewer_template()))],
    )
}

fn create_scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let scroll_bar = ScrollBar::new();
        scroll_bar.set_name(Some("verticalScrollBar".to_string()));
        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.children().add(scroll_bar);
        panel.upcast()
    })
}

fn create_test_container_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        TestContainer::TYPE,
        [
            Setter::new(TemplatedControl::template_property(), Some(create_test_container_template())),
            Setter::new(Layoutable::height_property(), 100.0),
        ],
    )
}

fn create_test_container_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TestContainer>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        for property in [
            ContentControl::content_property().as_property(),
            ContentControl::content_template_property().as_property(),
        ] {
            presenter.bind_binding(property, &TemplateBinding::new(property));
        }
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn layout(target: &Target) {
    target.root.layout_manager().execute_layout_pass();
}

fn selected_containers(target: &SelectingItemsControl) -> Vec<i32> {
    target
        .items_panel_root()
        .unwrap()
        .children()
        .to_vec()
        .iter()
        .map(|x| if SelectingItemsControl::get_is_selected(x) { target.index_from_container(x) } else { -1 })
        .filter(|x| *x != -1)
        .collect()
}

fn strs(values: &[&str]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().copied()))
}

fn string_list(values: &[&str]) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items(values.iter().map(|value| value.to_string())))
}

fn boxed_strs(values: &[&str]) -> Vec<Option<BoxedValue>> {
    values.iter().map(|value| boxed_str(value)).collect()
}

fn selected_list(values: &[&str]) -> Option<SelectedItemsList> {
    Some(SelectedItemsList::from_items(boxed_strs(values)))
}

fn controls<T: ferroui_base::ObjectType + ferroui_base::Upcast<Control>>(values: &[Ref<T>]) -> Vec<Option<BoxedValue>> {
    values.iter().map(|c| Some(Control::boxed(c.clone()))).collect()
}

fn list_box_items() -> Vec<Ref<ListBoxItem>> {
    vec![ListBoxItem::new(), ListBoxItem::new(), ListBoxItem::new()]
}

fn view_models(items: &[Rc<ItemViewModel>]) -> Vec<Option<BoxedValue>> {
    items.iter().map(|item| Some(item.clone() as BoxedValue)).collect()
}

fn item_view_models() -> Vec<Rc<ItemViewModel>> {
    vec![ItemViewModel::new("Item 0", true), ItemViewModel::new("Item 1", false), ItemViewModel::new("Item 2", true)]
}

fn is_selected_setter() -> Rc<Setter> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new("IsSelected");
    Setter::new_binding_base(SelectingItemsControl::is_selected_property().as_property(), binding)
}

/// Asserts that a sequence of items holds the given strings.
#[track_caller]
fn assert_strs(expected: &[&str], actual: &[Option<BoxedValue>]) {
    let actual: Vec<Option<String>> = actual.iter().map(|item| item.as_ref().and_then(string_of)).collect();
    let expected: Vec<Option<String>> = expected.iter().map(|value| Some(value.to_string())).collect();
    assert_eq!(expected, actual);
}

/// Asserts that two sequences of items are equal.
#[track_caller]
fn assert_items(expected: &[Option<BoxedValue>], actual: &[Option<BoxedValue>]) {
    assert_eq!(expected.len(), actual.len());
    assert!(expected.iter().zip(actual.iter()).all(|(a, b)| items_equal(a, b)));
}

fn selection_items(target: &SelectingItemsControl) -> Vec<Option<BoxedValue>> {
    target.selection().selected_items().to_vec()
}

fn selection_indexes(target: &SelectingItemsControl) -> Vec<i32> {
    target.selection().selected_indexes().to_vec()
}

fn on_selection_changed(target: &Target, handler: impl Fn(&SelectionChangedEventArgs) + 'static) {
    target.selection_changed(move |_, e| handler(e));
}

// --- tests ----------------------------------------------------------------

#[test]
fn setting_selected_index_should_add_to_selected_items() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.set_selected_index(1);

    assert_strs(&["bar"], &target.selected_items().to_vec());
}

#[test]
fn adding_selected_items_should_set_selected_index() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.selected_items().add(boxed_str("bar"));

    assert_eq!(1, target.selected_index());
}

#[test]
fn assigning_single_selected_items_should_set_selected_index() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.set_selected_items(selected_list(&["bar"]));

    assert_eq!(1, target.selected_index());
    assert_strs(&["bar"], &target.selected_items().to_vec());
    assert_eq!(vec![1], selected_containers(&target));
}

#[test]
fn assigning_multiple_selected_items_should_set_selected_index() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    target.set_selected_items(selected_list(&["foo", "bar", "baz"]));

    assert_eq!(0, target.selected_index());
    assert_strs(&["foo", "bar", "baz"], &target.selected_items().to_vec());
    assert_eq!(vec![0, 1, 2], selected_containers(&target));
}

#[test]
fn selected_items_should_be_marked_when_panel_created_after_selected_items_is_set() {
    // Issue #2565.
    let target = create_target(|o| {
        o.items_source = strs(&["foo", "bar", "baz"]);
        o.no_layout = true;
    });

    assert!(target.items_panel_root().is_none());
    target.set_selected_items(selected_list(&["foo", "bar", "baz"]));

    target.root.execute_initial_layout_pass();

    assert_eq!(0, target.selected_index());
    assert_strs(&["foo", "bar", "baz"], &target.selected_items().to_vec());
    assert_eq!(vec![0, 1, 2], selected_containers(&target));
}

#[test]
fn reassigning_selected_items_should_clear_selection() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.selected_items().add(boxed_str("bar"));
    target.set_selected_items(selected_list(&[]));

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn adding_first_selected_item_should_raise_selected_index_selected_item_changed() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));
    let index_raised = Rc::new(Cell::new(false));
    let item_raised = Rc::new(Cell::new(false));

    let (index_flag, item_flag) = (index_raised.clone(), item_raised.clone());
    target.property_changed(move |e| {
        index_flag.set(
            index_flag.get()
                | (e.property().name() == "SelectedIndex"
                    && e.get_old_value::<i32>() == Some(-1)
                    && e.get_new_value::<i32>() == 1),
        );
        item_flag.set(
            item_flag.get()
                | (e.property().name() == "SelectedItem"
                    && e.get_old_value::<Option<BoxedValue>>().unwrap().is_none()
                    && items_equal(&e.get_new_value::<Option<BoxedValue>>(), &boxed_str("bar"))),
        );
    });

    target.selected_items().add(boxed_str("bar"));

    assert!(index_raised.get());
    assert!(item_raised.get());
}

#[test]
fn adding_subsequent_selected_items_should_not_raise_selected_index_selected_item_changed() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.selected_items().add(boxed_str("foo"));

    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    target.property_changed(move |e| {
        flag.set(flag.get() | (e.property().name() == "SelectedIndex" || e.property().name() == "SelectedItem"))
    });

    target.selected_items().add(boxed_str("bar"));

    assert!(!raised.get());
}

#[test]
fn removing_last_selected_item_should_raise_selected_index_changed() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.selected_items().add(boxed_str("foo"));

    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    target.property_changed(move |e| {
        flag.set(
            flag.get()
                | (e.property().name() == "SelectedIndex"
                    && e.get_old_value::<i32>() == Some(0)
                    && e.get_new_value::<i32>() == -1),
        )
    });

    target.selected_items().remove_at(0);

    assert!(raised.get());
}

#[test]
fn adding_selected_items_should_set_item_is_selected() {
    let items = list_box_items();

    let target = create_target(|o| o.items = Some(controls(&items)));

    target.selected_items().add(target.items().get_at(0));
    target.selected_items().add(target.items().get_at(1));

    assert!(items[0].is_selected());
    assert!(items[1].is_selected());
    assert!(!items[2].is_selected());
}

#[test]
fn assigning_selected_items_should_set_item_is_selected() {
    let items = list_box_items();

    let target = create_target(|o| o.items = Some(controls(&items)));

    target.set_selected_items(Some(SelectedItemsList::from_items(controls(&items[..2]))));

    assert!(items[0].is_selected());
    assert!(items[1].is_selected());
    assert!(!items[2].is_selected());
}

#[test]
fn removing_selected_items_should_clear_item_is_selected() {
    let items = list_box_items();

    let target = create_target(|o| o.items = Some(controls(&items)));

    target.selected_items().add(Some(Control::boxed(items[0].clone())));
    target.selected_items().add(Some(Control::boxed(items[1].clone())));
    target.selected_items().remove(&Some(Control::boxed(items[1].clone())));

    assert!(items[0].is_selected());
    assert!(!items[1].is_selected());
}

#[test]
fn reassigning_selected_items_should_not_clear_item_is_selected() {
    let items = list_box_items();

    let target = create_target(|o| o.items = Some(controls(&items)));

    target.selected_items().add(target.items().get_at(0));
    target.selected_items().add(target.items().get_at(1));
    target.set_selected_items(Some(SelectedItemsList::from_items(controls(&items[..2]))));

    assert!(items[0].is_selected());
    assert!(items[1].is_selected());
    assert!(!items[2].is_selected());
}

#[test]
fn setting_selected_index_should_unmark_previously_selected_containers() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    target.selected_items().add(boxed_str("foo"));
    target.selected_items().add(boxed_str("bar"));

    assert_eq!(vec![0, 1], selected_containers(&target));

    target.set_selected_index(2);

    assert_eq!(vec![2], selected_containers(&target));
}

const SIX_ITEMS: [&str; 6] = ["foo", "bar", "baz", "qux", "qiz", "lol"];

#[test]
fn range_select_should_select_range() {
    let target = create_target(|o| o.items = Some(boxed_strs(&SIX_ITEMS)));

    target.set_selected_index(1);
    target.select_range(3);

    assert_strs(&["bar", "baz", "qux"], &target.selected_items().to_vec());
}

#[test]
fn range_select_backwards_should_select_range() {
    let target = create_target(|o| o.items = Some(boxed_strs(&SIX_ITEMS)));

    target.set_selected_index(3);
    target.select_range(1);

    assert_strs(&["qux", "bar", "baz"], &target.selected_items().to_vec());
}

#[test]
fn second_range_select_backwards_should_select_from_original_selection() {
    let target = create_target(|o| o.items = Some(boxed_strs(&SIX_ITEMS)));

    target.set_selected_index(2);
    target.select_range(5);
    target.select_range(4);

    assert_strs(&["baz", "qux", "qiz"], &target.selected_items().to_vec());
}

#[test]
fn setting_selected_index_after_range_should_unmark_previously_selected_containers() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz", "qux"]));

    target.select_range(2);

    assert_eq!(vec![0, 1, 2], selected_containers(&target));

    target.set_selected_index(3);

    assert_eq!(vec![3], selected_containers(&target));
}

#[test]
fn toggling_selection_after_range_should_work() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz", "foo", "bar", "baz"]));

    target.select_range(3);

    assert_eq!(vec![0, 1, 2, 3], selected_containers(&target));

    target.toggle(4);

    assert_eq!(vec![0, 1, 2, 3, 4], selected_containers(&target));
}

#[test]
fn suprious_selected_index_changes_should_not_be_triggered() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    let selected_indexes = Rc::new(RefCell::new(Vec::new()));
    let sink = selected_indexes.clone();
    let target_object: &FerroObject = &target;
    FerroObjectExtensions::get_observable(target_object, SelectingItemsControl::selected_index_property())
        .subscribe_fn(move |x| sink.borrow_mut().push(x));

    target.set_selected_items(selected_list(&["bar", "baz"]));
    target.set_selected_item(boxed_str("foo"));

    assert_eq!(0, target.selected_index());
    assert_eq!(vec![-1, 1, 0], *selected_indexes.borrow());
}

#[test]
fn can_set_selected_index_to_another_selected_item() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    target.selected_items().add(boxed_str("foo"));
    target.selected_items().add(boxed_str("bar"));

    assert_eq!(0, target.selected_index());
    assert_strs(&["foo", "bar"], &target.selected_items().to_vec());
    assert_eq!(vec![0, 1], selected_containers(&target));

    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    on_selection_changed(&target, move |e| {
        flag.set(true);
        assert!(e.added_items().is_empty());
        assert_strs(&["foo"], e.removed_items());
    });

    target.set_selected_index(1);

    assert!(raised.get());
    assert_eq!(1, target.selected_index());
    assert_strs(&["bar"], &target.selected_items().to_vec());
    assert_eq!(vec![1], selected_containers(&target));
}

fn one_way(path: &str) -> Rc<ReflectionBinding> {
    ReflectionBinding::new(path).with_mode(BindingMode::OneWay)
}

/// Tests a problem discovered with the list box with selection.
///
/// - The items are bound to the data context first, followed by say the
///   selected index
/// - When the list box is removed from the visual tree, the data context
///   becomes null (as it's inherited)
/// - This changes the items to null, which changes the selected index to
///   null as there are no longer any items
/// - However, the news that the data context is now null hasn't yet reached
///   the selected items binding and so the unselection is sent back to the
///   view model
///
/// This is a similar problem to that of writing to an old data context
/// through a property binding. However, that is a general property binding
/// problem: here we are writing directly to the selected items collection -
/// not via a binding - so it's something that the binding system cannot
/// solve. Instead we solve it by not clearing the selected items when the
/// data context is in the process of changing.
#[test]
fn should_not_write_selected_items_to_old_data_context() {
    let target = create_target(|_| {});
    let vm = OldDataContextViewModel::new();

    let items_binding = one_way("Items");
    let selected_items_binding = one_way("SelectedItems");

    // Bind the items source and the selected items to the view model.
    target.bind_binding(ItemsControl::items_source_property().as_property(), &items_binding);
    target.bind_binding(SelectingItemsControl::selected_items_property().as_property(), &selected_items_binding);

    // Set the data context and the selected index.
    target.set_data_context(Some(vm.clone() as BoxedValue));
    target.set_selected_index(1);

    // Make sure the selected items are written back to the view model.
    assert_strs(&["bar"], &vm.selected_items.to_vec());

    // Clear the data context and ensure that the selected items are still
    // set in the view model.
    target.set_data_context(None);
    assert_strs(&["bar"], &vm.selected_items.to_vec());

    // Ensure the target's selected items are now clear.
    assert!(target.selected_items().is_empty());
}

/// See [`should_not_write_selected_items_to_old_data_context`].
#[test]
fn should_not_write_selection_model_to_old_data_context() {
    let target = create_target(|_| {});
    let vm = OldDataContextViewModel::new();

    let items_binding = one_way("Items");
    let selection_binding = one_way("Selection");

    // Bind the items source and the selection to the view model.
    target.bind_binding(ItemsControl::items_source_property().as_property(), &items_binding);
    target.bind_binding(SelectingItemsControl::selection_property().as_property(), &selection_binding);

    // Set the data context and the selected index.
    target.set_data_context(Some(vm.clone() as BoxedValue));
    target.set_selected_index(1);

    // Make sure the selection is written to the selection model.
    assert_eq!(1, vm.selection.selected_index());

    // Clear the data context and ensure that the selection is still set in
    // the model.
    target.set_data_context(None);
    assert_eq!(1, vm.selection.selected_index());

    // Ensure the target's selected items are now clear.
    assert!(target.selected_items().is_empty());
}

#[test]
fn unbound_selected_items_should_be_cleared_when_data_context_cleared() {
    let data = Model::new_model(ItemsViewModel { items: ItemsSource::from_strs(["foo", "bar", "baz"]) });

    let target = create_target(|o| o.data_context = Some(data.clone() as BoxedValue));
    register_binding_conversions();
    let items_binding = ReflectionBinding::new("Items");
    target.bind_binding(ItemsControl::items_source_property().as_property(), &items_binding);

    assert!(data.items.ptr_eq(&target.items_source().unwrap()));

    target.selected_items().add(boxed_str("bar"));
    target.set_data_context(None);

    assert!(target.selected_items().is_empty());
}

#[test]
fn adding_to_selected_items_should_raise_selection_changed() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));
    let called = Rc::new(Cell::new(false));

    let flag = called.clone();
    on_selection_changed(&target, move |e| {
        assert_strs(&["bar"], e.added_items());
        assert!(e.removed_items().is_empty());
        flag.set(true);
    });

    target.selected_items().add(boxed_str("bar"));

    assert!(called.get());
}

#[test]
fn removing_from_selected_items_should_raise_selection_changed() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));
    let called = Rc::new(Cell::new(false));

    target.set_selected_item(boxed_str("bar"));
    let flag = called.clone();
    on_selection_changed(&target, move |e| {
        assert_strs(&["bar"], e.removed_items());
        assert!(e.added_items().is_empty());
        flag.set(true);
    });

    target.selected_items().remove(&boxed_str("bar"));

    assert!(called.get());
}

#[test]
fn assigning_selected_items_should_raise_selection_changed() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    target.set_selected_item(boxed_str("bar"));

    let called = Rc::new(Cell::new(false));

    let flag = called.clone();
    on_selection_changed(&target, move |e| {
        assert_strs(&["foo", "baz"], e.added_items());
        assert_strs(&["bar"], e.removed_items());
        flag.set(true);
    });

    target.set_selected_items(selected_list(&["foo", "baz"]));

    assert!(called.get());
}

#[test]
fn select_all_sets_selected_index_and_selected_item() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    target.select_all();

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("foo"), &target.selected_item()));
}

#[test]
fn select_all_raises_selection_changed_event() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    let received_args: Rc<RefCell<Option<SelectionChangedEventArgs>>> = Rc::new(RefCell::new(None));

    let sink = received_args.clone();
    on_selection_changed(&target, move |args| *sink.borrow_mut() = Some(args.clone()));

    target.select_all();

    let received_args = received_args.borrow();
    let received_args = received_args.as_ref().unwrap();
    assert_items(&target.items_source().unwrap().to_vec(), received_args.added_items());
    assert!(received_args.removed_items().is_empty());
}

#[test]
fn unselect_all_clears_selected_index_and_selected_item() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));

    target.set_selected_index(0);
    target.unselect_all();

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn select_all_handles_duplicate_items() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz", "foo", "bar", "baz"]));

    target.select_all();

    assert_strs(&["foo", "bar", "baz", "foo", "bar", "baz"], &target.selected_items().to_vec());
}

#[test]
fn adding_item_before_selected_items_should_update_selection() {
    let items = string_list(&["foo", "bar", "baz"]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    target.select_all();
    items.insert(0, "qux".to_string());
    layout(&target);

    assert_eq!(1, target.selected_index());
    assert!(items_equal(&boxed_str("foo"), &target.selected_item()));
    assert_strs(&["foo", "bar", "baz"], &target.selected_items().to_vec());
    assert_eq!(vec![1, 2, 3], selected_containers(&target));
}

#[test]
fn removing_item_before_selected_item_should_update_selection() {
    let items = string_list(&["foo", "bar", "baz"]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    target.set_selected_index(1);
    target.select_range(2);

    assert_strs(&["bar", "baz"], &target.selected_items().to_vec());

    items.remove_at(0);

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("bar"), &target.selected_item()));
    assert_strs(&["bar", "baz"], &target.selected_items().to_vec());
    assert_eq!(vec![0, 1], selected_containers(&target));
}

#[test]
fn removing_selected_item_with_multiple_selection_active_should_update_selection() {
    let items = string_list(&["foo", "bar", "baz"]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    target.select_all();
    items.remove_at(0);

    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed_str("bar"), &target.selected_item()));
    assert_strs(&["bar", "baz"], &target.selected_items().to_vec());
    assert_eq!(vec![0, 1], selected_containers(&target));
}

#[test]
fn replacing_selected_item_should_update_selected_items() {
    let items = string_list(&["foo", "bar", "baz"]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    target.select_all();
    items.set(1, "qux".to_string());

    assert_strs(&["foo", "baz"], &target.selected_items().to_vec());
}

#[test]
fn adding_selected_item_containers_should_update_selection() {
    let items = vec![TestContainer::new(), TestContainer::new()];

    let target = create_target(|o| o.items = Some(controls(&items)));

    target.items().add(Some(Control::boxed(TestContainer::selected())));
    target.items().add(Some(Control::boxed(TestContainer::selected())));

    assert_eq!(2, target.selected_index());
    assert!(items_equal(&target.items().get_at(2), &target.selected_item()));
    assert_items(&[target.items().get_at(2), target.items().get_at(3)], &target.selected_items().to_vec());
}

#[test]
fn adding_to_selection_should_set_selected_index() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.selected_items().add(boxed_str("bar"));

    assert_eq!(1, target.selected_index());
}

#[test]
fn assigning_null_to_selection_should_create_new_selection_model() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));
    let old_selection = target.selection();

    target.set_selection(None);

    assert!(!crate::selection::selection_model_ptr_eq(&old_selection, &target.selection()));
}

#[test]
#[should_panic(expected = "The supplied ISelectionModel already has an assigned Source")]
fn assigning_selection_model_with_different_source_to_selection_should_fail() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));
    let selection = SelectionModel::<String>::with_source(strs(&["baz"]));

    target.set_selection(Some(selection));
}

#[test]
fn assigning_selection_model_with_null_source_to_selection_should_set_source() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));
    let selection = SelectionModel::<String>::new();

    target.set_selection(Some(selection.clone()));

    assert!(target.items_source().unwrap().ptr_eq(&selection.source().unwrap()));
}

#[test]
fn assigning_single_selected_item_to_selection_should_set_selected_index() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));
    let selection = SelectionModel::<String>::new();
    selection.set_single_select(false);

    selection.select(1);
    target.set_selection(Some(selection));

    assert_eq!(1, target.selected_index());
    assert_strs(&["bar"], &selection_items(&target));
    assert_eq!(vec![1], selected_containers(&target));
}

#[test]
fn assigning_multiple_selected_items_to_selection_should_set_selected_index() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar", "baz"]));
    let selection = SelectionModel::<String>::new();
    selection.set_single_select(false);

    selection.select_range(0, 2);
    target.set_selection(Some(selection));

    assert_eq!(0, target.selected_index());
    assert_strs(&["foo", "bar", "baz"], &selection_items(&target));
    assert_eq!(vec![0, 1, 2], selected_containers(&target));
}

#[test]
fn reassigning_selection_should_clear_selection() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    target.selection().select(1);
    target.set_selection(Some(SelectionModel::<String>::new()));

    assert_eq!(-1, target.selected_index());
    assert!(target.selected_item().is_none());
}

#[test]
fn assigning_selection_should_set_item_is_selected() {
    let items = list_box_items();

    let target = create_target(|o| o.items = Some(controls(&items)));
    let selection = SelectionModel::<BoxedValue>::new();
    selection.set_single_select(false);

    selection.select_range(0, 1);
    target.set_selection(Some(selection));

    assert!(items[0].is_selected());
    assert!(items[1].is_selected());
    assert!(!items[2].is_selected());
}

#[test]
fn assigning_selection_should_raise_selection_changed() {
    let items = ItemsSource::from_strs(["foo", "bar", "baz"]);
    let target = create_target(|o| o.items_source = Some(items.clone()));
    let raised = Rc::new(Cell::new(0));

    target.set_selected_item(boxed_str("bar"));

    let counter = raised.clone();
    on_selection_changed(&target, move |e| {
        if counter.get() == 0 {
            assert!(e.added_items().is_empty());
            assert_strs(&["bar"], e.removed_items());
        } else {
            assert_strs(&["foo", "baz"], e.added_items());
            assert!(e.removed_items().is_empty());
        }

        counter.set(counter.get() + 1);
    });

    let selection = SelectionModel::<String>::with_source(Some(items));
    selection.set_single_select(false);
    selection.select(0);
    selection.select(2);
    target.set_selection(Some(selection));

    assert_eq!(2, raised.get());
}

#[test]
fn can_bind_initial_selected_state_via_item_container_theme() {
    let items = item_view_models();
    let item_theme = ControlTheme::with_setters(ContentPresenter::TYPE, [is_selected_setter()]);

    let target = create_target(|o| {
        o.items_source = Some(ItemsSource::from_items(view_models(&items)));
        o.item_container_theme = Some(item_theme);
    });

    assert_eq!(vec![0, 2], selected_containers(&target));
    assert_eq!(0, target.selected_index());
    assert!(items_equal(&view_models(&items)[0], &target.selected_item()));
    assert_eq!(vec![0, 2], selection_indexes(&target));
    assert_items(&[view_models(&items)[0].clone(), view_models(&items)[2].clone()], &selection_items(&target));
}

#[test]
fn can_bind_initial_selected_state_via_style() {
    let items = item_view_models();
    let style = Style::with_setters(Selectors::of_type::<ContentPresenter>(), [is_selected_setter()]);

    let target = create_target(|o| {
        o.items_source = Some(ItemsSource::from_items(view_models(&items)));
        o.styles = vec![style];
    });

    assert_eq!(vec![0, 2], selected_containers(&target));
    assert_eq!(0, target.selected_index());
    assert!(items_equal(&view_models(&items)[0], &target.selected_item()));
    assert_eq!(vec![0, 2], selection_indexes(&target));
    assert_items(&[view_models(&items)[0].clone(), view_models(&items)[2].clone()], &selection_items(&target));
}

#[test]
fn selection_state_is_updated_via_is_selected_binding() {
    let items = item_view_models();
    let boxed = view_models(&items);
    let item_theme = ControlTheme::with_setters(TestContainer::TYPE, [is_selected_setter()]);
    item_theme.set_based_on(Some(create_test_container_theme()));

    // For the container selection state to be communicated back to the
    // selecting items control we need a container which raises the
    // `is_selected_changed_event` when the `IsSelected` property changes.
    let target = create_target_of(TestSelectorWithContainers::new().upcast(), |o| {
        o.items_source = Some(ItemsSource::from_items(boxed.clone()));
        o.item_container_theme = Some(item_theme);
    });

    items[1].set_is_selected(true);

    assert_eq!(vec![0, 1, 2], selected_containers(&target));
    assert_eq!(0, target.selected_index());
    assert!(items_equal(&boxed[0], &target.selected_item()));
    assert_eq!(vec![0, 1, 2], selection_indexes(&target));
    assert_items(&boxed, &selection_items(&target));

    items[0].set_is_selected(false);

    assert_eq!(vec![1, 2], selected_containers(&target));
    assert_eq!(1, target.selected_index());
    assert!(items_equal(&boxed[1], &target.selected_item()));
    assert_eq!(vec![1, 2], selection_indexes(&target));
    assert_items(&boxed[1..], &selection_items(&target));
}

#[test]
fn selection_state_is_written_back_to_item_via_is_selected_binding() {
    let items = item_view_models();
    let item_theme = ControlTheme::with_setters(ContentPresenter::TYPE, [is_selected_setter()]);

    let target = create_target(|o| {
        o.items_source = Some(ItemsSource::from_items(view_models(&items)));
        o.item_container_theme = Some(item_theme);
    });
    let container0 = target.container_from_index(0).unwrap();
    let container1 = target.container_from_index(1).unwrap();

    SelectingItemsControl::set_is_selected(&container1, true);

    assert!(items[1].is_selected());

    SelectingItemsControl::set_is_selected(&container0, false);

    assert!(!items[0].is_selected());
}


mod virtualizing {
    use super::*;
    use crate::VirtualizingStackPanel;

    fn panel_of(target: &Target) -> Ref<VirtualizingStackPanel> {
        target.items_panel_root().unwrap().cast::<VirtualizingStackPanel>().unwrap()
    }

    fn scroll_of(panel: &VirtualizingStackPanel) -> Ref<ScrollViewer> {
        panel.find_ancestor_of_type::<ScrollViewer>(false).unwrap()
    }

    fn hundred_view_models() -> Vec<Rc<ItemViewModel>> {
        (0..100).map(|x| ItemViewModel::new(&format!("Item {x}"), false)).collect()
    }

    fn is_selected_and_height_theme() -> Ref<ControlTheme> {
        ControlTheme::with_setters(
            ContentPresenter::TYPE,
            [is_selected_setter(), Setter::new(Layoutable::height_property(), 100.0)],
        )
    }

    #[test]
    fn selection_is_updated_on_container_realization_with_is_selected_binding() {
        let items = hundred_view_models();
        let boxed = view_models(&items);
        items[0].set_is_selected(true);
        items[15].set_is_selected(true);

        let item_theme = is_selected_and_height_theme();

        // Create a selecting items control with a virtualizing stack panel.
        let target = create_target(|o| {
            o.items_source = Some(ItemsSource::from_items(boxed.clone()));
            o.item_container_theme = Some(item_theme);
            o.virtualizing = true;
        });
        let panel = panel_of(&target);
        let scroll = scroll_of(&panel);

        // The selecting items control does not yet know anything about item
        // 15's selection state.
        assert_eq!(vec![0], selected_containers(&target));
        assert_eq!(0, target.selected_index());
        assert!(items_equal(&boxed[0], &target.selected_item()));
        assert_eq!(vec![0], selection_indexes(&target));
        assert_items(&boxed[..1], &selection_items(&target));

        // Scroll item 15 into view.
        scroll.set_offset(Vector::new(0.0, 1000.0));
        layout(&target);

        assert_eq!(10, panel.first_realized_index());
        assert_eq!(19, panel.last_realized_index());

        // The final selection should be in place.
        assert!(items[0].is_selected());
        assert!(items[15].is_selected());
        assert_eq!(0, target.selected_index());
        assert!(items_equal(&boxed[0], &target.selected_item()));
        assert_eq!(vec![0, 15], selection_indexes(&target));
        assert_items(&[boxed[0].clone(), boxed[15].clone()], &selection_items(&target));

        // Although item 0 is selected, it's not realized.
        assert_eq!(vec![15], selected_containers(&target));
    }

    #[test]
    fn can_change_selection_for_containers_outside_of_viewport() {
        // Issue #11119
        let items: Vec<Ref<TestContainer>> = (0..100)
            .map(|x| {
                let container = TestContainer::new();
                container.set_content(boxed_str(&format!("Item {x}")));
                container.set_height(100.0);
                container
            })
            .collect();
        let boxed = controls(&items);

        // Create a selecting items control with a virtualizing stack panel.
        let target = create_target(|o| {
            o.items_source = Some(ItemsSource::from_items(boxed.clone()));
            o.virtualizing = true;
        });
        target.set_auto_scroll_to_selected_item(false);

        let panel = panel_of(&target);
        let scroll = scroll_of(&panel);

        // Select item 1.
        target.set_selected_index(1);

        // Scroll item 1 and 2 out of view.
        scroll.set_offset(Vector::new(0.0, 1000.0));
        layout(&target);

        assert_eq!(10, panel.first_realized_index());
        assert_eq!(19, panel.last_realized_index());

        // Select item 2 now that items 1 and 2 are both unrealized.
        target.set_selected_index(2);

        // The selection should be updated.
        assert!(selected_containers(&target).is_empty());
        assert_eq!(2, target.selected_index());
        assert!(items_equal(&boxed[2], &target.selected_item()));
        assert_eq!(vec![2], selection_indexes(&target));
        assert_items(&boxed[2..3], &selection_items(&target));

        // Scroll selected item back into view.
        scroll.set_offset(Vector::new(0.0, 0.0));

        layout(&target);

        // The selection should be preserved.
        assert_eq!(vec![2], selected_containers(&target));
        assert_eq!(2, target.selected_index());
        assert!(items_equal(&boxed[2], &target.selected_item()));
        assert_eq!(vec![2], selection_indexes(&target));
        assert_items(&boxed[2..3], &selection_items(&target));
    }

    #[test]
    fn selection_is_not_cleared_on_recycling_containers() {
        let items = hundred_view_models();
        let boxed = view_models(&items);

        // Create a selecting items control that creates containers that
        // raise the `is_selected_changed_event`, with a virtualizing stack
        // panel.
        let target = create_target_of(TestSelectorWithContainers::new().upcast(), |o| {
            o.items_source = Some(ItemsSource::from_items(boxed.clone()));
            o.virtualizing = true;
        });
        target.set_auto_scroll_to_selected_item(false);

        let panel = panel_of(&target);
        let scroll = scroll_of(&panel);

        // Select item 1.
        target.set_selected_index(1);

        // Scroll item 1 out of view.
        scroll.set_offset(Vector::new(0.0, 1000.0));
        layout(&target);

        assert_eq!(10, panel.first_realized_index());
        assert_eq!(19, panel.last_realized_index());

        // The selection should be preserved.
        assert_eq!(vec![1], selected_containers(&target));
        assert_eq!(1, target.selected_index());
        assert!(items_equal(&boxed[1], &target.selected_item()));
        assert_eq!(vec![1], selection_indexes(&target));
        assert_items(&boxed[1..2], &selection_items(&target));
    }

    #[test]
    fn selection_state_change_on_unrealized_item_is_respected_with_is_selected_binding() {
        let items = hundred_view_models();
        let boxed = view_models(&items);
        let item_theme = is_selected_and_height_theme();

        // Create a selecting items control with a virtualizing stack panel.
        let target = create_target(|o| {
            o.items_source = Some(ItemsSource::from_items(boxed.clone()));
            o.item_container_theme = Some(item_theme);
            o.virtualizing = true;
        });
        let panel = panel_of(&target);
        let scroll = scroll_of(&panel);

        // Scroll item 1 out of view.
        scroll.set_offset(Vector::new(0.0, 1000.0));
        layout(&target);

        assert_eq!(10, panel.first_realized_index());
        assert_eq!(19, panel.last_realized_index());

        // Select item 1 now it's unrealized.
        items[1].set_is_selected(true);

        // The selecting items control does not yet know anything about the
        // selection change.
        assert!(selected_containers(&target).is_empty());
        assert_eq!(-1, target.selected_index());
        assert!(target.selected_item().is_none());
        assert!(selection_indexes(&target).is_empty());
        assert!(selection_items(&target).is_empty());

        // Scroll item 1 back into view.
        scroll.set_offset(Vector::new(0.0, 0.0));
        layout(&target);

        // The item and container should be marked as selected.
        assert!(items[1].is_selected());
        assert_eq!(vec![1], selected_containers(&target));
        assert_eq!(1, target.selected_index());
        assert!(items_equal(&boxed[1], &target.selected_item()));
        assert_eq!(vec![1], selection_indexes(&target));
        assert_items(&boxed[1..2], &selection_items(&target));
    }
}
