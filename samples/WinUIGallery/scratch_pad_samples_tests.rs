//! The ScratchPad page: the code behind of `ScratchPadPage.xaml.cs`.

use crate::scratch_pad_samples::{
    add_xml_namespace, auto_insert, indent_edit, previous_line, selection_span, zone_color, zones, AutoInsert, Zone,
    DEFAULT_XAML,
};
use crate::{examples, tests::start};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::media::Colors;
use ferroui_base::Ref;
use ferroui_controls::rich_text::TextGetOptions;
use ferroui_controls::{Button, Control, RichEditBox, ScrollViewer, TextBlock};

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn page() -> Ref<Control> {
    examples::create(examples::find("ScratchPad").unwrap()).unwrap_or_else(|error| panic!("ScratchPad: {error}"))
}

fn click(button: &Button) {
    button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event()));
}

/// Raises a key event on the editor as the keyboard does; returns whether it was handled.
fn key(textbox: &RichEditBox, key: Key, modifiers: KeyModifiers, up: bool) -> bool {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.key_modifiers = modifiers;
    args.set_routed_event(Some(if up { InputElement::key_up_event() } else { InputElement::key_down_event() }));
    textbox.raise_event(&args);
    args.handled()
}

/// A full press of Enter: the editor inserts the paragraph on the way down, the page indents on the way up.
fn press_enter(textbox: &RichEditBox) {
    key(textbox, Key::Enter, KeyModifiers::empty(), false);
    key(textbox, Key::Enter, KeyModifiers::empty(), true);
}

/// The text of the editor with `\n` line ends, without the mark of the last paragraph.
fn text_of(textbox: &RichEditBox) -> String {
    let text = textbox.text_document().get_text(TextGetOptions::USE_LF).unwrap();
    text.strip_suffix('\n').unwrap_or(&text).to_owned()
}

fn set_content(textbox: &RichEditBox, text: &str, selection: std::ops::Range<usize>) {
    let document = textbox.text_document();
    document.set_text(text).unwrap();
    document.selection().set_range(selection.start, selection.end).unwrap();
}

#[test]
fn scratch_pad_zones_follow_the_source_formatter() {
    let text = "<a b=\"c\">d</a> <!-- e -->";
    let of = |zone| {
        zones(text)
            .into_iter()
            .filter(|(_, found)| *found == zone)
            .map(|(range, _)| text[range].to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(of(Zone::TagName), ["a", "a"]);
    assert_eq!(of(Zone::PropertyName), ["b"]);
    assert_eq!(of(Zone::PropertyValue), ["=\"c\""]);
    assert_eq!(of(Zone::Content), ["d"]);
    // The zone still open at the end of the text is not reported, as in the source.
    assert!(of(Zone::Comment).is_empty());
    assert_eq!(zone_color(Zone::Comment, true), Colors::GREEN);
    assert_eq!(zone_color(Zone::PropertyName, false), Colors::LIGHT_SKY_BLUE);
}

#[test]
fn scratch_pad_zones_use_utf16_offsets() {
    let zones = zones("<a>😀<b>");
    let content = zones.iter().find(|(_, zone)| *zone == Zone::Content).unwrap();
    assert_eq!(content.0, 3..5);
}

#[test]
fn scratch_pad_adds_the_namespaces_to_the_root() {
    let xml = add_xml_namespace("  <Button Content=\"x\"/>").unwrap();
    assert!(xml.starts_with("<Button xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation'"));
    assert!(xml.ends_with(" Content=\"x\"/>"));
    assert_eq!(add_xml_namespace("Button"), None);
}

#[test]
fn scratch_pad_auto_inserts_close_tags_and_quotes() {
    let close = |text: &str| auto_insert(&utf16(text), text.encode_utf16().count());
    assert_eq!(close("<Button>"), Some(AutoInsert::CloseTag("</Button>".into())));
    assert_eq!(close("<Button a=\"b\">"), Some(AutoInsert::CloseTag("</Button>".into())));
    assert_eq!(close("<Button/>"), None);
    assert_eq!(close("<!-- a -->"), None);
    assert_eq!(close("<a></a>"), None, "a close tag is closed already");
    assert_eq!(close("<Button Content="), Some(AutoInsert::Quotes));
    assert_eq!(close("a="), None, "outside a tag");
    assert_eq!(close("<a b=\"c="), None, "inside a value");
}

#[test]
fn scratch_pad_indents_and_unindents_the_selected_lines() {
    let text = utf16("ab\rcd\ref");
    let edit = indent_edit(&text, 1..4, false);
    assert_eq!(edit.lines, [(0, 0), (3, 0)]);
    assert_eq!(edit.selection, 5..12);

    let text = utf16("    ab\r  cd\rx y");
    let edit = indent_edit(&text, 5..9, true);
    assert_eq!(edit.lines, [(0, 4), (7, 2)]);
    assert_eq!(edit.selection, 1..3);

    // Only leading whitespace goes: the source would delete the "x" here.
    let text = utf16("x y z");
    assert_eq!(indent_edit(&text, 0..3, true).lines, [(0, 0)]);
    assert_eq!(previous_line(&utf16("  ab\rcd"), 5), utf16("  ab"));
    assert!(previous_line(&utf16("ab"), 1).is_empty());
}

#[test]
fn scratch_pad_loads_the_default_content_and_reports_errors() {
    let _application = start();
    let root = page();
    let textbox = root.get_control::<RichEditBox>("textbox");
    let host = root.get_control::<ScrollViewer>("scratchPad");
    let status = root.get_control::<TextBlock>("loadStatus");
    let text = textbox.text_document().get_text(TextGetOptions::USE_LF).unwrap();
    assert_eq!(text.trim_end(), DEFAULT_XAML);

    click(&root.get_control::<Button>("loadButton"));
    let loaded = status.text().unwrap_or_default();
    assert!(loaded.starts_with("Load successful."), "{loaded}");
    let content = host.content().and_then(|content| Control::from_boxed(&content)).unwrap();
    assert!(content.cast::<ferroui_controls::Border>().is_some());

    textbox.text_document().set_text("<NoSuchControl/>").unwrap();
    click(&root.get_control::<Button>("loadButton"));
    let message = status.text().unwrap_or_default();
    assert!(!message.is_empty() && !message.starts_with("Load successful."), "{message}");
}

#[test]
fn scratch_pad_tab_indents_and_shift_tab_unindents_the_selection() {
    let _application = start();
    let root = page();
    let textbox = root.get_control::<RichEditBox>("textbox");
    set_content(&textbox, "ab\ncd\nef", 1..4);

    assert!(key(&textbox, Key::Tab, KeyModifiers::empty(), false), "Tab stays in the editor");
    assert_eq!(text_of(&textbox), "    ab\n    cd\nef");
    assert_eq!(selection_span(&textbox.text_document()), 5..12);

    assert!(key(&textbox, Key::Tab, KeyModifiers::SHIFT, false), "Shift+Tab stays in the editor");
    assert_eq!(text_of(&textbox), "ab\ncd\nef");
    assert_eq!(selection_span(&textbox.text_document()), 1..4);
}

#[test]
fn scratch_pad_tab_leaves_a_caret_and_a_read_only_editor_alone() {
    let _application = start();
    let root = page();
    let textbox = root.get_control::<RichEditBox>("textbox");

    // One character or less is not a block: no line gets the indent.
    set_content(&textbox, "ab\ncd", 1..2);
    key(&textbox, Key::Tab, KeyModifiers::empty(), false);
    assert!(!text_of(&textbox).contains("    "), "{:?}", text_of(&textbox));

    // A read-only editor keeps Tab for focus navigation.
    set_content(&textbox, "ab\ncd", 0..4);
    textbox.set_is_read_only(true);
    assert!(!key(&textbox, Key::Tab, KeyModifiers::empty(), false));
    assert_eq!(text_of(&textbox), "ab\ncd");
}

#[test]
fn scratch_pad_enter_keeps_the_indent_and_opens_an_end_tag() {
    let _application = start();
    let root = page();
    let textbox = root.get_control::<RichEditBox>("textbox");

    set_content(&textbox, "  ab", 4..4);
    press_enter(&textbox);
    assert_eq!(text_of(&textbox), "  ab\n  ");
    assert_eq!(selection_span(&textbox.text_document()), 7..7);

    // Between a start tag and its end tag: the caret line is indented once more, the end tag
    // moves to its own line at the indent of the start tag.
    set_content(&textbox, "    <a></a>", 7..7);
    press_enter(&textbox);
    assert_eq!(text_of(&textbox), "    <a>\n        \n    </a>");
    assert_eq!(selection_span(&textbox.text_document()), 16..16);
}

#[test]
fn scratch_pad_f5_loads_on_key_up_even_when_handled() {
    let _application = start();
    let root = page();
    let textbox = root.get_control::<RichEditBox>("textbox");
    let host = root.get_control::<ScrollViewer>("scratchPad");
    let status = root.get_control::<TextBlock>("loadStatus");
    set_content(&textbox, "<Button Content=\"x\"/>", 0..0);

    key(&textbox, Key::F5, KeyModifiers::empty(), false);
    assert_eq!(status.text().unwrap_or_default(), "", "the page loads on the way up");

    let mut args = KeyEventArgs::new();
    args.key = Key::F5;
    args.set_routed_event(Some(InputElement::key_up_event()));
    args.set_handled(true);
    textbox.raise_event(&args);
    let loaded = status.text().unwrap_or_default();
    assert!(loaded.starts_with("Load successful."), "{loaded}");
    let content = host.content().and_then(|content| Control::from_boxed(&content)).unwrap();
    assert!(content.cast::<Button>().is_some());

    textbox.text_document().set_text("<NoSuchControl/>").unwrap();
    key(&textbox, Key::F5, KeyModifiers::empty(), true);
    let message = status.text().unwrap_or_default();
    assert!(!message.is_empty() && !message.starts_with("Load successful."), "{message}");
}

#[cfg(feature = "theme")]
#[test]
fn scratch_pad_reset_asks_and_restores_the_default_content() {
    use ferroui_controls::primitives::OverlayLayer;
    use ferroui_controls::{ContentDialog, Window};

    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    let textbox = root.get_control::<RichEditBox>("textbox");
    textbox.text_document().set_text("<Button/>").unwrap();

    click(&root.get_control::<Button>("resetButton"));
    let overlay = OverlayLayer::get_overlay_layer(&window).unwrap();
    let dialog = overlay
        .children()
        .snapshot()
        .iter()
        .find_map(|child| child.clone().cast::<ContentDialog>())
        .expect("the confirmation");
    assert_eq!(textbox.text_document().text(), "<Button/>", "nothing changes before the answer");
    let primary = dialog
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Button>())
        .find(|button| button.name().as_deref() == Some("PrimaryButton"))
        .unwrap();
    click(&primary);
    assert_eq!(textbox.text_document().text(), DEFAULT_XAML);
    window.close();
}
