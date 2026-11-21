//! Ribbon.Menus.cs BuildDisplayOptionsMenu and native display-options popup.
//! Copyright Wiesław Šoltés, MIT. Source menu order and conditions are retained.
use super::*;
use crate::{
    compat::{FontIcon, MenuFlyoutSeparator, RadioMenuFlyoutItem, ToggleMenuFlyoutItem},
    controls::RibbonMenu,
};
use ferroribbon_core::{RibbonMinimizeBehavior, localization::RibbonStrings};
use ferroui_controls::{MenuFlyout, PlacementMode};
fn native_text(text: Option<RibbonString>) -> Option<String> {
    text.map(|s| s.to_utf8().expect("native display-options text"))
}
impl Ribbon {
    pub fn is_simplified_mode_available(&self) -> bool {
        self.get_value(Self::is_simplified_mode_available_property())
    }
    pub fn set_is_simplified_mode_available(&self, value: bool) {
        self.set_value(Self::is_simplified_mode_available_property(), value);
    }

    pub fn build_display_options_menu(&self) -> Ref<MenuFlyout> {
        let strings = RibbonStrings::current();
        let menu = MenuFlyout::new();
        let add_mode = |text, mode, glyph: &str| {
            let item = RadioMenuFlyoutItem::new();
            item.set_text(native_text(text));
            item.set_group_name(Some("visibility".into()));
            item.set_is_checked(self.visibility_mode() == mode);
            let icon = FontIcon::new();
            icon.set_glyph(Some(glyph.into()));
            item.set_icon(Some(Control::boxed(icon)));
            let weak = self.to_ref().downgrade();
            item.add_click_action(move |_, _| {
                if let Some(ribbon) = weak.upgrade() {
                    ribbon.set_visibility_mode(mode);
                }
            });
            menu.items().add(Some(Control::boxed(item)));
        };
        add_mode(
            strings.full_screen_mode(),
            RibbonVisibilityMode::FullScreen,
            "\u{e740}",
        );
        if self.minimize_behavior() != RibbonMinimizeBehavior::Tabs
            || self.is_minimize_button_visible()
            || Self::is_minimized_mode(self.visibility_mode())
                && self.visibility_mode() != RibbonVisibilityMode::TabsOnly
        {
            add_mode(
                strings.show_panel_titles(),
                RibbonVisibilityMode::PanelTitles,
                "\u{e8fd}",
            );
            add_mode(
                strings.show_panel_buttons(),
                RibbonVisibilityMode::PanelButtons,
                "\u{e8a9}",
            );
        }
        add_mode(
            strings.show_tabs_only(),
            RibbonVisibilityMode::TabsOnly,
            "\u{e70e}",
        );
        add_mode(
            strings.always_show_ribbon(),
            RibbonVisibilityMode::AlwaysShow,
            "\u{e70d}",
        );
        if self.is_simplified_mode_available() {
            menu.items()
                .add(Some(Control::boxed(MenuFlyoutSeparator::new())));
            for (text, mode) in [
                (strings.use_classic_ribbon(), RibbonDisplayMode::Classic),
                (
                    strings.use_simplified_ribbon(),
                    RibbonDisplayMode::Simplified,
                ),
            ] {
                let item = RadioMenuFlyoutItem::new();
                item.set_text(native_text(text));
                item.set_group_name(Some("layout".into()));
                item.set_is_checked(self.display_mode() == mode);
                let weak = self.to_ref().downgrade();
                item.add_click_action(move |_, _| {
                    if let Some(ribbon) = weak.upgrade() {
                        ribbon.set_display_mode(mode);
                    }
                });
                menu.items().add(Some(Control::boxed(item)));
            }
        }
        menu.items()
            .add(Some(Control::boxed(MenuFlyoutSeparator::new())));
        let qat = ToggleMenuFlyoutItem::new();
        qat.set_text(native_text(strings.show_quick_access_toolbar()));
        qat.set_is_checked(self.is_quick_access_visible());
        let weak = self.to_ref().downgrade();
        qat.add_click_action(move |sender, _| {
            if let Some(ribbon) = weak.upgrade() {
                ribbon.set_is_quick_access_visible(
                    sender
                        .to_ref()
                        .cast::<ToggleMenuFlyoutItem>()
                        .expect("display options QAT sender")
                        .is_checked(),
                );
            }
        });
        menu.items().add(Some(Control::boxed(qat)));
        RibbonMenu::apply_theme(Some(&menu));
        menu
    }
    pub(super) fn show_display_options_menu(&self, anchor: &Control) -> Ref<MenuFlyout> {
        let menu = self.build_display_options_menu();
        menu.set_placement(PlacementMode::BottomEdgeAlignedRight);
        self.show_owned_menu(menu.clone(), anchor, None);
        menu
    }
}
#[cfg(test)]
#[path = "../display_options_tests.rs"]
mod tests;
