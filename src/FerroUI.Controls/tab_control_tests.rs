//! The reference tests use user controls and content pages; here a
//! `ContentControl` stands in for both (neither class is part of this crate
//! yet).

use crate::platform::ITopLevelImpl;
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{HeaderedContentControl, SelectingItemsControl, TemplatedControl, TemplatedControlImpl};
use crate::testing::{MockImplKind, MockWindowImpl, TestServices, UnitTestApplication};
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::shapes::Path;
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::{
    items_equal, AssignedBinding, Border, Button, ContentControl, Control, Decorator, Dock, DockPanel, ItemsSource,
    Panel, StackPanel, TabControl, TabItem, TextBlock,
};
use crate::{ContentControlImpl, ControlImpl, TopLevel, TopLevelImpl};
use ferroui_base::animation::{CrossFade, IPageTransition, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{Maybe, ModelRef, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingBase, ReflectionBinding, RelativeSource, RelativeSourceMode, TemplateBinding};
use ferroui_base::input::{
    AccessKeyHandler, FocusManager, IAccessKeyHandler, IKeyboardDevice, IKeyboardNavigationHandler, InputElement,
    InputElementImpl, Key, KeyEventArgs, KeyModifiers, KeyboardDevice, KeyboardNavigationHandler, NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::EllipseGeometry;
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherTask};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, instantiate, AnyValue, BoxedValue, FerroLocator, FerroObjectImpl,
    ObjectType, Rect, Ref, Size, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
    scope
}

fn tab_control_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<TabControl>(|_, scope| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));

        let host2 = ContentPresenter::new();
        host2.set_name(Some("PART_SelectedContentHost2".to_string()));
        host2.set_is_visible(false);

        let host = ContentPresenter::new();
        host.set_name(Some("PART_SelectedContentHost".to_string()));

        let panel = Panel::new();
        panel.children().add(host2.register_in_name_scope(&**scope));
        panel.children().add(host.register_in_name_scope(&**scope));

        let stack_panel = StackPanel::new();
        stack_panel.children().add(items_presenter.register_in_name_scope(&**scope));
        stack_panel.children().add(panel);
        stack_panel.upcast()
    }))
}

fn header_presenter() -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some("PART_ContentPresenter".to_string()));
    presenter.bind_binding(
        ContentPresenter::content_property().as_property(),
        &TemplateBinding::new(HeaderedContentControl::header_property().as_property()),
    );
    presenter.bind_binding(
        ContentPresenter::content_template_property().as_property(),
        &TemplateBinding::new(HeaderedContentControl::header_template_property().as_property()),
    );
    presenter.set_recognizes_access_key(true);
    presenter
}

fn tab_item_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<TabItem>(|_, scope| {
        header_presenter().register_in_name_scope(&**scope).upcast()
    }))
}

fn tab_item_with_icon_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<TabItem>(|_, scope| {
        let icon_presenter = ContentPresenter::new();
        icon_presenter.set_name(Some("PART_IconPresenter".to_string()));
        icon_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(TabItem::icon_property().as_property()),
        );
        icon_presenter.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &TemplateBinding::new(TabItem::icon_template_property().as_property()),
        );

        let stack_panel = StackPanel::new();
        stack_panel.children().add(icon_presenter.register_in_name_scope(&**scope));
        stack_panel.children().add(header_presenter().register_in_name_scope(&**scope));
        stack_panel.upcast()
    }))
}

fn theme_resource(theme: Ref<ControlTheme>) -> Option<BoxedValue> {
    Some(Rc::new(theme))
}

fn create_tab_control_control_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        TabControl::TYPE,
        [Setter::new(TemplatedControl::template_property(), tab_control_template())],
    )
}

fn create_tab_item_control_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(TabItem::TYPE, [Setter::new(TemplatedControl::template_property(), tab_item_template())])
}

fn create_root(child: &Ref<TabControl>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.resources().add(TabControl::TYPE, theme_resource(create_tab_control_control_theme()));
    root.resources().add(TabItem::TYPE, theme_resource(create_tab_item_control_theme()));
    root.data_templates().add(FuncDataTemplate::for_type::<Rc<Item>>(
        |x, _| {
            let button = Button::new();
            button.set_content(boxed_str(&x.value));
            Some(button.upcast())
        },
        false,
    ));
    root.set_child(child.clone());
    root
}

fn prepare(target: &TabControl) {
    apply_template(target);
    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
}

fn raise_key_event(target: &Control, key: Key) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    target.raise_event(&args);
}

fn tab_items(target: &TabControl) -> Vec<Ref<TabItem>> {
    target.logical_children().to_vec().into_iter().filter_map(|child| child.cast::<TabItem>()).collect()
}

fn apply_template(target: &TabControl) {
    target.apply_template();

    target.presenter().unwrap().apply_template();

    for tab_item in tab_items(target) {
        tab_item.set_template(tab_item_template());

        tab_item.apply_template();

        tab_item.presenter().unwrap().update_child();
    }

    target.content_part().unwrap().apply_template();
}

/// A model item with identity equality.
struct Item {
    value: String,
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Item {
    fn new(value: &str) -> Rc<Item> {
        Rc::new(Item { value: value.to_string() })
    }
}

struct TabDataContextViewModel {
    selected_item: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for TabDataContextViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TabDataContextViewModel, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("SelectedItem", |vm| vm.selected_item.borrow().clone(), |vm, v| vm
        .set_selected_item(v)));

impl TabDataContextViewModel {
    fn new(selected_item: &str) -> Rc<Self> {
        Model::new_model(Self {
            selected_item: RefCell::new(Some(selected_item.to_string())),
            property_changed: Event::new(),
        })
    }

    fn set_selected_item(&self, value: Option<String>) {
        if *self.selected_item.borrow() != value {
            *self.selected_item.borrow_mut() = value;
            self.property_changed.raise("SelectedItem");
        }
    }
}

struct MainViewModel {
    tab1: Rc<Tab1ViewModel>,
    tab2: Rc<Tab2ViewModel>,
}

ferro_model!(MainViewModel, |b| b
    .read_only::<ModelRef<Tab1ViewModel>>("Tab1", |vm| Some(vm.tab1.clone()))
    .read_only::<ModelRef<Tab2ViewModel>>("Tab2", |vm| Some(vm.tab2.clone())));

impl MainViewModel {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            tab1: Model::new_model(Tab1ViewModel { name: "Tab 1 message here".to_string() }),
            tab2: Model::new_model(Tab2ViewModel { name: "Tab 2 message here".to_string() }),
        })
    }
}

struct Tab1ViewModel {
    name: String,
}

ferro_model!(Tab1ViewModel, |b| b.read_only::<Value<String>>("Name", |vm| vm.name.clone()));

struct Tab2ViewModel {
    name: String,
}

ferro_model!(Tab2ViewModel, |b| b.read_only::<Value<String>>("Name", |vm| vm.name.clone()));

/// Whether an untyped value is the given model instance.
fn is_model<M: PartialEq + 'static>(value: &Option<BoxedValue>, model: &Rc<M>) -> bool {
    value.as_ref().is_some_and(|value| {
        let value: &dyn AnyValue = &**value;
        value.downcast_ref::<M>().is_some_and(|m| std::ptr::eq(m, &**model))
    })
}

/// Whether an untyped value is the given control.
fn is_control<T: ObjectType>(value: &Option<BoxedValue>, control: &Ref<T>) -> bool
where
    Ref<T>: Clone,
    T: ferroui_base::Upcast<Control>,
{
    let control: Ref<Control> = control.clone().upcast();
    value.as_ref().and_then(Control::from_boxed) == Some(control)
}

fn str_of(value: &Option<BoxedValue>) -> Option<String> {
    value.as_ref().and_then(string_of)
}

fn boxed<T: ObjectType + ferroui_base::Upcast<Control>>(control: &Ref<T>) -> Option<BoxedValue> {
    Some(Control::boxed(control.clone().upcast::<Control>()))
}

fn binding(path: &str) -> Rc<ReflectionBinding> {
    ReflectionBinding::new(path)
}

/// A text block with the given text.
fn text_block(text: &str) -> Ref<Control> {
    let text_block = TextBlock::new();
    text_block.set_text(Some(text));
    text_block.upcast()
}

/// A text block whose text is bound to `path` of its data context.
fn bound_text_block(path: &str) -> Ref<TextBlock> {
    let text_block = TextBlock::new();
    text_block.bind_binding(TextBlock::text_property().as_property(), &binding(path));
    text_block
}

fn tab(configure: impl FnOnce(&Ref<TabItem>)) -> Ref<TabItem> {
    let item = TabItem::new();
    configure(&item);
    item
}

fn named_tab(name: &str, content: &str) -> Ref<TabItem> {
    tab(|x| {
        x.set_name(Some(name.to_string()));
        x.set_content(boxed_str(content));
    })
}

fn headered_tab(header: &str) -> Ref<TabItem> {
    tab(|x| x.set_header(boxed_str(header)))
}

fn create_target(template: bool, items: &[&Ref<TabItem>]) -> Ref<TabControl> {
    let target = TabControl::new();
    if template {
        target.set_template(tab_control_template());
    }
    for item in items {
        target.items().add(boxed(item));
    }
    target
}

fn selected_tab(target: &TabControl) -> Option<Ref<TabItem>> {
    target.selected_item().as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<TabItem>())
}

fn item_tab(target: &TabControl, index: usize) -> Ref<TabItem> {
    let item = target.items().get_at(index).as_ref().and_then(Control::from_boxed).unwrap();
    assert_eq!(item.get_type(), TabItem::TYPE);
    item.cast::<TabItem>().unwrap()
}

fn content_child(target: &TabControl) -> Ref<Control> {
    target.content_part().unwrap().child().unwrap()
}

fn focused_element(target: &TabControl) -> Option<Ref<InputElement>> {
    FocusManager::get_focus_manager(target).unwrap().get_focused_element()
}

fn named_presenter(target: &TabControl, name: &str) -> Ref<ContentPresenter> {
    let mut found: Vec<Ref<ContentPresenter>> = target
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<ContentPresenter>())
        .filter(|presenter| presenter.name().as_deref() == Some(name))
        .collect();
    assert_eq!(found.len(), 1);
    found.remove(0)
}

fn template_presenter(tab_item: &TabItem, name: &str) -> Ref<ContentPresenter> {
    tab_item
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<ContentPresenter>())
        .find(|presenter| presenter.name().as_deref() == Some(name))
        .unwrap()
}

/// A recorded call of [`TestTransition::start`]: the contents of the two
/// presenters and the direction.
type TransitionStart = (Option<BoxedValue>, Option<BoxedValue>, bool);

/// Stands in for the mocked page transition of the reference tests.
struct TestTransition {
    starts: RefCell<Vec<TransitionStart>>,
    /// Whether the returned task never completes.
    gated: bool,
}

impl TestTransition {
    fn new() -> Rc<Self> {
        Rc::new(Self { starts: RefCell::new(Vec::new()), gated: false })
    }

    fn gated() -> Rc<Self> {
        Rc::new(Self { starts: RefCell::new(Vec::new()), gated: true })
    }

    fn value(self: &Rc<Self>) -> Option<Rc<dyn IPageTransition>> {
        let transition: Rc<dyn IPageTransition> = self.clone();
        Some(transition)
    }

    fn count(&self, forward: bool) -> usize {
        self.starts.borrow().iter().filter(|start| start.2 == forward).count()
    }
}

impl IPageTransition for TestTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        _cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let content = |visual: Option<&Ref<Visual>>| {
            let presenter = visual.and_then(|visual| visual.clone().cast::<ContentPresenter>());
            presenter.and_then(|presenter| presenter.content())
        };
        self.starts.borrow_mut().push((content(from), content(to), forward));

        let scheduler = Dispatcher::current_dispatcher().to_task_scheduler();
        if self.gated {
            scheduler.start_local(std::future::pending::<()>())
        } else {
            scheduler.start_local(async {})
        }
    }
}

#[test]
fn first_tab_should_be_selected_by_default() {
    let _scope = start();
    let selected = named_tab("first", "foo");
    let target = create_target(true, &[&selected, &named_tab("second", "bar")]);

    target.apply_template();

    assert_eq!(0, target.selected_index());
    assert_eq!(Some(selected), selected_tab(&target));
}

#[test]
fn pre_selecting_tab_item_should_set_selected_content_after_it_was_added() {
    let _scope = start();
    const SECOND_CONTENT: &str = "Second";
    let target = create_target(
        true,
        &[
            &headered_tab("First"),
            &tab(|x| {
                x.set_header(boxed_str("Second"));
                x.set_content(boxed_str(SECOND_CONTENT));
                x.set_is_selected(true);
            }),
        ],
    );

    apply_template(&target);

    assert_eq!(Some(SECOND_CONTENT.to_string()), str_of(&target.selected_content()));
}

#[test]
fn logical_children_should_be_tab_items() {
    let _scope = start();
    let target = create_target(
        true,
        &[&tab(|x| x.set_content(boxed_str("foo"))), &tab(|x| x.set_content(boxed_str("bar")))],
    );

    let items = || -> Vec<Ref<StyledElement>> {
        target
            .items()
            .to_vec()
            .iter()
            .map(|item| item.as_ref().and_then(Control::from_boxed).unwrap().upcast())
            .collect()
    };

    assert_eq!(items(), target.logical_children().to_vec());
    target.apply_template();
    assert_eq!(items(), target.logical_children().to_vec());
}

#[test]
fn removal_should_set_first_tab() {
    let _scope = start();
    let target =
        create_target(true, &[&named_tab("first", "foo"), &named_tab("second", "bar"), &named_tab("3rd", "barf")]);

    prepare(&target);
    target.set_selected_item(target.items().get_at(1));

    let item = item_tab(&target, 1);
    assert_eq!(Some(item.clone()), selected_tab(&target));
    assert!(items_equal(&item.content(), &target.selected_content()));

    target.items().remove_at(1);

    let item = item_tab(&target, 0);
    assert_eq!(Some(item.clone()), selected_tab(&target));
    assert!(items_equal(&item.content(), &target.selected_content()));
}

#[test]
fn removal_should_set_new_item0_when_item0_selected() {
    let _scope = start();
    let target =
        create_target(true, &[&named_tab("first", "foo"), &named_tab("second", "bar"), &named_tab("3rd", "barf")]);

    prepare(&target);
    target.set_selected_item(target.items().get_at(0));

    let item = item_tab(&target, 0);
    assert_eq!(Some(item.clone()), selected_tab(&target));
    assert!(items_equal(&item.content(), &target.selected_content()));

    target.items().remove_at(0);

    let item = item_tab(&target, 0);
    assert_eq!(Some(item.clone()), selected_tab(&target));
    assert!(items_equal(&item.content(), &target.selected_content()));
}

#[test]
fn removal_should_set_new_item0_when_item0_selected_with_data_template() {
    let _scope = start();

    let collection = Rc::new(FerroList::from_items([Item::new("first"), Item::new("second"), Item::new("3rd")]));
    let item = |index: usize| -> Option<BoxedValue> { Some(Rc::new(collection.get(index))) };

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_items_source(Some(ItemsSource::from(collection.clone())));

    prepare(&target);
    target.set_selected_item(item(0));

    assert!(items_equal(&item(0), &target.selected_item()));
    assert!(items_equal(&item(0), &target.selected_content()));

    collection.remove_at(0);

    assert!(items_equal(&item(0), &target.selected_item()));
    assert!(items_equal(&item(0), &target.selected_content()));
}

#[test]
fn tab_item_templates_should_be_set_before_tab_item_apply_template() {
    let _scope = start();
    let template: Option<Rc<dyn IControlTemplate>> =
        Some(FuncControlTemplate::for_type::<TabItem>(|_, _| Decorator::new().upcast()));
    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<TabItem>(),
        [Setter::new(TemplatedControl::template_property(), template.clone())],
    ));
    let target =
        create_target(true, &[&named_tab("first", "foo"), &named_tab("second", "bar"), &named_tab("3rd", "barf")]);
    root.set_child(target.clone());

    let collection: Vec<Ref<TabItem>> = (0..3).map(|index| item_tab(&target, index)).collect();
    assert!(collection[0].template() == template);
    assert!(collection[1].template() == template);
    assert!(collection[2].template() == template);
}

#[test]
fn data_contexts_should_be_correctly_set() {
    let _scope = start();
    let items: Vec<Option<BoxedValue>> = vec![
        boxed_str("Foo"),
        Some(Rc::new(Item::new("Bar"))),
        Some(Control::boxed(text_block("Baz"))),
        boxed(&tab(|x| x.set_content(boxed_str("Qux")))),
        boxed(&tab(|x| x.set_content(Some(Control::boxed(text_block("Bob")))))),
        boxed(&tab(|x| {
            x.set_data_context(boxed_str("Rob"));
            x.set_content(Some(Control::boxed(text_block("Bob"))));
        })),
    ];

    let target = TabControl::new();
    target.set_data_context(boxed_str("Base"));
    target.set_items_source(Some(ItemsSource::from_items(items.clone())));

    let root = create_root(&target);
    root.execute_initial_layout_pass();

    assert!(content_child(&target).is::<TextBlock>());
    let data_context = content_child(&target).data_context();
    assert!(items_equal(&items[0], &data_context));

    target.set_selected_index(1);
    assert!(content_child(&target).is::<Button>());
    let data_context = content_child(&target).data_context();
    assert!(items_equal(&items[1], &data_context));

    target.set_selected_index(2);
    assert!(content_child(&target).is::<TextBlock>());
    let data_context = content_child(&target).data_context();
    assert_eq!(Some("Base".to_string()), str_of(&data_context));

    target.set_selected_index(3);
    assert!(content_child(&target).is::<TextBlock>());
    let data_context = content_child(&target).data_context();
    assert_eq!(Some("Qux".to_string()), str_of(&data_context));

    target.set_selected_index(4);
    let data_context = target.content_part().unwrap().data_context();
    assert_eq!(Some("Base".to_string()), str_of(&data_context));

    target.set_selected_index(5);
    let data_context = content_child(&target).data_context();
    assert_eq!(Some("Rob".to_string()), str_of(&data_context));
}

/// Non-headered control items should result in tab items with empty header.
///
/// If a tab control is created with non headered controls as its items,
/// don't try to display the control in the header: if the control is part of
/// the header then *that* control would also end up in the content region,
/// resulting in dual-parentage breakage.
#[test]
fn non_i_headered_control_items_should_be_ignored() {
    let _scope = start();
    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.items().add(Some(Control::boxed(text_block("foo"))));
    target.items().add(Some(Control::boxed(text_block("bar"))));

    apply_template(&target);

    let result: Vec<Option<BoxedValue>> = tab_items(&target).iter().map(|x| x.header()).collect();

    assert_eq!(2, result.len());
    assert!(result.iter().all(Option::is_none));
}

#[test]
fn should_handle_changing_to_tab_item_with_null_content() {
    let _scope = start();
    let target = create_target(
        true,
        &[
            &headered_tab("Foo"),
            &tab(|x| {
                x.set_header(boxed_str("Foo"));
                x.set_content(Some(Control::boxed(Decorator::new())));
            }),
            &headered_tab("Baz"),
        ],
    );

    apply_template(&target);

    target.set_selected_index(2);

    let page = selected_tab(&target).unwrap();

    assert!(page.content().is_none());
}

fn bar_template() -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::for_type::<String>(
        |x, _| {
            let text_block = TextBlock::new();
            text_block.set_tag(boxed_str("bar"));
            text_block.set_text(Some(x));
            Some(text_block.upcast())
        },
        false,
    ))
}

#[test]
fn data_template_created_content_should_be_logical_child_after_apply_template() {
    let _scope = start();
    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_content_template(bar_template());
    target.set_items_source(Some(ItemsSource::from_strs(["Foo"])));
    let _root = TestRoot::with_child(&target);

    apply_template(&target);
    target.content_part().unwrap().update_child();

    let content = content_child(&target);
    assert!(content.is::<TextBlock>());
    assert_eq!(Some("bar".to_string()), str_of(&content.tag()));
    assert_eq!(Some(target.clone().upcast::<StyledElement>()), content.parent());
    let logical: Ref<StyledElement> = content.upcast();
    assert_eq!(1, target.logical_children().to_vec().iter().filter(|child| **child == logical).count());
}

#[test]
fn selected_content_template_updates_after_new_content_template() {
    let _scope = start();
    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_items_source(Some(ItemsSource::from_strs(["Foo"])));
    let _root = TestRoot::with_child(&target);

    apply_template(&target);
    target.content_part().unwrap().update_child();

    assert!(content_child(&target).is::<TextBlock>());
    assert!(content_child(&target).tag().is_none());

    target.set_content_template(bar_template());

    assert!(content_child(&target).is::<TextBlock>());
    assert_eq!(Some("bar".to_string()), str_of(&content_child(&target).tag()));
}

#[test]
fn previous_content_template_is_not_reused_when_tab_item_changes() {
    let _scope = start();

    let templates_built = Rc::new(Cell::new(0));

    let tab_item_factory = |content: &str| {
        let content = boxed_str(content);
        let templates_built = templates_built.clone();
        tab(|x| {
            x.set_content(content.clone());
            x.set_content_template(Some(FuncDataTemplate::new(
                |_| true,
                move |actual, _| {
                    assert!(items_equal(&content, actual));
                    templates_built.set(templates_built.get() + 1);
                    Some(Border::new().upcast())
                },
                false,
            )));
        })
    };

    let target =
        create_target(true, &[&tab_item_factory("First tab content"), &tab_item_factory("Second tab content")]);

    let _root = TestRoot::with_child(&target);
    apply_template(&target);

    target.set_selected_index(0);
    target.set_selected_index(1);

    assert_eq!(2, templates_built.get());
}

#[test]
fn should_not_propagate_data_context_to_tab_item_content() {
    let _scope = start();
    let data_context = boxed_str("DataContext");

    let tab_item = TabItem::new();

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(data_context.clone());
    target.items().add(boxed(&tab_item));

    apply_template(&target);

    assert!(!items_equal(&data_context, &tab_item.content()));
}

/// The data of `can_have_empty_tab_control` (an anonymous object in the
/// reference).
struct TabsData {
    tabs: ItemsSource,
}

ferroui_base::ferro_model!(TabsData, |b| b
    .read_only::<Value<ItemsSource>>("Tabs", |data| data.tabs.clone()));

/// The reference test loads the window from markup; it is built in code.
#[test]
fn can_have_empty_tab_control() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ItemsSource::register_binding_conversion::<ItemsSource>();

    let tab_control = TabControl::new();
    tab_control.set_name(Some("tabs".to_string()));
    tab_control.bind_binding(crate::ItemsControl::items_source_property().as_property(), &*binding("Tabs"));
    let window = crate::Window::new();
    window.set_content(boxed(&tab_control));

    tab_control.set_data_context(Some(Model::new_model(TabsData { tabs: ItemsSource::from_strs([]) })));
    window.apply_template();

    assert_eq!(0, tab_control.items_source().expect("an items source").count());
}

/// The reference test loads the control from markup; the loader brackets
/// the property assignments with the initialization of the control.
#[test]
fn should_have_initial_selected_value() {
    let _scope = start();

    let tab_control = TabControl::new();
    tab_control.begin_init();
    tab_control.set_name(Some("tabs".to_string()));
    tab_control.set_tag(boxed_str("World"));
    let selected_value = binding("Tag");
    selected_value.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::SelfMode)));
    tab_control.bind_binding(SelectingItemsControl::selected_value_property().as_property(), &selected_value);
    let header: Rc<dyn BindingBase> = Rc::new(binding("Header"));
    tab_control.set_selected_value_binding(Some(AssignedBinding::new(header)));
    tab_control.items().add(boxed(&headered_tab("Hello")));
    tab_control.items().add(boxed(&headered_tab("World")));
    tab_control.end_init();

    assert_eq!(Some("World".to_string()), str_of(&tab_control.selected_value()));
    assert_eq!(1, tab_control.selected_index());
}

#[test]
fn tab_navigation_should_move_to_first_tab_item_when_no_anchor_element_selected() {
    let _scope = start();

    let target = create_target(true, &[&headered_tab("foo"), &headered_tab("bar"), &headered_tab("baz")]);

    let button = Button::new();
    button.set_content(boxed_str("Button"));
    DockPanel::set_dock(&button, Dock::Top);

    let dock_panel = DockPanel::new();
    dock_panel.children().add(button.clone());
    dock_panel.children().add(target.clone());
    let root = TestRoot::with_child(&dock_panel);

    let navigation = KeyboardNavigationHandler::new();
    navigation.set_owner(&root.clone().upcast());

    root.execute_initial_layout_pass();

    button.focus();
    raise_key_event(&button, Key::Tab);

    let item = target.container_from_index(0);
    assert!(item.is_some());
    assert_eq!(item.map(Ref::upcast::<InputElement>), focused_element(&target));
}

#[test]
fn tab_navigation_should_move_to_anchor_tab_item() {
    let _scope = start();

    let target = create_target(true, &[&headered_tab("foo"), &headered_tab("bar"), &headered_tab("baz")]);

    let button = Button::new();
    button.set_content(boxed_str("Button"));
    DockPanel::set_dock(&button, Dock::Top);

    let dock_panel = DockPanel::new();
    dock_panel.children().add(button.clone());
    dock_panel.children().add(target.clone());
    let root = TestRoot::with_child(&dock_panel);
    root.set_width(1000.0);
    root.set_height(1000.0);

    let navigation = KeyboardNavigationHandler::new();
    navigation.set_owner(&root.clone().upcast());

    root.execute_initial_layout_pass();

    button.focus();
    target.selection().set_anchor_index(1);
    raise_key_event(&button, Key::Tab);

    let item = target.container_from_index(1);
    assert!(item.is_some());
    assert_eq!(item.clone().map(Ref::upcast::<InputElement>), focused_element(&target));

    raise_key_event(&item.unwrap(), Key::Tab);

    assert_eq!(Some(button.clone().upcast::<InputElement>()), focused_element(&target));

    target.selection().set_anchor_index(2);
    raise_key_event(&button, Key::Tab);

    let item = target.container_from_index(2);
    assert_eq!(item.map(Ref::upcast::<InputElement>), focused_element(&target));
}

#[test]
fn tab_item_header_should_be_settable_by_style_when_data_context_is_set() {
    let _scope = start();
    let tab_item = TabItem::new();
    tab_item.set_data_context(boxed_str("Some DataContext"));

    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<TabItem>(),
        [Setter::new(HeaderedContentControl::header_property(), boxed_str("Header from style"))],
    ));
    root.set_child(tab_item.clone());

    assert_eq!(Some("Header from style".to_string()), str_of(&tab_item.header()));
}

fn placement_items() -> Option<ItemsSource> {
    Some(ItemsSource::from_items([
        boxed_str("Foo"),
        boxed(&tab(|x| x.set_content(Some(Control::boxed(text_block("Baz")))))),
    ]))
}

fn placements(target: &TabControl) -> Vec<Option<Dock>> {
    tab_items(target).iter().map(|x| x.tab_strip_placement()).collect()
}

#[test]
fn tab_item_tab_strip_placement_should_be_correctly_set() {
    let _scope = start();

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(boxed_str("Base"));
    target.set_items_source(placement_items());

    apply_template(&target);

    assert_eq!(vec![Some(Dock::Top), Some(Dock::Top)], placements(&target));

    target.set_tab_strip_placement(Dock::Right);
    assert_eq!(vec![Some(Dock::Right), Some(Dock::Right)], placements(&target));
}

#[test]
fn tab_item_tab_strip_placement_should_be_correctly_set_for_new_items() {
    let _scope = start();

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(boxed_str("Base"));

    apply_template(&target);

    target.set_items_source(placement_items());

    assert_eq!(vec![Some(Dock::Top), Some(Dock::Top)], placements(&target));

    target.set_tab_strip_placement(Dock::Right);
    assert_eq!(vec![Some(Dock::Right), Some(Dock::Right)], placements(&target));
}

#[repr(C)]
struct TestTopLevel {
    base: TopLevel,
}

ferro_class!(TestTopLevel: TopLevel);
ferro_impl_classes!(
    TestTopLevel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl
);

impl TestTopLevel {
    fn new(platform_impl: &Rc<MockWindowImpl>) -> Ref<Self> {
        let platform_impl: Rc<dyn ITopLevelImpl> = platform_impl.clone();
        instantiate(Self { base: TopLevel::construct(platform_impl) })
    }
}

fn should_tab_control_recognizes_access_key(access_key: Key, access_key_symbol: &str, selected_tab_index: usize) {
    fn create_template() -> Option<Rc<dyn IControlTemplate>> {
        Some(FuncControlTemplate::new(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".to_string()));
            presenter.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(ContentControl::content_property().as_property()),
            );
            presenter.bind_binding(
                ContentPresenter::content_template_property().as_property(),
                &TemplateBinding::new(ContentControl::content_template_property().as_property()),
            );
            presenter.register_in_name_scope(&**scope).upcast()
        }))
    }

    fn create_mock_top_level_impl() -> Rc<MockWindowImpl> {
        MockWindowImpl::bare(MockImplKind::TopLevel)
    }

    fn key_event(down: bool, key: Key, key_symbol: Option<&str>, modifiers: KeyModifiers) -> KeyEventArgs {
        let mut e = KeyEventArgs::new();
        e.set_routed_event(Some(if down { InputElement::key_down_event() } else { InputElement::key_up_event() }));
        e.key = key;
        e.key_symbol = key_symbol.map(str::to_string);
        e.key_modifiers = modifiers;
        e
    }

    let kd = KeyboardDevice::new();
    let device = kd.clone();
    let _app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_access_key_handler(|| {
                let handler: Rc<dyn IAccessKeyHandler> = AccessKeyHandler::new();
                Some(handler)
            })
            .with_keyboard_device(move || {
                let device: Rc<dyn IKeyboardDevice> = device.clone();
                Some(device)
            }),
    );
    let platform_impl = create_mock_top_level_impl();

    let tab_control = create_target(
        true,
        &[
            &headered_tab("General"),
            &headered_tab("_Arch"),
            &headered_tab("_Leaf"),
            &tab(|x| {
                x.set_header(boxed_str("_Disabled"));
                x.set_is_enabled(false);
            }),
        ],
    );
    kd.set_focused_element(
        Some(&item_tab(&tab_control, selected_tab_index).upcast()),
        NavigationMethod::Unspecified,
        KeyModifiers::NONE,
    );

    let root = TestTopLevel::new(&platform_impl);
    root.set_template(create_template());
    root.set_content(Some(Control::boxed(&tab_control)));

    root.apply_template();
    root.presenter().unwrap().update_child();
    apply_template(&tab_control);

    root.raise_event(&key_event(true, Key::LeftAlt, None, KeyModifiers::NONE));
    root.raise_event(&key_event(true, access_key, Some(access_key_symbol), KeyModifiers::ALT));
    root.raise_event(&key_event(false, access_key, Some(access_key_symbol), KeyModifiers::ALT));
    root.raise_event(&key_event(false, Key::LeftAlt, None, KeyModifiers::NONE));

    assert_eq!(selected_tab_index as i32, tab_control.selected_index());
}

#[test]
fn should_tab_control_recognizes_access_key_a() {
    should_tab_control_recognizes_access_key(Key::A, "a", 1);
}

#[test]
fn should_tab_control_recognizes_access_key_l() {
    should_tab_control_recognizes_access_key(Key::L, "l", 2);
}

#[test]
fn should_tab_control_recognizes_access_key_d() {
    should_tab_control_recognizes_access_key(Key::D, "d", 0);
}

#[test]
fn page_transition_is_null_by_default() {
    let _scope = start();
    let target = TabControl::new();
    target.set_template(tab_control_template());
    assert!(target.page_transition().is_none());
}

#[test]
fn page_transition_round_trips() {
    let _scope = start();
    let transition: Rc<dyn IPageTransition> = Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(100.0)));
    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_page_transition(Some(transition.clone()));
    assert!(target.page_transition() == Some(transition));
}

#[test]
fn page_transition_start_is_called_when_tab_switches() {
    let _scope = start();

    let transition = TestTransition::new();

    let target = create_target(false, &[&named_tab("first", "Alpha"), &named_tab("second", "Beta")]);
    target.set_page_transition(transition.value());

    let root = create_root(&target);
    root.execute_initial_layout_pass();

    // Switch tab: triggers the transition and invalidates the arrange.
    target.set_selected_index(1);

    // Execute layout pass to invoke the arrange override, which fires the
    // transition.
    root.layout_manager().execute_layout_pass();

    // forward = true (index 1 > 0)
    assert_eq!(1, transition.count(true));
}

#[test]
fn page_transition_forward_is_false_when_switching_to_earlier_tab() {
    let _scope = start();

    let transition = TestTransition::new();

    let target = create_target(
        false,
        &[&named_tab("first", "Alpha"), &named_tab("second", "Beta"), &named_tab("third", "Gamma")],
    );
    target.set_page_transition(transition.value());

    let root = create_root(&target);
    root.execute_initial_layout_pass();

    // Go forward to tab 2
    target.set_selected_index(2);
    root.layout_manager().execute_layout_pass();

    // Now go backward to tab 0
    target.set_selected_index(0);
    root.layout_manager().execute_layout_pass();

    // forward = false (index 0 < 2)
    assert_eq!(1, transition.count(false));
}

fn content_page(content: &str) -> Ref<ContentControl> {
    let page = ContentControl::new();
    page.set_content(boxed_str(content));
    page
}

fn page_tab(name: &str, page: &Ref<ContentControl>) -> Ref<TabItem> {
    tab(|x| {
        x.set_name(Some(name.to_string()));
        x.set_content(boxed(page));
    })
}

#[test]
fn interrupted_page_transition_can_select_original_control_before_previous_transition_completes() {
    let _scope = start();

    let first_page = content_page("Alpha");
    let second_page = content_page("Beta");
    let transition = TestTransition::gated();

    let target = create_target(false, &[&page_tab("first", &first_page), &page_tab("second", &second_page)]);
    target.set_page_transition(transition.value());

    let root = create_root(&target);
    root.execute_initial_layout_pass();

    target.set_selected_index(1);
    root.layout_manager().execute_layout_pass();

    {
        let starts = transition.starts.borrow();
        assert_eq!(1, starts.len());
        assert!(is_control(&starts[0].0, &first_page));
        assert!(is_control(&starts[0].1, &second_page));
        assert!(starts[0].2);
    }

    target.set_selected_index(0);

    root.layout_manager().execute_layout_pass();

    let starts = transition.starts.borrow();
    assert_eq!(2, starts.len());
    assert!(starts[1].0.is_none());
    assert!(is_control(&starts[1].1, &first_page));
    assert!(!starts[1].2);
    assert!(is_control(&target.selected_content(), &first_page));
}

#[test]
fn pending_page_transition_can_select_original_control_before_transition_starts() {
    let _scope = start();

    let first_page = content_page("Alpha");
    let second_page = content_page("Beta");
    let transition = TestTransition::new();

    let target = create_target(false, &[&page_tab("first", &first_page), &page_tab("second", &second_page)]);
    target.set_page_transition(transition.value());

    let root = create_root(&target);
    root.execute_initial_layout_pass();

    target.set_selected_index(1);
    target.set_selected_index(0);

    root.layout_manager().execute_layout_pass();

    let starts = transition.starts.borrow();
    assert_eq!(1, starts.len());
    assert!(starts[0].0.is_none());
    assert!(is_control(&starts[0].1, &first_page));
    assert!(!starts[0].2);
    assert!(is_control(&target.selected_content(), &first_page));
}

#[test]
fn interrupted_page_transition_clears_reused_control_from_owning_selected_content_host() {
    let _scope = start();

    let first_page = content_page("Alpha");
    let second_page = content_page("Beta");
    let transition = TestTransition::new();

    let target = create_target(false, &[&page_tab("first", &first_page), &page_tab("second", &second_page)]);

    let root = create_root(&target);
    root.execute_initial_layout_pass();

    target.set_selected_index(1);
    root.layout_manager().execute_layout_pass();

    let primary = named_presenter(&target, "PART_SelectedContentHost");
    let secondary = named_presenter(&target, "PART_SelectedContentHost2");

    // Simulate the stale presenter ownership that can happen when tab
    // changes interrupt a transition: the page is still parented by the
    // named content host, but the active field no longer points at that
    // host.
    primary.set_content_with_data_context(boxed(&first_page), None);
    secondary.set_is_visible(false);
    target.set_content_presenters_for_test(Some(secondary.clone()), Some(secondary.clone()));

    target.set_page_transition(transition.value());
    target.set_selected_index(0);

    assert!(is_control(&target.selected_content(), &first_page));
    assert!(primary.content().is_none());
    assert!(is_control(&secondary.content(), &first_page));
}

/// Issue #18280: When switching tabs, a user control inside a tab item has
/// its data context set to null, causing two-way bindings on child controls
/// to propagate null back to the view model. Verify that after switching
/// away and back, the data context binding still resolves correctly.
#[test]
fn switching_tab_should_preserve_data_context_binding_on_user_control_content() {
    let _scope = start();

    let view_model = TabDataContextViewModel::new("Item1");

    // Create a user control with an explicit data context binding, matching
    // the issue scenario.
    let user_control = ContentControl::new();
    user_control.bind_binding(StyledElement::data_context_property().as_property(), &binding("SelectedItem"));

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab1"));
        x.set_content(boxed(&user_control));
    })));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab2"));
        x.set_content(boxed_str("Other content"));
    })));

    let _root = TestRoot::with_child(&target);
    prepare(&target);

    // Verify initial state
    assert_eq!(0, target.selected_index());
    assert_eq!(Some("Item1".to_string()), str_of(&user_control.data_context()));

    // Switch to second tab and back
    target.set_selected_index(1);
    target.set_selected_index(0);

    // The user control's data context binding should still resolve
    // correctly.
    assert_eq!(Some("Item1".to_string()), str_of(&user_control.data_context()));

    // Verify the binding is still live by changing the source property.
    view_model.set_selected_item(Some("Item2".to_string()));
    assert_eq!(Some("Item2".to_string()), str_of(&user_control.data_context()));
}

/// Issue #20845: When a data context binding is placed on the child of a
/// tab item, the data context is null. The binding hasn't resolved when the
/// content's data context is captured while updating the selected content,
/// so the captured value is null.
#[test]
fn tab_item_child_data_context_binding_should_work() {
    let _scope = start();

    let view_model = MainViewModel::new();

    let tab1_view = ContentControl::new();
    tab1_view.bind_binding(StyledElement::data_context_property().as_property(), &binding("Tab1"));

    // Add a child text block that binds to a property on the view model of
    // the first tab.
    let text_block = bound_text_block("Name");
    tab1_view.set_content(boxed(&text_block));

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab1"));
        x.set_content(boxed(&tab1_view));
    })));

    let _root = TestRoot::with_child(&target);
    prepare(&target);

    // The user control's data context should be the view model of the first
    // tab.
    assert!(is_model(&tab1_view.data_context(), &view_model.tab1));

    // The text block should display the name from the view model of the
    // first tab.
    assert_eq!(Some("Tab 1 message here".to_string()), text_block.text());
}

/// Issue #20845 (comment): Putting the data context binding on the tab item
/// itself is also broken. The child should inherit the tab item's data
/// context.
#[test]
fn tab_item_child_with_data_context_binding_should_propagate_to_children() {
    let _scope = start();

    let view_model = MainViewModel::new();

    let text_block = bound_text_block("Name");
    let tab1_view = ContentControl::new();
    tab1_view.set_content(boxed(&text_block));

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab1"));
        x.bind_binding(StyledElement::data_context_property().as_property(), &binding("Tab1"));
        x.set_content(boxed(&tab1_view));
    })));

    let _root = TestRoot::with_child(&target);
    prepare(&target);

    // The tab item's data context should be the view model of the first
    // tab.
    let tab_item = item_tab(&target, 0);
    assert!(is_model(&tab_item.data_context(), &view_model.tab1));

    // The user control should inherit the tab item's data context.
    assert!(is_model(&tab1_view.data_context(), &view_model.tab1));

    // The text block should display the name from the view model of the
    // first tab.
    assert_eq!(Some("Tab 1 message here".to_string()), text_block.text());
}

/// Issue #20845: the data context binding should survive tab switches.
#[test]
fn switching_tabs_should_not_null_out_data_context_bound_properties() {
    let _scope = start();

    let view_model = MainViewModel::new();

    let tab1_view = ContentControl::new();
    tab1_view.bind_binding(StyledElement::data_context_property().as_property(), &binding("Tab1"));
    let text_block = bound_text_block("Name");
    tab1_view.set_content(boxed(&text_block));

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab1"));
        x.set_content(boxed(&tab1_view));
    })));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab2"));
        x.set_content(boxed_str("Other content"));
    })));

    let _root = TestRoot::with_child(&target);
    prepare(&target);

    assert!(is_model(&tab1_view.data_context(), &view_model.tab1));
    assert_eq!(Some("Tab 1 message here".to_string()), text_block.text());

    // Switch to tab 2 and back
    target.set_selected_index(1);
    target.set_selected_index(0);

    // The data context binding should still be resolved correctly.
    assert!(is_model(&tab1_view.data_context(), &view_model.tab1));
    assert_eq!(Some("Tab 1 message here".to_string()), text_block.text());
}

fn bound_tab(header: &str, path: &str, view: &Ref<ContentControl>) -> Ref<TabItem> {
    tab(|x| {
        x.set_header(boxed_str(header));
        x.bind_binding(StyledElement::data_context_property().as_property(), &binding(path));
        x.set_content(boxed(view));
    })
}

fn track_data_contexts(control: &Control) -> Rc<RefCell<Vec<Option<BoxedValue>>>> {
    let data_contexts = Rc::new(RefCell::new(Vec::new()));
    let result = data_contexts.clone();
    // The subscription lives as long as the control.
    let _ = control.property_changed(move |e| {
        if e.property() == StyledElement::data_context_property().as_property() {
            data_contexts.borrow_mut().push(e.get_new_value::<Option<BoxedValue>>());
        }
    });
    result
}

/// When the content of the content part is set, updating the child of the
/// presenter clears its data context before we can set it to the container's
/// data context. This causes the content to briefly inherit the tab
/// control's data context instead of the tab item's.
#[test]
fn content_should_not_temporarily_get_wrong_data_context_when_switching_tabs() {
    let _scope = start();

    let view_model = MainViewModel::new();

    let tab1_view = ContentControl::new();
    let tab2_view = ContentControl::new();

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&bound_tab("Tab1", "Tab1", &tab1_view)));
    target.items().add(boxed(&bound_tab("Tab2", "Tab2", &tab2_view)));

    let _root = TestRoot::with_child(&target);
    prepare(&target);

    assert!(is_model(&tab1_view.data_context(), &view_model.tab1));

    // Track all data context values the new content receives during the
    // switch.
    let data_contexts = track_data_contexts(&tab2_view);

    target.set_selected_index(1);

    // The second view should only have received the correct data context
    // (the view model of the second tab). It should NOT have temporarily
    // received the tab control's data context (the main view model).
    assert!(data_contexts.borrow().iter().all(|dc| is_model(dc, &view_model.tab2)));
    assert!(is_model(&tab2_view.data_context(), &view_model.tab2));
}

/// When a page transition is set, the old content stays in the content part
/// while the new content goes into the second content presenter. The data
/// context subscription for the new container should not update the content
/// part's data context (which still holds the old content).
#[test]
fn transition_should_not_apply_new_data_context_to_old_content() {
    let _scope = start();

    let view_model = MainViewModel::new();

    let tab1_view = ContentControl::new();
    let tab2_view = ContentControl::new();

    let transition = TestTransition::new();

    let target = TabControl::new();
    target.set_page_transition(transition.value());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&bound_tab("Tab1", "Tab1", &tab1_view)));
    target.items().add(boxed(&bound_tab("Tab2", "Tab2", &tab2_view)));

    let root = create_root(&target);
    root.execute_initial_layout_pass();

    assert!(is_model(&tab1_view.data_context(), &view_model.tab1));

    // Track all data context values the OLD content receives during the
    // transition.
    let old_content_data_contexts = track_data_contexts(&tab1_view);

    // Switch tab: triggers the transition
    target.set_selected_index(1);
    root.layout_manager().execute_layout_pass();

    // The old content (the first view) should NOT have received the second
    // tab's data context.
    assert!(!old_content_data_contexts.borrow().iter().any(|dc| is_model(dc, &view_model.tab2)));
}

/// A template for user control content whose child binds its text to the
/// `Tag` of the content.
fn user_control_template(template_child: &Rc<RefCell<Option<Ref<TextBlock>>>>) -> Option<Rc<dyn IDataTemplate>> {
    let template_child = template_child.clone();
    Some(FuncDataTemplate::for_type_with_match::<Ref<Control>>(
        |control| control.is::<ContentControl>(),
        move |_, _| {
            let child = bound_text_block("Tag");
            *template_child.borrow_mut() = Some(child.clone());
            Some(child.upcast())
        },
        false,
    ))
}

/// When a tab item has a content template and its content is a control, the
/// content presenter should set its data context to the content (so the
/// template can bind to the control's properties), not the tab item's data
/// context.
#[test]
fn content_template_with_control_content_should_set_data_context_to_content() {
    let _scope = start();

    let view_model = MainViewModel::new();
    let user_control = ContentControl::new();
    user_control.set_tag(boxed_str("my-content"));

    let template_child = Rc::new(RefCell::new(None));
    let content_template = user_control_template(&template_child);

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab1"));
        x.bind_binding(StyledElement::data_context_property().as_property(), &binding("Tab1"));
        x.set_content_template(content_template.clone());
        x.set_content(boxed(&user_control));
    })));

    let _root = TestRoot::with_child(&target);
    prepare(&target);

    // The content presenter's data context should be the content (the user
    // control), not the tab item's data context (the view model of the first
    // tab), because a content template is set.
    assert!(is_control(&target.content_part().unwrap().data_context(), &user_control));
    let template_child = template_child.borrow().clone();
    assert!(template_child.is_some());
    assert_eq!(Some("my-content".to_string()), template_child.unwrap().text());
}

/// Same as above but verifies the behavior after switching tabs.
#[test]
fn content_template_with_control_content_should_set_data_context_to_content_after_tab_switch() {
    let _scope = start();

    let view_model = MainViewModel::new();
    let user_control = ContentControl::new();
    user_control.set_tag(boxed_str("my-content"));

    let template_child = Rc::new(RefCell::new(None));
    let content_template = user_control_template(&template_child);

    let target = TabControl::new();
    target.set_template(tab_control_template());
    target.set_data_context(Some(view_model.clone() as BoxedValue));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab1"));
        x.bind_binding(StyledElement::data_context_property().as_property(), &binding("Tab1"));
        x.set_content_template(content_template.clone());
        x.set_content(boxed(&user_control));
    })));
    target.items().add(boxed(&tab(|x| {
        x.set_header(boxed_str("Tab2"));
        x.set_content(boxed_str("Other content"));
    })));

    let _root = TestRoot::with_child(&target);
    prepare(&target);

    assert!(is_control(&target.content_part().unwrap().data_context(), &user_control));

    // Switch away and back.
    target.set_selected_index(1);
    target.set_selected_index(0);

    // The data context should still be the content, not the tab item's data
    // context.
    assert!(is_control(&target.content_part().unwrap().data_context(), &user_control));
    let template_child = template_child.borrow().clone();
    assert!(template_child.is_some());
    assert_eq!(Some("my-content".to_string()), template_child.unwrap().text());
}

#[test]
fn tab_item_icon_template_creates_content_from_non_control_icon() {
    let _scope = start();
    let tab_item = TabItem::new();
    tab_item.set_icon(boxed_str("home"));
    tab_item.set_icon_template(Some(FuncDataTemplate::new(
        |_| true,
        |val, _| {
            let text_block = TextBlock::new();
            text_block.set_text(str_of(val).as_deref());
            Some(text_block.upcast())
        },
        false,
    )));
    tab_item.set_template(tab_item_with_icon_template());

    let _root = TestRoot::with_child(&tab_item);
    tab_item.apply_template();
    tab_item.presenter().unwrap().update_child();

    let icon_presenter = template_presenter(&tab_item, "PART_IconPresenter");
    assert_eq!(Some("home".to_string()), str_of(&icon_presenter.content()));
    assert!(icon_presenter.content_template().is_some());

    icon_presenter.update_child();
    let text_block = icon_presenter.child().and_then(|child| child.cast::<TextBlock>());
    assert!(text_block.is_some());
    assert_eq!(Some("home".to_string()), text_block.unwrap().text());
}

#[test]
fn tab_item_icon_without_template_renders_control_directly() {
    let _scope = start();
    let icon = Path::new();
    icon.set_data(EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0)));
    let tab_item = TabItem::new();
    tab_item.set_icon(boxed(&icon));
    tab_item.set_template(tab_item_with_icon_template());

    let _root = TestRoot::with_child(&tab_item);
    tab_item.apply_template();
    tab_item.presenter().unwrap().update_child();

    let icon_presenter = template_presenter(&tab_item, "PART_IconPresenter");
    assert!(is_control(&icon_presenter.content(), &icon));
    assert!(icon_presenter.content_template().is_none());
}

#[test]
fn tab_item_icon_change_updates_presenter_content() {
    let _scope = start();
    let tab_item = TabItem::new();
    tab_item.set_icon(boxed_str("first"));
    tab_item.set_template(tab_item_with_icon_template());

    let _root = TestRoot::with_child(&tab_item);
    tab_item.apply_template();
    tab_item.presenter().unwrap().update_child();

    let icon_presenter = template_presenter(&tab_item, "PART_IconPresenter");
    assert_eq!(Some("first".to_string()), str_of(&icon_presenter.content()));

    tab_item.set_icon(boxed_str("second"));
    assert_eq!(Some("second".to_string()), str_of(&icon_presenter.content()));
}

fn border_template() -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::new(|_| true, |_, _| Some(Border::new().upcast()), false))
}

#[test]
fn tab_item_indicator_template_default_is_null() {
    let _scope = start();
    let tab_item = TabItem::new();
    assert!(tab_item.indicator_template().is_none());
}

#[test]
fn tab_item_indicator_template_round_trips() {
    let _scope = start();
    let template = border_template();
    let tab_item = TabItem::new();
    tab_item.set_indicator_template(template.clone());
    assert!(tab_item.indicator_template() == template);
}

#[test]
fn tab_item_indicator_template_can_be_set_to_null() {
    let _scope = start();
    let template = border_template();
    let tab_item = TabItem::new();
    tab_item.set_indicator_template(template);
    tab_item.set_indicator_template(None);
    assert!(tab_item.indicator_template().is_none());
}

#[test]
fn tab_control_indicator_template_default_is_null() {
    let _scope = start();
    let tc = TabControl::new();
    assert!(tc.indicator_template().is_none());
}

#[test]
fn tab_control_indicator_template_round_trips() {
    let _scope = start();
    let template = border_template();
    let tc = TabControl::new();
    tc.set_indicator_template(template.clone());
    assert!(tc.indicator_template() == template);
}

#[test]
fn tab_control_indicator_template_can_be_set_to_null() {
    let _scope = start();
    let template = border_template();
    let tc = TabControl::new();
    tc.set_indicator_template(template);
    tc.set_indicator_template(None);
    assert!(tc.indicator_template().is_none());
}

#[test]
fn tab_control_indicator_template_does_not_overwrite_user_set_tab_item_indicator_template() {
    let _scope = start();
    let tab_items = [headered_tab("A"), headered_tab("B")];
    let user_template = border_template();
    tab_items[0].set_indicator_template(user_template.clone());

    let tab_control_template: Option<Rc<dyn IDataTemplate>> =
        Some(FuncDataTemplate::new(|_| true, |_, _| Some(TextBlock::new().upcast()), false));
    let tc = TabControl::new();
    tc.set_items_source(Some(ItemsSource::from_items(tab_items.iter().map(boxed))));
    tc.set_indicator_template(tab_control_template.clone());
    tc.set_template(Some(FuncControlTemplate::for_type::<TabControl>(|_, scope| {
        let ip = ItemsPresenter::new();
        ip.set_name(Some("PART_ItemsPresenter".to_string()));
        let cp = ContentPresenter::new();
        cp.set_name(Some("PART_SelectedContentHost".to_string()));
        let panel = Panel::new();
        panel.children().add(ip.register_in_name_scope(&**scope));
        panel.children().add(cp.register_in_name_scope(&**scope));
        panel.upcast()
    })));

    let _root = TestRoot::with_child(&tc);
    tc.apply_template();
    if let Some(presenter) = tc.presenter() {
        presenter.apply_template();
    }

    // The tab item with a local value must keep it
    assert!(tab_items[0].indicator_template() == user_template);
    // The tab item without a local value gets the tab control template
    assert!(tab_items[1].indicator_template() == tab_control_template);
}

#[test]
fn only_first_visible_and_enabled_tab_should_be_selected_by_default() {
    let _scope = start();
    let target = create_target(
        true,
        &[
            &tab(|x| {
                x.set_header(boxed_str("hidden"));
                x.set_is_visible(false);
            }),
            &headered_tab("visible"),
        ],
    );

    apply_template(&target);

    assert_eq!(1, target.selected_index());
}

#[test]
fn only_first_enabled_tab_should_be_selected_by_default() {
    let _scope = start();
    let target = create_target(
        true,
        &[
            &tab(|x| {
                x.set_header(boxed_str("disabled"));
                x.set_is_enabled(false);
            }),
            &headered_tab("enabled"),
        ],
    );

    apply_template(&target);

    assert_eq!(1, target.selected_index());
}
