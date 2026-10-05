//! Tests of the main view model, its models and the temporary shell.
//!
//! Not ports: the upstream sample has no tests.

use super::support::*;
use crate::models::{HomeSection, PageItem};
use crate::temporary::shell::create_shell;
use crate::view_models::MainWindowViewModel;
use ferroui_base::media::StreamGeometry;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::chrome::TitleBarDecorations;
use ferroui_controls::ContentPage;
use std::cell::RefCell;
use std::rc::Rc;

fn icon() -> ferroui_base::Ref<StreamGeometry> {
    StreamGeometry::parse("M0,0L1,1").unwrap()
}

fn page_item(header: &str, description: &str, section: Option<&Rc<HomeSection>>) -> Rc<PageItem> {
    PageItem::new(header, || ContentPage::new().upcast(), icon(), description, section, None)
}

fn record_changes(section: &Rc<HomeSection>) -> Rc<RefCell<Vec<String>>> {
    use ferroui_base::data::model::INotifyPropertyChanged;
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    section.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
    seen
}

#[test]
fn a_page_item_matches_its_header_description_and_section() {
    let _app = start_application();
    let section = HomeSection::new("Basic Input", icon());
    let item = page_item("CheckBox", "Two- and three-state check boxes", Some(&section));

    assert_eq!("Basic Input", item.section());
    assert!(item.matches_search(&PageItem::create_search_key(&["check"])));
    assert!(item.matches_search(&PageItem::create_search_key(&["three state"])));
    assert!(item.matches_search(&PageItem::create_search_key(&["basic"])));
    assert!(!item.matches_search(&PageItem::create_search_key(&["slider"])));
}

#[test]
fn hiding_a_page_item_notifies_the_visibility_of_its_section() {
    let _app = start_application();
    let section = HomeSection::new("Text", icon());
    let first = page_item("Label", "", Some(&section));
    let second = page_item("TextBox", "", Some(&section));
    section.set_items(Some(Rc::new(vec![first.clone(), second.clone()])));
    let seen = record_changes(&section);

    first.set_is_visible(false);
    assert!(section.is_section_visible());
    second.set_is_visible(false);
    assert!(!section.is_section_visible());
    assert_eq!(vec!["IsSectionVisible".to_string(), "IsSectionVisible".to_string()], *seen.borrow());
}

#[test]
fn a_section_shows_the_selection_for_its_own_page_or_while_it_is_collapsed() {
    let _app = start_application();
    let section = HomeSection::new("Text", icon());
    let item = page_item("Label", "", Some(&section));
    section.set_items(Some(Rc::new(vec![item.clone()])));
    assert!(!section.shows_selection());

    section.set_current_page(Some(item));
    assert!(section.is_current());
    assert!(section.shows_selection());
    section.set_is_expanded(true);
    assert!(!section.shows_selection());

    section.set_current_page(Some(section.page_item()));
    assert!(section.shows_selection());
}

#[test]
fn the_view_model_lists_every_upstream_page_as_available_or_unavailable() {
    let _app = start_catalog_application();
    let view_model = MainWindowViewModel::new();

    assert_eq!(11, view_model.home_sections().len());
    let available: usize = view_model.home_sections().iter().map(|s| s.items().map_or(0, |items| items.len())).sum();
    // The upstream list has 74 entries.
    assert_eq!(74, available + view_model.unavailable_pages().len());
    for page in view_model.unavailable_pages() {
        assert!(!page.reason.is_empty(), "{} has no reason", page.document);
    }
}

#[test]
fn filter_shows_the_matching_pages_and_expands_their_sections() {
    let _app = start_catalog_application();
    let view_model = MainWindowViewModel::new();
    let sections = view_model.home_sections();
    let all_pages = || sections.iter().flat_map(|s| s.items().unwrap().to_vec()).collect::<Vec<_>>();
    assert!(all_pages().iter().all(|page| page.is_visible()));
    assert!(sections.iter().all(|section| !section.is_expanded()));

    view_model.set_query(Some("border".to_string()));
    let visible: Vec<String> = all_pages().iter().filter(|p| p.is_visible()).map(|p| p.header()).collect();
    assert!(visible.contains(&"Border".to_string()), "{visible:?}");
    assert!(visible.len() < all_pages().len());
    for section in sections.iter() {
        assert_eq!(section.is_section_visible(), section.is_expanded(), "{}", section.title());
    }

    view_model.set_query(Some("  ".to_string()));
    assert!(all_pages().iter().all(|page| page.is_visible()));
    assert!(sections.iter().all(|section| !section.is_expanded()));
}

#[test]
fn the_title_bar_decoration_switches_change_the_decorations() {
    let _app = start_application();
    let view_model = MainWindowViewModel::new();
    assert!(view_model.show_title());

    view_model.set_show_title(false);
    assert!(!view_model.show_title());
    assert_eq!(TitleBarDecorations::ALL & !TitleBarDecorations::TITLE, view_model.title_bar_decorations());
    view_model.set_show_title(true);
    assert_eq!(TitleBarDecorations::ALL, view_model.title_bar_decorations());
}

#[test]
fn navigating_to_an_item_replaces_the_page_and_marks_its_section() {
    let _app = start_catalog_application();
    let view_model = MainWindowViewModel::new();
    let shell = create_shell(view_model.clone());
    shell.window.show();
    let item = shell.first_page.clone().expect("a page that loads");

    view_model.navigate_to_item(&item);
    Dispatcher::ui_thread().run_jobs(None);

    assert!(view_model.current_page_item().is_some_and(|current| Rc::ptr_eq(&current, &item)));
    assert_eq!(1, shell.navigation_page.stack_depth());
    let section = view_model.home_sections().iter().find(|s| s.title() == item.section()).cloned().unwrap();
    assert!(section.is_current());
    assert!(section.is_expanded());
    shell.window.close();
}

/// Every page the application offers constructs and is shown by the
/// navigation page of the shell (TEMPORARY shell, see `temporary.rs`).
#[test]
fn every_available_page_is_shown_by_the_shell() {
    let _app = start_catalog_application();
    let view_model = MainWindowViewModel::new();
    let shell = create_shell(view_model.clone());
    shell.window.show();

    let mut shown = 0;
    for section in view_model.home_sections().iter() {
        for item in section.items().unwrap().iter() {
            view_model.navigate_to_item(item);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                view_model.current_page_item().is_some_and(|current| Rc::ptr_eq(&current, item)),
                "{} is not the current page",
                item.header()
            );
            shown += 1;
        }
    }
    assert!(shown > 0);
    shell.window.close();
}
