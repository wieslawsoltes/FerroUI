//! Tests of `InfoBar` (WinUI; docs/porting/winui-controls.md, section 10.2), adapted from the
//! tests of #286. The control has no template of its own (the WinUI theme has it,
//! `src/FerroUI.Themes.WinUI/Controls/InfoBar.xaml`), so a test gives the bar the parts the control
//! looks up (`StandardIcon`, `CloseButton`); the looks of the states are the theme's, so the tests
//! check the pseudo-classes the control sets.

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::{AccessibilityView, AutomationElementIdentifiers, AutomationLiveSetting, AutomationProperties};
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Border, Button, FontIcon, FontIconSource, InfoBar, InfoBarCloseReason, InfoBarSeverity, StackPanel, TextBlock,
    ToolTip,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ObjectType, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::styled_window())
}

/// Gives `bar` the parts of the template of the WinUI theme that the control looks up.
fn apply_parts(bar: &InfoBar) {
    bar.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let root = StackPanel::new();
        let icon = TextBlock::new();
        icon.set_name(Some("StandardIcon".to_string()));
        root.children().add(icon.register_in_name_scope(&**scope));
        let close = Button::new();
        close.set_name(Some("CloseButton".to_string()));
        root.children().add(close.register_in_name_scope(&**scope));
        root.upcast()
    })));
    bar.apply_template();
}

fn part<T: ObjectType>(bar: &InfoBar, name: &str) -> Ref<T> {
    bar.get_template_descendants()
        .into_iter()
        .find(|element| element.name().as_deref() == Some(name))
        .and_then(|element| element.cast::<T>())
        .unwrap_or_else(|| panic!("the part {name}"))
}

fn has(bar: &InfoBar, class: &str) -> bool {
    bar.classes().contains(class)
}

#[test]
fn the_defaults_are_those_of_winui() {
    let _app = start();
    let bar = InfoBar::new();
    assert!(!bar.is_open());
    assert!(bar.is_closable());
    assert!(bar.is_icon_visible());
    assert_eq!(InfoBarSeverity::Informational, bar.severity());
    assert!(bar.title().is_empty() && bar.message().is_empty());
    assert!(bar.icon_source().is_none() && bar.action_button().is_none());
    assert!(bar.template_settings().icon_element().is_none());
    assert_ne!(bar.template_settings(), InfoBar::new().template_settings());
    assert!(InfoBar::template_settings_property().setter().is_none());
    assert_eq!(0, InfoBarSeverity::Informational as i32);
    assert_eq!(3, InfoBarSeverity::Error as i32);
    assert_eq!(0, InfoBarCloseReason::CloseButton as i32);
    assert_eq!(1, InfoBarCloseReason::Programmatic as i32);
}

#[test]
fn the_states_of_winui_are_pseudo_classes() {
    let _app = start();
    let bar = InfoBar::new();
    // SeverityLevels, IconStates, the close button and the position of the content.
    assert!(has(&bar, ":informational"));
    assert!(has(&bar, ":standard-icon-visible") && !has(&bar, ":user-icon-visible"));
    assert!(!has(&bar, ":close-button-collapsed"));
    assert!(has(&bar, ":no-banner-content"));

    bar.set_severity(InfoBarSeverity::Error);
    assert!(has(&bar, ":error") && !has(&bar, ":informational"));
    bar.set_severity(InfoBarSeverity::Warning);
    assert!(has(&bar, ":warning") && !has(&bar, ":error"));
    bar.set_severity(InfoBarSeverity::Success);
    assert!(has(&bar, ":success") && !has(&bar, ":warning"));

    bar.set_is_closable(false);
    assert!(has(&bar, ":close-button-collapsed"));

    bar.set_title("Title");
    assert!(!has(&bar, ":no-banner-content"));
    bar.set_title("");
    bar.set_action_button(Some(Button::new()));
    assert!(!has(&bar, ":no-banner-content"));
    bar.set_action_button(None);
    assert!(has(&bar, ":no-banner-content"));

    let source = FontIconSource::new();
    bar.set_icon_source(Some(source.upcast()));
    assert!(has(&bar, ":user-icon-visible") && !has(&bar, ":standard-icon-visible"));
    bar.set_is_icon_visible(false);
    assert!(!has(&bar, ":user-icon-visible") && !has(&bar, ":standard-icon-visible"));
}

#[test]
fn the_bar_is_shown_once_the_template_is_applied() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_is_open(true);
    // InfoBar::UpdateVisibility waits for the template.
    assert!(!has(&bar, ":open"));
    apply_parts(&bar);
    assert!(has(&bar, ":open"));
    assert_eq!(AccessibilityView::Control, AutomationProperties::get_accessibility_view(&bar));
    bar.set_is_open(false);
    assert!(!has(&bar, ":open"));
    assert_eq!(AccessibilityView::Raw, AutomationProperties::get_accessibility_view(&bar));
}

#[test]
fn the_close_button_and_the_application_close_the_bar_in_the_order_of_winui() {
    let _app = start();
    let bar = InfoBar::new();
    apply_parts(&bar);
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let _opened = bar.opened(move |_| seen.borrow_mut().push("opened".to_string()));
    let seen = events.clone();
    let _click = bar.close_button_click(move || seen.borrow_mut().push("click".to_string()));
    let seen = events.clone();
    let _closing = bar.closing(move |args| seen.borrow_mut().push(format!("closing:{:?}", args.reason())));
    let seen = events.clone();
    let _closed = bar.closed(move |args| seen.borrow_mut().push(format!("closed:{:?}", args.reason())));

    bar.set_is_open(true);
    part::<Button>(&bar, "CloseButton").on_click();
    assert!(!bar.is_open());
    assert_eq!(&["opened", "click", "closing:CloseButton", "closed:CloseButton"], &events.borrow()[..]);

    events.borrow_mut().clear();
    bar.set_is_open(true);
    bar.set_is_open(false);
    bar.set_is_open(false);
    assert_eq!(&["opened", "closing:Programmatic", "closed:Programmatic"], &events.borrow()[..]);
}

#[test]
fn a_cancelled_closing_opens_the_bar_again_without_closed() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_is_open(true);
    apply_parts(&bar);
    let reasons = Rc::new(RefCell::new(Vec::new()));
    let recorded = reasons.clone();
    let closing = bar.closing(move |args| {
        recorded.borrow_mut().push(args.reason());
        // The clones of the arguments (the handlers of markup) share the cancellation.
        args.clone().set_cancel(true);
        assert!(args.cancel());
    });
    let closed = Rc::new(Cell::new(0));
    let counter = closed.clone();
    let _closed = bar.closed(move |_| counter.set(counter.get() + 1));
    let opened = Rc::new(Cell::new(0));
    let counter = opened.clone();
    let _opened = bar.opened(move |_| counter.set(counter.get() + 1));

    part::<Button>(&bar, "CloseButton").on_click();
    assert!(bar.is_open());
    assert!(has(&bar, ":open"));
    assert_eq!(0, closed.get());
    assert_eq!(1, opened.get());
    assert_eq!(&[InfoBarCloseReason::CloseButton], &reasons.borrow()[..]);

    closing.dispose();
    bar.set_is_open(false);
    assert_eq!(1, closed.get());
    assert!(!bar.is_open());
}

#[test]
fn a_closed_handler_may_open_the_bar_and_an_old_close_button_does_nothing() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_is_open(true);
    apply_parts(&bar);
    let old = part::<Button>(&bar, "CloseButton");
    let weak = bar.downgrade();
    let reopen = bar.closed(move |_| {
        if let Some(bar) = weak.upgrade() {
            bar.set_is_open(true);
        }
    });
    old.on_click();
    assert!(bar.is_open());
    reopen.dispose();

    // A new template: the button of the old one no longer closes the bar.
    apply_parts(&bar);
    old.on_click();
    assert!(bar.is_open());
    part::<Button>(&bar, "CloseButton").on_click();
    assert!(!bar.is_open());
}

#[test]
fn the_close_button_and_the_icon_get_the_names_of_winui() {
    let _app = start();
    let bar = InfoBar::new();
    apply_parts(&bar);
    let close = part::<Button>(&bar, "CloseButton");
    assert_eq!(Some("Close".to_string()), AutomationProperties::get_name(&close));
    let tip = ToolTip::get_tip(&close).expect("a tool tip");
    assert_eq!(Some("Close"), tip.downcast_ref::<String>().map(String::as_str));
    let icon = part::<TextBlock>(&bar, "StandardIcon");
    assert_eq!(Some("Informational icon".to_string()), AutomationProperties::get_name(&icon));
    bar.set_severity(InfoBarSeverity::Error);
    assert_eq!(Some("Error icon".to_string()), AutomationProperties::get_name(&icon));

    // A name of the application is kept.
    let named = InfoBar::new();
    named.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let close = Button::new();
        close.set_name(Some("CloseButton".to_string()));
        AutomationProperties::set_name(&close, Some("Dismiss"));
        close.register_in_name_scope(&**scope).upcast()
    })));
    named.apply_template();
    assert_eq!(Some("Dismiss".to_string()), AutomationProperties::get_name(&part::<Button>(&named, "CloseButton")));
}

#[test]
fn the_icon_source_gives_the_template_settings_a_live_icon() {
    let _app = start();
    let bar = InfoBar::new();
    let source = FontIconSource::new();
    source.set_glyph("A");
    bar.set_icon_source(Some(source.clone().upcast()));
    let icon = bar.template_settings().icon_element().expect("an icon").cast::<FontIcon>().expect("a font icon");
    assert_eq!("A", icon.glyph());
    source.set_glyph("B");
    assert_eq!("B", icon.glyph());
    bar.set_icon_source(None);
    assert!(bar.template_settings().icon_element().is_none());
    assert!(has(&bar, ":standard-icon-visible"));
}

#[test]
fn a_local_foreground_is_foreground_set() {
    let _app = start();
    let bar = InfoBar::new();
    assert!(!has(&bar, ":foreground-set"));
    let style = bar.set_value_with_priority(
        TemplatedControl::foreground_property(),
        Some(Brushes::red() as Rc<dyn IBrush>),
        BindingPriority::Style,
    );
    assert!(!has(&bar, ":foreground-set"));
    bar.set_foreground(Some(Brushes::blue() as Rc<dyn IBrush>));
    assert!(has(&bar, ":foreground-set"));
    bar.clear_value(TemplatedControl::foreground_property());
    assert!(!has(&bar, ":foreground-set"));
    if let Some(style) = style {
        style.dispose();
    }

    // An inherited foreground is not set on the bar.
    let parent = Border::new();
    parent.set_child(bar.clone());
    crate::documents::TextElement::set_foreground_on(&parent, Some(Brushes::green() as Rc<dyn IBrush>));
    assert!(!has(&bar, ":foreground-set"));
}

#[test]
fn the_peer_is_a_status_bar_named_by_the_title_and_the_message() {
    let _app = start();
    let bar = InfoBar::new();
    bar.set_title("Network");
    bar.set_message("Disconnected");
    let peer = ControlAutomationPeer::create_peer_for_element(&bar);
    assert_eq!(AutomationControlType::StatusBar, peer.get_automation_control_type());
    assert_eq!("InfoBar", peer.get_class_name());
    assert_eq!("Network Disconnected", peer.get_name());

    let changed = Rc::new(Cell::new(0));
    let count = changed.clone();
    let _changes = peer.property_changed(move |args| {
        if std::ptr::eq(args.property(), AutomationElementIdentifiers::name_property()) {
            count.set(count.get() + 1);
        }
    });
    bar.set_message("Connected");
    assert_eq!("Network Connected", peer.get_name());
    assert_eq!(1, changed.get());

    // A name of the application wins.
    AutomationProperties::set_name(&bar, Some("Connection status"));
    let count = changed.get();
    bar.set_message("Ready");
    assert_eq!("Connection status", peer.get_name());
    assert_eq!(count, changed.get());
}

#[test]
fn the_live_setting_of_the_peer_follows_the_severity() {
    let _app = start();
    let bar = InfoBar::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&bar);
    assert_eq!(AutomationLiveSetting::Polite, peer.get_live_setting());
    bar.set_severity(InfoBarSeverity::Warning);
    assert_eq!(AutomationLiveSetting::Assertive, peer.get_live_setting());
    bar.set_severity(InfoBarSeverity::Error);
    assert_eq!(AutomationLiveSetting::Assertive, peer.get_live_setting());
    // A live setting of the application wins.
    AutomationProperties::set_live_setting(&bar, AutomationLiveSetting::Polite);
    assert_eq!(AutomationLiveSetting::Polite, peer.get_live_setting());
}
