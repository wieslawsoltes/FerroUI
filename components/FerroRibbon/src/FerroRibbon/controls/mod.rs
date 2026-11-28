mod iribbon_item;
mod xmlns_definitions;
pub use xmlns_definitions::RibbonXmlns;
mod ribbon_key_tip;
pub use iribbon_item::{
    IRibbonItem, IRibbonItemEditable, RibbonItemLayout, RibbonKeyTipResult, RibbonKeyTipTargets,
    RibbonOverflowMenuItems,
};
pub use ribbon_key_tip::RibbonKeyTip;
mod ribbon_item_helper;
#[cfg(test)]
pub(crate) use ribbon_item_helper::IRibbonItemInternal;
#[cfg(test)]
pub(crate) use ribbon_item_helper::OwnedItemValues;
pub(crate) use ribbon_item_helper::register_contract_values;
pub use ribbon_item_helper::{
    IRibbonCommandSource, IRibbonLayoutHost, RibbonItemHelper, RibbonItemHelperError,
    RibbonSimplifiedLabel,
};

mod ribbon_control_base;
pub use ribbon_control_base::{RibbonControlBase, RibbonControlBaseImpl, RibbonControlBaseImplExt};

mod ribbon_screen_tip;
pub use ribbon_screen_tip::{RibbonScreenTip, RibbonScreenTipError, RibbonScreenTipService};

mod ribbon_simple_items;
pub use ribbon_simple_items::{
    RibbonButtonGroup, RibbonItemsContainer, RibbonItemsContainerImpl, RibbonItemsContainerImplExt,
    RibbonLabel, RibbonSeparator, RibbonStackPanel,
};

mod iribbon_item_owner;
pub use iribbon_item_owner::IRibbonItemOwner;

mod ribbon_button;
pub use ribbon_button::{
    RibbonButton, RibbonButtonImpl, RibbonButtonImplExt, RibbonDropDownButton,
};

mod iribbon_checkable;
pub use iribbon_checkable::IRibbonCheckable;

mod ribbon_group;
pub use ribbon_group::RibbonGroup;
mod ribbon_group_panels;
pub use ribbon_group_panels::RibbonPanelPresentation;

mod ribbon_tab;
mod ribbon_tab_overflow;
pub use ribbon_tab::{RibbonContextualTabGroup, RibbonTab, RibbonTabHeader};
pub(crate) use ribbon_tab_overflow::register_display_groups;
pub use ribbon_tab_overflow::{RibbonDisplayGroups, RibbonDisplayGroupsMetadata};
mod ribbon_handled_event_args;
pub use ribbon_handled_event_args::RibbonHandledEventArgs;
mod ribbon;
#[cfg(test)]
#[path = "../tab_overflow_tests.rs"]
mod tab_overflow_tests;
pub use ribbon::{
    Ribbon, RibbonContextMenuError, RibbonContextMenuEventArgs, RibbonCustomizeRequestedEventArgs,
    RibbonItemInvokedEventArgs, RibbonTabChangedEventArgs,
};

mod ribbon_checked_events;
mod ribbon_toggle_button;
mod ribbon_toggle_groups;
pub use ribbon_toggle_button::{
    RibbonToggleButton, RibbonToggleButtonImpl, RibbonToggleButtonImplExt,
};
mod ribbon_check_box;
pub use ribbon_check_box::{RibbonCheckBox, RibbonCheckBoxImpl, RibbonCheckBoxImplExt};

mod ribbon_menu;
pub(crate) use ribbon_menu::action_delegate;
pub use ribbon_menu::{RibbonMenu, RibbonMenuError};

mod ribbon_menu_cloner;
pub use ribbon_menu_cloner::RibbonMenuCloner;
mod ribbon_split_button;
pub use ribbon_split_button::{RibbonSplitButton, RibbonSplitButtonImpl, RibbonSplitButtonImplExt};
mod ribbon_split_button_automation_peer;
pub use ribbon_split_button_automation_peer::RibbonSplitButtonAutomationPeer;
#[cfg(test)]
#[path = "../split_button_tests.rs"]
mod split_button_tests;

mod ribbon_group_automation_peer;
pub use ribbon_group_automation_peer::RibbonGroupAutomationPeer;
mod ribbon_tab_header_automation_peer;
pub use ribbon_tab_header_automation_peer::RibbonTabHeaderAutomationPeer;
mod ribbon_automation_peer;
pub use ribbon_automation_peer::RibbonAutomationPeer;

mod ribbon_quick_access_tool_bar;
pub use ribbon_quick_access_tool_bar::{RibbonQuickAccessToolBar, RibbonToolBarAutomationPeer};
mod ribbon_input_base;
pub use ribbon_input_base::{RibbonInputBase, RibbonInputBaseImpl, RibbonInputBaseImplExt};
mod ribbon_text_box;
pub use ribbon_text_box::RibbonTextBox;

mod ribbon_combo_box;
pub use ribbon_combo_box::{
    RibbonComboBox, RibbonComboBoxCommittedEventArgs, RibbonComboBoxImpl, RibbonComboBoxImplExt,
    RibbonObjectArray, RibbonObjectItems, RibbonObjectItemsExt,
};
mod ribbon_font_combo_boxes;
pub use ribbon_font_combo_boxes::{RibbonFontComboBox, RibbonFontSizeComboBox};

mod ribbon_spinner;
pub use ribbon_spinner::{RibbonSpinner, RibbonValueChangedEventArgs};

mod ribbon_slider;
pub use ribbon_slider::RibbonSlider;

mod ribbon_search;
pub use ribbon_search::{RibbonAdditionalSearchEntries, RibbonSearch, RibbonSearchEntries};

mod ribbon_backstage;
pub use ribbon_backstage::{RibbonBackstage, RibbonBackstageItem};

mod ribbon_grid_picker;
pub use ribbon_grid_picker::RibbonGridPicker;

mod ribbon_segmented_control;
pub use ribbon_segmented_control::{RibbonSegment, RibbonSegmentedControl, RibbonSegments};

mod ribbon_color_palette;
mod ribbon_gallery;
pub use ribbon_color_palette::{RibbonColorNames, RibbonColorPalette, RibbonColors};
pub use ribbon_gallery::{RibbonGallery, RibbonGalleryItem, RibbonGalleryItemEventArgs};

mod ribbon_color_dialog;
mod ribbon_color_dialog_content;
pub(crate) use ribbon_color_dialog::continue_color_dialog;
pub use ribbon_color_dialog::{RibbonColorDialog, RibbonColorDialogError, RibbonColorDialogFuture};
mod ribbon_color_picker;
pub use ribbon_color_picker::RibbonColorPicker;

mod ribbon_dialog_host;
mod ribbon_dialog_session;
pub use ribbon_dialog_host::RibbonDialogHost;
mod ribbon_color_dialog_host;
pub use ribbon_color_dialog_host::RibbonColorDialogHost;

mod ribbon_tool_bar;
pub use ribbon_tool_bar::{
    RibbonContextualToolBar, RibbonMenuBar, RibbonMenuBarItem, RibbonMenuBarItems, RibbonToolBar,
};

mod ribbon_command_palette;
mod ribbon_search_box;
mod ribbon_search_query;
mod ribbon_search_results;
pub use ribbon_command_palette::RibbonCommandPalette;
pub use ribbon_search_box::RibbonSearchBox;
pub(crate) use ribbon_search_query::register as register_search_ui;
pub use ribbon_search_query::{RibbonQuerySearchEntries, RibbonSearchQueryEventArgs};

mod ribbon_zoom_control;
pub use ribbon_zoom_control::RibbonZoomControl;

mod ribbon_status_bar;
pub use ribbon_status_bar::RibbonStatusBar;

mod ribbon_title_bar;
pub use ribbon_title_bar::{RibbonTitleBar, RibbonTitleBarWindowError, RibbonTitleBarWindowState};

mod ribbon_application_menu_items;
pub use ribbon_application_menu_items::{
    RibbonApplicationMenuItem, RibbonApplicationMenuItemEventArgs, RibbonApplicationMenuItems,
    RibbonApplicationMenuRecentItem, RibbonApplicationMenuRecentItems,
};

mod ribbon_application_menu;
pub use ribbon_application_menu::{RibbonApplicationMenu, RibbonApplicationMenuFooterItems};

mod ribbon_customization_list;

mod ribbon_customize_dialog;
pub use ribbon_customize_dialog::{
    RibbonCustomizeDialog, RibbonCustomizeDialogError, RibbonCustomizePage,
};

mod ribbon_dialog_outcome;
pub use ribbon_dialog_outcome::RibbonDialogOutcome;

mod ribbon_dialog_buttons;
pub use ribbon_dialog_buttons::{RibbonDialogButtonClickEventArgs, RibbonDialogButtonDeferral};

mod ribbon_dialog_lifecycle;
pub use ribbon_dialog_lifecycle::{
    RibbonDialogClosedEventArgs, RibbonDialogClosingDeferral, RibbonDialogClosingEventArgs,
    RibbonDialogDeferralError, RibbonDialogOpenedEventArgs,
};

mod ribbon_dialog_future;
pub use ribbon_dialog_future::RibbonDialogFuture;
pub use ribbon_dialog_session::RibbonDialogOpenError;

mod ribbon_command_enabled;
