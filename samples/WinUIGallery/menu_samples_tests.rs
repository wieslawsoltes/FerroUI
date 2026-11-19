//! The Flyout, MenuFlyout, MenuBar and Popup pages: the source's examples with the handlers of
//! their code behind (`menu_samples.rs`, and `platform_samples.rs` for the popup) and x:Bind.

use crate::menu_samples::{item_text, MENU_BAR_ITEMS};
use crate::{examples, tests::start};
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::primitives::Popup;
use ferroui_controls::{
    Button, Control, MenuItem, MenuItemToggleType, NumberBox, TextBlock, ToggleSwitch, Window,
};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn click_item(root: &Ref<Control>, name: &str) {
    root.get_control::<MenuItem>(name)
        .raise_event(&RoutedEventArgs::with_event(&MenuItem::click_event()));
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

fn text(root: &Ref<Control>, name: &str) -> Option<String> {
    root.get_control::<TextBlock>(name).text()
}

fn shown(root: &Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    window
}

#[test]
fn flyout_confirmation_hides_the_flyout() {
    let _application = start();
    let root = page("Flyout");
    let window = shown(&root);
    let button = root.get_control::<Button>("Control1");
    let flyout = button.flyout().unwrap();
    flyout.show_at(&button);
    assert!(flyout.is_open());
    click(&root.get_control::<Button>("DeleteConfirmation"));
    assert!(!flyout.is_open());
    window.close();
}

#[test]
fn menu_flyout_items_write_the_sort_option_and_the_clicked_item() {
    let _application = start();
    let root = page("MenuFlyout");
    click_item(&root, "SortByMatch");
    assert_eq!(
        text(&root, "Control1Output").as_deref(),
        Some("Sort by: match")
    );
    click_item(&root, "SortByDistance");
    assert_eq!(
        text(&root, "Control1Output").as_deref(),
        Some("Sort by: distance")
    );
    click_item(&root, "SaveAsPdf");
    assert_eq!(
        text(&root, "Control3bOutput").as_deref(),
        Some("Clicked: Save as .pdf")
    );
    click_item(&root, "ShareViaLink");
    assert_eq!(
        text(&root, "Control3bOutput").as_deref(),
        Some("Clicked: Share via link")
    );
}

#[test]
fn menu_flyout_items_keep_the_source_toggles_groups_and_accelerators() {
    let _application = start();
    let root = page("MenuFlyout");
    for name in ["RepeatToggleMenuFlyoutItem", "ShuffleToggleMenuFlyoutItem"] {
        let item = root.get_control::<MenuItem>(name);
        assert_eq!(item.toggle_type(), MenuItemToggleType::CheckBox);
        assert!(item.is_checked());
    }
    let compressed = root.get_control::<MenuItem>("CompressedFileSubItem");
    assert_eq!(compressed.item_count(), 3);
    for (name, checked) in [
        ("LandscapeItem", false),
        ("PortraitItem", true),
        ("SmallIconsItem", false),
        ("MediumIconsItem", true),
        ("LargeIconsItem", false),
    ] {
        let item = root.get_control::<MenuItem>(name);
        assert_eq!(item.toggle_type(), MenuItemToggleType::Radio, "{name}");
        assert_eq!(item.is_checked(), checked, "{name}");
    }
    assert_eq!(
        root.get_control::<MenuItem>("CopyAcceleratorItem")
            .hot_key(),
        Some(KeyGesture::new(Key::C, KeyModifiers::CONTROL))
    );
    assert_eq!(
        root.get_control::<MenuItem>("DeleteAcceleratorItem")
            .input_gesture(),
        Some(KeyGesture::from_key(Key::Delete))
    );
}

#[test]
fn menu_bar_items_write_their_text_to_the_output_of_their_bar() {
    let _application = start();
    let root = page("MenuBar");
    for (output, names) in MENU_BAR_ITEMS {
        for name in names {
            click_item(&root, name);
            let item = root.get_control::<MenuItem>(name);
            assert_eq!(
                text(&root, output),
                Some(format!("You clicked: {}", item_text(&item))),
                "{name}"
            );
        }
    }
    assert_eq!(
        text(&root, "SelectedOptionText1").as_deref(),
        Some("You clicked: About")
    );
    assert_eq!(
        root.get_control::<MenuItem>("t1").hot_key(),
        Some(KeyGesture::new(Key::O, KeyModifiers::CONTROL))
    );
    assert!(root.get_control::<MenuItem>("z14").is_checked());
}

#[test]
fn popup_buttons_open_and_close_and_disable_the_light_dismiss_option() {
    let _application = start();
    let root = page("Popup");
    let popup = root.get_control::<Popup>("StandardPopup");
    let toggle = root.get_control::<ToggleSwitch>("IsLightDismissEnabledToggleSwitch");
    // The source's OneWay x:Bind of the options.
    assert_eq!(
        (popup.horizontal_offset(), popup.vertical_offset()),
        (200.0, 0.0)
    );
    assert!(popup.is_light_dismiss_enabled());
    root.get_control::<NumberBox>("HorizontalOffset")
        .set_numeric_value(42.0);
    root.get_control::<NumberBox>("VerticalOffset")
        .set_numeric_value(-12.0);
    assert_eq!(
        (popup.horizontal_offset(), popup.vertical_offset()),
        (42.0, -12.0)
    );
    toggle.set_is_checked(Some(false));
    assert!(!popup.is_light_dismiss_enabled());
    click(&root.get_control::<Button>("ShowPopup"));
    assert!(popup.is_open());
    assert!(!toggle.is_enabled());
    click(&root.get_control::<Button>("ClosePopup"));
    assert!(!popup.is_open());
    assert!(toggle.is_enabled());
    // PopupClosed (a popup closed by light dismiss) needs a popup open in a window, which the
    // test theme cannot show with this page's ToggleSwitch: it is checked by hand.
}
