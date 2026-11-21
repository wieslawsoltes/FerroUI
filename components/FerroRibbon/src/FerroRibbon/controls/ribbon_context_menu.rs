//! Ribbon.Menus.cs item menus and Ribbon.Panels.cs visibility/panel entries.
//! Copyright Wiesław Šoltés, MIT. Native popup/event ownership; no WinUI service facade.
use super::*;
use crate::{
    compat::{MenuFlyoutItem, MenuFlyoutSeparator, MenuFlyoutSubItem, ToggleMenuFlyoutItem},
    controls::{
        RibbonCustomizePage, RibbonGroup, RibbonItemHelper, RibbonMenu,
        ribbon_menu::action_delegate, ribbon_quick_access_tool_bar::as_item,
    },
};
use ferroribbon_core::{RibbonQuickAccessPosition, localization::RibbonStrings};
use ferroui_base::{
    Point, Rect,
    reactive::{Disposable, IDisposable},
    utilities::HandlerList,
};
use ferroui_controls::{MenuFlyout, PlacementMode};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RibbonContextMenuError {
    InvalidState(String),
    UnsupportedText(Vec<u16>),
    Icon(String),
}
impl std::fmt::Display for RibbonContextMenuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ribbon context menu: {self:?}")
    }
}
impl std::error::Error for RibbonContextMenuError {}
pub struct RibbonContextMenuEventArgs {
    pub element: Ref<Control>,
    pub menu: Ref<MenuFlyout>,
    pub handled: Cell<bool>,
}
impl PartialEq for RibbonContextMenuEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RibbonContextMenuEventArgs {namespace:"FerroRibbon.Controls",this:Rc<RibbonContextMenuEventArgs>,handles:[Rc<RibbonContextMenuEventArgs>],properties:[Element:Ref<Control>{get:|args:&Rc<RibbonContextMenuEventArgs>|args.element.clone()},Menu:Ref<MenuFlyout>{get:|args:&Rc<RibbonContextMenuEventArgs>|args.menu.clone()},Handled:bool{get:|args:&Rc<RibbonContextMenuEventArgs>|args.handled.get(),set:|args:&Rc<RibbonContextMenuEventArgs>,value|args.handled.set(value)}]});
type ContextHandler = dyn Fn(&Rc<RibbonContextMenuEventArgs>);
#[derive(Default)]
pub(super) struct ContextMenuState {
    opening: HandlerList<ContextHandler>,
    markup_opening: RefCell<Vec<(MarkupDelegate, u64)>>,
}
fn text(value: Option<RibbonString>) -> Result<Option<String>, RibbonContextMenuError> {
    value
        .map(|s| {
            s.to_utf8()
                .map_err(|_| RibbonContextMenuError::UnsupportedText(s.as_utf16().to_vec()))
        })
        .transpose()
}
fn item(
    label: Option<RibbonString>,
    glyph: Option<&str>,
    action: impl Fn() + 'static,
) -> Result<Ref<MenuFlyoutItem>, RibbonContextMenuError> {
    RibbonMenu::item(
        text(label)?,
        Some(action_delegate(action)),
        glyph.map(|s| Rc::new(s.to_owned()) as BoxedValue),
        None,
    )
    .map_err(|e| RibbonContextMenuError::Icon(e.to_string()))
}
fn separator(menu: &MenuFlyout) {
    menu.items()
        .add(Some(Control::boxed(MenuFlyoutSeparator::new())));
}
impl Ribbon {
    pub fn add_context_menu_opening(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let weak = self.to_ref().downgrade();
        let token = self.context_menu_state.opening.add(Rc::new(move |args| {
            if let Some(root) = weak.upgrade() {
                callback
                    .try_invoke(&[Some(Control::boxed(root)), Some(Rc::new(args.clone()))])
                    .expect("ContextMenuOpening markup handler");
            }
        }));
        self.context_menu_state
            .markup_opening
            .borrow_mut()
            .push((handler, token));
    }
    pub fn remove_context_menu_opening(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut entries = self.context_menu_state.markup_opening.borrow_mut();
            entries
                .iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|index| entries.remove(index))
        };
        if let Some((_, token)) = removed {
            self.context_menu_state.opening.remove(token);
        }
    }

    pub fn is_context_menu_enabled(&self) -> bool {
        self.get_value(Self::is_context_menu_enabled_property())
    }
    pub fn set_is_context_menu_enabled(&self, value: bool) {
        self.set_value(Self::is_context_menu_enabled_property(), value);
    }
    pub fn can_customize(&self) -> bool {
        self.get_value(Self::can_customize_property())
    }
    pub fn set_can_customize(&self, value: bool) {
        self.set_value(Self::can_customize_property(), value);
    }
    pub fn is_visibility_menu_enabled(&self) -> bool {
        self.get_value(Self::is_visibility_menu_enabled_property())
    }
    pub fn set_is_visibility_menu_enabled(&self, value: bool) {
        self.set_value(Self::is_visibility_menu_enabled_property(), value);
    }
    pub fn context_menu_opening(
        &self,
        handler: impl Fn(&RibbonContextMenuEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self
            .context_menu_state
            .opening
            .add(Rc::new(move |args| handler(args)));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(root) = weak.upgrade() {
                root.context_menu_state.opening.remove(token);
            }
        })
    }
    pub fn show_item_context_menu(
        &self,
        element: Ref<Control>,
        position: Point,
    ) -> Result<bool, RibbonContextMenuError> {
        if !self.is_context_menu_enabled() {
            return Ok(false);
        }
        let menu = self.build_item_context_menu(&element)?;
        let args = Rc::new(RibbonContextMenuEventArgs {
            element: element.clone(),
            menu: menu.clone(),
            handled: Cell::new(false),
        });
        for (_, handler) in self.context_menu_state.opening.snapshot().iter() {
            handler(&args);
        }
        if args.handled.get() || menu.items().count() == 0 {
            return Ok(args.handled.get());
        }
        // A zero-size placement rectangle is the source's element-relative point.
        // Native positioning supplies DPI conversion, constraint handling and RTL.
        menu.set_placement(PlacementMode::BottomEdgeAlignedLeft);
        self.show_owned_menu(
            menu,
            &element,
            Some(Rect::new(position.x, position.y, 0., 0.)),
        );
        Ok(true)
    }
    fn build_item_context_menu(
        &self,
        element: &Control,
    ) -> Result<Ref<MenuFlyout>, RibbonContextMenuError> {
        let strings = RibbonStrings::current();
        let menu = MenuFlyout::new();
        let source = RibbonItemHelper::get_source_item(element).unwrap_or_else(|| element.to_ref());
        if as_item(&source).is_some_and(|i| i.can_add_to_quick_access()) {
            let weak = self.to_ref().downgrade();
            let target = source.clone();
            let entry = if self.is_in_quick_access(&source) {
                item(
                    strings.remove_from_quick_access_toolbar(),
                    None,
                    move || {
                        if let Some(root) = weak.upgrade() {
                            root.remove_from_quick_access(&target);
                        }
                    },
                )?
            } else {
                item(
                    strings.add_to_quick_access_toolbar(),
                    Some("\u{e710}"),
                    move || {
                        if let Some(root) = weak.upgrade() {
                            root.add_to_quick_access(&target);
                        }
                    },
                )?
            };
            menu.items().add(Some(Control::boxed(entry)));
            separator(&menu);
        }
        self.add_panel_menu_entries(&menu, element)?;
        if self.can_customize() {
            let label = text(strings.customize_quick_access_toolbar())?.unwrap_or_default() + "...";
            let weak = self.to_ref().downgrade();
            menu.items().add(Some(Control::boxed(item(
                Some(label.into()),
                None,
                move || {
                    if let Some(root) = weak.upgrade() {
                        root.request_customize_dialog(RibbonCustomizePage::QuickAccessToolbar);
                    }
                },
            )?)));
        }
        let weak = self.to_ref().downgrade();
        menu.items().add(Some(Control::boxed(item(
            if self.quick_access_position() == RibbonQuickAccessPosition::AboveRibbon {
                strings.show_below_ribbon()
            } else {
                strings.show_above_ribbon()
            },
            None,
            move || {
                if let Some(root) = weak.upgrade() {
                    root.toggle_quick_access_position();
                }
            },
        )?)));
        if self.can_customize() {
            separator(&menu);
            let weak = self.to_ref().downgrade();
            menu.items().add(Some(Control::boxed(item(
                strings.customize_ribbon(),
                None,
                move || {
                    if let Some(root) = weak.upgrade() {
                        root.request_customize_dialog(RibbonCustomizePage::Ribbon);
                    }
                },
            )?)));
        }
        if self.is_collapsible() {
            let always = self.visibility_mode() == RibbonVisibilityMode::AlwaysShow;
            let weak = self.to_ref().downgrade();
            menu.items().add(Some(Control::boxed(item(
                if always {
                    strings.collapse_ribbon()
                } else {
                    strings.pin_ribbon()
                },
                Some(if always { "\u{e70e}" } else { "\u{e718}" }),
                move || {
                    if let Some(root) = weak.upgrade() {
                        root.set_visibility_mode(
                            if root.visibility_mode() == RibbonVisibilityMode::AlwaysShow {
                                root.minimized_state()
                            } else {
                                RibbonVisibilityMode::AlwaysShow
                            },
                        );
                    }
                },
            )?)));
        }
        RibbonMenu::apply_theme(Some(&menu));
        Ok(menu)
    }
    fn add_panel_menu_entries(
        &self,
        menu: &MenuFlyout,
        element: &Control,
    ) -> Result<(), RibbonContextMenuError> {
        let strings = RibbonStrings::current();
        let group = element
            .to_ref()
            .cast::<RibbonGroup>()
            .or_else(|| self.find_location(element).1);
        let mut added = false;
        if self.can_float_groups()
            && let Some(group) = &group
        {
            let target = group.clone();
            let entry = if group.is_floating() {
                item(
                    strings.return_panel_to_ribbon(),
                    Some("\u{e70e}"),
                    move || target.return_to_ribbon(),
                )?
            } else {
                item(strings.float_panel(), Some("\u{e8a7}"), move || {
                    target.float(None)
                })?
            };
            menu.items().add(Some(Control::boxed(entry)));
            added = true;
        }
        if self.can_float_groups()
            && self.floating_groups().count()
                > if group.as_ref().is_some_and(|g| g.is_floating()) {
                    1
                } else {
                    0
                }
        {
            let weak = self.to_ref().downgrade();
            menu.items().add(Some(Control::boxed(item(
                strings.return_panels_to_ribbon(),
                None,
                move || {
                    if let Some(root) = weak.upgrade() {
                        root.return_all_panels_to_ribbon();
                    }
                },
            )?)));
            added = true;
        }
        if self.is_visibility_menu_enabled() {
            menu.items()
                .add(Some(Control::boxed(self.build_show_tabs_menu()?)));
            if let Some(tab) = self.selected_tab()
                && tab.groups().count() > 0
            {
                menu.items()
                    .add(Some(Control::boxed(self.build_show_panels_menu(tab)?)));
            }
            let titles = ToggleMenuFlyoutItem::new();
            titles.set_text(text(strings.show_group_titles())?);
            titles.set_is_checked(self.show_group_captions());
            let weak = self.to_ref().downgrade();
            titles.add_click_action(move |sender, _| {
                if let Some(root) = weak.upgrade() {
                    root.set_show_group_captions(
                        sender
                            .to_ref()
                            .cast::<ToggleMenuFlyoutItem>()
                            .expect("panel title toggle")
                            .is_checked(),
                    );
                }
            });
            menu.items().add(Some(Control::boxed(titles)));
            added = true;
        }
        if added {
            separator(menu);
        }
        Ok(())
    }
    pub fn build_show_tabs_menu(&self) -> Result<Ref<MenuFlyoutSubItem>, RibbonContextMenuError> {
        let menu = MenuFlyoutSubItem::new();
        menu.set_text(text(RibbonStrings::current().show_tabs())?);
        for tab in self.tabs().iter() {
            let tab = tab
                .ok_or_else(|| RibbonContextMenuError::InvalidState("Tabs contains null".into()))?;
            if tab.is_contextual() {
                continue;
            }
            let Some(id) = tab.effective_id() else {
                continue;
            };
            let entry = ToggleMenuFlyoutItem::new();
            entry.set_text(tab.header().or_else(|| Some(id.clone())));
            let key: RibbonString = id.clone().into();
            let hidden = self.customization().hidden_tab_ids().ok_or_else(|| {
                RibbonContextMenuError::InvalidState("HiddenTabIds is null".into())
            })?;
            entry.set_is_checked(!hidden.iter().any(|value| value.as_ref() == Some(&key)));
            let weak = self.to_ref().downgrade();
            entry.add_click_action(move |sender, _| {
                if let Some(root) = weak.upgrade() {
                    root.set_hidden_by_user(
                        id.clone().into(),
                        !sender
                            .to_ref()
                            .cast::<ToggleMenuFlyoutItem>()
                            .expect("tab visibility toggle")
                            .is_checked(),
                        true,
                    )
                    .expect("tab visibility customization");
                }
            });
            menu.items().add(Some(entry.upcast()));
        }
        Ok(menu)
    }
    pub fn build_show_panels_menu(
        &self,
        tab: Ref<RibbonTab>,
    ) -> Result<Ref<MenuFlyoutSubItem>, RibbonContextMenuError> {
        let menu = MenuFlyoutSubItem::new();
        menu.set_text(text(RibbonStrings::current().show_panels())?);
        for group in tab.display_groups().iter() {
            let group = group.ok_or_else(|| {
                RibbonContextMenuError::InvalidState("DisplayGroups contains null".into())
            })?;
            let Some(id) = group.effective_id() else {
                continue;
            };
            let entry = ToggleMenuFlyoutItem::new();
            entry.set_text(group.header().or_else(|| Some(id.clone())));
            let key: RibbonString = id.clone().into();
            let hidden = self.customization().hidden_group_ids().ok_or_else(|| {
                RibbonContextMenuError::InvalidState("HiddenGroupIds is null".into())
            })?;
            entry.set_is_checked(!hidden.iter().any(|value| value.as_ref() == Some(&key)));
            let weak = self.to_ref().downgrade();
            entry.add_click_action(move |sender, _| {
                if let Some(root) = weak.upgrade() {
                    root.set_hidden_by_user(
                        id.clone().into(),
                        !sender
                            .to_ref()
                            .cast::<ToggleMenuFlyoutItem>()
                            .expect("panel visibility toggle")
                            .is_checked(),
                        false,
                    )
                    .expect("panel visibility customization");
                }
            });
            menu.items().add(Some(entry.upcast()));
        }
        Ok(menu)
    }
    fn set_hidden_by_user(
        &self,
        id: RibbonString,
        hidden: bool,
        is_tab: bool,
    ) -> Result<(), RibbonContextMenuError> {
        let customization = state::clone_customization(self.customization())
            .map_err(RibbonContextMenuError::InvalidState)?;
        let list = if is_tab {
            customization.hidden_tab_ids()
        } else {
            customization.hidden_group_ids()
        }
        .ok_or_else(|| RibbonContextMenuError::InvalidState("Cloned hidden IDs are null".into()))?;
        list.remove(&Some(id.clone()));
        if hidden {
            list.add(Some(id));
        }
        self.apply_customization(Some(customization))
            .map_err(RibbonContextMenuError::InvalidState)
    }
}

#[cfg(test)]
#[path = "../context_menu_tests.rs"]
mod tests;
