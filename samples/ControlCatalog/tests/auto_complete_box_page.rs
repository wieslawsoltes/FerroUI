//! Tests of the auto-complete box page.
//!
//! Not ports: the upstream sample has no tests.

use super::support::*;
use crate::pages::AutoCompleteBoxPage;
use ferroui_base::threading::{CancellationTokenSource, Dispatcher, OperationCanceledError};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{AutoCompleteBox, Control, Window};
use std::task::{Context, Poll, Waker};

fn show(page: &Ref<AutoCompleteBoxPage>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

fn boxes(page: &Ref<AutoCompleteBoxPage>) -> Vec<Ref<AutoCompleteBox>> {
    page.get_logical_descendants().filter_map(|element| element.cast::<AutoCompleteBox>()).collect()
}

#[test]
fn the_boxes_of_the_page_list_the_states_and_the_custom_box_the_words() {
    let _app = start_catalog_application();
    let page = AutoCompleteBoxPage::new();
    let window = show(&page);

    let boxes = boxes(&page);
    assert_eq!(13, boxes.len());
    for auto_complete_box in &boxes {
        let count = auto_complete_box.items_source().expect("the box has items").count();
        if auto_complete_box.name().as_deref() == Some("CustomAutocompleteBox") {
            // The words of the four sentences.
            assert_eq!(2 + 4 + 5 + 7, count);
        } else {
            assert_eq!(50, count);
        }
    }
    // One list for the boxes of the states.
    let first = boxes[0].items_source().expect("the box has items");
    assert!(boxes[1].items_source().is_some_and(|items| items.ptr_eq(&first)));
    window.close();
}

#[test]
fn the_text_of_a_selected_state_follows_the_binding_of_the_box() {
    let _app = start_catalog_application();
    let page = AutoCompleteBoxPage::new();
    let window = show(&page);
    let alaska = page.states()[1].clone();

    // Without a binding: the text form of the state, its name.
    let plain = boxes(&page).into_iter().next().expect("the first box");
    plain.set_selected_item(Some(alaska.clone() as BoxedValue));
    assert_eq!(Some("Alaska"), plain.text().as_deref());

    // The binding of the document: the capital.
    let capital = boxes(&page)
        .into_iter()
        .find(|auto_complete_box| auto_complete_box.name().is_none() && auto_complete_box.value_member_binding().is_some())
        .expect("the box with the value member binding");
    capital.set_selected_item(Some(alaska.clone() as BoxedValue));
    assert_eq!(Some("Juneau"), capital.text().as_deref());

    // The multi-binding of the page: the name and the abbreviation.
    let multi = page.get_control::<AutoCompleteBox>("MultiBindingBox");
    multi.set_selected_item(Some(alaska as BoxedValue));
    assert_eq!(Some("Alaska (AK)"), multi.text().as_deref());
    window.close();
}

#[test]
fn the_custom_box_filters_by_the_last_word_and_appends_the_selected_word() {
    let _app = start_catalog_application();
    let page = AutoCompleteBoxPage::new();
    let custom = page.get_control::<AutoCompleteBox>("CustomAutocompleteBox");
    let filter = custom.text_filter().expect("the text filter of the page");
    let selector = custom.text_selector().expect("the text selector of the page");
    let word = |text: &str| Some(text.to_string());

    // The first word of a sentence that contains the text.
    assert!(filter.invoke(Some("hel"), &word("Hello")));
    assert!(!filter.invoke(Some("hel"), &word("world")));
    assert!(filter.invoke(Some("n"), &word("No")));
    assert!(filter.invoke(Some("n"), &word("Never")));
    assert!(!filter.invoke(Some("n"), &word("Hello")));
    // The word that follows the words typed so far.
    assert!(filter.invoke(Some("hello w"), &word("world")));
    assert!(!filter.invoke(Some("hello w"), &word("Hello")));
    assert!(!filter.invoke(Some("hel"), &None));

    assert_eq!("Hello world", selector.invoke(Some("Hello w"), &word("world")));
    assert_eq!("world", selector.invoke(Some("w"), &word("world")));
    assert_eq!("world", selector.invoke(None, &word("world")));
    assert_eq!("", selector.invoke(Some("Hello w"), &None));
}

#[test]
fn the_population_of_the_asynchronous_box_waits_and_ends_when_it_is_cancelled() {
    let _app = start_catalog_application();
    let page = AutoCompleteBoxPage::new();
    let populator =
        page.get_control::<AutoCompleteBox>("AsyncBox").async_populator().expect("the populator of the page");
    let mut context = Context::from_waker(Waker::noop());

    // The population waits for its delay.
    let source = CancellationTokenSource::new();
    let mut population = populator.invoke(Some("mont".to_string()), source.token());
    assert!(population.as_mut().poll(&mut context).is_pending());
    source.cancel();
    assert!(matches!(population.as_mut().poll(&mut context), Poll::Ready(Err(OperationCanceledError))));

    // A population that is cancelled before it starts does not wait.
    let mut population = populator.invoke(Some("mont".to_string()), source.token());
    assert!(matches!(population.as_mut().poll(&mut context), Poll::Ready(Err(OperationCanceledError))));
}
