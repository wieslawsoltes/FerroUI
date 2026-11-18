//! The ported Button, RepeatButton, HyperlinkButton, ToggleButton, DropDownButton and SplitButton
//! pages and the ToggleSplitButton page: each catalog example of the source with its handlers and
//! options (`button_samples.rs`). The Button page's click outputs are in `tests.rs`
//! (`button_click_and_disabled_option_update_native_controls`), the CheckBox handlers too.

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

/// The headers and icon glyphs of the items of a drop-down button's menu flyout.
fn menu_items(button: &ferroui_controls::DropDownButton) -> Vec<(String, Option<String>)> {
    use ferroui_controls::{FontIcon, MenuFlyout, MenuItem};
    let flyout = button.flyout().and_then(|flyout| flyout.cast::<MenuFlyout>()).expect("a menu flyout");
    assert_eq!(flyout.placement(), ferroui_controls::PlacementMode::BottomEdgeAlignedLeft);
    flyout
        .items()
        .iter()
        .map(|item| {
            let item = item.and_then(|item| Control::from_boxed(&item)).and_then(|item| item.cast::<MenuItem>());
            let item = item.expect("a menu item");
            let header = item.header().and_then(|header| header.downcast_ref::<String>().cloned()).unwrap_or_default();
            let glyph = item
                .icon()
                .and_then(|icon| Control::from_boxed(&icon))
                .and_then(|icon| icon.cast::<FontIcon>())
                .map(|icon| icon.glyph());
            (header, glyph)
        })
        .collect()
}

#[test]
fn drop_down_buttons_open_the_source_menus_with_their_icons() {
    use ferroui_controls::{DropDownButton, FontIcon};
    let _application = start();
    let root = page("DropDownButton");
    let headers = ["Send", "Reply", "Reply All"];
    let simple = menu_items(&root.get_control::<DropDownButton>("EmailDropDownButton"));
    assert_eq!(simple, headers.map(|header| (header.to_string(), None)).to_vec());
    let icons = root.get_control::<DropDownButton>("EmailIconDropDownButton");
    assert_eq!(root.get_control::<FontIcon>("EmailIcon").glyph(), "\u{e715}");
    assert_eq!(
        ferroui_controls::automation::AutomationProperties::get_name(&icons).as_deref(),
        Some("Email")
    );
    let glyphs = ["\u{e725}", "\u{e8ca}", "\u{e8c2}"];
    let expected: Vec<_> = headers
        .iter()
        .zip(glyphs)
        .map(|(header, glyph)| (header.to_string(), Some(glyph.to_string())))
        .collect();
    assert_eq!(menu_items(&icons), expected);
}

/// The color of the source's text in the split button page's editor (`None` when it is mixed).
fn text_color(editor: &ferroui_controls::RichEditBox) -> Option<ferroui_base::media::Color> {
    use ferroui_controls::rich_text::TextFormatValue;
    let length = crate::button_samples::SPLIT_BUTTON_TEXT.len();
    let range = editor.text_document().get_range(0, length).ok()?;
    match range.character_format().ok()? {
        TextFormatValue::Uniform(format) => format
            .foreground
            .flatten()
            .and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color())),
        TextFormatValue::Mixed => None,
    }
}

fn selection_color(editor: &ferroui_controls::RichEditBox) -> Option<ferroui_base::media::Color> {
    editor
        .text_document()
        .selection()
        .character_format()
        .ok()
        .and_then(|format| format.foreground.flatten())
        .and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
}

#[test]
fn split_button_colors_the_editor_text_with_its_swatches() {
    use ferroui_base::media::Colors;
    use ferroui_controls::{Border, RichEditBox, SplitButton};
    let _application = start();
    let root = page("SplitButton");
    let editor = root.get_control::<RichEditBox>("myRichEditBox");
    // The constructor writes the source's text in the current color, green.
    assert_eq!(editor.text_document().text().trim_end(), crate::button_samples::SPLIT_BUTTON_TEXT);
    assert_eq!(text_color(&editor), Some(Colors::GREEN));
    // `GridView_ItemClick`: a swatch of the flyout colors the selection and the primary part.
    click(&root.get_control::<Button>("BlueSwatchButton"));
    assert_eq!(selection_color(&editor), Some(Colors::BLUE));
    let current = root.get_control::<Border>("CurrentColor");
    assert_eq!(
        current.background().and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color())),
        Some(Colors::BLUE)
    );
    // `myColorButton_Click`: the primary part applies the color it shows, blue now.
    let _ = editor.text_document().selection().select_all();
    let _ = editor.text_document().selection().set_character_format(&ferroui_controls::documents::CharacterFormat {
        foreground: Some(Some(std::rc::Rc::new(ferroui_base::media::immutable::ImmutableSolidColorBrush::new(
            Colors::GRAY,
        )))),
        ..Default::default()
    });
    assert_eq!(text_color(&editor), Some(Colors::GRAY));
    let split = root.get_control::<SplitButton>("myColorButton");
    split.raise_event(&RoutedEventArgs::with_event(&SplitButton::click_event()));
    assert_eq!(text_color(&editor), Some(Colors::BLUE));
    // Every swatch of both flyouts is a button with the source's color name.
    for name in crate::button_samples::SWATCHES {
        root.get_control::<Button>(&format!("{name}SwatchButton"));
        root.get_control::<Button>(&format!("{name}RevealButton"));
    }
    root.get_control::<Button>("BlackRevealButton");
}

#[test]
fn split_button_swatches_close_their_flyouts() {
    use ferroui_controls::{SplitButton, Window};
    let _application = start();
    let root = page("SplitButton");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    for (button, swatch) in [("myColorButton", "RedSwatchButton"), ("myColorButtonReveal", "BlackRevealButton")] {
        let split = root.get_control::<SplitButton>(button);
        split.open_flyout();
        let flyout = split.flyout().expect("the flyout of the split button");
        assert!(flyout.is_open(), "{button}");
        click(&root.get_control::<Button>(swatch));
        assert!(!flyout.is_open(), "{button}");
    }
    window.close();
}

#[test]
fn toggle_split_button_bullets_set_the_symbol_the_name_and_the_checked_state() {
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::{Symbol, SymbolIcon, ToggleSplitButton};
    let _application = start();
    let root = page("ToggleSplitButton");
    let list_button = root.get_control::<ToggleSplitButton>("myListButton");
    let icon = root.get_control::<SymbolIcon>("mySymbolIcon");
    assert!(!list_button.is_checked());
    click(&root.get_control::<Button>("RomanNumeralsListButton"));
    assert_eq!(icon.symbol(), Symbol::Bullets);
    assert_eq!(AutomationProperties::get_name(&list_button).as_deref(), Some("Roman Numerals"));
    assert!(list_button.is_checked());
    list_button.set_is_checked(false);
    click(&root.get_control::<Button>("BulletedListButton"));
    assert_eq!(icon.symbol(), Symbol::List);
    assert_eq!(AutomationProperties::get_name(&list_button).as_deref(), Some("Bullets"));
    assert!(list_button.is_checked());
}
