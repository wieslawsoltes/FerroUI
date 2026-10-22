//! Direct native control examples from the pinned Gallery page code behind.

use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{BoxedValue, ObjectType, Ref};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    AutoSuggestBox, AutoSuggestionBoxTextChangeReason, Button, ComboBox, Control,
    HyperlinkButton, Image, InfoBar, InfoBarSeverity, ItemsSource, TextBlock,
};
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) {
    wire_cats(root);
    wire_control_search(root);
    wire_info_bars(root);
}

/// The source matches every query word in a title and sorts prefix matches
/// before the other titles. Unicode lowercase is explicit here; matching the
/// host's CurrentCulture comparison remains separate acceptance work.
pub(crate) fn controls_for_query(query: &str) -> Vec<&'static crate::CatalogEntry> {
    let query = query.to_lowercase();
    let tokens: Vec<_> = query.split(' ').collect();
    let mut found: Vec<_> = crate::CATALOG
        .iter()
        .filter(|entry| {
            let title = entry.title.to_lowercase();
            tokens.iter().all(|token| title.contains(token))
        })
        .collect();
    found.sort_by(|left, right| {
        let left_title = left.title.to_lowercase();
        let right_title = right.title.to_lowercase();
        right_title
            .starts_with(&query)
            .cmp(&left_title.starts_with(&query))
            .then_with(|| left_title.cmp(&right_title))
            .then_with(|| left.id.cmp(right.id))
    });
    found
}

fn control_from_item(item: &BoxedValue) -> Option<&'static crate::CatalogEntry> {
    let title = item.downcast_ref::<String>()?;
    crate::CATALOG
        .iter()
        .find(|entry| entry.title == title.as_str())
}

fn show_control(root: &Ref<Control>, entry: &crate::CatalogEntry) {
    root.get_control::<Control>("ControlDetails")
        .set_is_visible(true);
    root.get_control::<TextBlock>("ControlTitle")
        .set_text(Some(entry.title));
    root.get_control::<TextBlock>("ControlSubtitle")
        .set_text(Some(entry.subtitle));
    let image = root.get_control::<Image>("ControlImage");
    let error = root.get_control::<TextBlock>("ControlImageError");
    match crate::assets::load_bitmap(entry.image_uri) {
        Ok(bitmap) => {
            image.set_source(Some(Rc::new(bitmap)));
            error.set_text(None);
            error.set_is_visible(false);
        }
        Err(failure) => {
            image.set_source(None);
            error.set_text(Some(&format!(
                "Control image could not be decoded: {failure}"
            )));
            error.set_is_visible(true);
        }
    }
}

fn wire_control_search(root: &Ref<Control>) {
    let Some(suggestions) = find::<AutoSuggestBox>(root, "ControlSuggestions") else {
        return;
    };
    let weak_suggestions = suggestions.downgrade();
    suggestions.text_changed(move |args| {
        if args.reason() != AutoSuggestionBoxTextChangeReason::UserInput {
            return;
        }
        if let Some(suggestions) = weak_suggestions.upgrade() {
            let found = controls_for_query(&suggestions.text());
            let titles = if found.is_empty() {
                vec!["No results found"]
            } else {
                found.iter().map(|entry| entry.title).collect()
            };
            suggestions.set_items_source(Some(ItemsSource::from_items(
                titles
                    .into_iter()
                    .map(|title| Some(Rc::new(title.to_string()) as BoxedValue)),
            )));
        }
    });
    let weak_suggestions = suggestions.downgrade();
    suggestions.suggestion_chosen(move |args| {
        if let (Some(suggestions), Some(entry)) = (
            weak_suggestions.upgrade(),
            control_from_item(&args.selected_item),
        ) {
            suggestions.set_text(entry.title);
        }
    });
    let weak_root = root.downgrade();
    suggestions.query_submitted(move |args| {
        let chosen = args.chosen_suggestion.as_ref().and_then(control_from_item);
        let entry = chosen.or_else(|| {
            if args.query_text.is_empty() {
                None
            } else {
                controls_for_query(&args.query_text).first().copied()
            }
        });
        if let (Some(root), Some(entry)) = (weak_root.upgrade(), entry) {
            show_control(&root, entry);
        }
    });
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

/// The original basic AutoSuggestBox uses every space-delimited token. Empty
/// tokens match every item. No result is represented by its original string.
pub(crate) fn cats_for_query(query: &str) -> Vec<&'static str> {
    let query = query.to_lowercase();
    let tokens: Vec<_> = query.split(' ').collect();
    let found: Vec<_> = crate::cat_data::CATS
        .iter()
        .copied()
        .filter(|cat| {
            let cat = cat.to_lowercase();
            tokens.iter().all(|token| cat.contains(token))
        })
        .collect();
    if found.is_empty() {
        vec!["No results found"]
    } else {
        found
    }
}

fn wire_cats(root: &Ref<Control>) {
    let Some(suggestions) = find::<AutoSuggestBox>(root, "CatSuggestions") else {
        return;
    };
    let weak_suggestions = suggestions.downgrade();
    suggestions.text_changed(move |args| {
        if args.reason() != AutoSuggestionBoxTextChangeReason::UserInput {
            return;
        }
        if let Some(suggestions) = weak_suggestions.upgrade() {
            suggestions.set_items_source(Some(ItemsSource::from_items(
                cats_for_query(&suggestions.text())
                    .into_iter()
                    .map(|cat| Some(Rc::new(cat.to_string()) as BoxedValue)),
            )));
        }
    });
    let weak_output = root
        .get_control::<TextBlock>("SuggestionOutput")
        .downgrade();
    suggestions.suggestion_chosen(move |args| {
        if let (Some(output), Some(cat)) = (
            weak_output.upgrade(),
            args.selected_item.downcast_ref::<String>(),
        ) {
            output.set_text(Some(cat));
        }
    });
}

const LONG_MESSAGE: &str = "A long essential app message for your users to be informed of, acknowledge, or take action on. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Proin dapibus dolor vitae justo rutrum, ut lobortis nibh mattis. Aenean id elit commodo, semper felis nec.";

fn wire_info_bars(root: &Ref<Control>) {
    let Some(first) = find::<InfoBar>(root, "TestInfoBar1") else {
        return;
    };
    let severity = root.get_control::<ComboBox>("SeverityComboBox");
    let weak_first = first.downgrade();
    let weak_severity = severity.downgrade();
    severity.property_changed(move |change| {
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            if let (Some(first), Some(severity)) = (weak_first.upgrade(), weak_severity.upgrade()) {
                first.set_severity(match severity.selected_index() {
                    1 => InfoBarSeverity::Success,
                    2 => InfoBarSeverity::Warning,
                    3 => InfoBarSeverity::Error,
                    _ => InfoBarSeverity::Informational,
                });
            }
        }
    });
    let second = root.get_control::<InfoBar>("TestInfoBar2");
    second.set_message(LONG_MESSAGE);
    let message = root.get_control::<ComboBox>("MessageComboBox");
    let weak_second = second.downgrade();
    let weak_message = message.downgrade();
    message.property_changed(move |change| {
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            if let (Some(second), Some(message)) = (weak_second.upgrade(), weak_message.upgrade()) {
                second.set_message(if message.selected_index() == 0 {
                    "A short essential app message."
                } else {
                    LONG_MESSAGE
                });
            }
        }
    });
    let action = root.get_control::<ComboBox>("ActionButtonComboBox");
    let weak_second = second.downgrade();
    let weak_action = action.downgrade();
    action.property_changed(move |change| {
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            if let (Some(second), Some(action)) = (weak_second.upgrade(), weak_action.upgrade()) {
                let button = match action.selected_index() {
                    1 => {
                        let button = Button::new();
                        button.set_content(Some(Rc::new("Action".to_string())));
                        Some(button)
                    }
                    2 => {
                        let link = HyperlinkButton::new();
                        link.set_content(Some(Rc::new("Informational link".to_string())));
                        link.set_navigate_uri(Some(
                            Uri::new("http://www.microsoft.com/", UriKind::Absolute)
                                .expect("source URI"),
                        ));
                        Some(link.upcast::<Button>())
                    }
                    _ => None,
                };
                second.set_action_button(button);
            }
        }
    });
}
