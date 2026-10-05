//! The reference tests run against a test root whose renderer does not hit
//! test: the root of these tests hit tests nothing unless asked to.

use crate::generators::RecycleKey;
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{
    HeaderedItemsControl, HeaderedItemsControlImpl, SelectingItemsControl, TemplatedControl, TemplatedControlImpl,
};
use crate::templates::{
    DataTemplates, FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, FuncTreeDataTemplate,
    IControlTemplate, IDataTemplate, IDataTemplateHost, IRecyclingDataTemplate, ITemplateWithParam,
    ITreeDataTemplate,
};
use crate::mouse_test_helper::MouseTestHelper;
use crate::test_support::{boxed_str, test_scope, NullHitTester, TestRoot, TestScope};
use crate::{
    items_equal, Border, Button, Canvas, Control, ControlImpl, IGlobalDataTemplates, ItemsControl,
    ItemsControlImpl, ItemsSource, Panel, SelectionMode, StackPanel, TextBlock, TreeView, TreeViewImpl, TreeViewItem,
    TreeViewItemImpl,
};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::NameScope;
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingBase, ReflectionBinding, TemplateBinding};
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{
    IKeyboardDevice, IKeyboardNavigationHandler, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, KeyboardDevice,
    KeyboardNavigationHandler, MouseButton, NavigationDirection,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::Brushes;
use ferroui_base::platform::DefaultPlatformSettings;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, instantiate, AnyValue, BoxedValue, FerroLocator, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

// --- test data ------------------------------------------------------------

type NodeList = Rc<FerroList<BoxedValue>>;

struct Node {
    value: Option<String>,
    children: RefCell<NodeList>,
    is_selected: Cell<bool>,
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for Node {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Node, |b| b
    .notify_property_changed()
    .read_only::<Maybe<String>>("Value", |node| node.value.clone())
    .read_only::<Value<ItemsSource>>("Children", |node| ItemsSource::from(node.children()))
    .property::<Value<bool>>("IsSelected", |node| node.is_selected(), |node, v| node.set_is_selected(v)));

impl Node {
    fn create(value: Option<&str>, children: &[Rc<Node>]) -> Rc<Self> {
        Model::new_model(Self {
            value: value.map(str::to_string),
            children: RefCell::new(node_list(children)),
            is_selected: Cell::new(false),
            property_changed: Event::new(),
        })
    }

    fn empty() -> Rc<Self> {
        Self::create(None, &[])
    }

    fn new(value: &str) -> Rc<Self> {
        Self::create(Some(value), &[])
    }

    fn with_children(value: &str, children: &[Rc<Node>]) -> Rc<Self> {
        Self::create(Some(value), children)
    }

    fn children(&self) -> NodeList {
        self.children.borrow().clone()
    }

    fn set_children(&self, value: NodeList) {
        *self.children.borrow_mut() = value;
        self.property_changed.raise("Children");
    }

    fn child(&self, index: usize) -> Rc<Node> {
        node_at(&self.children(), index)
    }

    fn last_child(&self) -> Rc<Node> {
        let children = self.children();
        node_at(&children, children.count() - 1)
    }

    fn child_nodes(&self) -> Vec<Rc<Node>> {
        self.children().to_vec().iter().map(node_of).collect()
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

fn node_list(nodes: &[Rc<Node>]) -> NodeList {
    Rc::new(FerroList::from_items(nodes.iter().map(|node| node.clone() as BoxedValue)))
}

fn node_of(value: &BoxedValue) -> Rc<Node> {
    let any: Rc<dyn Any> = value.clone();
    any.downcast::<Node>().ok().expect("the item is not a node")
}

fn node_at(list: &NodeList, index: usize) -> Rc<Node> {
    node_of(&list.get(index))
}

/// A node as an item.
fn item(node: &Rc<Node>) -> Option<BoxedValue> {
    Some(node.clone() as BoxedValue)
}

fn is_node(item: &Option<BoxedValue>, node: &Rc<Node>) -> bool {
    match item {
        Some(item) => Rc::ptr_eq(&node_of(item), node),
        None => false,
    }
}

fn source(data: &NodeList) -> Option<ItemsSource> {
    Some(ItemsSource::from(data.clone()))
}

fn create_test_tree_data() -> NodeList {
    node_list(&[Node::with_children(
        "Root",
        &[
            Node::new("Child1"),
            Node::with_children("Child2", &[Node::new("Grandchild2a")]),
            Node::new("Child3"),
        ],
    )])
}

struct TestDataContext {
    items: Rc<FerroList<String>>,
    selected_item: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for TestDataContext {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TestDataContext, |b| b
    .notify_property_changed()
    .read_only::<Value<ItemsSource>>("Items", |vm| ItemsSource::from(vm.items.clone()))
    .property::<Maybe<String>>("SelectedItem", |vm| vm.selected_item(), |vm, v| vm.set_selected_item(v)));

impl TestDataContext {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            items: Rc::new(FerroList::from_items((0..5).map(|i| format!("Item {i}")))),
            selected_item: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    fn selected_item(&self) -> Option<String> {
        self.selected_item.borrow().clone()
    }

    fn set_selected_item(&self, value: Option<String>) {
        *self.selected_item.borrow_mut() = value;
        self.property_changed.raise("SelectedItem");
    }
}

/// Stands in for the tree data template of the markup: builds a control for
/// the data and binds the children of an item with a binding.
struct TreeDataTemplate {
    matches_nodes_only: bool,
    content: Box<dyn Fn() -> Ref<Control>>,
    items_source: Rc<ReflectionBinding>,
}

impl TreeDataTemplate {
    fn new(matches_nodes_only: bool, content: impl Fn() -> Ref<Control> + 'static) -> Rc<dyn IDataTemplate> {
        Rc::new(Self { matches_nodes_only, content: Box::new(content), items_source: ReflectionBinding::new("Children") })
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for TreeDataTemplate {
    fn build(&self, _param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        Some((self.content)())
    }
}

impl IDataTemplate for TreeDataTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        if !self.matches_nodes_only {
            return true;
        }

        data.is_some_and(|data| {
            let data: &dyn AnyValue = &**data;
            data.is::<Node>()
        })
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        None
    }

    fn as_tree_data_template(&self) -> Option<&dyn ITreeDataTemplate> {
        Some(self)
    }
}

impl ITreeDataTemplate for TreeDataTemplate {
    fn bind_children(
        &self,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        item: &BoxedValue,
    ) -> Rc<dyn IDisposable> {
        let binding = ReflectionBinding::new(&self.items_source.path()).with_source(Some(item.clone()));
        let expression = target.bind_binding(target_property, &binding);
        Disposable::create(move || expression.dispose())
    }
}

/// Stands in for the application as the host of the global data templates.
struct GlobalDataTemplates {
    data_templates: DataTemplates,
}

impl IDataTemplateHost for GlobalDataTemplates {
    fn data_templates(&self) -> DataTemplates {
        self.data_templates.clone()
    }

    fn is_data_templates_initialized(&self) -> bool {
        true
    }
}

impl IGlobalDataTemplates for GlobalDataTemplates {}

// --- derived classes --------------------------------------------------------

#[repr(C)]
struct DerivedTreeView {
    base: TreeView,
}

ferro_class!(DerivedTreeView: TreeView);
ferro_impl_classes!(
    DerivedTreeView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    TreeViewImpl
);

impl DerivedTreeView {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeView::construct() })
    }
}

#[repr(C)]
struct DerivedTreeViewWithDerivedTreeViewItems {
    base: TreeView,
}

ferro_class!(DerivedTreeViewWithDerivedTreeViewItems: TreeView);
ferro_impl_classes!(
    DerivedTreeViewWithDerivedTreeViewItems: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TreeViewImpl
);

impl ItemsControlImpl for DerivedTreeViewWithDerivedTreeViewItems {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        DerivedTreeViewItem::new().upcast()
    }
}

impl DerivedTreeViewWithDerivedTreeViewItems {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeView::construct() })
    }
}

#[repr(C)]
struct DerivedTreeViewItem {
    base: TreeViewItem,
}

ferro_class!(DerivedTreeViewItem: TreeViewItem);
ferro_impl_classes!(
    DerivedTreeViewItem: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    HeaderedItemsControlImpl,
    TreeViewItemImpl
);

impl DerivedTreeViewItem {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeViewItem::construct() })
    }
}

// --- helpers ----------------------------------------------------------------

struct Options {
    /// The items source; `None` is the test tree.
    data: Option<Option<ItemsSource>>,
    expand_all: bool,
    item_container_theme: Option<Ref<ControlTheme>>,
    item_template: Option<Rc<dyn IDataTemplate>>,
    multi_select: bool,
    styles: Vec<Ref<Style>>,
    /// Stands in for the root with a compositor of the reference.
    hit_testing: bool,
}

struct Target {
    root: Ref<TestRoot>,
    target: Ref<TreeView>,
}

impl Deref for Target {
    type Target = Ref<TreeView>;

    fn deref(&self) -> &Ref<TreeView> {
        &self.target
    }
}

fn start() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
    FerroLocator::current_mutable()
        .bind::<PlatformHotkeyConfiguration>()
        .to_constant(Rc::new(PlatformHotkeyConfiguration::default()));
    ItemsSource::register_binding_conversion::<ItemsSource>();
    scope
}

fn text_block() -> Ref<Control> {
    let result = TextBlock::new();
    result.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::new("Value"));
    result.upcast()
}

/// The text of a control, if it is a text block.
fn text_of(control: &Ref<Control>) -> Option<Option<String>> {
    control.downcast_ref::<TextBlock>().map(|text_block| text_block.text())
}

fn create_target(configure: impl FnOnce(&mut Options)) -> Target {
    let mut options = Options {
        data: None,
        expand_all: true,
        item_container_theme: None,
        item_template: None,
        multi_select: false,
        styles: Vec::new(),
        hit_testing: false,
    };
    configure(&mut options);

    let target = TreeView::new();
    target.set_item_container_theme(options.item_container_theme);
    target.set_items_source(match options.data {
        Some(data) => data,
        None => source(&create_test_tree_data()),
    });
    target.set_item_template(options.item_template);
    target.set_selection_mode(if options.multi_select { SelectionMode::MULTIPLE } else { SelectionMode::SINGLE });

    let root = create_root_with(&target.clone().upcast(), |root| {
        if options.hit_testing {
            root.set_hit_tester(None);
        }

        for style in options.styles {
            root.styles().add(style);
        }
    });

    root.execute_initial_layout_pass();

    if options.expand_all {
        expand_all(&target);
    }

    Target { root, target }
}

fn create_root(child: &Ref<Control>) -> Ref<TestRoot> {
    create_root_with(child, |_| {})
}

fn create_root_with(child: &Ref<Control>, configure: impl FnOnce(&TestRoot)) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_hit_tester(Some(Rc::new(NullHitTester)));
    root.set_platform_settings(Some(Rc::new(DefaultPlatformSettings::new())));
    configure_root(&root);
    configure(&root);
    root.set_child(child.clone());
    root
}

fn theme_resource(theme: Ref<ControlTheme>) -> Option<BoxedValue> {
    Some(Rc::new(theme))
}

fn configure_root(root: &Control) {
    root.resources().add(TreeView::TYPE, theme_resource(create_tree_view_control_theme()));
    root.resources().add(TreeViewItem::TYPE, theme_resource(create_tree_view_item_control_theme()));
    root.data_templates().add(TreeDataTemplate::new(true, text_block));
}

fn create_tree_view_control_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        TreeView::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(create_tree_view_template()))],
    )
}

fn create_tree_view_item_control_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        TreeViewItem::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(create_tree_view_item_template()))],
    )
}

fn items_presenter() -> Ref<ItemsPresenter> {
    let presenter = ItemsPresenter::new();
    presenter.set_name(Some("PART_ItemsPresenter".to_string()));
    let property = ItemsControl::items_panel_property().as_property();
    presenter.bind_binding(property, &TemplateBinding::new(property));
    presenter
}

fn create_tree_view_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeView>(|_, scope| items_presenter().register_in_name_scope(&**scope).upcast())
}

fn create_tree_view_item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeViewItem>(|_, scope| {
        let header_presenter = ContentPresenter::new();
        header_presenter.set_name(Some("PART_HeaderPresenter".to_string()));
        header_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(HeaderedItemsControl::header_property().as_property()),
        );
        header_presenter.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &TemplateBinding::new(HeaderedItemsControl::header_template_property().as_property()),
        );

        let header = Border::new();
        header.set_name(Some("PART_Header".to_string()));
        header.set_background(Some(Brushes::transparent()));
        header.set_child(header_presenter.register_in_name_scope(&**scope));

        let items = items_presenter();
        items.bind_binding(
            Visual::is_visible_property().as_property(),
            &TemplateBinding::new(TreeViewItem::is_expanded_property().as_property()),
        );

        let panel = Panel::new();
        panel.children().add(header.register_in_name_scope(&**scope));
        panel.children().add(items.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

fn tree_view_item(control: Ref<Control>) -> Ref<TreeViewItem> {
    assert!(control.get_type() == TreeViewItem::TYPE);
    control.cast::<TreeViewItem>().unwrap()
}

/// The container of a node, which must be a tree view item.
fn container_of(target: &TreeView, node: &Rc<Node>) -> Ref<TreeViewItem> {
    tree_view_item(target.tree_container_from_item(&item(node)).unwrap())
}

fn expand_all(tree: &TreeView) {
    for i in tree.get_realized_containers() {
        tree.expand_sub_tree(&i.cast::<TreeViewItem>().unwrap());
    }
}

fn collapse_all(tree: &TreeView) {
    for i in tree.get_realized_containers() {
        tree.collapse_sub_tree(&i.cast::<TreeViewItem>().unwrap());
    }
}

fn get_item(target: &TreeView, indexes: &[usize]) -> Ref<TreeViewItem> {
    let mut c: Ref<ItemsControl> = target.to_ref().upcast();

    for index in indexes {
        let item = c.items_view().get_at(*index);
        c = target.tree_container_from_item(&item).unwrap().cast::<ItemsControl>().unwrap();
    }

    c.cast::<TreeViewItem>().unwrap()
}

fn header_child(item: &TreeViewItem) -> Option<Ref<Control>> {
    item.header_presenter().and_then(|presenter| presenter.child())
}

fn extract_item_header(tree: &TreeView, level: i32) -> Vec<Option<String>> {
    let mut containers = Vec::new();
    extract_item_content(tree.items_panel_root(), 0, level, &mut containers);
    containers.iter().filter_map(|x| header_child(x)).filter_map(|x| text_of(&x)).collect()
}

fn extract_item_content(panel: Option<Ref<Panel>>, current_level: i32, level: i32, result: &mut Vec<Ref<TreeViewItem>>) {
    let Some(panel) = panel else { return };

    for c in panel.children().to_vec() {
        let container = tree_view_item(c);

        if current_level == level {
            result.push(container);
        } else if let Some(child_panel) = container.items_panel_root() {
            extract_item_content(Some(child_panel), current_level + 1, level, result);
        }
    }
}

fn texts(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|value| Some(value.to_string())).collect()
}

fn layout(c: &Control) {
    if let Some(layout_manager) = c.get_layout_manager() {
        layout_manager.execute_layout_pass();
    }
}

fn click_container(mouse: &MouseTestHelper, container: &Interactive, modifiers: KeyModifiers) {
    mouse.click_with(container, container, MouseButton::Left, None, modifiers);
}

fn assert_container_selection(tree_view: &TreeView, expected: &[Rc<Node>]) {
    fn evaluate(container: &Ref<Control>, remaining: &mut Vec<Rc<Node>>) {
        let tree_view_item = tree_view_item(container.clone());
        let node = node_of(&container.data_context().unwrap());
        let position = remaining.iter().position(|x| Rc::ptr_eq(x, &node));

        assert_eq!(position.is_some(), tree_view_item.is_selected());
        if let Some(position) = position {
            remaining.remove(position);
        }

        for child in tree_view_item.get_realized_containers() {
            evaluate(&child, remaining);
        }
    }

    let mut remaining = expected.to_vec();
    for container in tree_view.get_realized_containers() {
        evaluate(&container, &mut remaining);
    }
    assert!(remaining.is_empty());
}

fn assert_all_child_containers_selected(tree_view: &TreeView, node: &Rc<Node>) {
    for child in node.child_nodes() {
        let container = container_of(tree_view, &child);
        assert!(container.is_selected());
    }
}

fn assert_data_selection(data: &NodeList, expected: &[Rc<Node>]) {
    fn evaluate(root_node: &Rc<Node>, remaining: &mut Vec<Rc<Node>>) {
        let position = remaining.iter().position(|x| Rc::ptr_eq(x, root_node));

        assert_eq!(position.is_some(), root_node.is_selected());
        if let Some(position) = position {
            remaining.remove(position);
        }

        for child in root_node.child_nodes() {
            evaluate(&child, remaining);
        }
    }

    let mut remaining = expected.to_vec();
    for node in data.to_vec().iter().map(node_of) {
        evaluate(&node, &mut remaining);
    }
    assert!(remaining.is_empty());
}

/// Asserts that the selected items are the given nodes, in order.
fn assert_selected_items(target: &TreeView, expected: &[Rc<Node>]) {
    let actual = target.selected_items().to_vec();
    assert_eq!(expected.len(), actual.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(is_node(actual, expected));
    }
}

fn key_down(key: Key, key_modifiers: KeyModifiers) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    args.key_modifiers = key_modifiers;
    args
}

fn select_all_gesture(target: &TreeView) -> KeyEventArgs {
    let keymap = target.get_platform_settings().unwrap().hotkey_configuration();
    let select_all_gesture = keymap.select_all.first().unwrap();
    key_down(select_all_gesture.key(), select_all_gesture.key_modifiers())
}

fn assert_each_item_with_children_is_collapsed(target: &TreeView, node: &Rc<Node>) {
    let container = container_of(target, node);

    if node.children().count() > 0 {
        assert!(!container.is_expanded());
        for c in node.child_nodes() {
            assert_each_item_with_children_is_collapsed(target, &c);
        }
    } else {
        assert!(container.is_expanded());
    }
}

fn assert_each_item_with_children_is_expanded(target: &TreeView, node: &Rc<Node>) {
    let container = container_of(target, node);

    if node.children().count() > 0 {
        assert!(container.is_expanded());
        for c in node.child_nodes() {
            assert_each_item_with_children_is_expanded(target, &c);
        }
    } else {
        assert!(!container.is_expanded());
    }
}

fn canvas_tree_template() -> Rc<dyn IDataTemplate> {
    FuncTreeDataTemplate::for_type::<Node>(
        |_, _| Some(Canvas::new().upcast()),
        |x| Rc::new(Some(ItemsSource::from(x.children()))),
    )
}

fn is_selected_setter() -> Rc<Setter> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new("IsSelected");
    Setter::new_binding_base(SelectingItemsControl::is_selected_property().as_property(), binding)
}

fn is_selected_item_theme() -> Ref<ControlTheme> {
    let item_theme = ControlTheme::with_setters(TreeViewItem::TYPE, [is_selected_setter()]);
    item_theme.set_based_on(Some(create_tree_view_item_control_theme()));
    item_theme
}

fn button() -> Ref<Control> {
    Button::new().upcast()
}

// --- tests ------------------------------------------------------------------

#[test]
fn items_should_be_created() {
    let _app = start();
    let target = create_target(|_| {});

    assert_eq!(texts(&["Root"]), extract_item_header(&target, 0));
    assert_eq!(texts(&["Child1", "Child2", "Child3"]), extract_item_header(&target, 1));
    assert_eq!(texts(&["Grandchild2a"]), extract_item_header(&target, 2));
}

#[test]
fn items_should_be_created_using_item_template_if_present() {
    let _app = start();
    let item_template = canvas_tree_template();
    let target = create_target(|o| o.item_template = Some(item_template));

    let items: Vec<Ref<TreeViewItem>> =
        target.get_realized_tree_containers().into_iter().filter_map(|x| x.cast::<TreeViewItem>()).collect();

    assert_eq!(5, items.len());
    for x in &items {
        assert!(header_child(x).unwrap().get_type() == Canvas::TYPE);
    }
}

#[test]
fn items_should_be_created_using_item_template_if_present_2() {
    let _app = start();
    let item_template = TreeDataTemplate::new(false, || Canvas::new().upcast());
    let target = create_target(|o| o.item_template = Some(item_template));

    let items: Vec<Ref<TreeViewItem>> =
        target.get_realized_tree_containers().into_iter().filter_map(|x| x.cast::<TreeViewItem>()).collect();

    assert_eq!(5, items.len());
    for x in &items {
        assert!(header_child(x).unwrap().get_type() == Canvas::TYPE);
    }
}

#[test]
fn items_should_be_created_using_item_conatiner_theme_if_present() {
    let _app = start();
    let theme = create_tree_view_item_control_theme();
    let item_template = canvas_tree_template();

    let target = create_target(|o| {
        o.item_container_theme = Some(theme.clone());
        o.item_template = Some(item_template);
    });

    let items: Vec<Ref<TreeViewItem>> =
        target.get_realized_tree_containers().into_iter().filter_map(|x| x.cast::<TreeViewItem>()).collect();

    assert_eq!(5, items.len());
    for x in &items {
        assert!(x.item_container_theme() == Some(theme.clone()));
    }
}

#[test]
fn finds_correct_data_template_when_application_data_template_is_present() {
    // #10398
    let _app = start();

    let global = Rc::new(GlobalDataTemplates { data_templates: DataTemplates::new() });
    global.data_templates.add(FuncDataTemplate::new(|_| true, |_, _| Some(Canvas::new().upcast()), false));
    FerroLocator::current_mutable().bind::<dyn IGlobalDataTemplates>().to_constant(global);

    let target = create_target(|_| {});

    assert_eq!(texts(&["Root"]), extract_item_header(&target, 0));
    assert_eq!(texts(&["Child1", "Child2", "Child3"]), extract_item_header(&target, 1));
    assert_eq!(texts(&["Grandchild2a"]), extract_item_header(&target, 2));
}

#[test]
fn root_item_container_generator_containers_should_be_root_containers() {
    let _app = start();
    let target = create_target(|_| {});

    let containers = target.get_realized_containers();
    assert_eq!(1, containers.len());
    let container = containers[0].clone().cast::<TreeViewItem>().unwrap();
    let header = header_child(&container).unwrap();
    assert!(header.get_type() == TextBlock::TYPE);
    assert_eq!(Some(Some("Root".to_string())), text_of(&header));
}

#[test]
fn root_tree_container_from_item_should_return_descendant_item() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    let container = target.tree_container_from_item(&item(&node_at(&data, 0).child(1).child(0)));

    assert!(container.is_some());

    let header = container.unwrap().cast::<TreeViewItem>().unwrap().header_presenter().unwrap();
    let header_content = text_of(&header.child().unwrap()).unwrap();

    assert_eq!(Some("Grandchild2a".to_string()), header_content);
}

#[test]
fn clicking_item_should_select_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    let item = node_at(&data, 0).child(1).child(0);
    let container = container_of(&target, &item);

    mouse.click(&container);

    assert!(is_node(&target.selected_item(), &item));
    assert!(container.is_selected());
}

#[test]
fn clicking_with_control_modifier_selected_item_should_deselect_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    let node = node_at(&data, 0).child(1).child(0);
    let container = container_of(&target, &node);

    target.set_selected_item(item(&node));

    assert!(container.is_selected());

    mouse.click_with(&container, &container, MouseButton::Left, None, KeyModifiers::CONTROL);

    assert!(target.selected_item().is_none());
    assert!(!container.is_selected());
}

#[test]
fn clicking_with_control_modifier_not_selected_item_should_select_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    let item1 = node_at(&data, 0).child(1).child(0);
    let container1 = container_of(&target, &item1);

    let item2 = node_at(&data, 0).child(1);
    let container2 = container_of(&target, &item2);

    target.set_selected_item(item(&item1));

    assert!(container1.is_selected());

    mouse.click_with(&container2, &container2, MouseButton::Left, None, KeyModifiers::CONTROL);

    assert!(is_node(&target.selected_item(), &item2));
    assert!(!container1.is_selected());
    assert!(container2.is_selected());
}

#[test]
fn clicking_with_control_modifier_selected_item_should_deselect_and_remove_from_selected_items() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let item1 = root_node.child(0);
    let item2 = root_node.last_child();
    let item1_container = container_of(&target, &item1);
    let item2_container = container_of(&target, &item2);

    click_container(&mouse, &item1_container, KeyModifiers::CONTROL);
    assert!(item1_container.is_selected());

    click_container(&mouse, &item2_container, KeyModifiers::CONTROL);
    assert!(item2_container.is_selected());

    assert_selected_items(&target, &[item1.clone(), item2.clone()]);

    click_container(&mouse, &item1_container, KeyModifiers::CONTROL);
    assert!(!item1_container.is_selected());

    assert!(!target.selected_items().to_vec().iter().any(|x| is_node(x, &item1)));
}

#[test]
fn clicking_with_shift_modifier_down_direction_should_select_range_of_items() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let from = root_node.child(0);
    let to = root_node.last_child();
    let from_container = container_of(&target, &from);
    let to_container = container_of(&target, &to);

    click_container(&mouse, &from_container, KeyModifiers::NONE);
    assert!(from_container.is_selected());

    click_container(&mouse, &to_container, KeyModifiers::SHIFT);
    assert_all_child_containers_selected(&target, &root_node);
}

#[test]
fn clicking_with_shift_modifier_up_direction_should_select_range_of_items() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let from = root_node.last_child();
    let to = root_node.child(0);
    let from_container = container_of(&target, &from);
    let to_container = container_of(&target, &to);

    click_container(&mouse, &from_container, KeyModifiers::NONE);
    assert!(from_container.is_selected());

    click_container(&mouse, &to_container, KeyModifiers::SHIFT);
    assert_all_child_containers_selected(&target, &root_node);
}

#[test]
fn clicking_first_item_of_selected_items_should_select_only_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let from = root_node.last_child();
    let to = root_node.child(0);
    let from_container = container_of(&target, &from);
    let to_container = container_of(&target, &to);

    click_container(&mouse, &from_container, KeyModifiers::NONE);
    click_container(&mouse, &to_container, KeyModifiers::SHIFT);
    assert_all_child_containers_selected(&target, &root_node);

    click_container(&mouse, &from_container, KeyModifiers::NONE);
    assert!(from_container.is_selected());

    for child in root_node.child_nodes() {
        if Rc::ptr_eq(&child, &from) {
            continue;
        }

        let container = container_of(&target, &child);
        assert!(!container.is_selected());
    }
}

/// The reference configures a root with a compositor for hit testing; here
/// the test root hit tests its visuals instead.
fn configure_hit_test_tree_view() -> (Target, Ref<TreeViewItem>, Ref<TreeViewItem>) {
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.hit_testing = true;
        o.styles = vec![Style::with_setters(
            Selectors::of_type::<TreeViewItem>(),
            [
                Setter::new(InputElement::is_holding_enabled_property(), true),
                Setter::new(InputElement::is_hold_with_mouse_enabled_property(), true),
            ],
        )];
    });

    layout(&target);
    target.set_selected_item(None);

    let parent = container_of(&target, &node_at(&data, 0));
    let child = container_of(&target, &node_at(&data, 0).child(0));
    (target, parent, child)
}

#[test]
fn swiping_onto_child_should_not_select() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let (target, parent, child) = configure_hit_test_tree_view();

    mouse.down(&parent);

    // The pointer is captured on mouse down, so we don't want to issue a
    // mouse up event on child itself but on parent at a position ABOVE
    // child, which is nested within the parent.
    let position = child.get_transformed_bounds().unwrap().bounds.center();
    mouse.up_at(&parent, MouseButton::Left, position);

    assert!(target.selected_item().is_none());
}

#[test]
fn swiping_onto_parent_should_not_select() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let (target, parent, child) = configure_hit_test_tree_view();

    mouse.down(&child);

    // When swiping from a child to a parent, behaviour is a little
    // different: first there is a pointer released event on the child, then
    // on the parent. This is because the pointer pressed event went
    // unhandled and so was seen by the parent.
    let mouse_up_position = parent.header_presenter().unwrap().get_transformed_bounds().unwrap().bounds.center();
    mouse.up_at(&child, MouseButton::Left, mouse_up_position);
    mouse.up_at(&parent, MouseButton::Left, mouse_up_position);

    assert!(target.selected_item().is_none());
}

#[test]
fn double_clicking_item_header_should_expand_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    collapse_all(&target);

    let item = node_at(&data, 0).child(1);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(!container.is_expanded());
    assert!(header.is_some());

    mouse.double_click(&header.unwrap());

    assert!(container.is_expanded());
}

#[test]
fn double_clicking_item_header_with_no_children_does_not_expand_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    collapse_all(&target);

    let item = node_at(&data, 0).child(1).child(0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(!container.is_expanded());
    assert!(header.is_some());

    mouse.double_click(&header.unwrap());

    assert!(!container.is_expanded());
}

#[test]
fn double_clicking_item_header_should_collapse_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let item = node_at(&data, 0).child(1);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(container.is_expanded());
    assert!(header.is_some());

    mouse.double_click(&header.unwrap());

    assert!(!container.is_expanded());
}

#[test]
fn enter_key_should_collapse_tree_view_item() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::NONE));

    assert!(!container.is_expanded());
}

#[test]
fn enter_plus_ctrl_key_should_collapse_tree_view_item_recursively() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::CONTROL));

    assert!(!container.is_expanded());

    assert_each_item_with_children_is_collapsed(&target, &item);
}

#[test]
fn enter_key_should_expand_tree_view_item() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    collapse_all(&target);

    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(!container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::NONE));

    assert!(container.is_expanded());
}

#[test]
fn enter_plus_ctrl_key_should_expand_tree_view_item_recursively() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    collapse_all(&target);

    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(!container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::CONTROL));

    assert!(container.is_expanded());

    assert_each_item_with_children_is_expanded(&target, &item);
}

#[test]
fn space_key_should_collapse_tree_view_item() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::NONE));

    assert!(!container.is_expanded());
}

#[test]
fn space_plus_ctrl_key_should_collapse_tree_view_item_recursively() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::CONTROL));

    assert!(!container.is_expanded());

    assert_each_item_with_children_is_collapsed(&target, &item);
}

#[test]
fn space_key_should_expand_tree_view_item() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    collapse_all(&target);

    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(!container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::NONE));

    assert!(container.is_expanded());
}

#[test]
fn space_plus_ctrl_key_should_expand_tree_view_item_recursively() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    collapse_all(&target);

    let item = node_at(&data, 0);
    let container = container_of(&target, &item);
    let header = header_child(&container);

    assert!(!container.is_expanded());
    assert!(header.is_some());

    container.raise_event(&key_down(Key::Enter, KeyModifiers::CONTROL));

    assert!(container.is_expanded());

    assert_each_item_with_children_is_expanded(&target, &item);
}

#[test]
fn numpad_star_should_expand_all_children_recursively() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    collapse_all(&target);

    let item = node_at(&data, 0);
    let container = container_of(&target, &item);

    container.raise_event(&key_down(Key::Multiply, KeyModifiers::NONE));

    assert_each_item_with_children_is_expanded(&target, &item);
}

#[test]
fn numpad_slash_should_collapse_all_children_recursively() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let item = node_at(&data, 0);
    let container = container_of(&target, &item);

    container.raise_event(&key_down(Key::Divide, KeyModifiers::NONE));

    assert_each_item_with_children_is_collapsed(&target, &item);
}

#[test]
fn setting_selected_item_should_set_container_selected() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let node = node_at(&data, 0).child(1).child(0);
    let container = container_of(&target, &node);

    target.set_selected_item(item(&node));

    assert!(container.is_selected());
}

#[test]
fn setting_selected_item_should_raise_selected_item_changed_event() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let node = node_at(&data, 0).child(1).child(0);

    let called = Rc::new(Cell::new(false));
    let (flag, expected) = (called.clone(), node.clone());
    target.selection_changed(move |_, e| {
        assert!(e.removed_items().is_empty());
        assert_eq!(1, e.added_items().len());
        assert!(is_node(&e.added_items()[0], &expected));
        flag.set(true);
    });

    target.set_selected_item(item(&node));
    assert!(called.get());
}

#[test]
fn removing_selected_root_item_should_clear_selection() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let node = node_at(&data, 0);

    target.set_selected_item(item(&node));

    data.remove_at(0);

    assert!(target.selected_item().is_none());
    assert!(target.selected_items().is_empty());
}

#[test]
fn resetting_root_items_should_clear_selection() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let node = node_at(&data, 0);

    target.set_selected_item(item(&node));

    data.clear();

    assert!(target.selected_item().is_none());
    assert!(target.selected_items().is_empty());
}

#[test]
fn removing_selected_child_item_should_clear_selection() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let node = node_at(&data, 0).child(1);

    target.set_selected_item(item(&node));

    node_at(&data, 0).children().remove_at(1);

    assert!(target.selected_item().is_none());
    assert!(target.selected_items().is_empty());
}

#[test]
fn replacing_selected_child_item_should_clear_selection() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let node = node_at(&data, 0).child(1);

    target.set_selected_item(item(&node));

    node_at(&data, 0).children().set(1, Node::empty() as BoxedValue);

    assert!(target.selected_item().is_none());
    assert!(target.selected_items().is_empty());
}

#[test]
fn clearing_child_items_should_clear_selection() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let node = node_at(&data, 0).child(1);

    target.set_selected_item(item(&node));

    node_at(&data, 0).children().clear();

    assert!(target.selected_item().is_none());
    assert!(target.selected_items().is_empty());
}

#[test]
fn selected_item_should_be_valid_when_selected_item_changed_event_raised() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    let node = node_at(&data, 0).child(1).child(0);
    let container = container_of(&target, &node);

    let called = Rc::new(Cell::new(false));
    let (flag, expected, tree) = (called.clone(), node.clone(), target.downgrade());
    target.selection_changed(move |_, e| {
        assert!(is_node(&e.added_items()[0], &expected));
        assert!(is_node(&tree.upgrade().unwrap().selected_item(), &expected));
        flag.set(true);
    });

    mouse.click(&container);

    assert!(is_node(&target.selected_item(), &node));
    assert!(container.is_selected());
    assert!(called.get());
}

#[test]
fn bound_selected_item_should_not_be_cleared_when_changing_selection() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data_context = TestDataContext::new();
    let target = create_target(|_| {});

    target.set_data_context(Some(data_context.clone() as BoxedValue));
    target.bind_binding(ItemsControl::items_source_property().as_property(), &ReflectionBinding::new("Items"));
    target.bind_binding(TreeView::selected_item_property().as_property(), &ReflectionBinding::new("SelectedItem"));

    let selected_values: Rc<RefCell<Vec<Option<String>>>> = Rc::new(RefCell::new(Vec::new()));

    let (values, vm) = (selected_values.clone(), Rc::downgrade(&data_context));
    data_context.property_changed.add(Rc::new(move |property_name: &str| {
        if property_name == "SelectedItem" {
            values.borrow_mut().push(vm.upgrade().unwrap().selected_item());
        }
    }));

    selected_values.borrow_mut().push(data_context.selected_item());

    let children = target.items_panel_root().unwrap().children().to_vec();
    mouse.click_with(&children[0], &children[0], MouseButton::Left, None, KeyModifiers::NONE);
    mouse.click_with(&children[2], &children[2], MouseButton::Left, None, KeyModifiers::NONE);

    assert_eq!(3, selected_values.borrow().len());
    assert_eq!(vec![None, Some("Item 0".to_string()), Some("Item 2".to_string())], *selected_values.borrow());
}

#[test]
fn expanding_selected_item_to_be_visible_should_result_in_selected_container() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.expand_all = false;
    });

    target.set_selected_item(item(&node_at(&data, 0).child(1)));

    let root_item = tree_view_item(target.container_from_index(0).unwrap());
    root_item.set_is_expanded(true);
    layout(&target);

    let container = tree_view_item(root_item.container_from_index(1).unwrap());
    assert!(container.is_selected());
}

#[test]
fn logical_children_should_be_set() {
    let _app = start();
    let target = create_target(|o| o.data = Some(None));

    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz "])));
    layout(&target);

    let result: Vec<Option<String>> = StyledElement::logical_children(&target)
        .to_vec()
        .into_iter()
        .filter_map(|x| x.cast::<TreeViewItem>())
        .filter_map(|x| header_child(&x))
        .filter_map(|x| text_of(&x))
        .collect();

    assert_eq!(texts(&["Foo", "Bar", "Baz "]), result);
}

#[test]
fn data_contexts_should_be_correctly_set() {
    let _app = start();
    let baz = TextBlock::new();
    baz.set_text(Some("Baz"));
    let qux = TreeViewItem::new();
    qux.set_header(boxed_str("Qux"));
    let items: Vec<Option<BoxedValue>> = vec![
        boxed_str("Foo"),
        item(&Node::new("Bar")),
        Some(Control::boxed(baz)),
        Some(Control::boxed(qux)),
    ];

    let target = create_target(|_| {});
    let root = target.root.clone();

    root.data_templates().add(FuncDataTemplate::for_type::<Node>(|_, _| Some(button()), false));
    target.set_data_context(boxed_str("Base"));
    target.set_items_source(Some(ItemsSource::from_items(items.clone())));

    let data_contexts: Vec<Option<BoxedValue>> =
        target.items_panel_root().unwrap().children().to_vec().iter().map(|x| x.data_context()).collect();

    let expected = [items[0].clone(), items[1].clone(), boxed_str("Base"), boxed_str("Base")];
    assert_eq!(expected.len(), data_contexts.len());
    for (actual, expected) in data_contexts.iter().zip(expected.iter()) {
        assert!(items_equal(actual, expected));
    }
}

#[test]
fn control_item_should_not_be_name_scope() {
    let _app = start();
    let target = create_target(|o| o.data = Some(None));
    let item = TreeViewItem::new();

    target.items().add(Some(Control::boxed(item.clone())));

    assert!(StyledElement::logical_children(&target).get(0) == item.clone().upcast::<StyledElement>());
    assert!(NameScope::get_name_scope(&item).is_none());
}

#[test]
fn should_react_to_children_changing() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    assert_eq!(texts(&["Root"]), extract_item_header(&target, 0));
    assert_eq!(texts(&["Child1", "Child2", "Child3"]), extract_item_header(&target, 1));
    assert_eq!(texts(&["Grandchild2a"]), extract_item_header(&target, 2));

    // The reference makes sure here that the binding to the children of the
    // node does not get collected.

    node_at(&data, 0).set_children(node_list(&[Node::new("NewChild1")]));

    layout(&target);

    assert_eq!(texts(&["Root"]), extract_item_header(&target, 0));
    assert_eq!(texts(&["NewChild1"]), extract_item_header(&target, 1));
}

#[test]
fn keyboard_navigation_should_move_to_last_selected_node() {
    let _app = start();
    let navigation = KeyboardNavigationHandler::new();
    let data = create_test_tree_data();

    let target = TreeView::new();
    target.set_items_source(source(&data));

    let button = button();

    let panel = StackPanel::new();
    panel.children().add(target.clone());
    panel.children().add(button.clone());
    let root = create_root(&panel.upcast());
    let focus = root.focus_manager();

    root.execute_initial_layout_pass();
    expand_all(&target);

    let node_item = node_at(&data, 0).child(0);
    let node = target.tree_container_from_item(&item(&node_item));
    assert!(node.is_some());
    let node: Ref<InputElement> = node.unwrap().upcast();

    target.set_selected_item(item(&node_item));
    node.focus();
    assert!(focus.get_focused_element() == Some(node.clone()));

    navigation.move_(focus.get_focused_element().as_ref(), NavigationDirection::Next, KeyModifiers::NONE, None);
    assert!(focus.get_focused_element() == Some(button.clone().upcast()));

    navigation.move_(focus.get_focused_element().as_ref(), NavigationDirection::Next, KeyModifiers::NONE, None);
    assert!(focus.get_focused_element() == Some(node));
}

#[test]
fn keyboard_navigation_should_not_crash_if_selected_item_is_not_in_tree() {
    let _app = start();
    let data = create_test_tree_data();

    let selected_node = Node::new("Out of Tree Selected Item");

    let target = TreeView::new();
    TemplatedControl::set_template(&target, Some(create_tree_view_template()));
    target.set_items_source(source(&data));
    target.set_selected_item(item(&selected_node));

    let button = button();

    let panel = StackPanel::new();
    panel.children().add(target.clone());
    panel.children().add(button);
    let root = create_root(&panel.upcast());
    let focus = root.focus_manager();

    root.execute_initial_layout_pass();
    expand_all(&target);

    let node_item = node_at(&data, 0).child(0);
    let node = target.tree_container_from_item(&item(&node_item));
    assert!(node.is_some());
    let node: Ref<InputElement> = node.unwrap().upcast();

    target.set_selected_item(item(&selected_node));
    node.focus();
    assert!(focus.get_focused_element() == Some(node));
}

#[test]
fn pressing_select_all_gesture_should_select_all_nodes() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);

    let key_event = select_all_gesture(&target);

    target.raise_event(&key_event);

    assert_all_child_containers_selected(&target, &root_node);
}

#[test]
fn pressing_select_all_gesture_with_downward_range_selected_should_select_all_nodes() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let from = root_node.child(0);
    let to = root_node.last_child();
    let from_container = container_of(&target, &from);
    let to_container = container_of(&target, &to);

    click_container(&mouse, &from_container, KeyModifiers::NONE);
    click_container(&mouse, &to_container, KeyModifiers::SHIFT);

    let key_event = select_all_gesture(&target);

    target.raise_event(&key_event);

    assert_all_child_containers_selected(&target, &root_node);
}

#[test]
fn pressing_select_all_gesture_with_upward_range_selected_should_select_all_nodes() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let from = root_node.last_child();
    let to = root_node.child(0);
    let from_container = container_of(&target, &from);
    let to_container = container_of(&target, &to);

    click_container(&mouse, &from_container, KeyModifiers::NONE);
    click_container(&mouse, &to_container, KeyModifiers::SHIFT);

    let key_event = select_all_gesture(&target);

    target.raise_event(&key_event);

    assert_all_child_containers_selected(&target, &root_node);
}

#[test]
fn right_click_on_selected_item_should_not_clear_existing_selection() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });

    target.select_all();

    assert_all_child_containers_selected(&target, &node_at(&data, 0));
    assert_eq!(5, target.selected_items().count());

    let children = target.items_panel_root().unwrap().children().to_vec();
    mouse.click_with(&children[0], &children[0], MouseButton::Right, None, KeyModifiers::NONE);

    assert_eq!(5, target.selected_items().count());
}

#[test]
fn right_click_on_unselected_item_should_clear_existing_selection() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let to = root_node.child(0);
    let then = root_node.child(1);
    let from_container = container_of(&target, &root_node);
    let to_container = container_of(&target, &to);
    let then_container = container_of(&target, &then);

    click_container(&mouse, &from_container, KeyModifiers::NONE);
    click_container(&mouse, &to_container, KeyModifiers::SHIFT);

    assert_eq!(2, target.selected_items().count());

    mouse.click_with(&then_container, &then_container, MouseButton::Right, None, KeyModifiers::NONE);

    assert_eq!(1, target.selected_items().count());
}

#[test]
fn shift_right_click_should_not_select_multiple() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let from = root_node.child(0);
    let to = root_node.child(1);
    let from_container = container_of(&target, &from);
    let to_container = container_of(&target, &to);

    mouse.click(&from_container);
    mouse.click_with(&to_container, &to_container, MouseButton::Right, None, KeyModifiers::SHIFT);

    assert_eq!(1, target.selected_items().count());
}

#[test]
fn ctrl_right_click_should_not_select_multiple() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let data = create_test_tree_data();
    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
    });
    let root_node = node_at(&data, 0);
    let from = root_node.child(0);
    let to = root_node.child(1);
    let from_container = container_of(&target, &from);
    let to_container = container_of(&target, &to);

    mouse.click(&from_container);
    mouse.click_with(&to_container, &to_container, MouseButton::Right, None, KeyModifiers::CONTROL);

    assert_eq!(1, target.selected_items().count());
}

#[test]
fn tree_view_items_level_should_be_set() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));

    assert_eq!(0, get_item(&target, &[0]).level());
    assert_eq!(1, get_item(&target, &[0, 0]).level());
    assert_eq!(1, get_item(&target, &[0, 1]).level());
    assert_eq!(1, get_item(&target, &[0, 2]).level());
    assert_eq!(2, get_item(&target, &[0, 1, 0]).level());
}

#[test]
fn tree_view_items_level_should_be_set_for_derived_tree_view() {
    let _app = start();
    let tree = create_test_tree_data();
    let target = DerivedTreeView::new();
    TemplatedControl::set_template(&target, Some(create_tree_view_template()));
    target.set_items_source(source(&tree));

    let root = create_root(&target.clone().upcast());
    root.execute_initial_layout_pass();
    expand_all(&target);

    assert_eq!(0, get_item(&target, &[0]).level());
    assert_eq!(1, get_item(&target, &[0, 0]).level());
    assert_eq!(1, get_item(&target, &[0, 1]).level());
    assert_eq!(1, get_item(&target, &[0, 2]).level());
    assert_eq!(2, get_item(&target, &[0, 1, 0]).level());
}

#[test]
fn adding_node_to_removed_and_re_added_parent_should_not_crash() {
    // Issue #2985
    let _app = start();
    let data = create_test_tree_data();
    let _target = create_target(|o| o.data = Some(source(&data)));
    let parent = node_at(&data, 0);
    let node = parent.child(1);

    parent.children().remove(&(node.clone() as BoxedValue));
    parent.children().add(node.clone() as BoxedValue);

    // #2985 causes an exception here.
    node.children().add(Node::empty() as BoxedValue);
}

#[test]
fn auto_expanding_in_style_should_not_break_range_selection() {
    // Issue #2980.
    let _app = start();
    let mouse = MouseTestHelper::new();

    let data = node_list(&[Node::new("Root1"), Node::new("Root2")]);

    let style = Style::with_setters(
        Selectors::of_type::<TreeViewItem>(),
        [Setter::new(TreeViewItem::is_expanded_property(), true)],
    );

    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.styles = vec![style];
        o.multi_select = true;
    });

    mouse.click(&get_item(&target, &[0]));
    let to = get_item(&target, &[1]);
    mouse.click_with(&to, &to, MouseButton::Left, None, KeyModifiers::SHIFT);
}

#[test]
fn removing_tree_view_from_root_should_preserve_tree_view_items() {
    // Issue #3328
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let root = target.root.clone();

    assert_eq!(5, target.get_realized_tree_containers().len());

    root.set_child(None);

    assert_eq!(5, target.get_realized_tree_containers().len());
    assert_eq!(1, target.items_panel_root().unwrap().children().count());

    let root_node = tree_view_item(target.items_panel_root().unwrap().children().get(0));
    assert_eq!(3, root_node.get_realized_containers().len());
    assert_eq!(3, root_node.items_panel_root().unwrap().children().count());

    let child2_node = tree_view_item(root_node.items_panel_root().unwrap().children().get(1));
    assert_eq!(1, child2_node.get_realized_containers().len());
    assert_eq!(1, child2_node.items_panel_root().unwrap().children().count());
}

#[test]
fn clearing_tree_view_items_clears_index() {
    // Issue #3551
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let root = target.root.clone();
    let root_node = node_at(&data, 0);
    let _container = container_of(&target, &root_node);

    root.set_child(None);

    data.clear();

    assert!(target.get_realized_containers().is_empty());
}

#[test]
fn can_use_derived_tree_view_item() {
    let _app = start();
    let tree = create_test_tree_data();
    let target = DerivedTreeViewWithDerivedTreeViewItems::new();
    TemplatedControl::set_template(&target, Some(create_tree_view_template()));
    target.set_items_source(source(&tree));

    let root = create_root(&target.clone().upcast());
    root.execute_initial_layout_pass();
    expand_all(&target);

    // Verify that all items are derived tree view items.
    for container in target.get_realized_tree_containers() {
        assert!(container.get_type() == DerivedTreeViewItem::TYPE);
    }
}

#[test]
fn can_bind_initial_selected_state_via_item_container_theme() {
    let _app = start();
    let data = create_test_tree_data();
    let selected = [node_at(&data, 0), node_at(&data, 0).child(1)];

    for node in &selected {
        node.set_is_selected(true);
    }

    let item_theme = is_selected_item_theme();

    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.item_container_theme = Some(item_theme);
        o.multi_select = true;
    });

    assert_data_selection(&data, &selected);
    assert_container_selection(&target, &selected);
    assert!(is_node(&target.selected_item(), &selected[0]));
    assert_selected_items(&target, &selected);
}

#[test]
fn can_bind_initial_selected_state_via_style() {
    let _app = start();
    let data = create_test_tree_data();
    let selected = [node_at(&data, 0), node_at(&data, 0).child(1)];

    for node in &selected {
        node.set_is_selected(true);
    }

    let style = Style::with_setters(Selectors::of_type::<TreeViewItem>(), [is_selected_setter()]);

    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.multi_select = true;
        o.styles = vec![style];
    });

    assert_data_selection(&data, &selected);
    assert_container_selection(&target, &selected);
    assert!(is_node(&target.selected_item(), &selected[0]));
    assert_selected_items(&target, &selected);
}

#[test]
fn selection_state_is_updated_via_is_selected_binding() {
    let _app = start();
    let data = create_test_tree_data();
    let selected = [node_at(&data, 0), node_at(&data, 0).child(1)];

    selected[0].set_is_selected(true);

    let item_theme = is_selected_item_theme();

    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.item_container_theme = Some(item_theme);
        o.multi_select = true;
    });

    selected[1].set_is_selected(true);

    assert_data_selection(&data, &selected);
    assert_container_selection(&target, &selected);
    assert!(is_node(&target.selected_item(), &selected[0]));
    assert_selected_items(&target, &selected);
}

#[test]
fn selection_state_is_updated_via_is_selected_binding_on_expand() {
    let _app = start();
    let data = create_test_tree_data();
    let selected = [node_at(&data, 0), node_at(&data, 0).child(1)];

    for node in &selected {
        node.set_is_selected(true);
    }

    let item_theme = is_selected_item_theme();

    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.expand_all = false;
        o.item_container_theme = Some(item_theme);
        o.multi_select = true;
    });

    let root_container = tree_view_item(target.container_from_index(0).unwrap());

    // Root tree view item isn't expanded so selection for child won't have
    // been picked up by the is selected binding yet.
    assert_container_selection(&target, &selected[..1]);
    assert!(is_node(&target.selected_item(), &selected[0]));
    assert_selected_items(&target, &selected[..1]);

    root_container.set_is_expanded(true);
    layout(&target);

    // Root is expanded so now all expected items will be selected.
    assert_data_selection(&data, &selected);
    assert_container_selection(&target, &selected);
    assert!(is_node(&target.selected_item(), &selected[0]));
    assert_selected_items(&target, &selected);
}

#[test]
fn selection_state_is_updated_via_is_selected_binding_on_expand_single_select() {
    let _app = start();
    let data = create_test_tree_data();
    let selected = [node_at(&data, 0), node_at(&data, 0).child(1)];

    for node in &selected {
        node.set_is_selected(true);
    }

    let item_theme = is_selected_item_theme();

    let target = create_target(|o| {
        o.data = Some(source(&data));
        o.expand_all = false;
        o.item_container_theme = Some(item_theme);
    });

    let root_container = tree_view_item(target.container_from_index(0).unwrap());

    // Root tree view item isn't expanded so selection for child won't have
    // been picked up by the is selected binding yet.
    assert_container_selection(&target, &selected[..1]);
    assert!(is_node(&target.selected_item(), &selected[0]));
    assert_selected_items(&target, &selected[..1]);

    root_container.set_is_expanded(true);
    layout(&target);

    // Root is expanded and newly revealed selected node will replace current
    // selection given that we're in single selection mode.
    let selected = [selected[1].clone()];
    assert_data_selection(&data, &selected);
    assert_container_selection(&target, &selected);
    assert!(is_node(&target.selected_item(), &selected[0]));
    assert_selected_items(&target, &selected);
}

#[test]
fn collapse_event_can_be_captured_by_tree_view_when_collapsing_tree_view_item() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let item = node_at(&data, 0);
    let container = container_of(&target, &item);

    let raised = Rc::new(Cell::new(false));
    let source: Rc<RefCell<Option<Ref<FerroObject>>>> = Rc::new(RefCell::new(None));
    let (flag, sink) = (raised.clone(), source.clone());
    target.add_handler(TreeViewItem::collapsed_event(), move |_, e| {
        flag.set(true);
        *sink.borrow_mut() = e.source();
    });
    container.set_is_expanded(false);
    assert!(raised.get());
    assert!(*source.borrow() == Some(container.upcast()));
}

#[test]
fn collapse_event_should_be_raised_when_collapsing_tree_view_item() {
    let _app = start();
    let data = create_test_tree_data();
    let target = create_target(|o| o.data = Some(source(&data)));
    let item = node_at(&data, 0);
    let container = container_of(&target, &item);

    let raised = Rc::new(Cell::new(false));
    let source: Rc<RefCell<Option<Ref<FerroObject>>>> = Rc::new(RefCell::new(None));
    let (flag, sink) = (raised.clone(), source.clone());
    container.add_handler(TreeViewItem::collapsed_event(), move |_, e| {
        flag.set(true);
        *sink.borrow_mut() = e.source();
    });
    container.set_is_expanded(false);
    assert!(raised.get());
    assert!(*source.borrow() == Some(container.upcast()));
}
