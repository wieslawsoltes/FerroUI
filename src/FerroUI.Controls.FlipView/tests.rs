//! The navigation buttons of `FlipView`, with its control theme.

use crate::{register_types, FlipView, ASSEMBLY};
use ferroui_base::animation::{PageSlide, SlideAxis};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::{Ref, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, Border, Button, Control, TextBlock, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    register_types();
    let services = TestServices::styled_window().with_theme(|| ferroui_themes_winui::WinUITheme::new().as_style());
    let scope = UnitTestApplication::start(services);
    let document = RuntimeXamlLoaderDocument::from_str(
        r#"<StyleInclude xmlns="https://github.com/ferroui" Source="ferres://FerroUI.Controls.FlipView/Themes/WinUI/WinUI.xaml" />"#,
    );
    let loaded = FerroRuntimeXamlLoader::load(document, RuntimeXamlLoaderConfiguration::default())
        .expect("the theme of the crate loads");
    let style = loaded.cast_style().expect("a style");
    Application::current().expect("the application").styles().add(style);
    let _ = ASSEMBLY.name;
    scope
}

fn hosted(items: usize) -> (Ref<Window>, Ref<FlipView>) {
    let target = FlipView::new();
    for index in 0..items {
        let page = Border::new();
        page.set_child(Some(TextBlock::with_text(&format!("page {index}")).upcast::<Control>()));
        target.items().add(Some(Control::boxed(page)));
    }
    let window = Window::new();
    window.set_content(Some(Rc::new(target.clone().upcast::<Control>())));
    window.show();
    window.measure(Size::new(400.0, 300.0));
    window.arrange(ferroui_base::Rect::from_size(Size::new(400.0, 300.0)));
    (window, target)
}

fn button(target: &FlipView, name: &str) -> Ref<Button> {
    target
        .get_template_children()
        .into_iter()
        .find_map(|child| child.find_descendant_of_type_named::<Button>(name))
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
    target.set_page_transition(Some(Rc::new(PageSlide::new(std::time::Duration::ZERO, SlideAxis::Vertical))));
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
