//! Port of the reference `MenuItemTests`.

use crate::i_clickable_control::as_clickable_control;
use crate::items_source::ItemsSource;
use crate::platform::{IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowImpl};
use crate::presenters::ContentPresenter;
use crate::primitives::HeaderedSelectingItemsControl;
use crate::templates::{FuncDataTemplate, IDataTemplate};
use crate::test_command::TestCommand;
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::testing::{
    create_test_theme, mock_screen, MockScreenImpl, MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use crate::{
    Button, ContextMenu, Control, ItemsControl, Menu, MenuFlyout, MenuItem, MenuItemToggleType, Panel, StackPanel,
    TextBlock, Window,
};
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::input::{ICommand, InputManager};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_model, BoxedValue, PixelRect, Ref};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The running application of a test, with the text services that the
/// headers are laid out with (the reference services of these tests carry a
/// font manager).
struct TestApplication {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

/// The reference `Application()`: the styled window services with an input
/// manager and windows whose screen is 100 by 100 and which create popups.
fn application() -> TestApplication {
    fn create_window_impl() -> Rc<dyn IWindowImpl> {
        let screen = PixelRect::new(0, 0, 100, 100);
        let screen_impl: Rc<dyn IScreenImpl> = MockScreenImpl::new(vec![mock_screen(1.0, screen, screen, true)]);

        let window_impl = MockWindowingPlatform::create_window_mock();
        let weak = Rc::downgrade(&window_impl);
        window_impl.setup_create_popup(move |_| {
            let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
            Some(MockWindowingPlatform::create_popup_mock(parent) as Rc<dyn IPopupImpl>)
        });
        window_impl.setup_feature::<dyn IScreenImpl>(screen_impl);
        window_impl
    }

    let services = TestServices::styled_window()
        .with_input_manager(Rc::new(InputManager::new()))
        .with_windowing_platform(MockWindowingPlatform::with_window_impl(create_window_impl));

    let app = UnitTestApplication::start(services);
    ItemsSource::register_binding_conversion::<ItemsSource>();
    TestApplication { _text: TextTestScope::new(), _app: app }
}

/// The reference `new TestRoot(true, child)`: a root that is styled by the
/// styles of the application, which here are the styles of the test theme
/// added to the root.
fn styled_test_root(child: &Ref<Menu>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.styles().add(create_test_theme());
    root.set_child(child.clone());
    root
}

struct MenuViewModel {
    header: String,
    children: ItemsSource,
}

ferro_model!(MenuViewModel, |b| b
    .read_only::<Value<String>>("Header", |vm| vm.header.clone())
    .read_only::<Value<ItemsSource>>("Children", |vm| vm.children.clone()));

impl MenuViewModel {
    fn new(header: &str) -> Rc<Self> {
        Self::with_children(header, &[])
    }

    fn with_children(header: &str, children: &[Rc<MenuViewModel>]) -> Rc<Self> {
        Model::new_model(Self { header: header.to_string(), children: items_source(children) })
    }
}

fn items_source(items: &[Rc<MenuViewModel>]) -> ItemsSource {
    ItemsSource::from_items(items.iter().map(|item| {
        let item: BoxedValue = item.clone();
        Some(item)
    }))
}

struct CommandViewModel {
    command: Rc<TestCommand>,
}

ferro_model!(CommandViewModel, |b| b
    .read_only::<Value<Option<Rc<dyn ICommand>>>>("Command", |vm| vm.command.as_command()));

/// A data context without a `Command` property.
struct EmptyViewModel;

ferro_model!(EmptyViewModel, |b| b);

fn binding_setter(property: &'static ferroui_base::FerroProperty, path: &str) -> Rc<Setter> {
    Setter::new_binding_base(property, Rc::new(ReflectionBinding::new(path)))
}

fn menu_item_with_header(header: &str) -> Ref<MenuItem> {
    let item = MenuItem::new();
    item.set_header(boxed_str(header));
    item
}

fn radio_menu_item(group_name: Option<&str>, is_checked: bool) -> Ref<MenuItem> {
    let item = MenuItem::new();
    item.set_group_name(group_name.map(str::to_string));
    item.set_is_checked(is_checked);
    item.set_toggle_type(MenuItemToggleType::Radio);
    item
}

fn add_item(items: impl std::borrow::Borrow<crate::ItemCollection>, item: &Ref<MenuItem>) {
    items.borrow().add(Some(Control::boxed(item)));
}

/// A command that counts the calls of its "can execute".
fn counting_command() -> (Rc<TestCommand>, Rc<Cell<i32>>) {
    let can_execute_call_count = Rc::new(Cell::new(0));
    let count = can_execute_call_count.clone();
    let command = TestCommand::with_can_execute(move |_| {
        count.set(count.get() + 1);
        true
    });
    (command, can_execute_call_count)
}

fn boxed_bool(value: bool) -> Option<BoxedValue> {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

fn run_loaded_jobs() {
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
}

#[test]
fn header_of_minus_should_apply_separator_pseudoclass() {
    let target = menu_item_with_header("-");

    assert!(target.classes().contains(":separator"));
}

#[test]
fn separator_item_should_set_focusable_false() {
    let target = menu_item_with_header("-");

    assert!(!target.focusable());
}

#[test]
fn menu_item_is_disabled_when_command_is_enabled_but_is_enabled_is_false() {
    let _scope = test_scope();
    let command = TestCommand::new(true);
    let target = MenuItem::new();
    target.set_is_enabled(false);
    target.set_command(command.as_command());

    let _root = TestRoot::with_child(&target);

    assert!(!target.is_effectively_enabled());
}

#[test]
fn menu_item_is_disabled_when_bound_command_doesnt_exist() {
    let target = MenuItem::new();
    target.bind_binding(MenuItem::command_property(), &ReflectionBinding::new("Command"));

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());
}

#[test]
fn menu_item_with_styled_command_binding_should_be_enabled_with_child_missing_command() {
    let _app = application();

    let view_model = MenuViewModel::with_children("Parent", &[MenuViewModel::new("Child")]);

    let context_menu = ContextMenu::new();
    context_menu.set_items_source(Some(items_source(&[view_model.clone()])));
    context_menu.styles().add(Style::with_setters(
        Selectors::of_type::<MenuItem>(),
        [
            binding_setter(HeaderedSelectingItemsControl::header_property().as_property(), "Header"),
            binding_setter(ItemsControl::items_source_property().as_property(), "Children"),
            binding_setter(MenuItem::command_property().as_property(), "Command"),
        ],
    ));

    let window = Window::new();
    window.set_context_menu(&context_menu);
    window.show();
    context_menu.open();

    let parent_menu_item =
        context_menu.container_from_index(0).and_then(|container| container.cast::<MenuItem>()).expect("a menu item");

    let view_model_value: BoxedValue = view_model.clone();
    assert!(parent_menu_item.data_context().is_some_and(|data_context| Rc::ptr_eq(&data_context, &view_model_value)));
    assert!(parent_menu_item.items_source().is_some_and(|source| source.ptr_eq(&view_model.children)));
    assert!(parent_menu_item.is_enabled());
    assert!(parent_menu_item.is_effectively_enabled());
}

#[test]
fn menu_item_is_disabled_when_bound_command_is_removed() {
    let view_model = Model::new_model(CommandViewModel { command: TestCommand::new(true) });

    let target = MenuItem::new();
    target.set_data_context(Some(view_model));
    target.bind_binding(MenuItem::command_property(), &ReflectionBinding::new("Command"));

    assert!(target.is_enabled());
    assert!(target.is_effectively_enabled());

    target.set_data_context(None);

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());
}

#[test]
fn menu_item_is_enabled_when_added_to_logical_tree_and_bound_command_is_added() {
    let _scope = test_scope();
    let view_model = Model::new_model(CommandViewModel { command: TestCommand::new(true) });

    let target = MenuItem::new();
    target.set_data_context(Some(Model::new_model(EmptyViewModel)));
    target.bind_binding(MenuItem::command_property(), &ReflectionBinding::new("Command"));
    let _root = TestRoot::with_child(&target);

    run_loaded_jobs();

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());

    target.set_data_context(Some(view_model));

    assert!(target.is_enabled());
    assert!(target.is_effectively_enabled());
}

#[test]
fn menu_item_is_disabled_when_disabled_bound_command_is_added() {
    let view_model = Model::new_model(CommandViewModel { command: TestCommand::new(false) });

    let target = MenuItem::new();
    target.set_data_context(Some(Model::new_model(EmptyViewModel)));
    target.bind_binding(MenuItem::command_property(), &ReflectionBinding::new("Command"));

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());

    target.set_data_context(Some(view_model));

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());
}

#[test]
fn menu_item_does_not_subscribe_to_command_can_execute_changed_until_added_to_logical_tree() {
    let command = TestCommand::new(true);
    let target = MenuItem::new();
    target.set_command(command.as_command());

    assert_eq!(0, command.subscription_count());
}

#[test]
fn menu_item_subscribes_to_command_can_execute_changed_when_added_to_logical_tree() {
    let _scope = test_scope();
    let command = TestCommand::new(true);
    let target = MenuItem::new();
    target.set_command(command.as_command());
    let _root = TestRoot::with_child(&target);

    assert_eq!(1, command.subscription_count());
}

#[test]
fn menu_item_unsubscribes_from_command_can_execute_changed_when_removed_from_logical_tree() {
    let _scope = test_scope();
    let command = TestCommand::new(true);
    let target = MenuItem::new();
    target.set_command(command.as_command());
    let root = TestRoot::with_child(&target);

    root.set_child(None);
    assert_eq!(0, command.subscription_count());
}

#[test]
fn menu_item_invokes_can_execute_when_added_to_logical_tree_and_command_parameter_changed() {
    let _scope = test_scope();
    let command = TestCommand::with_can_execute(|p| {
        p.and_then(|p| p.downcast_ref::<bool>()).is_some_and(|value| *value)
    });
    let target = MenuItem::new();
    target.set_command(command.as_command());
    let _root = TestRoot::with_child(&target);

    run_loaded_jobs();

    target.set_command_parameter(boxed_bool(true));
    assert!(target.is_effectively_enabled());

    target.set_command_parameter(boxed_bool(false));
    assert!(!target.is_effectively_enabled());
}

#[test]
fn menu_item_does_not_invoke_can_execute_when_context_menu_closed() {
    let _app = application();
    let (command, can_execute_call_count) = counting_command();
    let target = MenuItem::new();
    let context_menu = ContextMenu::new();
    add_item(context_menu.items(), &target);
    let panel = Panel::new();
    panel.set_context_menu(&context_menu);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    window.apply_styling();
    window.apply_template();
    window.presenter().unwrap().apply_template();
    run_loaded_jobs();

    assert!(target.is_effectively_enabled());
    target.set_command(command.as_command());
    assert_eq!(0, can_execute_call_count.get());

    target.set_command_parameter(boxed_bool(false));
    assert_eq!(0, can_execute_call_count.get());

    command.raise_can_execute_changed();
    assert_eq!(0, can_execute_call_count.get());

    context_menu.open();
    run_loaded_jobs();
    // 3 because popup is changing logical child and moreover we need to
    // invalidate again after the item is attached to the visual tree
    assert_eq!(3, can_execute_call_count.get());

    command.raise_can_execute_changed();
    assert_eq!(4, can_execute_call_count.get());

    target.set_command_parameter(boxed_bool(true));
    assert_eq!(5, can_execute_call_count.get());
}

/// The flyout measures its presenter before the popup opens, but that
/// measure realizes no item: in the presenter template the items presenter
/// is the content of a scroll viewer, and the scroll content presenter binds
/// its content only once it is attached to a visual tree. The item is
/// therefore attached to the logical tree once (in the first layout pass of
/// the popup root) and to the visual tree once: 2 calls. A context menu owns
/// its items as logical children, so they are attached when the menu becomes
/// the child of the popup, again when the popup host re-roots the menu, and
/// once to the visual tree: 3 calls.
#[test]
fn menu_item_does_not_invoke_can_execute_when_menu_flyout_closed() {
    let _app = application();
    let (command, can_execute_call_count) = counting_command();
    let target = MenuItem::new();
    let flyout = MenuFlyout::new();
    add_item(flyout.items(), &target);
    let button = Button::new();
    button.set_flyout(&flyout);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&button)));
    window.apply_styling();
    window.apply_template();
    window.presenter().unwrap().apply_template();
    run_loaded_jobs();
    assert!(target.is_effectively_enabled());
    target.set_command(command.as_command());
    assert_eq!(0, can_execute_call_count.get());

    target.set_command_parameter(boxed_bool(false));
    assert_eq!(0, can_execute_call_count.get());

    command.raise_can_execute_changed();
    assert_eq!(0, can_execute_call_count.get());

    flyout.show_at(&button);
    run_loaded_jobs();
    // 2 because we need to invalidate after the item is attached to the
    // visual tree
    assert_eq!(2, can_execute_call_count.get());

    command.raise_can_execute_changed();
    assert_eq!(3, can_execute_call_count.get());

    target.set_command_parameter(boxed_bool(true));
    assert_eq!(4, can_execute_call_count.get());
}

#[test]
fn menu_item_does_not_invoke_can_execute_when_parent_menu_item_closed() {
    let _app = application();
    let (command, can_execute_call_count) = counting_command();
    let target = MenuItem::new();
    let parent_menu_item = MenuItem::new();
    add_item(parent_menu_item.items(), &target);
    let context_menu = ContextMenu::new();
    add_item(context_menu.items(), &parent_menu_item);
    let panel = Panel::new();
    panel.set_context_menu(&context_menu);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    window.apply_styling();
    window.apply_template();
    window.presenter().unwrap().apply_template();
    context_menu.open();

    run_loaded_jobs();

    assert!(target.is_effectively_enabled());
    target.set_command(command.as_command());
    assert_eq!(0, can_execute_call_count.get());

    target.set_command_parameter(boxed_bool(false));
    assert_eq!(0, can_execute_call_count.get());

    command.raise_can_execute_changed();
    assert_eq!(0, can_execute_call_count.get());

    // The reference catches the exception of a failed popup host creation.
    let _ = catch_unwind(AssertUnwindSafe(|| parent_menu_item.set_is_sub_menu_open(true)));
    assert_eq!(1, can_execute_call_count.get());

    command.raise_can_execute_changed();
    assert_eq!(2, can_execute_call_count.get());

    target.set_command_parameter(boxed_bool(true));
    assert_eq!(3, can_execute_call_count.get());
}

#[test]
fn templated_parent_should_not_be_applied_to_submenus() {
    let _app = application();
    let child_menu1 = menu_item_with_header("Bar");
    let child_menu2 = menu_item_with_header("Baz");
    let top_level_menu = menu_item_with_header("Foo");
    add_item(top_level_menu.items(), &child_menu1);
    add_item(top_level_menu.items(), &child_menu2);
    let menu = Menu::new();
    add_item(menu.items(), &top_level_menu);

    let window = Window::new();
    window.set_content(Some(Control::boxed(&menu)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();

    top_level_menu.set_is_sub_menu_open(true);

    assert!(child_menu1.is_attached_to_visual_tree());
    assert!(child_menu1.templated_parent().is_none());
    assert!(child_menu2.templated_parent().is_none());

    top_level_menu.set_is_sub_menu_open(false);
    top_level_menu.set_is_sub_menu_open(true);

    assert!(child_menu1.templated_parent().is_none());
    assert!(child_menu2.templated_parent().is_none());
}

#[test]
fn menu_item_template_should_be_applied_to_top_level_menu_item_header() {
    let _app = application();

    let items = [MenuViewModel::new("Foo"), MenuViewModel::new("Bar")];

    let item_template: Rc<dyn IDataTemplate> = FuncDataTemplate::for_type::<MenuViewModel>(
        |x, _| {
            let text_block = TextBlock::new();
            text_block.set_text(Some(x.header.as_str()));
            Some(text_block.upcast())
        },
        false,
    );

    let menu = Menu::new();
    menu.set_item_template(Some(item_template.clone()));
    menu.set_items_source(Some(items_source(&items)));

    let window = Window::new();
    window.set_content(Some(Control::boxed(&menu)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();

    let panel = menu.presenter().unwrap().panel().and_then(|panel| panel.cast::<StackPanel>()).expect("a stack panel");
    assert_eq!(2, panel.children().count());

    for i in 0..panel.children().count() {
        let menu_item = panel.children().get(i).cast::<MenuItem>().expect("a menu item");

        let item: BoxedValue = items[i].clone();
        assert!(menu_item.header().is_some_and(|header| Rc::ptr_eq(&header, &item)));
        assert!(menu_item.header_template().is_some_and(|template| Rc::ptr_eq(&template, &item_template)));

        let header_presenter: Ref<ContentPresenter> = menu_item.header_presenter().expect("a content presenter");
        assert!(header_presenter.content_template().is_some_and(|template| Rc::ptr_eq(&template, &item_template)));

        let header_control =
            header_presenter.child().and_then(|child| child.cast::<TextBlock>()).expect("a text block");
        assert_eq!(Some(items[i].header.clone()), header_control.text());
    }
}

#[test]
fn header_and_items_source_can_be_bound_in_style() {
    let _app = application();
    let items = [MenuViewModel::with_children("Foo", &[MenuViewModel::new("FooChild")]), MenuViewModel::new("Bar")];

    let target = Menu::new();
    target.set_items_source(Some(items_source(&items)));
    target.styles().add(Style::with_setters(
        Selectors::of_type::<MenuItem>(),
        [
            binding_setter(HeaderedSelectingItemsControl::header_property().as_property(), "Header"),
            binding_setter(ItemsControl::items_source_property().as_property(), "Children"),
        ],
    ));

    let root = styled_test_root(&target);
    root.layout_manager().execute_initial_layout_pass();

    let children: Vec<Ref<MenuItem>> =
        target.get_realized_containers().into_iter().map(|x| x.cast::<MenuItem>().unwrap()).collect();
    assert_eq!(2, children.len());
    assert_eq!(Some("Foo".to_string()), children[0].header().as_ref().and_then(string_of));
    assert_eq!(Some("Bar".to_string()), children[1].header().as_ref().and_then(string_of));
    assert!(children[0].items_source().is_some_and(|source| source.ptr_eq(&items[0].children)));
}

#[test]
fn header_and_items_source_can_be_bound_in_item_container_theme() {
    let _app = application();
    let items = [MenuViewModel::with_children("Foo", &[MenuViewModel::new("FooChild")]), MenuViewModel::new("Bar")];

    let target = Menu::new();
    target.set_items_source(Some(items_source(&items)));
    target.set_item_container_theme(Some(ControlTheme::with_setters(
        MenuItem::TYPE,
        [
            binding_setter(HeaderedSelectingItemsControl::header_property().as_property(), "Header"),
            binding_setter(ItemsControl::items_source_property().as_property(), "Children"),
        ],
    )));

    let root = styled_test_root(&target);
    root.layout_manager().execute_initial_layout_pass();

    let children: Vec<Ref<MenuItem>> =
        target.get_realized_containers().into_iter().map(|x| x.cast::<MenuItem>().unwrap()).collect();
    assert_eq!(2, children.len());
    assert_eq!(Some("Foo".to_string()), children[0].header().as_ref().and_then(string_of));
    assert_eq!(Some("Bar".to_string()), children[1].header().as_ref().and_then(string_of));
    assert!(children[0].items_source().is_some_and(|source| source.ptr_eq(&items[0].children)));
}

fn show_menu(items: &[&Ref<MenuItem>]) -> (Ref<Menu>, Ref<Window>) {
    let menu = Menu::new();
    for item in items {
        add_item(menu.items(), item);
    }

    let window = Window::new();
    window.set_content(Some(Control::boxed(&menu)));
    window.show();
    (menu, window)
}

#[test]
fn radio_menu_item_in_same_group_is_unchecked() {
    let _app = application();

    let menu_item1 = radio_menu_item(Some("A"), false);
    let menu_item2 = radio_menu_item(Some("A"), true);
    let menu_item3 = radio_menu_item(Some("A"), false);

    let _menu = show_menu(&[&menu_item1, &menu_item2, &menu_item3]);

    assert!(!menu_item1.is_checked());
    assert!(menu_item2.is_checked());
    assert!(!menu_item3.is_checked());

    menu_item3.set_is_checked(true);

    assert!(!menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(menu_item3.is_checked());
}

#[test]
fn radio_menu_item_in_same_group_in_menu_flyout_is_unchecked() {
    let _app = application();

    let menu_item1 = MenuItem::new();
    menu_item1.set_group_name(Some("A".to_string()));
    menu_item1.set_is_checked(true);
    menu_item1.set_stays_open_on_click(true);
    menu_item1.set_toggle_type(MenuItemToggleType::Radio);
    let menu_item2 = MenuItem::new();
    menu_item2.set_group_name(Some("A".to_string()));
    menu_item2.set_is_checked(false);
    menu_item2.set_stays_open_on_click(true);
    menu_item2.set_toggle_type(MenuItemToggleType::Radio);

    let flyout = MenuFlyout::new();
    add_item(flyout.items(), &menu_item1);
    add_item(flyout.items(), &menu_item2);
    let button = Button::new();
    button.set_context_flyout(&flyout);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&button)));

    window.show();
    flyout.show_at(&button);
    run_loaded_jobs();

    menu_item2.set_is_checked(true);

    assert!(!menu_item1.is_checked());
    assert!(menu_item2.is_checked());
}

#[test]
fn radio_menu_group_can_be_changed_in_runtime() {
    let _app = application();

    let menu_item1 = radio_menu_item(Some("A"), false);
    let menu_item2 = radio_menu_item(Some("A"), true);
    let menu_item3 = radio_menu_item(None, false);

    let _menu = show_menu(&[&menu_item1, &menu_item2, &menu_item3]);

    assert!(!menu_item1.is_checked());
    assert!(menu_item2.is_checked());
    assert!(!menu_item3.is_checked());

    menu_item3.set_group_name(Some("A".to_string()));
    menu_item3.set_is_checked(true);

    assert!(!menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(menu_item3.is_checked());

    menu_item3.set_group_name(None);
    menu_item1.set_is_checked(true);

    assert!(menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(menu_item3.is_checked());
}

#[test]
fn radio_menu_item_in_same_group_but_submenu_is_unchecked() {
    let _app = application();

    let menu_item1 = radio_menu_item(Some("A"), false);
    let menu_item2 = radio_menu_item(Some("A"), false);
    let menu_item3 = radio_menu_item(Some("A"), true);
    let menu_item4 = radio_menu_item(Some("A"), true);
    add_item(menu_item3.items(), &menu_item4);

    let _menu = show_menu(&[&menu_item1, &menu_item2, &menu_item3]);

    assert!(!menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(menu_item3.is_checked());
    assert!(menu_item4.is_checked());

    menu_item2.set_is_checked(true);

    assert!(!menu_item1.is_checked());
    assert!(menu_item2.is_checked());
    assert!(!menu_item3.is_checked());
    assert!(!menu_item4.is_checked());
}

#[test]
fn radio_menu_item_in_same_group_but_submenu_is_checked() {
    let _app = application();

    let menu_item1 = radio_menu_item(Some("A"), false);
    let menu_item2 = radio_menu_item(Some("A"), true);
    let menu_item3 = radio_menu_item(Some("A"), false);
    let menu_item4 = radio_menu_item(Some("A"), false);
    add_item(menu_item3.items(), &menu_item4);

    let _menu = show_menu(&[&menu_item1, &menu_item2, &menu_item3]);

    assert!(!menu_item1.is_checked());
    assert!(menu_item2.is_checked());
    assert!(!menu_item3.is_checked());
    assert!(!menu_item4.is_checked());

    menu_item4.set_is_checked(true);

    assert!(!menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(menu_item3.is_checked());
    assert!(menu_item4.is_checked());
}

#[test]
fn radio_menu_item_empty_group_name_not_influence_other_groups() {
    let _app = application();

    let menu_item1 = radio_menu_item(Some("A"), true);
    let menu_item2 = radio_menu_item(Some("A"), false);
    let menu_item3 = radio_menu_item(None, false);
    let menu_item4 = radio_menu_item(None, true);

    let _menu = show_menu(&[&menu_item1, &menu_item2, &menu_item3, &menu_item4]);

    assert!(menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(!menu_item3.is_checked());
    assert!(menu_item4.is_checked());

    menu_item3.set_is_checked(true);

    assert!(menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(menu_item3.is_checked());
    assert!(!menu_item4.is_checked());
}

#[test]
fn radio_menus_with_empty_group_on_different_levels_can_be_checked_simultaneously() {
    let _app = application();

    let menu_item1 = radio_menu_item(None, true);
    let menu_item2 = radio_menu_item(None, false);
    let menu_item3 = radio_menu_item(None, false);
    let menu_item4 = radio_menu_item(None, false);
    add_item(menu_item2.items(), &menu_item3);
    add_item(menu_item2.items(), &menu_item4);

    let _menu = show_menu(&[&menu_item1, &menu_item2]);

    assert!(menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(!menu_item3.is_checked());
    assert!(!menu_item4.is_checked());

    menu_item3.set_is_checked(true);

    assert!(menu_item1.is_checked());
    assert!(!menu_item2.is_checked());
    assert!(menu_item3.is_checked());
    assert!(!menu_item4.is_checked());
}

/// The reference sets a random number as the parameter; a fixed one is set
/// here.
#[test]
fn menu_item_command_parameter_does_not_change_while_execution() {
    let _scope = test_scope();
    let target = MenuItem::new();
    let initial: BoxedValue = Rc::new("A".to_string());
    let last_parameter: Rc<RefCell<Option<BoxedValue>>> = Rc::new(RefCell::new(Some(initial.clone())));
    let only_once = Cell::new(false);

    let weak_target = target.downgrade();
    let last_in_can_execute = last_parameter.clone();
    let last_in_execute = last_parameter.clone();
    let command = TestCommand::with_can_execute_and_execute(
        move |parameter| {
            if !only_once.replace(true) {
                let next: BoxedValue = Rc::new(1234_i32);
                weak_target.upgrade().unwrap().set_command_parameter(Some(next));
            }
            *last_in_can_execute.borrow_mut() = parameter.cloned();
            true
        },
        move |parameter| {
            assert!(*last_in_execute.borrow() == parameter.cloned());
        },
    );
    target.set_command_parameter(Some(initial));
    target.set_command(command.as_command());
    let _root = TestRoot::with_child(&target);

    as_clickable_control(&target).unwrap().raise_click();
}
