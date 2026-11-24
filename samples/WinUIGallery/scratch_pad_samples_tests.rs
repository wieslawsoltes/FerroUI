//! The ScratchPad page: the code behind of `ScratchPadPage.xaml.cs`.

use crate::scratch_pad_samples::{
    add_xml_namespace, auto_insert, indent_edit, previous_line, zone_color, zones, AutoInsert, Zone, DEFAULT_XAML,
};
use crate::{examples, tests::start};
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
