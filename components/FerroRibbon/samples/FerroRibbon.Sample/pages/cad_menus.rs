// Copyright (c) 2026 Wiesław Šoltés. MIT. Original CAD menu ownership and discovery.
use super::CadPage;
use crate::cad::{CadMenuIconError, CadMenuIcons};
use ferroribbon::{
    compat::{MenuFlyoutItem, MenuFlyoutSeparator, RadioMenuFlyoutItem},
    controls::{
        RibbonComboBox, RibbonItemHelper, RibbonSplitButton, RibbonStackPanel, RibbonStatusBar,
        RibbonToolBar,
    },
};
use ferroribbon_core::model::{RibbonIcon, RibbonIconKind};
use ferroui_base::{
    FerroLocator, LocatorExtensions, Ref,
    metadata::{MarkupDelegate, from_markup_value},
    platform::{IPlatformSettings, PlatformThemeVariant},
    styling::ThemeVariant,
};
use ferroui_controls::{Button, Control, MenuFlyout};
use std::{collections::HashSet, rc::Rc};

pub(super) fn walk(
    items: impl Iterator<Item = Option<Ref<Control>>> + 'static,
) -> Box<dyn Iterator<Item = Ref<Control>>> {
    Box::new(items.flatten().flat_map(|item| {
        if let Some(stack) = item.cast::<RibbonStackPanel>() {
            walk(stack.items().iter())
        } else {
            Box::new(std::iter::once(item)) as Box<dyn Iterator<Item = Ref<Control>>>
        }
    }))
}
pub(super) fn discover_menu_flyouts(
    elements: impl Iterator<Item = Ref<Control>>,
) -> impl Iterator<Item = Ref<MenuFlyout>> {
    let mut seen: HashSet<Ref<Control>> = HashSet::new();
    elements
        .filter(move |element| seen.insert(element.clone()))
        .filter_map(|element| {
            let flyout = if let Some(split) = element.cast::<RibbonSplitButton>() {
                split.flyout()
            } else {
                element.cast::<Button>().and_then(|button| button.flyout())
            };
            flyout.and_then(|flyout| flyout.cast::<MenuFlyout>())
        })
}
pub(super) fn rebuild_minimize_menu(menu: &MenuFlyout, build: impl FnOnce() -> Ref<MenuFlyout>) {
    menu.items().clear();
    let source = build();
    let entries: Vec<_> = source.items().iter().collect();
    for item in entries {
        menu.items().add(item);
    }
}
impl CadPage {
    /// The source distincts elements, not flyouts: two different buttons may yield
    /// the same menu twice. Traversal stays lazy and keeps native object identity.
    pub(super) fn enumerate_menus(&self) -> impl Iterator<Item = Ref<MenuFlyout>> + use<> {
        let page = self.to_ref();
        let body = std::iter::once_with(move || page.cad_ribbon().get_all_items().iter()).flatten();
        let page = self.to_ref();
        let slide_out = std::iter::once_with(move || {
            page.cad_ribbon()
                .tabs()
                .iter()
                .flat_map(|tab| tab.expect("CAD null tab").groups().iter())
                .flat_map(|group| walk(group.expect("CAD null group").slide_out_items().iter()))
        })
        .flatten();
        let page = self.to_ref();
        let navigation = std::iter::once_with(move || {
            page.named::<RibbonToolBar>("NavToolBar")
                .items()
                .iter()
                .flatten()
        })
        .flatten();
        let page = self.to_ref();
        let status = std::iter::once_with(move || {
            page.named::<RibbonStatusBar>("StatusBar")
                .end_items()
                .iter()
                .flatten()
        })
        .flatten();
        let owner = self.to_ref();
        discover_menu_flyouts(body.chain(slide_out).chain(navigation).chain(status)).inspect(
            move |_| {
                let _keep_alive = &owner;
            },
        )
    }
    pub(super) fn install_cad_menu_converter(&self) {
        let dark = self.menu_icons_dark.clone();
        // Retain just the source's theme state. A global converter must not root
        // the page or its visual tree, and remains usable after the page drops.
        RibbonItemHelper::set_menu_icon_converter(Some(MarkupDelegate::new(move |args| {
            let icon = from_markup_value::<Rc<RibbonIcon>>(&args[0])?;
            if icon.kind != RibbonIconKind::Path || icon.view_box_size != 32. {
                return None;
            }
            CadMenuIcons::create(&icon, dark.get())
                .expect("CAD menu icon rasterization")
                .map(Control::boxed)
        })));
    }
    pub(super) fn apply_menu_icons(&self) -> Result<(), CadMenuIconError> {
        let variant = self.actual_theme_variant();
        let background = if variant.is_some() && variant == ThemeVariant::high_contrast_variant() {
            FerroLocator::current()
                .get_service::<dyn IPlatformSettings>()
                .and_then(|settings| settings.get_color_values().system_colors())
                .map(|colors| colors.theme_variant())
        } else {
            ThemeVariant::to_platform_theme_variant(variant.as_ref())
        };
        let dark = background != Some(PlatformThemeVariant::Light);
        self.menu_icons_dark.set(dark);
        for menu in self.enumerate_menus() {
            CadMenuIcons::apply(
                menu.items()
                    .iter()
                    .map(|value| value.and_then(|v| Control::from_boxed(&v))),
                dark,
            )?;
        }
        Ok(())
    }
    pub(super) fn build_workspace_menu(&self) {
        const WORKSPACES: [&str; 3] = ["Drafting & Annotation", "3D Basics", "3D Modeling"];
        let menu = MenuFlyout::new();
        for workspace in WORKSPACES {
            let item = RadioMenuFlyoutItem::new();
            item.set_text(Some(workspace.to_owned()));
            item.set_group_name(Some("workspace".into()));
            item.set_is_checked(workspace == WORKSPACES[0]);
            let model = Rc::downgrade(&self.view_model());
            item.add_click_action(move |_, _| {
                if let Some(model) = model.upgrade() {
                    model
                        .workspace_action()
                        .execute(Some(&workspace.to_owned()));
                }
            });
            menu.items().add(Some(Control::boxed(item)));
        }
        menu.items()
            .add(Some(Control::boxed(MenuFlyoutSeparator::new())));
        for label in [
            "Save Current As...",
            "Workspace Settings...",
            "Customize...",
        ] {
            let item = MenuFlyoutItem::new();
            item.set_text(Some(label.to_owned()));
            menu.items().add(Some(Control::boxed(item)));
        }
        let page = self.to_ref().downgrade();
        let weak_menu = menu.downgrade();
        self.subscriptions.borrow_mut().push(
            menu.opening(move |_| {
                if let (Some(page), Some(menu)) = (page.upgrade(), weak_menu.upgrade()) {
                    let text = page.named::<RibbonComboBox>("WorkspaceCombo").text();
                    for item in menu
                        .items()
                        .iter()
                        .flatten()
                        .filter_map(|value| Control::from_boxed(&value))
                        .filter_map(|c| c.cast::<RadioMenuFlyoutItem>())
                    {
                        item.set_is_checked(item.text() == text);
                    }
                }
            })
            .detach(),
        );
        self.named::<ferroribbon::controls::RibbonButton>("WorkspaceStatusButton")
            .set_flyout(Some(menu.upcast()));
    }
    pub(super) fn build_minimize_behavior_menu(&self) {
        let menu = MenuFlyout::new();
        let page = self.to_ref().downgrade();
        let weak_menu = menu.downgrade();
        self.subscriptions.borrow_mut().push(
            menu.opening(move |_| {
                if let (Some(page), Some(menu)) = (page.upgrade(), weak_menu.upgrade()) {
                    rebuild_minimize_menu(&menu, || {
                        page.cad_ribbon().build_minimize_behavior_menu()
                    });
                }
            })
            .detach(),
        );
        self.named::<ferroribbon::controls::RibbonButton>("MinimizeBehaviorButton")
            .set_flyout(Some(menu.upcast()));
    }
}
