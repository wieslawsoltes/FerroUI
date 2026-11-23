//! Opening and closing a `TeachingTip`, with its control theme.

use crate::{
    register_types, TeachingTip, TeachingTipCloseReason as Reason, TeachingTipPlacementMode as Placement,
    TeachingTipTailVisibility, ASSEMBLY,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::Ref;
use ferroui_controls::primitives::Popup;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, Button, Control, PlacementMode, StackPanel, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    register_types();
    let services = TestServices::styled_window().with_theme(|| ferroui_themes_winui::WinUITheme::new().as_style());
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    let xaml = "<Styles xmlns=\"https://github.com/ferroui\">\
                  <StyleInclude Source=\"ferres://FerroUI.Controls.TeachingTip/Themes/WinUI/WinUI.xaml\" />\
                </Styles>";
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .unwrap_or_else(|error| panic!("WinUI.xaml failed to load: {}", error.message()));
    let styles = from_markup_value::<Ref<Styles>>(&Some(loaded)).expect("a Styles");
    let style: Rc<dyn IStyle> = styles.into();
    Application::current().expect("the application").styles().add(style);
    scope
}

fn hosted() -> (Ref<Window>, Ref<TeachingTip>, Ref<Button>) {
    let target = Button::new();
    let tip = TeachingTip::new();
    tip.set_title(Some("Title"));
    tip.set_subtitle(Some("Subtitle"));
    tip.set_target(target.clone());
    let panel = StackPanel::new();
    panel.children().add(target.clone());
    panel.children().add(tip.clone());
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.update_layout();
    (window, tip, target)
}

fn popup(tip: &TeachingTip) -> Ref<Popup> {
    tip.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Popup>())
        .next()
        .expect("the template has its popup")
}

fn popup_button(tip: &TeachingTip, name: &str) -> Ref<Button> {
    let child = popup(tip).child().expect("the content of the popup");
    child
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Button>())
        .find(|button| button.name().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("the template has {name}"))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn is_open_opens_and_closes_the_popup_with_a_reason() {
    let _app = start();
    let (_window, tip, _target) = hosted();
    let reasons = Rc::new(RefCell::new(Vec::new()));
    let seen = reasons.clone();
    let _closed = tip.closed(move |args| seen.borrow_mut().push(args.reason()));

    tip.set_is_open(true);
    assert!(popup(&tip).is_open());

    tip.set_is_open(false);
    assert!(!popup(&tip).is_open());
    assert_eq!(vec![Reason::Programmatic], *reasons.borrow());
}

#[test]
fn the_close_button_closes_with_its_reason() {
    let _app = start();
    let (window, tip, _target) = hosted();
    let reason = Rc::new(Cell::new(None));
    let seen = reason.clone();
    let _closed = tip.closed(move |args| seen.set(Some(args.reason())));
    let clicked = Rc::new(Cell::new(false));
    let was_clicked = clicked.clone();
    let _click = tip.close_button_click(move || was_clicked.set(true));

    tip.set_is_open(true);
    window.update_layout();
    click(&popup_button(&tip, "AlternateCloseButton"));

    assert!(clicked.get());
    assert!(!tip.is_open());
    assert_eq!(Some(Reason::CloseButton), reason.get());
}

#[test]
fn a_closing_handler_keeps_the_tip_open() {
    let _app = start();
    let (_window, tip, _target) = hosted();
    let _closing = tip.closing(|args| args.set_cancel(true));

    tip.set_is_open(true);
    tip.set_is_open(false);

    assert!(tip.is_open());
    assert!(popup(&tip).is_open());
}

#[test]
fn the_action_button_raises_its_event_and_leaves_the_tip_open() {
    let _app = start();
    let (window, tip, _target) = hosted();
    tip.set_action_button_content(Some(Rc::new("Do it".to_string())));
    let clicked = Rc::new(Cell::new(false));
    let was_clicked = clicked.clone();
    let _click = tip.action_button_click(move || was_clicked.set(true));

    tip.set_is_open(true);
    window.update_layout();
    click(&popup_button(&tip, "ActionButton"));

    assert!(clicked.get());
    assert!(tip.is_open());
}

#[test]
fn the_placement_goes_to_the_popup_and_the_tail_faces_the_target() {
    let _app = start();
    let (_window, tip, target) = hosted();
    assert_eq!(PlacementMode::Top, popup(&tip).placement());
    assert!(popup(&tip).placement_target() == Some(target.upcast::<Control>()));
    assert!(tip.classes().contains(":tail-bottom"));

    tip.set_preferred_placement(Placement::Right);
    assert_eq!(PlacementMode::Right, popup(&tip).placement());
    assert!(tip.classes().contains(":tail-left"));
    assert!(!tip.classes().contains(":tail-bottom"));

    tip.set_tail_visibility(TeachingTipTailVisibility::Collapsed);
    assert!(!tip.classes().contains(":tail-left"));
}

#[test]
fn a_tip_without_a_target_has_no_tail() {
    let _app = start();
    let (_window, tip, _target) = hosted();
    tip.set_target(None::<Ref<Control>>);

    assert_eq!(PlacementMode::Center, popup(&tip).placement());
    assert!(!tip.classes().contains(":tail-bottom"));
}
