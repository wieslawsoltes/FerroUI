//! The navigation buttons of `FlipView`, with its control theme.

use crate::{register_types, FlipView, ASSEMBLY};
use ferroui_base::animation::{PageSlide, SignedDuration, SlideAxis};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, Border, Button, Control, TextBlock, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    register_types();
    let services = TestServices::styled_window().with_theme(|| ferroui_themes_winui::WinUITheme::new().as_style());
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    let xaml = "<Styles xmlns=\"https://github.com/ferroui\">\
                  <StyleInclude Source=\"ferres://FerroUI.Controls.FlipView/Themes/WinUI/WinUI.xaml\" />\
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

fn hosted(items: usize) -> (Ref<Window>, Ref<FlipView>) {
    let target = FlipView::new();
    target.set_width(400.0);
    target.set_height(300.0);
    for index in 0..items {
        let text = TextBlock::new();
        text.set_text(Some(&format!("page {index}")));
        let page = Border::new();
        page.set_child(Some(text.upcast::<Control>()));
        target.items().add(Some(Control::boxed(&page)));
    }
    let window = Window::new();
    window.set_content(Some(Control::boxed(&target)));
    window.show();
    window.update_layout();
    (window, target)
}

fn button(target: &FlipView, name: &str) -> Ref<Button> {
    target
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Button>())
        .find(|button| button.name().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("the template has {name}"))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn the_buttons_are_hidden_until_the_pointer_moves() {
    let _app = start();
    let (_window, target) = hosted(3);
    assert!(!button(&target, "NextButtonHorizontal").is_visible());
    assert!(!button(&target, "PreviousButtonHorizontal").is_visible());

    target.show_buttons();
    assert!(button(&target, "NextButtonHorizontal").is_visible());

    target.hide_buttons();
    assert!(!button(&target, "NextButtonHorizontal").is_visible());
}

#[test]
fn the_button_at_an_edge_is_not_shown() {
    let _app = start();
    let (_window, target) = hosted(3);
    target.show_buttons();
    assert!(!button(&target, "PreviousButtonHorizontal").is_visible());
    assert!(button(&target, "NextButtonHorizontal").is_visible());

    target.set_selected_index(2);
    assert!(button(&target, "PreviousButtonHorizontal").is_visible());
    assert!(!button(&target, "NextButtonHorizontal").is_visible());
}

#[test]
fn the_buttons_move_the_selection() {
    let _app = start();
    let (_window, target) = hosted(3);
    target.show_buttons();

    click(&button(&target, "NextButtonHorizontal"));
    assert_eq!(1, target.selected_index());
    click(&button(&target, "PreviousButtonHorizontal"));
    assert_eq!(0, target.selected_index());
}

#[test]
fn a_vertical_flip_view_shows_the_vertical_buttons() {
    let _app = start();
    let (_window, target) = hosted(3);
    target.set_page_transition(Some(Rc::new(PageSlide::with_duration(SignedDuration::ZERO, SlideAxis::Vertical))));
    target.show_buttons();

    assert!(button(&target, "NextButtonVertical").is_visible());
    assert!(!button(&target, "NextButtonHorizontal").is_visible());
}

#[test]
fn a_single_item_shows_no_buttons() {
    let _app = start();
    let (_window, target) = hosted(1);
    target.show_buttons();

    assert!(!button(&target, "NextButtonHorizontal").is_visible());
    assert!(!button(&target, "PreviousButtonHorizontal").is_visible());
}
