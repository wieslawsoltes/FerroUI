//! Visibility/minimize algorithms from Ribbon.cs and Ribbon.Panels.cs.
use super::*;
use ferroribbon_core::{RibbonMinimizeBehavior, RibbonVisibilityMode, localization::RibbonStrings};
use ferroui_base::{
    Point, Rect,
    input::{
        InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod, PointerPressedEventArgs,
    },
    interactivity::{RoutedEventHandlerToken, RoutingStrategies},
    styling::ThemeVariant,
};
use ferroui_controls::{
    Border, MenuFlyout, PlacementMode, ToolTip, TopLevel,
    primitives::{
        Popup,
        popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment},
    },
};
#[derive(Default)]
pub(super) struct VisibilityState {
    popup: RefCell<Option<Rc<MinimizedPopup>>>,
    input: RefCell<Option<InputHook>>,
    menus: RefCell<Vec<Ref<MenuFlyout>>>,
}
impl Drop for VisibilityState {
    fn drop(&mut self) {
        for m in self.menus.get_mut().drain(..) {
            m.hide();
        }
    }
}
struct MinimizedPopup {
    popup: Ref<Popup>,
    chrome: Ref<Border>,
    parent: RefCell<Option<Ref<Panel>>>,
    index: Cell<i32>,
}
struct InputHook {
    root: WeakRef<Control>,
    owner: WeakRef<Control>,
    key: RoutedEventHandlerToken,
    key_up: RoutedEventHandlerToken,
    pointer: RoutedEventHandlerToken,
    owner_key: Option<RoutedEventHandlerToken>,
    owner_key_up: Option<RoutedEventHandlerToken>,
}
impl Drop for InputHook {
    fn drop(&mut self) {
        if let Some(root) = self.root.upgrade() {
            root.remove_handler(InputElement::key_down_event(), self.key);
            root.remove_handler(InputElement::key_up_event(), self.key_up);
            root.remove_handler(InputElement::pointer_pressed_event(), self.pointer);
        }
        if let Some(owner) = self.owner.upgrade() {
            if let Some(key) = self.owner_key.take() {
                owner.remove_handler(InputElement::key_down_event(), key);
            }
            if let Some(key) = self.owner_key_up.take() {
                owner.remove_handler(InputElement::key_up_event(), key);
            }
        }
    }
}
macro_rules! prop {
    ($g:ident,$s:ident,$p:ident,$t:ty) => {
        pub fn $g(&self) -> $t {
            self.get_value(Self::$p())
        }
        pub fn $s(&self, v: $t) {
            self.set_value(Self::$p(), v)
        }
    };
}
impl Ribbon {
    prop!(
        visibility_mode,
        set_visibility_mode,
        visibility_mode_property,
        RibbonVisibilityMode
    );
    prop!(is_minimized, set_is_minimized, is_minimized_property, bool);
    prop!(
        is_collapsible,
        set_is_collapsible,
        is_collapsible_property,
        bool
    );
    prop!(
        is_full_screen_revealed,
        set_is_full_screen_revealed,
        is_full_screen_revealed_property,
        bool
    );
    prop!(
        minimize_behavior,
        set_minimize_behavior,
        minimize_behavior_property,
        RibbonMinimizeBehavior
    );
    prop!(
        is_minimize_button_visible,
        set_is_minimize_button_visible,
        is_minimize_button_visible_property,
        bool
    );
    pub fn next_minimize_state(&self) -> RibbonVisibilityMode {
        use RibbonVisibilityMode as V;
        match self.minimize_behavior() {
            RibbonMinimizeBehavior::CycleAll => match self.visibility_mode() {
                V::AlwaysShow => V::PanelButtons,
                V::PanelButtons => V::PanelTitles,
                V::PanelTitles => V::TabsOnly,
                _ => V::AlwaysShow,
            },
            _ if self.visibility_mode() != V::AlwaysShow => V::AlwaysShow,
            RibbonMinimizeBehavior::PanelTitles => V::PanelTitles,
            RibbonMinimizeBehavior::PanelButtons => V::PanelButtons,
            _ => V::TabsOnly,
        }
    }
    pub(super) fn minimized_state(&self) -> RibbonVisibilityMode {
        match self.minimize_behavior() {
            RibbonMinimizeBehavior::PanelTitles => RibbonVisibilityMode::PanelTitles,
            RibbonMinimizeBehavior::PanelButtons | RibbonMinimizeBehavior::CycleAll => {
                RibbonVisibilityMode::PanelButtons
            }
            _ => RibbonVisibilityMode::TabsOnly,
        }
    }
    pub(super) fn is_minimized_mode(mode: RibbonVisibilityMode) -> bool {
        matches!(
            mode,
            RibbonVisibilityMode::TabsOnly
                | RibbonVisibilityMode::PanelTitles
                | RibbonVisibilityMode::PanelButtons
        )
    }
    pub(super) fn current_panel_presentation(&self) -> RibbonPanelPresentation {
        if self.display_mode() == RibbonDisplayMode::Simplified {
            return RibbonPanelPresentation::Full;
        }
        match self.visibility_mode() {
            RibbonVisibilityMode::PanelButtons => RibbonPanelPresentation::Buttons,
            RibbonVisibilityMode::PanelTitles => RibbonPanelPresentation::Titles,
            _ => RibbonPanelPresentation::Full,
        }
    }
    pub(super) fn on_visibility_mode_changed(&self) {
        self.set_is_minimized(Self::is_minimized_mode(self.visibility_mode()));
        if self.visibility_mode() != RibbonVisibilityMode::FullScreen {
            self.set_is_full_screen_revealed(false);
        }
        self.close_minimized_popup();
        for tab in self.tabs.iter() {
            for group in tab.expect("NullReference: tab.Groups").groups().iter() {
                let group = group.expect("NullReference: group");
                group.close_popup();
                group.close_slide_out();
            }
        }
        self.apply_presentation();
        self.update_chrome();
        self.update_minimize_button();
        self.raise_state_changed();
        self.on_model_visibility_mode_changed();
    }
    pub(super) fn on_is_minimized_changed(&self, value: bool) {
        if value && self.visibility_mode() == RibbonVisibilityMode::AlwaysShow {
            self.set_visibility_mode(self.minimized_state());
        } else if !value && Self::is_minimized_mode(self.visibility_mode()) {
            self.set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        }
    }
    pub fn toggle_minimized(&self) {
        if self.is_collapsible() {
            self.set_visibility_mode(self.next_minimize_state());
        }
    }
    pub(super) fn update_visibility_chrome(&self) {
        let full = self.visibility_mode() == RibbonVisibilityMode::FullScreen;
        let show = !full || self.is_full_screen_revealed();
        if let Some(row) = { self.parts.borrow().tab_row.clone() } {
            row.set_is_visible(show);
        }
        if let Some(bar) = { self.parts.borrow().command_bar.clone() }
            && !self.is_minimized_popup_open()
        {
            bar.set_is_visible(show && self.visibility_mode() != RibbonVisibilityMode::TabsOnly);
        }
        if let Some(reveal) = { self.parts.borrow().reveal_bar.clone() } {
            reveal.set_is_visible(full);
        }
        self.update_quick_access_placement();
    }
    pub(super) fn on_header_clicked(&self, tab: Ref<RibbonTab>) {
        let selected = self.selected_tab().as_ref() == Some(&tab);
        self.set_selected_tab(Some(tab));
        if self.visibility_mode() == RibbonVisibilityMode::TabsOnly {
            if selected && self.is_minimized_popup_open() {
                self.close_minimized_popup();
            } else {
                let root = self.to_ref();
                ferroui_base::threading::Dispatcher::ui_thread().post_local(
                    move || root.open_minimized_popup(),
                    ferroui_base::threading::DispatcherPriority::NORMAL,
                );
            }
        }
    }
    pub fn is_minimized_popup_open(&self) -> bool {
        self.visibility
            .popup
            .borrow()
            .as_ref()
            .is_some_and(|p| p.popup.is_open())
    }
    pub fn open_minimized_popup(&self) {
        let (bar, row) = {
            let p = self.parts.borrow();
            (p.command_bar.clone(), p.tab_row.clone())
        };
        let (Some(_bar), Some(_row), Some(window)) =
            (bar, row, TopLevel::get_top_level(Some(self)))
        else {
            return;
        };
        if self.visibility_mode() != RibbonVisibilityMode::TabsOnly {
            return;
        }
        let existing = self.visibility.popup.borrow().clone();
        let state = existing.unwrap_or_else(|| {
            let chrome = Border::new();
            let popup = Popup::new();
            popup.set_child(chrome.clone());
            popup.set_is_light_dismiss_enabled(false);
            // Publish the raw popup before SetOwner: that property notification
            // can reenter OpenMinimizedPopup, as in the source.
            let state = Rc::new(MinimizedPopup {
                popup: popup.clone(),
                chrome: chrome.clone(),
                parent: RefCell::default(),
                index: Cell::new(0),
            });
            self.visibility.popup.replace(Some(state.clone()));
            super::super::RibbonItemHelper::set_owner(&chrome, Some(self.item_owner()))
                .expect("minimized popup owner");
            let weak = self.to_ref().downgrade();
            popup
                .closed(move || {
                    if let Some(root) = weak.upgrade() {
                        root.restore_command_bar();
                    }
                })
                .detach();
            let weak = self.to_ref().downgrade();
            chrome.key_down(move |_, e| {
                if let Some(root) = weak.upgrade()
                    && e.key == Key::Escape
                {
                    root.close_minimized_popup();
                    if let Some(h) = root.selected_tab().and_then(|t| t.header_element()) {
                        h.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
                    }
                    e.set_handled(true);
                }
            });
            state
        });
        if !self.is_minimized_popup_open() {
            let bar = self
                .parts
                .borrow()
                .command_bar
                .clone()
                .expect("command bar changed during popup creation");
            let parent = bar.visual_parent().and_then(|p| p.cast::<Panel>());
            state.index.set(
                parent
                    .as_ref()
                    .and_then(|p| p.children().index_of(&bar.clone().upcast()))
                    .map(|i| i32::try_from(i).expect("Int32 child index"))
                    .unwrap_or(-1),
            );
            state.parent.replace(parent.clone());
            if let Some(parent) = parent {
                parent.children().remove(bar.clone());
            }
            bar.set_is_visible(true);
            state.chrome.set_child(bar);
            state.chrome.set_width(self.bounds().width);
            state.chrome.set_value(
                ThemeVariant::requested_theme_variant_property(),
                self.actual_theme_variant(),
            );
        }
        self.attach_popup_keyboard(Some(&state.chrome));
        let row = self
            .parts
            .borrow()
            .tab_row
            .clone()
            .expect("tab row changed during popup creation");
        let origin = row
            .translate_point(Point::new(0., row.bounds().height), &window)
            .expect("minimized tab-row transform");
        let origin_x = self
            .translate_point(Point::default(), &window)
            .expect("minimized ribbon transform")
            .x;
        // Raw source offsets, without RibbonPopupPlacement's distinct clamping algorithm.
        // Source XamlRoot ownership maps to the native popup owner. Placement
        // alone can leave an unparented popup with IsOpen=true and no host.
        state
            .popup
            .set_popup_parent(Some(&self.to_ref().upcast::<Control>()));
        state.popup.set_placement_target(window.upcast::<Control>());
        state.popup.set_placement(PlacementMode::AnchorAndGravity);
        state.popup.set_placement_rect(Some(Rect::default()));
        state
            .popup
            .set_placement_anchor(PopupAnchor::TOP | PopupAnchor::LEFT);
        state
            .popup
            .set_placement_gravity(PopupGravity::BOTTOM | PopupGravity::RIGHT);
        state
            .popup
            .set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        state.popup.set_horizontal_offset(origin_x);
        state.popup.set_vertical_offset(origin.y);
        state.popup.open();
    }
    pub fn close_minimized_popup(&self) {
        let popup = self.visibility.popup.borrow().clone();
        if let Some(p) = popup
            && p.popup.is_open()
        {
            p.popup.close();
        }
    }
    pub(super) fn restore_command_bar(&self) {
        let bar = self.parts.borrow().command_bar.clone();
        let state = self.visibility.popup.borrow().clone();
        let (Some(bar), Some(state)) = (bar, state) else {
            return;
        };
        if !state.chrome.child().is_some_and(|c| c.ptr_eq(&bar)) {
            return;
        }
        state.chrome.set_child(None);
        let parent = state.parent.borrow().clone();
        if let Some(parent) = parent {
            let index = state.index.get().clamp(
                0,
                i32::try_from(parent.children().count()).expect("Int32 child Count"),
            );
            let current = self
                .parts
                .borrow()
                .command_bar
                .clone()
                .expect("command bar changed during restore");
            parent.children().insert(index as usize, current);
        }
        self.update_chrome();
    }
    pub(super) fn remove_part_handlers(&self, kind: PartClick) {
        let handlers = std::mem::take(&mut self.parts.borrow_mut().click_handlers);
        let (removed, keep): (Vec<_>, Vec<_>) =
            handlers.into_iter().partition(|(k, _, _)| *k == kind);
        self.parts.borrow_mut().click_handlers = keep;
        for (_, button, token) in removed {
            button.remove_handler(Button::click_event(), token);
        }
    }
    pub(super) fn attach_reveal_part(&self) {
        let b = self.parts.borrow().reveal.clone();
        if let Some(b) = b {
            let weak = self.to_ref().downgrade();
            let token = b.click(move |_, _| {
                if let Some(r) = weak.upgrade() {
                    r.set_is_full_screen_revealed(!r.is_full_screen_revealed());
                }
            });
            self.parts
                .borrow_mut()
                .click_handlers
                .push((PartClick::Reveal, b, token));
        }
    }
    pub(super) fn attach_minimize_parts(&self) {
        let b = self.parts.borrow().minimize.clone();
        if let Some(b) = b {
            let weak = self.to_ref().downgrade();
            let token = b.click(move |_, _| {
                if let Some(r) = weak.upgrade() {
                    r.toggle_minimized();
                }
            });
            self.parts
                .borrow_mut()
                .click_handlers
                .push((PartClick::Minimize, b, token));
        }
        let b = self.parts.borrow().minimize_behavior.clone();
        if let Some(b) = b {
            let weak = self.to_ref().downgrade();
            let token = b.click(move |sender, _| {
                if let Some(r) = weak.upgrade() {
                    let menu = r.build_minimize_behavior_menu();
                    menu.set_placement(PlacementMode::BottomEdgeAlignedRight);
                    r.show_owned_menu(
                        menu,
                        &sender
                            .to_ref()
                            .cast::<Control>()
                            .expect("minimize menu anchor"),
                        None,
                    );
                }
            });
            self.parts
                .borrow_mut()
                .click_handlers
                .push((PartClick::Behavior, b, token));
        }
    }
    pub(super) fn update_minimize_button(&self) {
        if let Some(b) = { self.parts.borrow().minimize_buttons.clone() } {
            b.set_is_visible(self.is_minimize_button_visible() && self.is_collapsible());
        }
        let strings = RibbonStrings::current();
        if let Some(g) = { self.parts.borrow().minimize_glyph.clone() } {
            g.set_glyph(Some(
                if Self::is_minimized_mode(self.visibility_mode()) {
                    "\u{e70d}"
                } else {
                    "\u{e70e}"
                }
                .into(),
            ));
        }
        if let Some(b) = { self.parts.borrow().minimize.clone() } {
            let text = match self.next_minimize_state() {
                RibbonVisibilityMode::PanelButtons => strings.minimize_to_panel_buttons(),
                RibbonVisibilityMode::PanelTitles => strings.minimize_to_panel_titles(),
                RibbonVisibilityMode::TabsOnly => strings.minimize_to_tabs(),
                _ => strings.show_full_ribbon(),
            }
            .map(|s| s.to_utf8().expect("native UTF-16 minimize label"));
            AutomationProperties::set_name(&b, text.as_deref());
            let current = self
                .parts
                .borrow()
                .minimize
                .clone()
                .expect("NullReference: minimize button replaced during SetName");
            ToolTip::set_tip(&current, text.map(|s| Rc::new(s) as BoxedValue));
        }
        if let Some(b) = { self.parts.borrow().minimize_behavior.clone() } {
            let text = strings
                .minimize_ribbon()
                .map(|s| s.to_utf8().expect("native UTF-16 minimize label"));
            AutomationProperties::set_name(&b, text.as_deref());
            let current = self
                .parts
                .borrow()
                .minimize_behavior
                .clone()
                .expect("NullReference: minimize behavior button replaced during SetName");
            let text = strings
                .minimize_ribbon()
                .map(|s| s.to_utf8().expect("native UTF-16 minimize label"));
            ToolTip::set_tip(&current, text.map(|s| Rc::new(s) as BoxedValue));
        }
    }
    pub(super) fn show_owned_menu(
        &self,
        menu: Ref<MenuFlyout>,
        anchor: &Control,
        placement_rect: Option<Rect>,
    ) {
        let closed = menu.downgrade();
        let weak = self.to_ref().downgrade();
        menu.closed(move || {
            if let Some(ribbon) = weak.upgrade() {
                ribbon
                    .visibility
                    .menus
                    .borrow_mut()
                    .retain(|m| !closed.points_to(m));
            }
        })
        .detach();
        if let Some(rect) = placement_rect {
            menu.show_at_rect_core(anchor, rect);
        } else {
            menu.show_at(anchor);
        }
        if menu.is_open() {
            self.visibility.menus.borrow_mut().push(menu);
        }
    }
    pub fn build_minimize_behavior_menu(&self) -> Ref<MenuFlyout> {
        let strings = RibbonStrings::current();
        let menu = MenuFlyout::new();
        for (text, behavior) in [
            (strings.minimize_to_tabs(), RibbonMinimizeBehavior::Tabs),
            (
                strings.minimize_to_panel_titles(),
                RibbonMinimizeBehavior::PanelTitles,
            ),
            (
                strings.minimize_to_panel_buttons(),
                RibbonMinimizeBehavior::PanelButtons,
            ),
            (
                strings.cycle_through_all(),
                RibbonMinimizeBehavior::CycleAll,
            ),
        ] {
            let item = crate::compat::RadioMenuFlyoutItem::new();
            item.set_text(text.map(|s| s.to_utf8().expect("native UTF-16 minimize menu")));
            item.set_group_name(Some("minimize".into()));
            item.set_is_checked(self.minimize_behavior() == behavior);
            let weak = self.to_ref().downgrade();
            item.add_click_action(move |_, _| {
                if let Some(r) = weak.upgrade() {
                    r.set_minimize_behavior(behavior);
                }
            });
            menu.items().add(Some(Control::boxed(item)));
        }
        super::super::RibbonMenu::apply_theme(Some(&menu));
        menu
    }
    pub(super) fn on_visibility_item_invoked(&self, item: &Control) {
        if self.is_minimized_popup_open()
            && !item.is::<RibbonTabHeader>()
            && !item
                .to_ref()
                .cast::<Button>()
                .is_some_and(|b| b.flyout().is_some())
            && !item
                .to_ref()
                .cast::<super::super::RibbonSplitButton>()
                .is_some_and(|b| b.is_drop_down_open())
        {
            self.close_minimized_popup();
        }
    }
    pub(super) fn on_visibility_key_down(&self, e: &KeyEventArgs) {
        self.on_root_key_down(e);
    }
    pub(super) fn is_effectively_visible_visibility(&self) -> bool {
        if TopLevel::get_top_level(Some(self)).is_none() {
            return false;
        }
        let mut node = Some(self.to_ref().upcast::<ferroui_base::Visual>());
        while let Some(v) = node {
            if !v.is_visible() {
                return false;
            }
            node = v.visual_parent();
        }
        true
    }
    pub(super) fn attach_visibility_input(&self) {
        let Some(window) = TopLevel::get_top_level(Some(self)) else {
            return;
        };
        let owner: Ref<Control> = window.clone().upcast();
        let root = window
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .unwrap_or_else(|| owner.clone());
        if self
            .visibility
            .input
            .borrow()
            .as_ref()
            .is_some_and(|h| h.root.points_to(&root) && h.owner.points_to(&owner))
        {
            return;
        }
        self.detach_visibility_input();
        let weak = self.to_ref().downgrade();
        let key = root.add_handler_with(
            InputElement::key_down_event(),
            move |_, e| {
                if let Some(r) = weak.upgrade() {
                    r.on_visibility_key_down(e);
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        let weak = self.to_ref().downgrade();
        let key_up = root.add_handler_with(
            InputElement::key_up_event(),
            move |_, e| {
                if let Some(r) = weak.upgrade() {
                    r.on_root_key_up(e);
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        let weak = self.to_ref().downgrade();
        let pointer = root.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e: &PointerPressedEventArgs| {
                if let Some(r) = weak.upgrade() {
                    r.on_key_tip_pointer_pressed();
                    let row = r.parts.borrow().tab_row.clone();
                    if r.is_minimized_popup_open() && row.is_some_and(|row| !within(e, &row)) {
                        r.close_minimized_popup();
                    }
                    if r.is_full_screen_revealed() && !within(e, &r) {
                        r.set_is_full_screen_revealed(false);
                    }
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        // Source XamlRoot.Content receives focusless keys. Native KeyboardDevice
        // instead originates those routed events at the TopLevel itself. Keep
        // normal content routing and its order; admit only that direct source
        // at the owner so descendant and popup routes are never handled twice.
        let (owner_key, owner_key_up) = if root.ptr_eq(&owner) {
            (None, None)
        } else {
            let weak = self.to_ref().downgrade();
            let down = owner.add_handler_with(
                InputElement::key_down_event(),
                move |sender, e| {
                    if e.source()
                        .is_some_and(|source| source.ptr_eq(&sender.to_ref()))
                        && let Some(ribbon) = weak.upgrade()
                    {
                        ribbon.on_visibility_key_down(e);
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            );
            let weak = self.to_ref().downgrade();
            let up = owner.add_handler_with(
                InputElement::key_up_event(),
                move |sender, e| {
                    if e.source()
                        .is_some_and(|source| source.ptr_eq(&sender.to_ref()))
                        && let Some(ribbon) = weak.upgrade()
                    {
                        ribbon.on_root_key_up(e);
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            );
            (Some(down), Some(up))
        };
        self.visibility.input.replace(Some(InputHook {
            root: root.downgrade(),
            owner: owner.downgrade(),
            key,
            key_up,
            pointer,
            owner_key,
            owner_key_up,
        }));
    }
    pub(super) fn detach_visibility_input(&self) {
        self.detach_popup_keyboard();
        let old = self.visibility.input.take();
        drop(old);
    }
}
fn within(e: &PointerPressedEventArgs, element: &Control) -> bool {
    let p = e.get_position(Some(element));
    p.x >= 0. && p.y >= 0. && p.x <= element.bounds().width && p.y <= element.bounds().height
}

#[cfg(test)]
#[path = "../visibility_tests.rs"]
mod tests;
