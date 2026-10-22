//! Inline notification behavior exercised through native templates, commands and layout.

use ferroui_base::input::ICommand;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::{HorizontalAlignment, ILayoutManager};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{BoxedValue, Rect, Ref, Size, Thickness};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::{
    AccessibilityView, AutomationElementIdentifiers, AutomationLiveSetting, AutomationProperties,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::{FuncControlTemplate, FuncDataTemplate};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    FontIcon, FontIconSource, InfoBar, InfoBarCloseReason, InfoBarPanel, InfoBarSeverity,
};
use ferroui_controls::{Border, Button, Control, Grid, TextBlock, Viewbox, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn part<T: ferroui_base::ObjectType>(bar: &InfoBar, name: &str) -> Ref<T> {
    bar.get_template_descendants()
        .iter()
        .find(|element| element.name().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing InfoBar part {name}"))
        .cast::<T>()
        .unwrap()
}

fn show(bar: &InfoBar) -> Ref<Window> {
    let window = Window::new();
    window.set_width(600.0);
    window.set_content(Some(Control::boxed(bar.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

#[test]
fn closed_default_collapses_template_content_and_open_restores_native_layout() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_title("Information");
    bar.set_message("A message shown inline.");
    assert!(!bar.is_open());
    assert!(bar.is_closable() && bar.is_icon_visible());
    assert_eq!(bar.severity(), InfoBarSeverity::Informational);
    let window = show(&bar);
    let root = part::<Border>(&bar, "ContentRoot");
    assert!(!root.is_visible());
    assert_eq!(bar.desired_size(), Size::default());
    assert!(bar.is_visible());
    bar.set_is_open(true);
    window.layout_manager().execute_layout_pass();
    assert!(root.is_visible());
    assert!(bar.desired_size().height >= 48.0);
    assert_eq!(
        AutomationProperties::get_accessibility_view(&bar),
        AccessibilityView::Control
    );
    bar.set_is_open(false);
    window.layout_manager().execute_layout_pass();
    assert_eq!(bar.desired_size(), Size::default());
    assert_eq!(
        AutomationProperties::get_accessibility_view(&bar),
        AccessibilityView::Raw
    );
}

#[test]
fn close_click_and_programmatic_close_have_the_pinned_order_and_reasons() {
    let _app = start();
    let bar = InfoBar::new();
    bar.apply_template();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    bar.opened(move |_| seen.borrow_mut().push("opened".to_string()));
    let seen = events.clone();
    bar.close_button_click(move || seen.borrow_mut().push("click".to_string()));
    let seen = events.clone();
    bar.closing(move |args| {
        seen.borrow_mut()
            .push(format!("closing:{:?}", args.reason()))
    });
    let seen = events.clone();
    bar.closed(move |args| {
        seen.borrow_mut()
            .push(format!("closed:{:?}", args.reason()))
    });
    bar.set_is_open(true);
    part::<Button>(&bar, "CloseButton").on_click();
    assert_eq!(
        &*events.borrow(),
        &[
            "opened",
            "click",
            "closing:CloseButton",
            "closed:CloseButton"
        ]
    );
    events.borrow_mut().clear();
    bar.set_is_open(true);
    bar.set_is_open(false);
    bar.set_is_open(false);
    assert_eq!(
        &*events.borrow(),
        &["opened", "closing:Programmatic", "closed:Programmatic"]
    );
    assert_eq!(InfoBarCloseReason::CloseButton as i32, 0);
    assert_eq!(InfoBarCloseReason::Programmatic as i32, 1);
}

#[test]
fn cancellation_reopens_without_closed_and_event_disposal_releases_the_handler() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_is_open(true);
    bar.apply_template();
    let reasons = Rc::new(RefCell::new(Vec::new()));
    let recorded = reasons.clone();
    let token = bar.closing(move |args| {
        recorded.borrow_mut().push(args.reason());
        let clone = args.clone();
        clone.set_cancel(true);
        assert!(args.cancel());
    });
    let closed = Rc::new(Cell::new(0));
    let counter = closed.clone();
    bar.closed(move |_| counter.set(counter.get() + 1));
    let opened = Rc::new(Cell::new(0));
    let counter = opened.clone();
    bar.opened(move |_| counter.set(counter.get() + 1));
    part::<Button>(&bar, "CloseButton").on_click();
    assert!(bar.is_open());
    assert!(part::<Border>(&bar, "ContentRoot").is_visible());
    assert_eq!(closed.get(), 0);
    assert_eq!(opened.get(), 1);
    assert_eq!(&*reasons.borrow(), &[InfoBarCloseReason::CloseButton]);
    token.dispose();
    bar.set_is_open(false);
    assert_eq!(closed.get(), 1);
    assert!(!bar.is_open());
}

#[test]
fn closed_handlers_can_reopen_and_old_template_buttons_cannot_close_the_owner() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_is_open(true);
    bar.apply_template();
    let old = part::<Button>(&bar, "CloseButton");
    let weak = bar.downgrade();
    let token = bar.closed(move |_| weak.upgrade().unwrap().set_is_open(true));
    old.on_click();
    assert!(bar.is_open());
    assert!(part::<Border>(&bar, "ContentRoot").is_visible());
    token.dispose();
    let template = bar.template();
    bar.set_template(Some(FuncControlTemplate::for_type::<InfoBar>(|_, _| {
        Border::new().upcast()
    })));
    bar.apply_template();
    bar.set_template(template);
    bar.apply_template();
    old.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(bar.is_open());
    part::<Button>(&bar, "CloseButton").on_click();
    assert!(!bar.is_open());
}

struct Command {
    enabled: Cell<bool>,
    invoked: Rc<RefCell<Vec<String>>>,
    changed: RefCell<Vec<Rc<dyn Fn()>>>,
}
impl ICommand for Command {
    fn can_execute(&self, _: Option<&BoxedValue>) -> bool {
        self.enabled.get()
    }
    fn execute(&self, parameter: Option<&BoxedValue>) {
        self.invoked
            .borrow_mut()
            .push(parameter.unwrap().downcast_ref::<String>().unwrap().clone());
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.changed.borrow_mut().push(handler);
        Disposable::empty()
    }
}

#[test]
fn close_command_parameter_and_enabled_state_use_the_native_button_contract() {
    let _app = start();
    let bar = InfoBar::new();
    let invoked = Rc::new(RefCell::new(Vec::new()));
    let command = Rc::new(Command {
        enabled: Cell::new(false),
        invoked: invoked.clone(),
        changed: RefCell::new(Vec::new()),
    });
    bar.set_close_button_command(Some(command.clone()));
    bar.set_close_button_command_parameter(Some(Rc::new("context".to_string())));
    bar.set_is_open(true);
    let _window = show(&bar);
    let close = part::<Button>(&bar, "CloseButton");
    assert!(!close.is_effectively_enabled());
    close.on_click();
    assert!(bar.is_open());
    assert!(invoked.borrow().is_empty());
    command.enabled.set(true);
    for handler in command.changed.borrow().clone() {
        handler();
    }
    assert!(close.is_effectively_enabled());
    close.on_click();
    assert!(!bar.is_open());
    assert_eq!(&*invoked.borrow(), &["context"]);
    bar.set_is_open(true);
    bar.set_is_closable(false);
    assert!(!close.is_visible());
    bar.set_is_open(false);
    assert!(!bar.is_open());
}

#[test]
fn severity_resources_update_live_and_explicit_background_foreground_take_precedence() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_title("Title");
    bar.set_message("Message");
    bar.resources().add(
        "InfoBarSuccessSeverityBackgroundBrush",
        Some(Rc::new(Brushes::green() as Rc<dyn IBrush>)),
    );
    bar.resources().add(
        "InfoBarTitleForeground",
        Some(Rc::new(Brushes::blue() as Rc<dyn IBrush>)),
    );
    bar.set_severity(InfoBarSeverity::Success);
    bar.set_is_open(true);
    bar.apply_template();
    let root = part::<Border>(&bar, "ContentRoot");
    let title = part::<TextBlock>(&bar, "Title");
    assert_eq!(root.background(), Some(Brushes::green() as Rc<dyn IBrush>));
    assert_eq!(title.foreground(), Some(Brushes::blue() as Rc<dyn IBrush>));
    bar.resources().set(
        "InfoBarSuccessSeverityBackgroundBrush",
        Some(Rc::new(Brushes::red() as Rc<dyn IBrush>)),
    );
    assert_eq!(root.background(), Some(Brushes::red() as Rc<dyn IBrush>));
    bar.set_background(Some(Brushes::blue() as Rc<dyn IBrush>));
    bar.set_foreground(Some(Brushes::green() as Rc<dyn IBrush>));
    bar.set_severity(InfoBarSeverity::Error);
    assert_eq!(
        part::<Border>(&bar, "BackgroundLayer").background(),
        Some(Brushes::blue() as Rc<dyn IBrush>)
    );
    assert_ne!(root.background(), Some(Brushes::blue() as Rc<dyn IBrush>));
    assert_eq!(title.foreground(), Some(Brushes::green() as Rc<dyn IBrush>));
    assert_eq!(
        part::<TextBlock>(&bar, "StandardIcon").text().as_deref(),
        Some("\u{F13D}")
    );
    assert_eq!(
        AutomationProperties::get_live_setting(&bar),
        AutomationLiveSetting::Assertive
    );
}

#[test]
fn custom_icon_is_live_template_settings_are_real_and_visibility_is_independent() {
    let _app = start();
    let bar = InfoBar::new();
    let source = FontIconSource::new();
    source.set_glyph("A");
    bar.set_icon_source(Some(source.clone().upcast()));
    bar.set_is_open(true);
    let window = show(&bar);
    let icon = bar
        .template_settings()
        .icon_element()
        .unwrap()
        .cast::<FontIcon>()
        .unwrap();
    assert_eq!(icon.glyph(), "A");
    assert_eq!(
        part::<Viewbox>(&bar, "UserIconBox").child(),
        Some(icon.clone().upcast())
    );
    assert!(!part::<Grid>(&bar, "StandardIconArea").is_visible());
    source.set_glyph("B");
    assert_eq!(icon.glyph(), "B");
    bar.set_is_icon_visible(false);
    assert!(!part::<Viewbox>(&bar, "UserIconBox").is_visible());
    bar.set_is_icon_visible(true);
    bar.set_icon_source(None);
    window.layout_manager().execute_layout_pass();
    assert!(bar.template_settings().icon_element().is_none());
    assert!(part::<Grid>(&bar, "StandardIconArea").is_visible());
    assert!(part::<Viewbox>(&bar, "UserIconBox").child().is_none());
    assert_eq!(source.property_changed_subscriber_count(), 0);
}

#[test]
fn title_message_action_content_template_and_close_theme_are_native_live_parts() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_is_open(true);
    let content_template = FuncDataTemplate::for_type::<String>(
        |data, _| {
            let text = TextBlock::new();
            text.set_text(Some(&format!("Content:{data}")));
            Some(text.upcast())
        },
        false,
    );
    bar.set_content(Some(Rc::new("payload".to_string())));
    bar.set_content_template(Some(content_template));
    let window = show(&bar);
    let content = part::<ContentPresenter>(&bar, "PART_ContentPresenter");
    assert_eq!(Grid::get_row(&content), 0);
    assert_eq!(
        content
            .child()
            .unwrap()
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Content:payload")
    );
    bar.set_title("New title");
    bar.set_message("New message");
    let action = Button::new();
    action.set_content(Some(Rc::new("Retry".to_string())));
    let invoked = Rc::new(Cell::new(false));
    let seen = invoked.clone();
    action.click(move |_, _| seen.set(true));
    bar.set_action_button(Some(action.clone()));
    let theme = ControlTheme::for_type::<Button>();
    bar.set_close_button_style(Some(theme.clone()));
    window.layout_manager().execute_layout_pass();
    assert_eq!(
        part::<TextBlock>(&bar, "Title").text().as_deref(),
        Some("New title")
    );
    assert_eq!(
        part::<TextBlock>(&bar, "Message").text().as_deref(),
        Some("New message")
    );
    assert_eq!(Grid::get_row(&content), 1);
    assert_eq!(
        part::<ContentPresenter>(&bar, "ActionButtonPresenter").child(),
        Some(action.clone().upcast())
    );
    assert_eq!(part::<Button>(&bar, "CloseButton").theme(), Some(theme));
    action.on_click();
    assert!(invoked.get());
}

#[test]
fn automation_exposes_status_name_changes_and_respects_explicit_names() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_title("Network");
    bar.set_message("Disconnected");
    let peer = ControlAutomationPeer::create_peer_for_element(&bar);
    assert_eq!(
        peer.get_automation_control_type(),
        AutomationControlType::StatusBar
    );
    assert_eq!(peer.get_class_name(), "InfoBar");
    assert_eq!(peer.get_name(), "Network Disconnected");
    let changed = Rc::new(Cell::new(0));
    let count = changed.clone();
    peer.property_changed(move |args| {
        if args.property() == AutomationElementIdentifiers::name_property() {
            count.set(count.get() + 1);
        }
    });
    bar.set_message("Connected");
    assert_eq!(peer.get_name(), "Network Connected");
    assert_eq!(changed.get(), 1);
    AutomationProperties::set_name(&bar, Some("Connection status"));
    let count = changed.get();
    bar.set_message("Ready");
    assert_eq!(peer.get_name(), "Connection status");
    assert_eq!(changed.get(), count);
}

#[test]
fn panel_switches_orientation_with_real_margins_and_invalidates_on_attached_changes() {
    let _app = start();
    let panel = InfoBarPanel::new();
    panel.set_vertical_orientation_padding(Thickness::new(0.0, 14.0, 0.0, 18.0));
    let title = Border::new();
    title.set_width(60.0);
    title.set_height(20.0);
    title.set_horizontal_alignment(HorizontalAlignment::Left);
    let message = Border::new();
    message.set_width(100.0);
    message.set_height(20.0);
    message.set_horizontal_alignment(HorizontalAlignment::Left);
    InfoBarPanel::set_horizontal_orientation_margin(&title, Thickness::new(0.0, 14.0, 0.0, 0.0));
    InfoBarPanel::set_horizontal_orientation_margin(&message, Thickness::new(12.0, 14.0, 0.0, 0.0));
    InfoBarPanel::set_vertical_orientation_margin(&message, Thickness::new(0.0, 4.0, 0.0, 0.0));
    panel.children().add(title.clone());
    panel.children().add(message.clone());
    panel.measure(Size::new(300.0, 300.0));
    panel.arrange(Rect::new(0.0, 0.0, 300.0, 48.0));
    assert_eq!(message.bounds().x, 72.0);
    assert_eq!(message.bounds().y, 14.0);
    panel.measure(Size::new(150.0, 300.0));
    panel.arrange(Rect::new(0.0, 0.0, 150.0, panel.desired_size().height));
    assert_eq!(message.bounds().x, 0.0);
    assert_eq!(message.bounds().y, 38.0);
    InfoBarPanel::set_vertical_orientation_margin(&message, Thickness::new(0.0, 10.0, 0.0, 0.0));
    assert!(!panel.is_measure_valid());
    panel.measure(Size::new(150.0, 300.0));
    panel.arrange(Rect::new(0.0, 0.0, 150.0, panel.desired_size().height));
    assert_eq!(message.bounds().y, 44.0);
}
