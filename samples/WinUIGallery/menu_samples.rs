//! The code behind of the pinned Flyout, MenuFlyout and MenuBar pages in Rust: the handlers of
//! `FlyoutPage.xaml.cs`, `MenuFlyoutPage.xaml.cs` and `MenuBarPage.xaml.cs`. WinUI's
//! MenuFlyoutItem is FerroUI's MenuItem, whose Header is the item's Text. The Popup handlers are
//! in `platform_samples.rs`. The flyouts and menus own their opening and input; the sample owns
//! only its outputs.

use ferroui_base::Ref;
use ferroui_controls::{Button, Control, MenuItem, TextBlock};

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "Flyout" => wire_flyout(root),
        "MenuFlyout" => wire_menu_flyout(root),
        "MenuBar" => wire_menu_bar(root),
        _ => {}
    }
}

/// The text of a menu item: its Header (WinUI's `MenuFlyoutItem.Text`).
pub(crate) fn item_text(item: &MenuItem) -> String {
    item.header()
        .and_then(|header| header.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

/// Calls `handler` with the item when one of the named items is clicked.
fn on_item_click(
    root: &Ref<Control>,
    names: &[&str],
    output: &str,
    handler: fn(&MenuItem) -> String,
) {
    let output = root.get_control::<TextBlock>(output);
    for name in names {
        let item = root.get_control::<MenuItem>(name);
        let weak_item = item.downgrade();
        let weak_output = output.downgrade();
        item.click(move |_, e| {
            if let (Some(item), Some(output)) = (weak_item.upgrade(), weak_output.upgrade()) {
                // The click of a sub item bubbles through a split item: it is the sub item's.
                let own = e.source().and_then(|source| source.cast::<MenuItem>()).is_none_or(|source| source == item);
                if own {
                    output.set_text(Some(&handler(&item)));
                }
            }
        });
    }
}

/// `DeleteConfirmation_Click` of `FlyoutPage.xaml.cs`: the confirmation hides the button's flyout.
fn wire_flyout(root: &Ref<Control>) {
    let weak_button = root.get_control::<Button>("Control1").downgrade();
    root.get_control::<Button>("DeleteConfirmation")
        .click(move |_, _| {
            if let Some(flyout) = weak_button.upgrade().and_then(|button| button.flyout()) {
                flyout.hide();
            }
        });
}

/// `MenuFlyoutItem_Click` and `SplitMenuFlyoutItem_Click` of `MenuFlyoutPage.xaml.cs`: a sort item
/// writes its Tag ("Sort by: rating"), and an item of the Save and Share menus its text
/// ("Clicked: Save as .pdf").
fn wire_menu_flyout(root: &Ref<Control>) {
    on_item_click(
        root,
        &["SortByRating", "SortByMatch", "SortByDistance"],
        "Control1Output",
        |item| {
            let tag = item.tag();
            let sort_option = tag
                .as_ref()
                .and_then(|tag| tag.downcast_ref::<String>())
                .map(String::as_str)
                .unwrap_or_default();
            format!("Sort by: {sort_option}")
        },
    );
    on_item_click(
        root,
        &[
            "SaveSplitItem",
            "SaveAsDocx",
            "SaveAsPdf",
            "SaveAsTxt",
            "ShareSplitItem",
            "ShareViaEmail",
            "ShareViaLink",
        ],
        "Control3bOutput",
        |item| format!("Clicked: {}", item_text(item)),
    );
}

/// The items of the three menu bars: the source's `OnElementClicked` picks the output by the first
/// letter of the item's name (`o`, `t`, `z`).
pub(crate) const MENU_BAR_ITEMS: [(&str, &[&str]); 3] = [
    (
        "SelectedOptionText",
        &["o1", "o2", "o3", "o4", "o5", "o6", "o7", "o8", "o9"],
    ),
    (
        "SelectedOptionText1",
        &["t1", "t2", "t3", "t4", "t5", "t6", "t7", "t8", "t9"],
    ),
    (
        "SelectedOptionText2",
        &[
            "z1", "z2", "z3", "z4", "z5", "z6", "z7", "z8", "z9", "z11", "z12", "z13", "z14",
            "z15", "z16", "z17", "z18",
        ],
    ),
];

/// `OnElementClicked` of `MenuBarPage.xaml.cs`: the clicked item's text goes to the output of its
/// menu bar ("You clicked: Open").
fn wire_menu_bar(root: &Ref<Control>) {
    for (output, names) in MENU_BAR_ITEMS {
        on_item_click(root, names, output, |item| {
            format!("You clicked: {}", item_text(item))
        });
    }
}
