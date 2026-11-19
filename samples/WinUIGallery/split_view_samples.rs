//! The code behind of the pinned SplitView page in Rust: `SplitViewPage.xaml.cs` with its model
//! `NavLink`.
//!
//! Adaptations: FerroUI has no ListView, so the links are a ListBox, whose `ItemClick` is a tap on
//! an item; the page's visual states are classes: `Red`, `Blue` and `Green` of the split view (the
//! PaneBackground states) and `RightIconLayout` of the list (the NavLinkItemLayoutStates, whose
//! LeftIconLayout is the list without the class), set by the handlers as `GoToState`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, TappedEventArgs};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, Visual};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::{
    ComboBox, Control, ItemsSource, ListBox, SplitView, SplitViewDisplayMode, SplitViewPanePlacement, Symbol, TextBlock,
};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "SplitView" {
        register();
        wire_split_view(root);
    }
}

/// `NavLink`: a link of the pane, with its label and symbol.
pub(crate) struct NavLink {
    pub(crate) label: &'static str,
    pub(crate) symbol: Symbol,
}

impl PartialEq for NavLink {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class NavLink {
    this: Rc<NavLink>,
    handles: [NavLink, Rc<NavLink>, Option<Rc<NavLink>>],
    properties: [
        Label: String { get: |this: &Rc<NavLink>| this.label.to_string() },
        Symbol: Symbol { get: |this: &Rc<NavLink>| this.symbol },
    ],
});

/// The markup type of the page, for the `{Binding}` paths of its templates.
fn register() {
    MarkupType::register_all(&[<NavLink as MarkupTyped>::MARKUP]);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_reference::<NavLink>();
}

/// `_navLinks`: People, Globe, Message and Mail.
pub(crate) fn nav_links() -> Vec<Rc<NavLink>> {
    [
        ("People", Symbol::People),
        ("Globe", Symbol::Globe),
        ("Message", Symbol::Message),
        ("Mail", Symbol::Mail),
    ]
    .into_iter()
    .map(|(label, symbol)| Rc::new(NavLink { label, symbol }))
    .collect()
}

/// `NavLinksList_ItemClick`: the content names the page of the clicked link.
pub(crate) fn nav_links_list_item_click(root: &Ref<Control>, item: &BoxedValue) {
    if let Some(link) = item.downcast_ref::<Rc<NavLink>>() {
        root.get_control::<TextBlock>("content")
            .set_text(Some(&format!("{} Page", link.label)));
    }
}

/// `UpdateNavLinkItemLayout`: the icons on the side of the pane's placement.
fn update_nav_link_item_layout(root: &Ref<Control>) {
    let right = root.get_control::<SplitView>("splitView").pane_placement() == SplitViewPanePlacement::Right;
    root.get_control::<ListBox>("NavLinksList")
        .classes()
        .set("RightIconLayout", right);
}

/// The display modes of the DisplayMode box, in its order (the source parses the item's text).
pub(crate) const DISPLAY_MODES: [SplitViewDisplayMode; 4] = [
    SplitViewDisplayMode::Inline,
    SplitViewDisplayMode::CompactInline,
    SplitViewDisplayMode::Overlay,
    SplitViewDisplayMode::CompactOverlay,
];

/// The PaneBackground states after the first, the theme brush of the markup.
const PANE_BACKGROUNDS: [&str; 3] = ["Red", "Blue", "Green"];

/// Calls `handler` with the selected index whenever the selection of `combo_box` changes.
fn on_selected_index(combo_box: &Ref<ComboBox>, root: &Ref<Control>, handler: fn(&Ref<Control>, i32)) {
    let weak_combo_box = combo_box.downgrade();
    let weak_root = root.downgrade();
    combo_box.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let (Some(combo_box), Some(root)) = (weak_combo_box.upgrade(), weak_root.upgrade()) {
            handler(&root, combo_box.selected_index());
        }
    });
}

/// Calls `handler` whenever `toggle` is checked or unchecked.
fn on_checked_changed(toggle: &Ref<ToggleButton>, root: &Ref<Control>, handler: fn(&Ref<Control>, bool)) {
    let weak_toggle = toggle.downgrade();
    let weak_root = root.downgrade();
    toggle.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        if let (Some(toggle), Some(root)) = (weak_toggle.upgrade(), weak_root.upgrade()) {
            handler(&root, toggle.is_checked() == Some(true));
        }
    });
}

fn wire_split_view(root: &Ref<Control>) {
    let list = root.get_control::<ListBox>("NavLinksList");
    list.set_items_source(Some(ItemsSource::from_items(
        nav_links().into_iter().map(|link| Some(Rc::new(link) as BoxedValue)),
    )));
    // `NavLinksList_ItemClick`, raised by a tap on an item.
    let weak_list = list.downgrade();
    let weak_root = root.downgrade();
    list.add_handler(InputElement::tapped_event(), move |_, e: &TappedEventArgs| {
        let (Some(list), Some(root)) = (weak_list.upgrade(), weak_root.upgrade()) else {
            return;
        };
        let source = e.source().and_then(|source| source.cast::<Visual>());
        if let Some(item) = list
            .get_container_from_event_source(source.as_ref())
            .and_then(|container| list.item_from_container(&container))
        {
            nav_links_list_item_click(&root, &item);
        }
    });
    // `SplitViewPage_Loaded` (the page is created with its layout), `togglePaneButton_CheckedChanged`
    // and `PanePlacement_Toggled`.
    update_nav_link_item_layout(root);
    on_checked_changed(
        &root.get_control::<ToggleButton>("togglePaneButton"),
        root,
        |root, _| {
            update_nav_link_item_layout(root);
        },
    );
    on_checked_changed(&root.get_control::<ToggleButton>("PanePlacement"), root, |root, on| {
        root.get_control::<SplitView>("splitView").set_pane_placement(if on {
            SplitViewPanePlacement::Right
        } else {
            SplitViewPanePlacement::Left
        });
        update_nav_link_item_layout(root);
    });
    // `displayModeCombobox_SelectionChanged`.
    on_selected_index(
        &root.get_control::<ComboBox>("displayModeCombobox"),
        root,
        |root, index| {
            if let Some(mode) = usize::try_from(index).ok().and_then(|index| DISPLAY_MODES.get(index)) {
                root.get_control::<SplitView>("splitView").set_display_mode(*mode);
            }
        },
    );
    // `paneBackgroundCombobox_SelectionChanged`: the state of the selected name.
    on_selected_index(
        &root.get_control::<ComboBox>("paneBackgroundCombobox"),
        root,
        |root, index| {
            let classes = root.get_control::<SplitView>("splitView").classes();
            for (state, name) in PANE_BACKGROUNDS.iter().enumerate() {
                classes.set(name, index == state as i32 + 1);
            }
        },
    );
}
