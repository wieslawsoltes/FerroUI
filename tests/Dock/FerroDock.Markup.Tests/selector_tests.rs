//! Real source selector theme, original entries and delegated automation providers.
use ferrodock_ferro::{
    controls::DockSelectorOverlay,
    selectors::{DockSelectorItem, DockSelectorMode},
};
use ferrodock_model::{collections::ReadOnlyListHandle, core::IDockable};
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::{
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{
    automation::{
        peers::ControlAutomationPeer,
        provider::{IScrollProvider, ISelectionProvider, ScrollAmount},
    },
    AppBuilder, Application, Border, ListBox, Panel, TextBlock, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_system::collections::object_model::ObservableCollection;
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-selector-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn item(index: i64) -> Rc<DockSelectorItem> {
    let doc = Document::new();
    doc.set_title(format!("Document {index}"));
    DockSelectorItem::new(doc, index, true, false, true, false, false)
}
#[test]
fn full_selector_theme_renders_original_entries_live_collections_selection_and_scroll_providers() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/SelectorConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                let overlay = panel
                    .children()
                    .get(0)
                    .cast::<DockSelectorOverlay>()
                    .unwrap();
                assert_eq!(overlay.mode(), DockSelectorMode::All);
                let collection = ObservableCollection::from_items((0..40).map(item).collect());
                let first = collection.get(0);
                let last = collection.get(39);
                let list = ReadOnlyListHandle::from_observable(collection.clone());
                overlay.set_items(Some(list.clone()));
                overlay.set_selected_item(Some(first.clone()));
                let window = Window::new();
                window.set_width(640.0);
                window.set_height(480.0);
                window.set_content(Some(Rc::new(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                let heading = overlay
                    .find_descendant_of_type_where::<TextBlock>(false, |c| {
                        c.text().as_deref() == Some("All")
                    })
                    .unwrap();
                overlay.set_mode(DockSelectorMode::Tools);
                assert_eq!(heading.text().as_deref(), Some("Tools"));
                overlay.set_mode(DockSelectorMode::All);
                let items = overlay
                    .find_descendant_of_type_where::<ListBox>(false, |c| {
                        c.name().as_deref() == Some("PART_ItemsList")
                    })
                    .unwrap();
                assert_eq!(items.items().count(), 40);
                let title = items
                    .find_descendant_of_type_where::<TextBlock>(false, |c| {
                        c.text().as_deref() == Some("Document 0")
                    })
                    .unwrap();
                assert_eq!(title.text().as_deref(), Some("Document 0"));
                for (label, visible) in [
                    ("DOC", true),
                    ("TOOL", false),
                    ("PIN", true),
                    ("FLOAT", false),
                    ("HIDDEN", false),
                ] {
                    let badge = items
                        .find_descendant_of_type_where::<TextBlock>(false, |c| {
                            c.text().as_deref() == Some(label)
                        })
                        .unwrap();
                    assert_eq!(
                        badge
                            .find_ancestor_of_type::<Border>(false)
                            .unwrap()
                            .is_visible(),
                        visible
                    );
                }
                let peer = ControlAutomationPeer::create_peer_for_element(&overlay);
                let list_peer = ControlAutomationPeer::create_peer_for_element(&items);
                let selection = peer.get_provider::<dyn ISelectionProvider>().unwrap();
                let expected = list_peer.get_provider::<dyn ISelectionProvider>().unwrap();
                assert_eq!(selection.get_selection(), expected.get_selection());
                assert_eq!(selection.get_selection().len(), 1);
                overlay.set_selected_item(Some(last.clone()));
                window.update_layout();
                assert_eq!(selection.get_selection(), expected.get_selection());
                assert_eq!(selection.get_selection().len(), 1);
                // Native model objects and their Rc transport handles denote the same reference.
                let selected = ferroui_base::metadata::from_markup_value::<Rc<DockSelectorItem>>(
                    &items.selected_item(),
                )
                .expect("native selected item retains its original model reference");
                assert!(Rc::ptr_eq(&selected, &last));
                let scroll = peer.get_provider::<dyn IScrollProvider>().unwrap();
                let expected = list_peer.get_provider::<dyn IScrollProvider>().unwrap();
                assert_eq!(
                    scroll.vertically_scrollable(),
                    expected.vertically_scrollable()
                );
                // The published ItemsControl peer exposes its scroller discovery separately
                // from get_provider. Verify Dock delegates its current state, then exercise
                // real scrolling after the native client initializes that provider.
                list_peer
                    .cast::<ferroui_controls::automation::peers::ItemsControlAutomationPeer>()
                    .unwrap()
                    .scroller();
                assert!(scroll.vertically_scrollable());
                assert_eq!(
                    scroll.horizontal_scroll_percent(),
                    expected.horizontal_scroll_percent()
                );
                assert_eq!(scroll.vertical_view_size(), expected.vertical_view_size());
                scroll.set_scroll_percent(-1.0, 0.0).unwrap();
                window.update_layout();
                assert_eq!(
                    scroll.vertical_scroll_percent(),
                    expected.vertical_scroll_percent()
                );
                scroll
                    .scroll(ScrollAmount::NoAmount, ScrollAmount::LargeIncrement)
                    .unwrap();
                window.update_layout();
                assert!(scroll.vertical_scroll_percent() > 0.0);
                collection.add(item(40));
                assert_eq!(items.items().count(), 41);
                collection.remove_at(0);
                assert_eq!(items.items().count(), 40);
                let replaced = item(100);
                collection.set(0, replaced.clone());
                window.update_layout();
                let actual = ferroui_base::metadata::from_markup_value::<Rc<DockSelectorItem>>(
                    &items.items().get_at(0),
                )
                .expect("native collection item retains its original model reference");
                assert!(Rc::ptr_eq(&actual, &replaced));
                assert_eq!(actual.title(), "Document 100");
                let replacement = ReadOnlyListHandle::from_items(vec![first.clone()]);
                overlay.set_items(Some(replacement));
                overlay.set_selected_item(Some(first));
                window.update_layout();
                collection.add(item(41));
                assert_eq!(items.items().count(), 1);
                overlay.set_items(None);
                assert_eq!(items.items().count(), 0);
                overlay.set_is_open(false);
                assert!(!overlay.is_visible());
                assert!(!overlay.is_hit_test_visible());
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
