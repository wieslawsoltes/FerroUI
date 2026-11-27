//! The TabView page: the source's adding and closing, markup and keyboard examples with their code
//! behind.

use crate::tab_view_samples::{create_new_tab, handle_accelerator, overlay_mode, width_mode};
use crate::{examples, tests::start};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::Ref;
use ferroui_controls::{
    ComboBox, Control, Frame, Symbol, SymbolIconSource, TabView, TabViewCloseButtonOverlayMode, TabViewItem,
    TabViewWidthMode,
};

fn page() -> Ref<Control> {
    examples::create(examples::find("TabView").unwrap()).unwrap_or_else(|error| panic!("TabView: {error}"))
}

fn header(view: &TabView, index: usize) -> String {
    let item = view.tab_items().get_at(index).unwrap();
    let tab = Control::from_boxed(&item).unwrap().cast::<TabViewItem>().unwrap();
    tab.header().and_then(|header| header.downcast_ref::<String>().cloned()).unwrap()
}

#[test]
fn new_tabs_are_documents_with_a_frame_on_a_sample_page() {
    let _application = start();
    let tab = create_new_tab(4);
    assert_eq!(tab.header().and_then(|header| header.downcast_ref::<String>().cloned()).as_deref(), Some("Document 4"));
    let icon = tab.icon_source().unwrap().cast::<SymbolIconSource>().unwrap();
    assert_eq!(icon.symbol(), Symbol::Document);
    let frame = Control::from_boxed(&tab.content().unwrap()).unwrap().cast::<Frame>().unwrap();
    assert!(frame.content().is_some(), "the frame shows the second sample page");
}

#[test]
fn the_examples_start_with_three_tabs_and_add_and_close_tabs() {
    let _application = start();
    let root = page();
    let first = root.get_control::<TabView>("TabView1");
    let markup = root.get_control::<TabView>("TabViewMarkup");
    let keyboard = root.get_control::<TabView>("TabView2");
    let custom = root.get_control::<TabView>("TabViewCustomContent");
    let accent = root.get_control::<TabView>("TabViewAccent");
    for view in [&first, &markup, &keyboard, &custom, &accent] {
        assert_eq!(view.tab_items().count(), 3);
        assert_eq!(header(view, 2), "Document 2");
    }
    // TabView_AddButtonClick: "Document {count}".
    let tab = create_new_tab(first.tab_items().count());
    first.tab_items().add(Some(Control::boxed(tab)));
    assert_eq!(header(&first, 3), "Document 3");
}

#[test]
fn the_option_boxes_set_the_width_mode_and_the_close_button_overlay_mode() {
    let _application = start();
    let root = page();
    let widths = root.get_control::<TabView>("TabView3");
    let closing = root.get_control::<TabView>("TabView4");
    assert_eq!(widths.tab_width_mode(), TabViewWidthMode::SizeToContent);
    root.get_control::<ComboBox>("TabWidthBehaviorComboBox").set_selected_index(2);
    assert_eq!(widths.tab_width_mode(), TabViewWidthMode::Compact);
    root.get_control::<ComboBox>("TabWidthBehaviorComboBox").set_selected_index(1);
    assert_eq!(widths.tab_width_mode(), TabViewWidthMode::Equal);
    root.get_control::<ComboBox>("TabCloseButtonOverlayModeComboBox").set_selected_index(2);
    assert_eq!(closing.close_button_overlay_mode(), TabViewCloseButtonOverlayMode::OnPointerOver);
    root.get_control::<ComboBox>("TabCloseButtonOverlayModeComboBox").set_selected_index(0);
    assert_eq!(closing.close_button_overlay_mode(), TabViewCloseButtonOverlayMode::Auto);
    assert_eq!(width_mode("Equal"), TabViewWidthMode::Equal);
    assert_eq!(overlay_mode("Always"), TabViewCloseButtonOverlayMode::Always);
}

#[test]
fn the_keyboard_accelerators_add_close_and_select_tabs() {
    let _application = start();
    let root = page();
    let view = root.get_control::<TabView>("TabView2");
    assert!(handle_accelerator(&view, Key::T, KeyModifiers::CONTROL));
    assert_eq!(view.tab_items().count(), 4);
    assert!(handle_accelerator(&view, Key::D9, KeyModifiers::CONTROL));
    assert_eq!(view.selected_index(), 3);
    assert!(handle_accelerator(&view, Key::D2, KeyModifiers::CONTROL));
    assert_eq!(view.selected_index(), 1);
    // Ctrl+8 with four tabs selects nothing new.
    assert!(handle_accelerator(&view, Key::D8, KeyModifiers::CONTROL));
    assert_eq!(view.selected_index(), 1);
    assert!(handle_accelerator(&view, Key::W, KeyModifiers::CONTROL));
    assert_eq!(view.tab_items().count(), 3);
    assert_eq!(header(&view, 1), "Document 2");
    assert!(!handle_accelerator(&view, Key::T, KeyModifiers::NONE));
}

/// The real templates of the WinUI theme: the tabs of the first example are in the strip, the
/// selected tab's content is shown and the add button is there.
#[cfg(feature = "theme")]
#[test]
fn the_tab_views_take_the_templates_of_the_winui_theme() {
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::presenters::ContentPresenter;
    use ferroui_controls::{Button, Window};

    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    let view = root.get_control::<TabView>("TabView1");
    let descendants = view.get_template_descendants();
    let named = |name: &str| descendants.iter().any(|visual| visual.name().as_deref() == Some(name));
    assert!(named("TabListView") && named("TabContentPresenter"));
    assert!(descendants.iter().any(|visual| visual.cast::<Button>().is_some_and(|button| button.name().as_deref() == Some("AddButton"))));
    let presenter = descendants
        .iter()
        .find_map(|visual| visual.cast::<ContentPresenter>().filter(|presenter| presenter.name().as_deref() == Some("TabContentPresenter")))
        .unwrap();
    let shown = Control::from_boxed(&presenter.content().unwrap()).unwrap();
    assert!(shown.cast::<Frame>().is_some(), "the content of the selected tab");

    // TabView_TabCloseRequested removes the tab whose close button is clicked.
    let markup = root.get_control::<TabView>("TabViewMarkup");
    let second = Control::from_boxed(&markup.tab_items().get_at(1).unwrap()).unwrap().cast::<TabViewItem>().unwrap();
    let close = second
        .get_template_descendants()
        .into_iter()
        .find_map(|visual| visual.cast::<Button>().filter(|button| button.name().as_deref() == Some("CloseButton")))
        .expect("the close button of the tab");
    close.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(markup.tab_items().count(), 2);
    assert_eq!(header(&markup, 1), "Document 2");
}
