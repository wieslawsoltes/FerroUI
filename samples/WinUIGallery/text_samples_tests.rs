//! The RichEditBox and RichTextBlock examples on their native controls, driven as a user does;
//! no WinUI visual-parity claim.

use crate::tests::{press, start};
use crate::{examples, text_samples};
use ferroui_base::input::{FocusChangedEventArgs, InputElement, Key, TextInputEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::Ref;
use ferroui_controls::documents::CharacterFormat;
use ferroui_controls::rich_text::TextFormatValue;
use ferroui_controls::{
    Button, ComboBox, Control, RichEditBox, RichTextBlock, RichTextBlockOverflow, StackPanel, TextBlock,
    TextBox,
};
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

/// Raises the text input of `text` on `element`, as the keyboard does.
fn type_text(element: &InputElement, text: &str) {
    let mut args = TextInputEventArgs::new();
    args.text = Some(text.to_string());
    args.set_routed_event(Some(InputElement::text_input_event()));
    element.raise_event(&args);
}

fn color_of(brush: &Rc<dyn IBrush>) -> Option<Color> {
    brush.as_solid_color_brush().map(|brush| brush.color())
}

/// The uniform character format of `start..end` of the editor's document.
fn format(editor: &RichEditBox, start: usize, end: usize) -> CharacterFormat {
    match editor
        .text_document()
        .get_range(start, end)
        .unwrap()
        .character_format()
        .unwrap()
    {
        TextFormatValue::Uniform(format) => format,
        TextFormatValue::Mixed => panic!("the format of {start}..{end} is mixed"),
    }
}

fn foreground(editor: &RichEditBox, start: usize, end: usize) -> Option<Color> {
    format(editor, start, end)
        .foreground
        .flatten()
        .and_then(|brush| color_of(&brush))
}

fn background(editor: &RichEditBox, start: usize, end: usize) -> Option<Color> {
    format(editor, start, end)
        .background
        .flatten()
        .and_then(|brush| color_of(&brush))
}

#[test]
fn rich_edit_box_page_loads_five_examples_and_records_the_unavailable_math_mode() {
    let _application = start();
    let root = page("RichEditBox");
    for index in 1..=5 {
        assert!(root
            .find_control::<StackPanel>(&format!("RichEditBoxExample{index}"))
            .is_some());
    }
    for name in ["SimpleEditor", "REBCustom", "editor", "MathEditor", "mathEditor2"] {
        assert!(root.find_control::<RichEditBox>(name).is_some(), "{name}");
    }
    assert!(!root.get_control::<Button>("SetMathmlFormulaBtn").is_enabled());
    assert!(root
        .get_control::<TextBlock>("MathModeDiagnostic")
        .text()
        .unwrap()
        .contains("RichEditMathMode"));
    assert!(root
        .get_control::<TextBlock>("MathmlDiagnostic")
        .text()
        .unwrap()
        .contains("SetMathML"));
}

#[test]
fn rich_edit_simple_editor_accepts_typing_and_backspace() {
    let _application = start();
    let root = page("RichEditBox");
    let editor = root.get_control::<RichEditBox>("SimpleEditor");
    type_text(&editor, "Hello");
    assert_eq!(editor.text(), "Hello");
    press(&editor, Key::Back, false);
    assert_eq!(editor.text(), "Hell");
}

#[test]
fn rich_edit_formatting_buttons_toggle_bold_and_italic_on_the_selection() {
    let _application = start();
    let root = page("RichEditBox");
    let editor = root.get_control::<RichEditBox>("editor");
    editor.set_text("Hello world").unwrap();
    editor.select(0, 5);
    let bold = root.get_control::<Button>("boldButton");
    let italic = root.get_control::<Button>("italicButton");
    click(&bold);
    assert!(editor.is_bold());
    click(&italic);
    assert!(editor.is_italic());
    click(&bold);
    assert!(!editor.is_bold());
    assert!(editor.is_italic());
    editor.select(6, 11);
    assert!(!editor.is_bold());
    assert!(!editor.is_italic());
}

#[test]
fn rich_edit_color_buttons_color_the_selection_and_the_text_that_follows() {
    let _application = start();
    let root = page("RichEditBox");
    let editor = root.get_control::<RichEditBox>("editor");
    editor.set_text("Hello world").unwrap();
    editor.select(0, 5);
    click(&root.get_control::<Button>("RedColorButton"));
    assert_eq!(foreground(&editor, 0, 5), Some(Colors::RED));
    assert_ne!(foreground(&editor, 6, 11), Some(Colors::RED));
    // Editor_TextChanged (a deferred routed event): new text takes the color chosen last.
    editor.select(11, 11);
    type_text(&editor, "!");
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    let insertion = editor
        .text_document()
        .selection()
        .character_format()
        .unwrap()
        .foreground
        .flatten()
        .and_then(|brush| color_of(&brush));
    assert_eq!(insertion, Some(Colors::RED));
    editor.select(0, 5);
    click(&root.get_control::<Button>("BlueColorButton"));
    assert_eq!(foreground(&editor, 0, 5), Some(Colors::BLUE));
}

#[test]
fn rich_edit_find_box_highlights_every_match_and_clears_them_when_it_loses_focus() {
    let _application = start();
    let root = page("RichEditBox");
    let editor = root.get_control::<RichEditBox>("editor");
    editor.set_text("one two one").unwrap();
    let find_box = root.get_control::<TextBox>("findBox");
    // No theme here: the highlight is the fallback system highlight color.
    let highlight = Color::from_rgb(0x00, 0x78, 0xD7);
    find_box.set_text(Some("one"));
    assert_eq!(background(&editor, 0, 3), Some(highlight));
    assert_eq!(foreground(&editor, 0, 3), Some(Colors::WHITE));
    assert_eq!(background(&editor, 8, 11), Some(highlight));
    assert_ne!(background(&editor, 4, 7), Some(highlight));
    find_box.set_text(Some("two"));
    assert_ne!(background(&editor, 0, 3), Some(highlight));
    assert_eq!(background(&editor, 4, 7), Some(highlight));
    find_box.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    for (start, end) in [(0, 3), (4, 7), (8, 11)] {
        assert_ne!(background(&editor, start, end), Some(highlight));
    }
    find_box.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert_eq!(background(&editor, 4, 7), Some(highlight));
    // The focused editor drops the background of the highlights.
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert_ne!(background(&editor, 4, 7), Some(highlight));
    // A search for text that is not there highlights nothing.
    text_samples::highlight_matches(&editor, "three");
    for (start, end) in [(0, 3), (4, 7), (8, 11)] {
        assert_ne!(background(&editor, start, end), Some(highlight));
    }
}

#[test]
fn rich_edit_file_buttons_report_a_missing_storage_service_or_picker() {
    use ferroui_controls::Window;
    let _application = start();
    let root = page("RichEditBox");
    let status = root.get_control::<TextBlock>("FileStatus");
    click(&root.get_control::<Button>("openFileButton"));
    assert_eq!(status.text().as_deref(), Some("This window has no storage service."));
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    click(&root.get_control::<Button>("saveFileButton"));
    assert_eq!(status.text().as_deref(), Some("This platform has no file picker."));
    window.close();
}

#[cfg(feature = "theme")]
#[test]
fn rich_edit_custom_menu_adds_share_only_when_it_opens_for_rebcustom() {
    use ferroui_controls::primitives::PopupFlyoutBase;
    use ferroui_controls::{MenuFlyout, MenuItem, Window};
    let _application = crate::tests::start_winui();
    let root = page("RichEditBox");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    let custom = root.get_control::<RichEditBox>("REBCustom");
    let simple = root.get_control::<RichEditBox>("SimpleEditor");
    let flyout = custom
        .context_flyout()
        .expect("the WinUI theme gives the RichEditBox its context menu");
    let menu = flyout.cast::<MenuFlyout>().unwrap();
    let popup = flyout.cast::<PopupFlyoutBase>().unwrap();
    let shares = || {
        let items = menu.items();
        (0..items.count())
            .filter_map(|index| items.get_at(index))
            .filter_map(|item| Control::from_boxed(&item))
            .filter_map(|item| item.cast::<MenuItem>())
            .filter(|item| {
                item.header()
                    .and_then(|header| header.downcast_ref::<String>().cloned())
                    .as_deref()
                    == Some(text_samples::SHARE_COMMAND_HEADER)
            })
            .count()
    };
    assert_eq!(shares(), 0);
    for _ in 0..2 {
        popup.show_at_with(&custom, false);
        assert_eq!(shares(), 1);
        flyout.set_is_open(false);
    }
    popup.show_at_with(&simple, false);
    assert_eq!(shares(), 0);
    flyout.set_is_open(false);
    window.close();
}

#[test]
fn rich_text_block_page_loads_four_examples_with_source_text_and_selection_highlight() {
    let _application = start();
    let root = page("RichTextBlock");
    for index in 1..=4 {
        assert!(root
            .find_control::<StackPanel>(&format!("RichTextBlockExample{index}"))
            .is_some());
    }
    assert_eq!(
        root.get_control::<RichTextBlock>("SimpleRichTextBlock").text(),
        "I am a RichTextBlock."
    );
    let highlighted = root.get_control::<RichTextBlock>("SelectionHighlightRichTextBlock");
    assert!(highlighted.is_text_selection_enabled());
    assert_eq!(
        highlighted
            .selection_highlight_color()
            .and_then(|brush| color_of(&brush)),
        Some(Colors::GREEN)
    );
    assert!(highlighted.text().contains("formatted text"));
    highlighted.select(0, 12);
    assert_eq!(highlighted.selected_text(), "RichTextBloc");
}

#[test]
fn rich_text_block_overflow_flows_into_two_linked_containers() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::testing::MockWindowingPlatform;
    use ferroui_controls::Window;
    let _application = start();
    let root = page("RichTextBlock");
    let primary = root.get_control::<RichTextBlock>("OverflowRichTextBlock");
    let first = root.get_control::<RichTextBlockOverflow>("firstOverflowContainer");
    let second = root.get_control::<RichTextBlockOverflow>("secondOverflowContainer");
    assert!(primary.overflow_content_target() == Some(first.clone()));
    assert!(first.overflow_content_target() == Some(second.clone()));
    assert!(second.overflow_content_target().is_none());
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(900.0, 1600.0));
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    assert!(primary.has_overflow_content());
    assert!(first.content_source() == Some(primary.clone()));
    window.close();
}

#[test]
fn rich_text_block_highlight_follows_the_chosen_color_over_the_source_range() {
    let _application = start();
    let root = page("RichTextBlock");
    let block = root.get_control::<RichTextBlock>("TextHighlightingRichTextBlock");
    let combo = root.get_control::<ComboBox>("HighlightColorCombobox");
    let check = |expected: Color| {
        let highlighters = block.text_highlighters();
        assert_eq!(highlighters.count(), 1);
        let highlighter = highlighters.get(0);
        assert_eq!(
            highlighter.background().and_then(|brush| color_of(&brush)),
            Some(expected)
        );
        let range = highlighter.ranges().get(0).unwrap();
        assert_eq!((range.start_index(), range.length()), (28, 11));
        let text: Vec<u16> = block.text().encode_utf16().collect();
        assert_eq!(String::from_utf16(&text[28..39]).unwrap(), "consectetur");
    };
    check(Colors::YELLOW);
    combo.set_selected_index(1);
    check(Colors::RED);
    combo.set_selected_index(2);
    check(Colors::BLUE);
    combo.set_selected_index(0);
    check(Colors::YELLOW);
}
