//! The ContentDialog page: the source's two examples with their code behind.

use crate::content_dialog_samples::{result_text, REPLACE, SAVE};
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::{Button, ContentDialogButton, ContentDialogResult, Control, TextBlock};

fn page() -> Ref<Control> {
    examples::create(examples::find("ContentDialog").unwrap()).unwrap_or_else(|error| panic!("ContentDialog: {error}"))
}

#[test]
fn content_dialog_examples_keep_the_source_texts_and_default_buttons() {
    let _application = start();
    let root = page();
    root.get_control::<Button>("ShowDialog");
    root.get_control::<Button>("ShowDialogNoDefault");
    assert_eq!(SAVE.default_button, ContentDialogButton::Primary);
    assert_eq!(REPLACE.default_button, ContentDialogButton::None);
    assert_eq!(result_text(&SAVE, ContentDialogResult::Primary), "User saved their work");
    assert_eq!(result_text(&SAVE, ContentDialogResult::Secondary), "User did not save their work");
    assert_eq!(result_text(&REPLACE, ContentDialogResult::Primary), "User replaced the file");
    assert_eq!(result_text(&REPLACE, ContentDialogResult::Secondary), "User kept the file");
    assert_eq!(result_text(&REPLACE, ContentDialogResult::None), "User cancelled the dialog");
}

/// The real template of the WinUI theme: the dialog opens over the window, its buttons follow its
/// texts, and a click closes it with the result the page shows.
#[cfg(feature = "theme")]
#[test]
fn content_dialog_opens_in_the_overlay_and_reports_its_result() {
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::primitives::OverlayLayer;
    use ferroui_controls::{ContentDialog, Window};

    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();

    let show = root.get_control::<Button>("ShowDialog");
    show.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let overlay = OverlayLayer::get_overlay_layer(&window).expect("the overlay layer of the window");
    let dialog = overlay
        .children()
        .snapshot()
        .iter()
        .find_map(|child| child.clone().cast::<ContentDialog>())
        .expect("the open dialog");
    assert!(dialog.is_open());
    assert_eq!(dialog.bounds().width, overlay.bounds().width, "the smoke layer covers the window");

    let button = |name: &str| {
        dialog
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Button>())
            .find(|button| button.name().as_deref() == Some(name))
            .unwrap_or_else(|| panic!("the part {name}"))
    };
    assert!(button("PrimaryButton").is_visible());
    assert!(button("SecondaryButton").is_visible());
    assert!(button("CloseButton").is_visible());

    button("PrimaryButton").raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(!dialog.is_open());
    // The result does not wait for the fade, during which the dialog takes no input.
    assert!(!dialog.is_hit_test_visible());
    assert_eq!(root.get_control::<TextBlock>("DialogResult").text().as_deref(), Some("User saved their work"));
    window.close();
}
