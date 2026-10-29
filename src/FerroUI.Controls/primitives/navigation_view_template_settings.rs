//! WinUI's `NavigationViewTemplateSettings` (`controls/dev/NavigationView/NavigationView.idl`,
//! `NavigationViewTemplateSettings.cpp` and the updates of `NavigationView.cpp` of microsoft-ui-xaml,
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// The values the template of a [`NavigationView`](crate::NavigationView) binds to
/// (`TemplateSettings`), which the navigation view sets. Each navigation view has its own.
///
/// WinUI's `Visibility` values are `bool`s here (`true` is `Visible`), as the `IsVisible` they are
/// bound to.
#[repr(C)]
pub struct NavigationViewTemplateSettings {
    base: FerroObject,
}

ferro_class!(NavigationViewTemplateSettings: FerroObject);
ferro_class_info!(NavigationViewTemplateSettings {
    new: NavigationViewTemplateSettings::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(NavigationViewTemplateSettings: FerroObjectImpl);

ferro_properties! { impl NavigationViewTemplateSettings {
    ferro_property!(
        /// Defines the `TopPadding` property: the height above the pane and the content (the title
        /// bar of WinUI's window when the content extends into it; 0 in FerroUI).
        pub fn top_padding_property() -> StyledProperty<f64> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("TopPadding", 0.0)
        }
    );
    ferro_property!(
        /// Defines the `OverflowButtonVisibility` property: whether the overflow button of the top pane
        /// is shown (always `false`: the overflow of the top pane is not ported).
        pub fn overflow_button_visibility_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("OverflowButtonVisibility", false)
        }
    );
    ferro_property!(
        /// Defines the `PaneToggleButtonVisibility` property: whether the pane toggle button is shown.
        pub fn pane_toggle_button_visibility_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("PaneToggleButtonVisibility", true)
        }
    );
    ferro_property!(
        /// Defines the `BackButtonVisibility` property: whether the back button is shown.
        pub fn back_button_visibility_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("BackButtonVisibility", false)
        }
    );
    ferro_property!(
        /// Defines the `TopPaneVisibility` property: whether the top pane is shown.
        pub fn top_pane_visibility_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("TopPaneVisibility", false)
        }
    );
    ferro_property!(
        /// Defines the `LeftPaneVisibility` property: whether the left pane is shown.
        pub fn left_pane_visibility_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("LeftPaneVisibility", true)
        }
    );
    ferro_property!(
        /// Defines the `SingleSelectionFollowsFocus` property: whether the selection follows the
        /// focus.
        pub fn single_selection_follows_focus_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("SingleSelectionFollowsFocus", false)
        }
    );
    ferro_property!(
        /// Defines the `PaneToggleButtonWidth` property: the width of the pane toggle button, the
        /// compact pane length.
        pub fn pane_toggle_button_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("PaneToggleButtonWidth", 48.0)
        }
    );
    ferro_property!(
        /// Defines the `SmallerPaneToggleButtonWidth` property: the compact pane length less 8, the width
        /// of the back button and of the icon column of the items.
        pub fn smaller_pane_toggle_button_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("SmallerPaneToggleButtonWidth", 40.0)
        }
    );
    ferro_property!(
        /// Defines the `OpenPaneLength` property: the open pane length, at most the width of the
        /// navigation view.
        pub fn open_pane_length_property() -> StyledProperty<f64> {
            FerroProperty::register::<NavigationViewTemplateSettings, _>("OpenPaneLength", 320.0)
        }
    );
} }

impl NavigationViewTemplateSettings {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates the settings of a navigation view.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the height above the pane and the content.
    pub fn top_padding(&self) -> f64 {
        self.get_value(Self::top_padding_property())
    }

    /// Gets whether the overflow button of the top pane is shown.
    pub fn overflow_button_visibility(&self) -> bool {
        self.get_value(Self::overflow_button_visibility_property())
    }

    /// Gets whether the pane toggle button is shown.
    pub fn pane_toggle_button_visibility(&self) -> bool {
        self.get_value(Self::pane_toggle_button_visibility_property())
    }

    /// Gets whether the back button is shown.
    pub fn back_button_visibility(&self) -> bool {
        self.get_value(Self::back_button_visibility_property())
    }

    /// Gets whether the top pane is shown.
    pub fn top_pane_visibility(&self) -> bool {
        self.get_value(Self::top_pane_visibility_property())
    }

    /// Gets whether the left pane is shown.
    pub fn left_pane_visibility(&self) -> bool {
        self.get_value(Self::left_pane_visibility_property())
    }

    /// Gets whether the selection follows the focus.
    pub fn single_selection_follows_focus(&self) -> bool {
        self.get_value(Self::single_selection_follows_focus_property())
    }

    /// Gets the width of the pane toggle button.
    pub fn pane_toggle_button_width(&self) -> f64 {
        self.get_value(Self::pane_toggle_button_width_property())
    }

    /// Gets the compact pane length less 8.
    pub fn smaller_pane_toggle_button_width(&self) -> f64 {
        self.get_value(Self::smaller_pane_toggle_button_width_property())
    }

    /// Gets the open pane length, at most the width of the navigation view.
    pub fn open_pane_length(&self) -> f64 {
        self.get_value(Self::open_pane_length_property())
    }

    /// Sets a value that the navigation view computes; a value equal to the current one raises
    /// nothing.
    pub(crate) fn set<T: ferroui_base::PropertyValue + PartialEq>(&self, property: &'static StyledProperty<T>, value: T) {
        if self.get_value(property) != value {
            self.set_value(property, value);
        }
    }
}
