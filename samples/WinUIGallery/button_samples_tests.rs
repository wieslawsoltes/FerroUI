//! The ported Button, RepeatButton, HyperlinkButton and ToggleButton pages: each catalog example
//! of the source with its handler and its Disable option (`button_samples.rs`). The Button page's
//! click outputs are in `tests.rs` (`button_click_and_disabled_option_update_native_controls`).

use crate::{examples, tests::start, GalleryView};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Button, CheckBox, Control, HyperlinkButton, Image, RepeatButton, TextBlock};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

fn text(root: &Ref<Control>, name: &str) -> Option<String> {
    root.get_control::<TextBlock>(name).text()
}

#[test]
fn button_page_shows_the_graphical_content_and_the_wrapping_layouts() {
    let _application = start();
    let root = page("Button");
    // The second example's image is the source's Slices.png where the platform decodes it (the
    // test platform may not); otherwise the page shows why.
    let decoded = crate::assets::load_bitmap("ferres://WinUIGallery/Assets/SampleMedia/Slices.png").is_ok();
    assert_eq!(decoded, root.get_control::<Image>("Button2Image").source().is_some());
    assert_eq!(!decoded, root.get_control::<TextBlock>("Button2ImageError").is_visible());
    let button = root.get_control::<Button>("Button2");
    assert_eq!((button.width(), button.height()), (50.0, 50.0));
    // The wrapped buttons of the fourth example keep the source's maximum width.
    for name in ["WrappedButton1", "WrappedButton2"] {
        assert_eq!(root.get_control::<Button>(name).max_width(), 240.0);
    }
}

#[cfg(feature = "theme")]
#[test]
fn button_page_applies_the_built_in_styles_of_the_winui_theme() {
    use ferroui_controls::Window;
    let _application = crate::tests::start_winui();
    let root = page("Button");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    // AccentButtonStyle and SubtleButtonStyle are control themes of the WinUI theme, found as
    // dynamic resources once the page is in the window.
    let accent = root.get_control::<Button>("AccentStyleButton").theme();
    let subtle = root.get_control::<Button>("SubtleStyleButton").theme();
    assert!(accent.is_some() && subtle.is_some());
    assert!(accent != subtle);
    assert!(accent != root.get_control::<Button>("Button1").theme());
    window.close();
}

#[test]
fn repeat_button_counts_every_click_and_follows_the_disable_option() {
    let _application = start();
    let root = page("RepeatButton");
    let button = root.get_control::<RepeatButton>("RepeatButton1");
    for _ in 0..3 {
        click(&button);
    }
    assert_eq!(text(&root, "RepeatButton1Output").as_deref(), Some("Number of clicks: 3"));
    // The count is the source's static field: it is kept when the page is opened again.
    let again = page("RepeatButton");
    click(&again.get_control::<RepeatButton>("RepeatButton1"));
    assert_eq!(text(&again, "RepeatButton1Output").as_deref(), Some("Number of clicks: 4"));
    let disable = root.get_control::<CheckBox>("DisableRepeatButton1");
    disable.set_is_checked(Some(true));
    assert!(!button.is_enabled());
    disable.set_is_checked(Some(false));
    assert!(button.is_enabled());
}

#[test]
fn toggle_button_reports_on_and_off_and_follows_the_disable_option() {
    let _application = start();
    let root = page("ToggleButton");
    // The constructor of the source sets the first output.
    assert_eq!(text(&root, "Toggle1Output").as_deref(), Some("Off"));
    let toggle = root.get_control::<ToggleButton>("Toggle1");
    toggle.set_is_checked(Some(true));
    assert_eq!(text(&root, "Toggle1Output").as_deref(), Some("On"));
    toggle.set_is_checked(Some(false));
    assert_eq!(text(&root, "Toggle1Output").as_deref(), Some("Off"));
    let disable = root.get_control::<CheckBox>("DisableToggle1");
    disable.set_is_checked(Some(true));
    assert!(!toggle.is_enabled());
    disable.set_is_checked(Some(false));
    assert!(toggle.is_enabled());
}

#[test]
fn hyperlink_button_navigates_to_its_uri_and_follows_the_disable_option() {
    let _application = start();
    let root = page("HyperlinkButton");
    let link = root.get_control::<HyperlinkButton>("Hyperlink1");
    assert_eq!(
        link.navigate_uri().and_then(|uri| uri.as_url().map(|url| url.as_str().to_string())).as_deref(),
        Some("https://www.microsoft.com/")
    );
    let disable = root.get_control::<CheckBox>("DisableHyperlink1");
    disable.set_is_checked(Some(true));
    assert!(!link.is_enabled());
    disable.set_is_checked(Some(false));
    assert!(link.is_enabled());
}

#[test]
fn hyperlink_button_click_opens_the_toggle_button_page_of_the_gallery() {
    use ferroui_base::threading::Dispatcher;
    let _application = start();
    let gallery = GalleryView::new().unwrap();
    assert!(gallery.navigate("HyperlinkButton"));
    let root = gallery.native_example().expect("the native HyperlinkButton page");
    click(&root.get_control::<HyperlinkButton>("GoToToggleButton"));
    // The gallery navigates after the click, as the source's App.MainWindow.Navigate does.
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(gallery.current_page(), Some("ToggleButton"));
    assert!(gallery.go_back());
    assert_eq!(gallery.current_page(), Some("HyperlinkButton"));
}
