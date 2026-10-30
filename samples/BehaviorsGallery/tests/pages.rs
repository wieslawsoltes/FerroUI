//! The table of the pages.

use super::support::start_application;
use crate::pages::placeholder::NOT_PORTED_YET;
use crate::{PageInfo, PAGES, SAMPLE};
use ferroui_controls::TextBlock;
use std::collections::BTreeSet;

/// The number of the pages of the reference (the tab items of its main view).
const REFERENCE_PAGES: usize = 234;

#[test]
fn the_table_has_every_page_of_the_reference_once() {
    assert_eq!(PAGES.len(), REFERENCE_PAGES);
    let headers: BTreeSet<&str> = PAGES.iter().map(|page| page.header).collect();
    let views: BTreeSet<&str> = PAGES.iter().map(|page| page.view_name()).collect();
    assert_eq!(headers.len(), PAGES.len(), "the headers are unique");
    assert_eq!(views.len(), PAGES.len(), "the views are unique");
}

/// A ported page has its document under `pages/`, named after its view, and every document
/// under `pages/` is the document of a page of the table.
#[test]
fn every_document_under_pages_is_a_ported_page() {
    let documents: BTreeSet<String> = SAMPLE
        .documents
        .iter()
        .map(|(path, _)| path.to_string())
        .filter(|path| path.starts_with("/pages/"))
        .collect();
    for page in PAGES.iter().filter(|page| page.is_ported()) {
        let document = format!("/pages/{}.xaml", page.view_name());
        assert!(
            documents.contains(&document),
            "{}: {document} is not a document of the gallery",
            page.header
        );
        assert!(
            page.classes.iter().any(|class| class.document == document),
            "{}: the page declares the class of {document}",
            page.header
        );
    }
    let pages: BTreeSet<String> = PAGES
        .iter()
        .flat_map(|page| page.classes.iter().map(|class| class.document.to_string()))
        .collect();
    for document in &documents {
        assert!(
            pages.contains(document),
            "{document} is the document of no page of the table"
        );
    }
}

#[test]
fn a_page_not_ported_yet_shows_its_name() {
    let _app = start_application();
    let page: &'static PageInfo = PAGES
        .iter()
        .find(|page| !page.is_ported())
        .expect("a page not ported yet");
    let view = page.create_view();
    let texts: Vec<String> = view
        .get_logical_descendants()
        .filter_map(|element| element.cast::<TextBlock>())
        .filter_map(|text| text.text())
        .collect();
    assert_eq!(
        texts,
        [page.view_name().to_string(), NOT_PORTED_YET.to_string()]
    );
}

#[test]
fn every_ported_page_is_created() {
    let _app = start_application();
    for page in PAGES.iter().filter(|page| page.is_ported()) {
        let _view = page.create_view();
    }
}
