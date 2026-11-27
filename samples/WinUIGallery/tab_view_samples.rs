//! The code behind of the pinned TabView page in Rust (`TabViewPage.xaml.cs`): `TabView_Loaded`,
//! `TabView_AddButtonClick`, `TabView_TabCloseRequested` and `CreateNewTab` for the first and the
//! keyboard example, the close requests of the markup example, and the keyboard accelerators of
//! the keyboard example (`NewTabKeyboardAccelerator_Invoked`,
//! `CloseSelectedTabKeyboardAccelerator_Invoked`, `NavigateToNumberedTabKeyboardAccelerator_Invoked`)
//! as a key handler of the tab view.

use crate::navigation_view_samples::{page_type, register_pages};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::{BoxedValue, ObjectType, Ref};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    ComboBox, ComboBoxItem, Control, Frame, SelectionChangedEventArgs, Symbol, SymbolIconSource, TabView,
    TabViewCloseButtonOverlayMode, TabViewItem, TabViewWidthMode,
};
use std::rc::Rc;

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name).and_then(|control| control.cast())
}

/// `CreateNewTab`: a tab "Document {index}" with the Document symbol, whose content is a frame on
/// one of the three sample pages.
pub(crate) fn create_new_tab(index: usize) -> Ref<TabViewItem> {
    let tab = TabViewItem::new();
    tab.set_header(Some(Rc::new(format!("Document {index}")) as BoxedValue));
    let icon = SymbolIconSource::new();
    icon.set_symbol(Symbol::Document);
    tab.set_icon_source(Some(icon.upcast()));
    let frame = Frame::new();
    if register_pages(&frame).is_ok() {
        if let Some(page) = page_type(["SamplePage1", "SamplePage2", "SamplePage3"][index % 3]) {
            let _ = frame.navigate(page, None);
        }
    }
    tab.set_content(Some(Control::boxed(frame)));
    tab
}

/// `TabItems.Remove(tab)`: the item that is the tab.
fn remove_tab(view: &TabView, tab: &Ref<TabViewItem>) {
    let items = view.tab_items();
    let index = (0..items.count()).find(|&index| {
        items.get_at(index).and_then(|item| Control::from_boxed(&item)).is_some_and(|item| item.ptr_eq(tab))
    });
    if let Some(index) = index {
        items.remove_at(index);
    }
}

/// The adding and closing of tabs that the examples share (SharedTabViewLogic).
fn wire_adding_and_closing(view: &Ref<TabView>, add_initial_tabs: bool) {
    if add_initial_tabs {
        // TabView_Loaded: three tabs.
        for index in 0..3 {
            view.tab_items().add(Some(Control::boxed(create_new_tab(index))));
        }
    }
    view.add_tab_button_click(|sender| {
        let count = sender.tab_items().count();
        sender.tab_items().add(Some(Control::boxed(create_new_tab(count))));
    })
    .detach();
    view.tab_close_requested(|sender, args| {
        remove_tab(sender, &args.tab());
    })
    .detach();
}

/// The keyboard accelerators of the keyboard example: Ctrl+T adds a tab, Ctrl+W closes the
/// selected one, Ctrl+1 to Ctrl+8 select that tab and Ctrl+9 the last.
pub(crate) fn handle_accelerator(view: &Ref<TabView>, key: Key, modifiers: KeyModifiers) -> bool {
    if modifiers != KeyModifiers::CONTROL {
        return false;
    }
    let count = view.tab_items().count();
    match key {
        Key::T => {
            view.tab_items().add(Some(Control::boxed(create_new_tab(count))));
            true
        }
        Key::W => {
            // Only remove the selected tab if it can be closed.
            let selected = view.selected_index();
            let tab = usize::try_from(selected)
                .ok()
                .and_then(|index| view.tab_items().get_at(index))
                .and_then(|item| Control::from_boxed(&item))
                .and_then(|control| control.cast::<TabViewItem>());
            if let Some(tab) = tab.filter(|tab| tab.is_closable()) {
                remove_tab(view, &tab);
            }
            true
        }
        Key::D1 | Key::D2 | Key::D3 | Key::D4 | Key::D5 | Key::D6 | Key::D7 | Key::D8 => {
            let index = key as i32 - Key::D1 as i32;
            // Only select the tab if it is in the list.
            if (index as usize) < count {
                view.set_selected_index(index);
            }
            true
        }
        Key::D9 => {
            view.set_selected_index(count as i32 - 1);
            true
        }
        _ => false,
    }
}

/// `TabWidthBehaviorComboBox_SelectionChanged`: the width mode the selected item names.
pub(crate) fn width_mode(name: &str) -> TabViewWidthMode {
    match name {
        "SizeToContent" => TabViewWidthMode::SizeToContent,
        "Compact" => TabViewWidthMode::Compact,
        _ => TabViewWidthMode::Equal,
    }
}

/// `TabCloseButtonOverlayModeComboBox_SelectionChanged`: the overlay mode the selected item names.
pub(crate) fn overlay_mode(name: &str) -> TabViewCloseButtonOverlayMode {
    match name {
        "OnHover" => TabViewCloseButtonOverlayMode::OnPointerOver,
        "Always" => TabViewCloseButtonOverlayMode::Always,
        _ => TabViewCloseButtonOverlayMode::Auto,
    }
}

/// The text of the selected item of an option box.
fn selected_text(combo: &ComboBox) -> Option<String> {
    let item = combo.selected_item()?;
    let item = Control::from_boxed(&item)?.cast::<ComboBoxItem>()?;
    item.content().and_then(|content| content.downcast_ref::<String>().cloned())
}

/// An option box that sets a property of a tab view from the text of its selected item.
fn wire_option(root: &Ref<Control>, combo: &str, view: &str, apply: impl Fn(&TabView, &str) + 'static) {
    let (Some(combo), Some(view)) = (find::<ComboBox>(root, combo), find::<TabView>(root, view)) else {
        return;
    };
    let (weak_view, weak_combo) = (view.downgrade(), combo.downgrade());
    combo.add_handler(SelectingItemsControl::selection_changed_event(), move |_, _: &SelectionChangedEventArgs| {
        let (Some(view), Some(combo)) = (weak_view.upgrade(), weak_combo.upgrade()) else { return };
        if let Some(text) = selected_text(&combo) {
            apply(&view, &text);
        }
    });
}

pub(crate) fn wire(root: &Ref<Control>) {
    for name in ["TabView1", "TabViewCustomContent", "TabViewAccent"] {
        if let Some(view) = find::<TabView>(root, name) {
            wire_adding_and_closing(&view, true);
        }
    }
    wire_option(root, "TabWidthBehaviorComboBox", "TabView3", |view, text| view.set_tab_width_mode(width_mode(text)));
    wire_option(root, "TabCloseButtonOverlayModeComboBox", "TabView4", |view, text| {
        view.set_close_button_overlay_mode(overlay_mode(text))
    });
    if let Some(view) = find::<TabView>(root, "TabViewMarkup") {
        wire_adding_and_closing(&view, false);
    }
    if let Some(view) = find::<TabView>(root, "TabView2") {
        wire_adding_and_closing(&view, true);
        let weak = view.downgrade();
        view.add_handler(InputElement::key_down_event(), move |_, args: &KeyEventArgs| {
            if args.handled() {
                return;
            }
            if let Some(view) = weak.upgrade() {
                if handle_accelerator(&view, args.key, args.key_modifiers) {
                    args.set_handled(true);
                }
            }
        });
    }
}
