use crate::test_support::test_scope;
use crate::{Control, FontIcon, SelectionMode, SelectorBar, SelectorBarItem, StackPanel, TextBlock};
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::Ref;

fn bar(texts: &[&str]) -> (Ref<SelectorBar>, Vec<Ref<SelectorBarItem>>) {
    let bar = SelectorBar::new();
    let items: Vec<_> = texts
        .iter()
        .map(|text| {
            let item = SelectorBarItem::new();
            item.set_text(Some(text));
            bar.items().add(Some(Control::boxed(item.clone())));
            item
        })
        .collect();
    (bar, items)
}

fn parts(item: &SelectorBarItem) -> Ref<StackPanel> {
    item.content()
        .and_then(|content| Control::from_boxed(&content))
        .and_then(|control| control.cast::<StackPanel>())
        .expect("the panel of the item")
}

#[test]
fn the_bar_starts_without_a_selection() {
    let _scope = test_scope();
    let (bar, _) = bar(&["Recent", "Shared"]);
    assert_eq!(bar.selection_mode(), SelectionMode::SINGLE);
    assert!(bar.selected_item().is_none());
    assert_eq!(bar.selected_index(), -1);
}

#[test]
fn an_item_shows_its_text_and_icon_and_collapses_what_it_lacks() {
    let _scope = test_scope();
    let item = SelectorBarItem::new();
    item.set_text(Some("Recent"));
    let panel = parts(&item);
    let children = panel.children().snapshot();
    assert!(!children[0].is_visible());
    let text = children[1].clone().cast::<TextBlock>().unwrap();
    assert_eq!(text.text().as_deref(), Some("Recent"));

    let icon = FontIcon::new();
    item.set_icon(Some(icon.upcast()));
    assert!(children[0].is_visible());

    item.set_text(None);
    assert!(!children[1].is_visible());
    assert!(panel.is_visible());
    item.set_icon(None);
    assert!(!panel.is_visible());
}

#[test]
fn focus_without_a_selection_selects_the_first_item() {
    let _scope = test_scope();
    let (bar, _) = bar(&["Recent", "Shared"]);
    bar.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert_eq!(bar.selected_index(), 0);
}

#[test]
fn focus_keeps_an_existing_selection() {
    let _scope = test_scope();
    let (bar, _) = bar(&["Recent", "Shared"]);
    bar.set_selected_index(1);
    bar.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert_eq!(bar.selected_index(), 1);
}
