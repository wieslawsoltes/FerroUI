// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Public navigation contracts adapted from the pinned NavigationView.idl.

use super::NavigationViewItemBase;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{BoxedValue, Ref, ferro_markup_enum, ferro_markup_type};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NavigationViewDisplayMode {
    #[default]
    Minimal = 0,
    Compact = 1,
    Expanded = 2,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NavigationViewPaneDisplayMode {
    #[default]
    Auto = 0,
    Left = 1,
    Top = 2,
    LeftCompact = 3,
    LeftMinimal = 4,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NavigationViewBackButtonVisible {
    Collapsed = 0,
    Visible = 1,
    #[default]
    Auto = 2,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NavigationViewSelectionFollowsFocus {
    #[default]
    Disabled = 0,
    Enabled = 1,
}
ferro_markup_enum!(NavigationViewDisplayMode { Minimal, Compact, Expanded }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(NavigationViewPaneDisplayMode { Auto, Left, Top, LeftCompact, LeftMinimal }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(NavigationViewBackButtonVisible { Collapsed, Visible, Auto }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(NavigationViewSelectionFollowsFocus { Disabled, Enabled }, { namespace: "FerroUI.Controls" });

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationViewDisplayModeChangedEventArgs {
    pub(crate) display_mode: NavigationViewDisplayMode,
}
impl NavigationViewDisplayModeChangedEventArgs {
    pub fn display_mode(&self) -> NavigationViewDisplayMode {
        self.display_mode
    }
}

/// The cancel flag is shared by clones passed to native and markup handlers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NavigationViewPaneClosingEventArgs {
    cancel: Rc<Cell<bool>>,
}
impl NavigationViewPaneClosingEventArgs {
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NavigationViewBackRequestedEventArgs;

#[derive(Clone)]
pub struct NavigationViewSelectionChangedEventArgs {
    pub(crate) selected_item: Option<BoxedValue>,
    pub(crate) selected_item_container: Option<Ref<NavigationViewItemBase>>,
    pub(crate) is_settings_selected: bool,
    pub(crate) identity: Rc<()>,
}
impl PartialEq for NavigationViewSelectionChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
}
impl NavigationViewSelectionChangedEventArgs {
    pub fn selected_item(&self) -> Option<BoxedValue> {
        self.selected_item.clone()
    }
    pub fn selected_item_container(&self) -> Option<Ref<NavigationViewItemBase>> {
        self.selected_item_container.clone()
    }
    pub fn is_settings_selected(&self) -> bool {
        self.is_settings_selected
    }
}

#[derive(Clone)]
pub struct NavigationViewItemInvokedEventArgs {
    pub(crate) invoked_item: Option<BoxedValue>,
    pub(crate) invoked_item_container: Option<Ref<NavigationViewItemBase>>,
    pub(crate) is_settings_invoked: bool,
    pub(crate) identity: Rc<()>,
}
impl PartialEq for NavigationViewItemInvokedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
}
impl Default for NavigationViewItemInvokedEventArgs {
    fn default() -> Self {
        Self {
            invoked_item: None,
            invoked_item_container: None,
            is_settings_invoked: false,
            identity: Rc::new(()),
        }
    }
}
impl NavigationViewItemInvokedEventArgs {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn invoked_item(&self) -> Option<BoxedValue> {
        self.invoked_item.clone()
    }
    pub fn invoked_item_container(&self) -> Option<Ref<NavigationViewItemBase>> {
        self.invoked_item_container.clone()
    }
    pub fn is_settings_invoked(&self) -> bool {
        self.is_settings_invoked
    }
}

#[derive(Clone)]
pub struct NavigationViewItemExpandingEventArgs {
    pub(crate) expanding_item: Option<BoxedValue>,
    pub(crate) expanding_item_container: Ref<NavigationViewItemBase>,
    pub(crate) identity: Rc<()>,
}
impl PartialEq for NavigationViewItemExpandingEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
}
impl NavigationViewItemExpandingEventArgs {
    pub fn expanding_item(&self) -> Option<BoxedValue> {
        self.expanding_item.clone()
    }
    pub fn expanding_item_container(&self) -> Ref<NavigationViewItemBase> {
        self.expanding_item_container.clone()
    }
}

#[derive(Clone)]
pub struct NavigationViewItemCollapsedEventArgs {
    pub(crate) collapsed_item: Option<BoxedValue>,
    pub(crate) collapsed_item_container: Ref<NavigationViewItemBase>,
    pub(crate) identity: Rc<()>,
}
impl PartialEq for NavigationViewItemCollapsedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
}
impl NavigationViewItemCollapsedEventArgs {
    pub fn collapsed_item(&self) -> Option<BoxedValue> {
        self.collapsed_item.clone()
    }
    pub fn collapsed_item_container(&self) -> Ref<NavigationViewItemBase> {
        self.collapsed_item_container.clone()
    }
}

ferro_markup_type!(class NavigationViewDisplayModeChangedEventArgs {
    namespace: "FerroUI.Controls", handles: [NavigationViewDisplayModeChangedEventArgs],
    properties: [DisplayMode: NavigationViewDisplayMode { get: |a: &NavigationViewDisplayModeChangedEventArgs| a.display_mode() }],
});
ferro_markup_type!(class NavigationViewPaneClosingEventArgs {
    namespace: "FerroUI.Controls", handles: [NavigationViewPaneClosingEventArgs],
    properties: [Cancel: bool { get: |a: &NavigationViewPaneClosingEventArgs| a.cancel(), set: |a: &NavigationViewPaneClosingEventArgs, v: bool| a.set_cancel(v) }],
});
ferro_markup_type!(class NavigationViewBackRequestedEventArgs {
    namespace: "FerroUI.Controls", handles: [NavigationViewBackRequestedEventArgs], properties: [],
});
ferro_markup_type!(class NavigationViewSelectionChangedEventArgs {
    namespace: "FerroUI.Controls", handles: [NavigationViewSelectionChangedEventArgs],
    properties: [
        SelectedItem: Option<BoxedValue> { get: |a: &NavigationViewSelectionChangedEventArgs| a.selected_item() },
        SelectedItemContainer: Option<Ref<NavigationViewItemBase>> { get: |a: &NavigationViewSelectionChangedEventArgs| a.selected_item_container() },
        IsSettingsSelected: bool { get: |a: &NavigationViewSelectionChangedEventArgs| a.is_settings_selected() },
    ],
});
ferro_markup_type!(class NavigationViewItemInvokedEventArgs {
    namespace: "FerroUI.Controls", handles: [NavigationViewItemInvokedEventArgs],
    properties: [
        InvokedItem: Option<BoxedValue> { get: |a: &NavigationViewItemInvokedEventArgs| a.invoked_item() },
        InvokedItemContainer: Option<Ref<NavigationViewItemBase>> { get: |a: &NavigationViewItemInvokedEventArgs| a.invoked_item_container() },
        IsSettingsInvoked: bool { get: |a: &NavigationViewItemInvokedEventArgs| a.is_settings_invoked() },
    ],
});
ferro_markup_type!(class NavigationViewItemExpandingEventArgs {
    namespace: "FerroUI.Controls", handles: [NavigationViewItemExpandingEventArgs],
    properties: [
        ExpandingItem: Option<BoxedValue> { get: |a: &NavigationViewItemExpandingEventArgs| a.expanding_item() },
        ExpandingItemContainer: Ref<NavigationViewItemBase> { get: |a: &NavigationViewItemExpandingEventArgs| a.expanding_item_container() },
    ],
});
ferro_markup_type!(class NavigationViewItemCollapsedEventArgs {
    namespace: "FerroUI.Controls", handles: [NavigationViewItemCollapsedEventArgs],
    properties: [
        CollapsedItem: Option<BoxedValue> { get: |a: &NavigationViewItemCollapsedEventArgs| a.collapsed_item() },
        CollapsedItemContainer: Ref<NavigationViewItemBase> { get: |a: &NavigationViewItemCollapsedEventArgs| a.collapsed_item_container() },
    ],
});

pub(crate) const TYPES: &[&MarkupType] = &[
    <NavigationViewDisplayMode as MarkupTyped>::MARKUP,
    <NavigationViewPaneDisplayMode as MarkupTyped>::MARKUP,
    <NavigationViewBackButtonVisible as MarkupTyped>::MARKUP,
    <NavigationViewSelectionFollowsFocus as MarkupTyped>::MARKUP,
    <NavigationViewDisplayModeChangedEventArgs as MarkupTyped>::MARKUP,
    <NavigationViewPaneClosingEventArgs as MarkupTyped>::MARKUP,
    <NavigationViewBackRequestedEventArgs as MarkupTyped>::MARKUP,
    <NavigationViewSelectionChangedEventArgs as MarkupTyped>::MARKUP,
    <NavigationViewItemInvokedEventArgs as MarkupTyped>::MARKUP,
    <NavigationViewItemExpandingEventArgs as MarkupTyped>::MARKUP,
    <NavigationViewItemCollapsedEventArgs as MarkupTyped>::MARKUP,
];

pub(crate) fn register_markup_types() {
    MarkupType::register_all(TYPES);
}
