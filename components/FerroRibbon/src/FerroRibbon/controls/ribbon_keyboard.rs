//! Ribbon.Keyboard.cs root KeyTip state machine with actual QAT presentations.
use super::super::{
    IRibbonItem, RibbonButton, RibbonControlBase, RibbonGroup, RibbonItemsContainer, RibbonKeyTip,
    RibbonSplitButton, RibbonToggleButton,
};
use super::*;
use crate::compat::{MenuFlyoutItem, MenuFlyoutSubItem};
use crate::metadata::key_tip_collections::{RibbonActiveKeyTips, RibbonCurrentKeyTips};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{
    RibbonItemSize,
    collections::{RibbonList, RibbonReadOnlyList},
    keytips::{
        RibbonKeyTipAssigner, RibbonKeyTipMatch, RibbonKeyTipNavigator, RibbonKeyTipRequest,
        RibbonKeyTipScope,
    },
    localization::RibbonStrings,
    model::RibbonItemsSource,
};
use ferroui_base::{
    Point, Rect, Size, Visual,
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod},
    interactivity::{RoutedEventHandlerToken, RoutingStrategies},
    styling::ThemeVariant,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Canvas, MenuFlyout, PlacementMode, TopLevel,
    primitives::{
        FlyoutBase, Popup, PopupFlyoutBase, PopupOverlayLayer,
        popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment},
    },
};
use std::collections::HashSet;
type Cleanup = Rc<dyn Fn()>;
ferroui_base::ferro_thread_local! {static EMPTY_CURRENT_TIPS:RibbonCurrentKeyTips=RibbonFixedItems::new(vec![]) as RibbonReadOnlyList<_>;}
pub(super) struct KeyboardState {
    navigator: Rc<RibbonKeyTipNavigator<Ref<Control>>>,
    cleanup: RefCell<Vec<Option<Cleanup>>>,
    badges: RibbonList<Option<Ref<RibbonKeyTip>>>,
    badge_view: RibbonActiveKeyTips,
    empty_tips: RibbonCurrentKeyTips,
    overlays: RefCell<Vec<Rc<BadgeOverlay>>>,
    alt_alone: Cell<bool>,
    focus_before: RefCell<Option<Ref<InputElement>>>,
    focus_target: RefCell<Option<Ref<Control>>>,
    popup_hooks: RefCell<Vec<PopupKeyHook>>,
    mode_changed: RefCell<Option<MarkupDelegate>>,
    search_requested: RefCell<Option<MarkupDelegate>>,
}
impl Default for KeyboardState {
    fn default() -> Self {
        let badges = RibbonItems::new();
        let badge_view = badges.clone() as RibbonReadOnlyList<_>;
        Self {
            navigator: RibbonKeyTipNavigator::new(),
            cleanup: RefCell::default(),
            badges,
            badge_view,
            empty_tips: EMPTY_CURRENT_TIPS.with(Clone::clone),
            overlays: RefCell::default(),
            alt_alone: Cell::new(false),
            focus_before: RefCell::default(),
            focus_target: RefCell::default(),
            popup_hooks: RefCell::default(),
            mode_changed: RefCell::default(),
            search_requested: RefCell::default(),
        }
    }
}
struct BadgeOverlay {
    owner: WeakRef<TopLevel>,
    in_window: bool,
    popup: Ref<Popup>,
    canvas: Ref<Canvas>,
}
impl BadgeOverlay {
    fn new(owner: &Ref<TopLevel>, in_window: bool) -> Self {
        let canvas = Canvas::new();
        canvas.set_is_hit_test_visible(false);
        let popup = Popup::new();
        popup.set_child(canvas.clone());
        popup.set_is_hit_test_visible(false);
        popup.set_is_light_dismiss_enabled(false);
        if in_window {
            popup.set_should_use_overlay_layer(true);
        }
        popup.set_placement_target(owner.clone().upcast::<Control>());
        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_rect(Some(Rect::default()));
        popup.set_placement_anchor(PopupAnchor::TOP | PopupAnchor::LEFT);
        popup.set_placement_gravity(PopupGravity::BOTTOM | PopupGravity::RIGHT);
        popup.set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        Self {
            owner: owner.downgrade(),
            in_window,
            popup,
            canvas,
        }
    }
}
impl Drop for BadgeOverlay {
    fn drop(&mut self) {
        self.popup.set_is_open(false);
    }
}
struct PopupKeyHook {
    root: WeakRef<Control>,
    down: RoutedEventHandlerToken,
    up: RoutedEventHandlerToken,
}
impl Drop for PopupKeyHook {
    fn drop(&mut self) {
        if let Some(root) = self.root.upgrade() {
            root.remove_handler(InputElement::key_down_event(), self.down);
            root.remove_handler(InputElement::key_up_event(), self.up);
        }
    }
}
// The selected SDK's detached popup interactive route ends at Popup (whose
// default interactive parent is its visual parent). Logical popup parenting is
// insufficient to reach the window content. Attach only the source keyboard
// handlers; negative controls and actual popup-root tests cover this mapping.
fn post(f: impl FnOnce() + 'static) {
    Dispatcher::ui_thread().post_local(f, DispatcherPriority::NORMAL);
}
fn cast_item(c: &Control) -> Option<Rc<dyn IRibbonItem>> {
    ValueTypes::try_cast(
        &Control::boxed(c.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
}
fn text(v: Option<String>) -> Option<RibbonString> {
    v.map(Into::into)
}
impl Ribbon {
    pub fn application_button_key_tip(&self) -> Option<String> {
        self.get_value(Self::application_button_key_tip_property())
    }
    pub fn set_application_button_key_tip(&self, v: Option<String>) {
        self.set_value(Self::application_button_key_tip_property(), v)
    }
    pub fn is_key_tip_mode(&self) -> bool {
        self.keyboard.navigator.is_active()
    }
    pub fn active_key_tips(&self) -> RibbonActiveKeyTips {
        self.keyboard.badge_view.clone()
    }
    pub fn current_key_tips(&self) -> RibbonCurrentKeyTips {
        if let Some(scope) = self.keyboard.navigator.current() {
            let entries = scope.entries();
            if !entries.is_empty() {
                return RibbonFixedItems::new(entries.to_vec()) as RibbonReadOnlyList<_>;
            }
        }
        self.keyboard.empty_tips.clone()
    }
    pub fn add_key_tip_mode_changed(&self, h: Option<MarkupDelegate>) {
        let old = self.keyboard.mode_changed.borrow().clone();
        self.keyboard
            .mode_changed
            .replace(MarkupDelegate::combine(old.as_ref(), h.as_ref()));
    }
    pub fn remove_key_tip_mode_changed(&self, h: Option<MarkupDelegate>) {
        let old = self.keyboard.mode_changed.borrow().clone();
        self.keyboard
            .mode_changed
            .replace(MarkupDelegate::remove(old.as_ref(), h.as_ref()));
    }
    pub fn add_search_requested(&self, h: Option<MarkupDelegate>) {
        let old = self.keyboard.search_requested.borrow().clone();
        self.keyboard
            .search_requested
            .replace(MarkupDelegate::combine(old.as_ref(), h.as_ref()));
    }
    pub fn remove_search_requested(&self, h: Option<MarkupDelegate>) {
        let old = self.keyboard.search_requested.borrow().clone();
        self.keyboard
            .search_requested
            .replace(MarkupDelegate::remove(old.as_ref(), h.as_ref()));
    }
    fn raise_key_tip_mode_changed(&self, value: bool) {
        let h = self.keyboard.mode_changed.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(Rc::new(value))])
                .expect("KeyTipModeChanged handler");
        }
    }
    pub fn show_key_tips(&self) {
        if !self.is_key_tips_enabled() || TopLevel::get_top_level(Some(self)).is_none() {
            return;
        }
        self.hide_key_tips();
        if let Some(backstage) = self.backstage().filter(|_| self.is_backstage_open()) {
            let targets = backstage.key_tip_targets();
            self.push_key_tip_scope(
                RibbonItemsSource::deferred(move || Box::new(targets.iter().flatten())),
                None,
            );
        } else {
            self.push_key_tip_scope(self.top_level_key_tip_targets(), None);
            self.move_focus_into_ribbon();
        }
        self.raise_key_tip_mode_changed(true);
    }
    fn move_focus_into_ribbon(&self) {
        let Some(window) = TopLevel::get_top_level(Some(self)) else {
            return;
        };
        let focused = window.focus_manager().get_focused_element();
        let target = self
            .selected_tab()
            .and_then(|t| t.header_element())
            .or_else(|| self.headers.borrow().first().cloned());
        let Some(target) = target else { return };
        if focused.as_ref().is_some_and(|f| f.ptr_eq(&target)) {
            return;
        }
        self.keyboard.focus_before.replace(focused);
        if target.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE) {
            self.keyboard.focus_target.replace(Some(target.upcast()));
        }
    }
    fn restore_focus_after_key_tips(&self, before: &InputElement, target: &Control) -> bool {
        if self.is_key_tip_mode() || before.visual_root().is_none() {
            return false;
        }
        TopLevel::get_top_level(Some(self))
            .and_then(|window| window.focus_manager().get_focused_element())
            .is_some_and(|focused| {
                focused.ptr_eq(&target.to_ref())
                    && before.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE)
            })
    }
    fn finish_key_tip_focus(&self, popup: Option<(Ref<PopupFlyoutBase>, u64)>) {
        let before = self.keyboard.focus_before.take();
        let target = self.keyboard.focus_target.take();
        let (Some(before), Some(target)) = (before, target) else {
            return;
        };
        let ribbon = self.to_ref().downgrade();
        post(move || {
            let Some(root) = ribbon.upgrade().filter(|root| !root.is_key_tip_mode()) else {
                return;
            };
            if root.restore_focus_after_key_tips(&before, &target) {
                return;
            }
            let Some((flyout, _)) = popup.filter(|(flyout, opening)| {
                let popup = flyout.popup();
                popup.opening_sequence() == Some(*opening)
                    && TopLevel::get_top_level(Some(&root))
                        .and_then(|window| window.focus_manager().get_focused_element())
                        .is_some_and(|focused| popup.is_inside_popup(&focused))
            }) else {
                return;
            };

            // The flyout owns focus until it closes. If its native teardown
            // returns to the temporary KeyTip header, restore the original
            // element after the application's Closed handlers have run.
            let before = before.downgrade();
            let target = target.downgrade();
            let subscription = Rc::new(RefCell::new(None));
            let once = subscription.clone();
            let weak_popup = flyout.popup().downgrade();
            subscription.replace(Some(flyout.closed(move || {
                drop(once.take());
                let (ribbon, before, target, popup) = (
                    ribbon.clone(),
                    before.clone(),
                    target.clone(),
                    weak_popup.clone(),
                );
                post(move || {
                    if let (Some(root), Some(before), Some(target)) =
                        (ribbon.upgrade(), before.upgrade(), target.upgrade())
                        && !popup
                            .upgrade()
                            .is_some_and(|popup| popup.opening_sequence().is_some())
                    {
                        root.restore_focus_after_key_tips(&before, &target);
                    }
                });
            })));
        });
    }
    pub fn cancel_key_tips(&self) {
        loop {
            let cleanup = self.keyboard.cleanup.borrow_mut().pop();
            match cleanup {
                Some(Some(cleanup)) => cleanup(),
                Some(None) => {}
                None => break,
            }
        }
        self.hide_key_tips();
    }
    pub fn hide_key_tips(&self) {
        self.hide_key_tips_after_opening(None);
    }
    fn hide_key_tips_after_opening(&self, popup: Option<(Ref<PopupFlyoutBase>, u64)>) {
        let active = self.is_key_tip_mode();
        self.keyboard.navigator.clear();
        let old = std::mem::take(&mut *self.keyboard.cleanup.borrow_mut());
        drop(old);
        self.clear_key_tip_badges();
        drop(self.keyboard.overlays.take());
        if active {
            self.finish_key_tip_focus(popup);
            self.raise_key_tip_mode_changed(false);
        }
    }
    fn top_level_key_tip_targets(&self) -> RibbonItemsSource<Ref<Control>> {
        let root = self.to_ref();
        RibbonItemsSource::deferred(move || {
            let r = root.clone();
            let mut phase = 0;
            let mut headers = None;
            let mut qat = None;
            let mut qat_items = None;
            let mut qat_started = false;
            let mut items = None;
            Box::new(std::iter::from_fn(move || {
                loop {
                    match phase {
                        0 => {
                            phase = 1;
                            let p = r.parts.borrow();
                            if let Some(app) = p.application.clone().filter(|a| a.is_visible())
                                && p.tab_row.as_ref().is_none_or(|row| row.is_visible())
                            {
                                return Some(app.upcast());
                            }
                        }
                        1 => {
                            if !qat_started {
                                qat_started = true;
                                qat = r.quick_access_tool_bar().filter(|q| {
                                    r.is_quick_access_visible() && q.get_visual_root().is_some()
                                });
                                qat_items = qat.as_ref().map(|q| q.items().iter());
                            }
                            if let Some(items) = qat_items.as_mut() {
                                for item in items.by_ref().flatten() {
                                    if item.is_visible() {
                                        return Some(item);
                                    }
                                }
                            }
                            // Retain the toolbar as the source iterator's captured local.
                            let _ = &qat;
                            phase = 2;
                            headers = Some(r.headers.borrow().clone().into_iter());
                        }
                        2 => {
                            if let Some(h) = headers.as_mut().and_then(Iterator::next) {
                                return Some(h.upcast());
                            }
                            phase = 3;
                            items = Some(Box::new(
                                r.tab_strip_items()
                                    .iter()
                                    .flat_map(|item| RibbonGroup::flatten(item).iter()),
                            ));
                        }
                        3 => return items.as_mut().and_then(Iterator::next),
                        _ => return None,
                    }
                }
            }))
        })
    }
    fn push_key_tip_scope(
        &self,
        targets: RibbonItemsSource<Ref<Control>>,
        cleanup: Option<Cleanup>,
    ) {
        let mut seen = HashSet::new();
        let list = targets
            .iter()
            .filter(|t| t.is_visible() && t.get_visual_root().is_some())
            .filter(|t| seen.insert(t.clone()))
            .collect::<Vec<_>>();
        let scope = RibbonKeyTipScope::new();
        let qat_items = self
            .quick_access_tool_bar()
            .map(|q| q.items().iter().flatten().collect::<HashSet<_>>())
            .unwrap_or_default();
        let qat_count = i32::try_from(list.iter().filter(|t| qat_items.contains(*t)).count())
            .expect("Int32 QAT targets");
        let qat_tips = RibbonKeyTipAssigner::assign_quick_access(qat_count);
        let mut reserved = Vec::new();
        let mut requests = Vec::new();
        let mut others = Vec::new();
        let mut qat_index = 0;
        for target in list {
            if qat_items.contains(&target) {
                let tip = self
                    .explicit_key_tip(&target)
                    .or_else(|| qat_tips.get(qat_index));
                qat_index += 1;
                reserved.push(tip.clone());
                scope.add(tip, Some(target));
            } else {
                requests.push(RibbonKeyTipRequest::new(
                    self.key_tip_label(&target),
                    self.explicit_key_tip(&target),
                ));
                others.push(target);
            }
        }
        let tips = RibbonKeyTipAssigner::assign(
            Some(&requests),
            Some(RibbonItemsSource::from_values(reserved)),
        );
        for (i, target) in others.into_iter().enumerate() {
            scope.add(tips.get(i), Some(target));
        }
        self.keyboard.navigator.push(Some(scope));
        self.keyboard.cleanup.borrow_mut().push(cleanup);
        let root = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || root.render_key_tip_badges(),
            // Native NORMAL work precedes layout. Badge bounds need the new
            // tab/popup's completed layout, as in the source dispatcher queue.
            DispatcherPriority::LOADED,
        );
    }
    fn backstage_tag(target: &Control) -> Option<Ref<super::super::RibbonBackstageItem>> {
        let button = target.to_ref().cast::<Button>()?;
        let value = button.tag()?;
        ValueTypes::as_object(&*value)?.cast::<super::super::RibbonBackstageItem>()
    }
    fn explicit_key_tip(&self, target: &Control) -> Option<RibbonString> {
        if let Some(tip) = RibbonKeyTip::get_key_tip(target).filter(|t| !t.is_empty()) {
            return text(Some(tip));
        }
        if self
            .application_button()
            .is_some_and(|b| b.ptr_eq(&target.to_ref()))
        {
            return text(self.application_button_key_tip());
        }
        if let Some(header) = target.to_ref().cast::<RibbonTabHeader>() {
            let tab = header.tab();
            return text(tab.key_tip()).or_else(|| Self::contextual_key_tip(&tab));
        }
        if let Some(tip) = cast_item(target)
            .and_then(|i| i.key_tip())
            .filter(|t| !t.is_empty())
        {
            return Some(tip);
        }
        if let Some(item) = Self::backstage_tag(target) {
            return text(item.key_tip());
        }
        self.group_of_collapsed_button(target)
            .and_then(|g| text(g.key_tip()))
    }
    fn contextual_key_tip(tab: &RibbonTab) -> Option<RibbonString> {
        let prefix = tab
            .contextual_group()?
            .key_tip()
            .filter(|p| !p.is_empty())?;
        let letters = RibbonKeyTipAssigner::normalize(text(tab.header()).as_ref());
        let first = *letters.as_utf16().first()?;
        let mut units = prefix.encode_utf16().collect::<Vec<_>>();
        units.push(first);
        Some(RibbonString::from_utf16(units))
    }
    fn all_groups(&self) -> RibbonItemsSource<Ref<RibbonGroup>> {
        let tabs = self.tabs();
        RibbonItemsSource::deferred(move || {
            Box::new(
                tabs.iter()
                    .flat_map(|tab| {
                        let tab = tab.expect("null tab");
                        tab.groups().iter().inspect(move |_| {
                            let _owner = &tab;
                        })
                    })
                    .map(|group| group.expect("null group")),
            )
        })
    }
    fn group_of_collapsed_button(&self, target: &Control) -> Option<Ref<RibbonGroup>> {
        self.all_groups().iter().find(|g| {
            g.collapsed_button()
                .is_some_and(|b| b.ptr_eq(&target.to_ref()))
        })
    }
    fn group_of_launcher(&self, target: &Control) -> Option<Ref<RibbonGroup>> {
        self.all_groups().iter().find(|g| {
            g.dialog_launcher()
                .is_some_and(|b| b.ptr_eq(&target.to_ref()))
                || g.popup_launcher()
                    .is_some_and(|b| b.ptr_eq(&target.to_ref()))
        })
    }
    fn group_of_slide_button(&self, target: &Control) -> Option<Ref<RibbonGroup>> {
        self.all_groups().iter().find(|g| {
            g.slide_out_button()
                .is_some_and(|b| b.ptr_eq(&target.to_ref()))
        })
    }
    fn key_tip_label(&self, target: &Control) -> Option<RibbonString> {
        if self
            .application_button()
            .is_some_and(|b| b.ptr_eq(&target.to_ref()))
        {
            return text(self.application_button_label())
                .or_else(|| RibbonStrings::current().file());
        }
        if let Some(g) = self.group_of_launcher(target) {
            return Some(format!("{} launcher", g.header().unwrap_or_default()).into());
        }
        if let Some(g) = self.group_of_slide_button(target) {
            return Some(format!("{} more", g.header().unwrap_or_default()).into());
        }
        if let Some(entry) = target.to_ref().cast::<MenuFlyoutItem>() {
            return text(entry.text());
        }
        if let Some(entry) = target.to_ref().cast::<MenuFlyoutSubItem>() {
            return text(entry.text());
        }
        if let Some(header) = target.to_ref().cast::<RibbonTabHeader>() {
            return text(header.tab().header());
        }
        if let Some(item) = cast_item(target) {
            return item.label().or_else(|| item.id());
        }
        if let Some(item) = Self::backstage_tag(target) {
            return text(item.header());
        }
        text(AutomationProperties::get_name(target))
    }
    /// Metadata's System.Char is a Rust scalar in the selected SDK. Keep the
    /// source method signature for representable BMP values, reject supplementary
    /// scalars, and expose the lossless UTF-16-unit Rust API separately.
    pub fn process_key_tip_char(&self, ch: char) -> Result<bool, String> {
        let unit = u16::try_from(ch as u32)
            .map_err(|_| "System.Char requires one UTF-16 code unit".to_string())?;
        Ok(self.process_key_tip_input(unit))
    }
    pub fn process_key_tip_input(&self, ch: u16) -> bool {
        let Some(scope) = self.keyboard.navigator.current() else {
            return false;
        };
        let (kind, target) = scope.process(ch);
        match (kind, target) {
            (RibbonKeyTipMatch::Partial, _) => {
                self.update_key_tip_badge_visibility();
                true
            }
            (RibbonKeyTipMatch::Complete, Some(target)) => {
                self.activate_key_tip(&target);
                true
            }
            _ => {
                self.update_key_tip_badge_visibility();
                false
            }
        }
    }
    pub fn pop_key_tip_level(&self) -> bool {
        let cleanup = self.keyboard.cleanup.borrow_mut().pop().flatten();
        if let Some(cleanup) = cleanup {
            cleanup();
        }
        if !self.keyboard.navigator.pop() {
            self.hide_key_tips();
            return false;
        }
        let root = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || root.render_key_tip_badges(),
            // Native NORMAL work precedes layout. Badge bounds need the new
            // tab/popup's completed layout, as in the source dispatcher queue.
            DispatcherPriority::LOADED,
        );
        true
    }
    fn handle_key_tip_key(&self, e: &KeyEventArgs) {
        e.set_handled(true);
        match e.key {
            Key::Escape => {
                self.pop_key_tip_level();
            }
            Key::F10 => self.cancel_key_tips(),
            Key::Back => {
                if let Some(scope) = self.keyboard.navigator.current() {
                    scope.backspace();
                }
                self.update_key_tip_badge_visibility();
            }
            key => {
                if let Some(ch) = key_to_char(key) {
                    self.process_key_tip_input(ch);
                }
            }
        }
    }
    /// Activate a concrete target through the existing KeyTip state machine.
    /// The native demo capture helper uses this typed Rust entry point in place
    /// of the original C# helper's reflection call to this private method.
    pub fn activate_key_tip(&self, target: &Control) {
        if !target.is_effectively_enabled() {
            return;
        }
        if self
            .application_button()
            .is_some_and(|b| b.ptr_eq(&target.to_ref()))
        {
            self.clear_key_tip_badges();
            self.invoke_application_button();
            if let Some(backstage) = self.backstage().filter(|_| self.is_backstage_open()) {
                self.keyboard.navigator.clear();
                self.keyboard.cleanup.borrow_mut().clear();
                let targets = backstage.key_tip_targets();
                self.push_key_tip_scope(
                    RibbonItemsSource::deferred(move || Box::new(targets.iter().flatten())),
                    None,
                );
            } else {
                self.hide_key_tips();
            }
            return;
        }
        if let Some(header) = target.to_ref().cast::<RibbonTabHeader>() {
            self.set_selected_tab(Some(header.tab()));
            let mut cleanup: Option<Cleanup> = None;
            if self.visibility_mode() == RibbonVisibilityMode::TabsOnly {
                self.open_minimized_popup();
                let weak = self.to_ref().downgrade();
                cleanup = Some(Rc::new(move || {
                    if let Some(r) = weak.upgrade() {
                        r.close_minimized_popup();
                    }
                }));
            } else if self.visibility_mode() == RibbonVisibilityMode::FullScreen
                && !self.is_full_screen_revealed()
            {
                self.set_is_full_screen_revealed(true);
                let weak = self.to_ref().downgrade();
                cleanup = Some(Rc::new(move || {
                    if let Some(r) = weak.upgrade() {
                        r.set_is_full_screen_revealed(false);
                    }
                }));
            }
            self.clear_key_tip_badges();
            let root = self.to_ref();
            post(move || root.push_key_tip_scope(header.tab().get_key_tip_targets(), cleanup));
            return;
        }
        if let Some(g) = self.group_of_slide_button(target) {
            g.open_slide_out();
            self.clear_key_tip_badges();
            let root = self.to_ref();
            post(move || {
                let group = g.clone();
                root.push_key_tip_scope(
                    g.get_slide_out_key_tip_targets(),
                    Some(Rc::new(move || {
                        if !group.is_slide_out_pinned() {
                            group.close_slide_out();
                        }
                    })),
                );
            });
            return;
        }
        if let Some(g) = self.group_of_collapsed_button(target) {
            g.open_popup();
            self.clear_key_tip_badges();
            let root = self.to_ref();
            post(move || {
                let group = g.clone();
                root.push_key_tip_scope(
                    g.get_key_tip_targets(),
                    Some(Rc::new(move || group.close_popup())),
                );
            });
            return;
        }
        if let Some(g) = self.group_of_launcher(target) {
            self.hide_key_tips();
            g.open_dialog_launcher().expect("KeyTip launcher handler");
            return;
        }
        if let Some(item) = Self::backstage_tag(target)
            && let Some(owner) = self.backstage()
        {
            self.hide_key_tips();
            owner.invoke(Some(item));
            return;
        }
        if let Some(item) = cast_item(target) {
            let previous_opening = popup_flyout(flyout_for(target))
                .and_then(|flyout| flyout.popup().opening_sequence());
            let result = item.on_key_tip();
            if let Some(next) = result.next_scope() {
                self.clear_key_tip_badges();
                let root = self.to_ref();
                post(move || {
                    let targets = next();
                    root.push_key_tip_scope(
                        RibbonItemsSource::deferred(move || {
                            Box::new(
                                targets
                                    .iter()
                                    .map(|target| target.expect("null NextScope target")),
                            )
                        }),
                        None,
                    )
                });
                return;
            }
            let flyout = flyout_for(target);
            if let Some(menu) = flyout.as_ref().and_then(|flyout| flyout.clone().cast())
                && self.continue_key_tips_in_menu(&menu)
            {
                return;
            }
            let popup = popup_flyout(flyout).and_then(|flyout| {
                let popup = flyout.popup();
                let opening = popup.opening_sequence()?;
                (Some(opening) != previous_opening
                    && popup
                        .placement_target()
                        .is_some_and(|owner| owner.ptr_eq(&target.to_ref())))
                .then_some((flyout, opening))
            });
            self.hide_key_tips_after_opening(popup);
            return;
        }
        if target.is::<MenuFlyoutItem>() || target.is::<MenuFlyoutSubItem>() {
            self.hide_key_tips();
            invoke_menu_entry(target);
            return;
        }
        self.hide_key_tips();
        if let Some(button) = target.to_ref().cast::<Button>() {
            ferroui_controls::automation::peers::ButtonAutomationPeer::new(&button)
                .invoke()
                .expect("KeyTip button invocation");
        } else {
            target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        }
    }
    fn continue_key_tips_in_menu(&self, menu: &MenuFlyout) -> bool {
        let entries = menu
            .items()
            .iter()
            .flatten()
            .filter_map(|v| ValueTypes::as_object(&*v).and_then(|o| o.cast::<Control>()))
            .filter(|i| i.is::<MenuFlyoutItem>() || i.is::<MenuFlyoutSubItem>())
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return false;
        }
        self.clear_key_tip_badges();
        let weak = self.to_ref().downgrade();
        let weak_menu = menu.to_ref().downgrade();
        let push = Rc::new(move || {
            if let (Some(root), Some(menu)) = (weak.upgrade(), weak_menu.upgrade()) {
                if !root.is_key_tip_mode() {
                    return;
                }
                // Native overlay popups share the window visual root; the
                // presenter's subtree is the source popup-root equivalent.
                root.attach_popup_keyboard(menu.popup().child().as_deref());
                let depth = root.keyboard.navigator.depth();
                let subscription = Rc::new(RefCell::new(
                    None::<Rc<dyn ferroui_base::reactive::IDisposable>>,
                ));
                let token = subscription.clone();
                let weak = root.downgrade();
                let handle = menu.closed(move || {
                    let handle = token.borrow_mut().take();
                    if let Some(handle) = handle {
                        handle.dispose();
                    }
                    if let Some(root) = weak.upgrade()
                        && root.is_key_tip_mode()
                        && root.keyboard.navigator.depth() == depth + 1
                    {
                        root.hide_key_tips();
                    }
                });
                subscription.replace(Some(handle.detach()));
                let menu = menu.clone();
                let entries = entries.clone();
                root.push_key_tip_scope(
                    RibbonItemsSource::deferred(move || {
                        Box::new(entries.clone().into_iter().filter(|item| item.is_visible()))
                    }),
                    Some(Rc::new(move || menu.hide())),
                );
            }
        });
        if menu.is_open() {
            post(move || push());
        } else {
            let token = Rc::new(RefCell::new(
                None::<Rc<dyn ferroui_base::reactive::IDisposable>>,
            ));
            let next = token.clone();
            let handle = menu.opened(move || {
                let handle = next.borrow_mut().take();
                if let Some(handle) = handle {
                    handle.dispose();
                }
                let push = push.clone();
                post(move || push());
            });
            token.replace(Some(handle.detach()));
        }
        true
    }
    fn clear_key_tip_badges(&self) {
        let overlays = self.keyboard.overlays.borrow().clone();
        for overlay in overlays {
            overlay.canvas.children().clear();
        }
        self.keyboard.badges.clear();
    }
    fn render_key_tip_badges(&self) {
        let (Some(scope), Some(window)) = (
            self.keyboard.navigator.current(),
            TopLevel::get_top_level(Some(self)),
        ) else {
            return;
        };
        self.clear_key_tip_badges();
        let x11 = window
            .platform_impl()
            .and_then(|platform| platform.handle())
            .is_some_and(|handle| handle.handle_descriptor() == Some("XID"));
        let mut previous = self.keyboard.overlays.take();
        let mut overlays: Vec<Rc<BadgeOverlay>> = Vec::new();
        for entry in scope.entries().iter() {
            let target = entry.target.expect("null scope target");
            if target.get_visual_root().is_none()
                || target.bounds().width <= 0.
                || !target.is_visible()
            {
                continue;
            }
            // An X11 native popup needs a compositor to show transparency.
            // Put passive badges in the target's own overlay layer, including
            // targets inside a detached menu or group popup.
            let overlay_owner = x11
                .then(|| TopLevel::get_top_level(Some(&target)))
                .flatten()
                .filter(|owner| {
                    // Window enables this in its template. A native PopupRoot
                    // has its own layer manager, with popup overlays off.
                    if let Some(layers) = owner.visual_layer_manager() {
                        layers.set_enable_popup_overlay_layer(true);
                    }
                    PopupOverlayLayer::get_popup_overlay_layer(owner).is_some()
                });
            let owner = overlay_owner.as_ref().unwrap_or(&window);
            let Some(bounds) = bounds_in_window(&target, owner) else {
                continue;
            };
            let overlay = if let Some(overlay) = overlays.iter().find(|o| o.owner.points_to(owner))
            {
                overlay
            } else {
                let overlay = previous
                    .iter()
                    .position(|o| {
                        o.owner.points_to(owner) && o.in_window == overlay_owner.is_some()
                    })
                    .map(|index| previous.remove(index))
                    .unwrap_or_else(|| Rc::new(BadgeOverlay::new(owner, overlay_owner.is_some())));
                overlay.canvas.set_width(owner.bounds().width);
                overlay.canvas.set_height(owner.bounds().height);
                overlay.canvas.set_value(
                    ThemeVariant::requested_theme_variant_property(),
                    self.actual_theme_variant(),
                );
                overlay
                    .popup
                    .set_popup_parent(Some(&self.to_ref().upcast::<Control>()));
                overlays.push(overlay);
                overlays.last().expect("inserted KeyTip overlay")
            };
            let badge = RibbonKeyTip::new();
            // Source off-tree Measure resolves DefaultStyleKey in application
            // resources; native detached elements have no global style parent.
            if let Some(theme) = ferroui_controls::Application::current()
                .and_then(|app| {
                    app.resources().try_get_resource(
                        &<RibbonKeyTip as ferroui_base::StaticType>::TYPE.into(),
                        None,
                    )
                })
                .flatten()
                .and_then(|v| {
                    ValueTypes::as_object(&*v)
                        .and_then(|o| o.cast::<ferroui_base::styling::ControlTheme>())
                })
            {
                badge.set_theme(theme);
            }
            badge.set_text(
                entry
                    .tip
                    .map(|s| s.to_utf8().expect("native UTF-16 KeyTip text")),
            );
            badge.target.replace(Some(target.clone()));
            badge.set_opacity(if target.is_effectively_enabled() {
                1.
            } else {
                0.5
            });
            badge.measure(Size::new(f64::INFINITY, f64::INFINITY));
            let size = badge.desired_size();
            let large = target.is::<RibbonTabHeader>()
                || (cast_item(&target).is_some_and(|i| i.current_size() == RibbonItemSize::Large)
                    && !target.is::<RibbonItemsContainer>()
                    && !simplified_item(&target))
                || self.group_of_collapsed_button(&target).is_some()
                || self
                    .quick_access_tool_bar()
                    .is_some_and(|qat| qat.items().contains(&Some(target.clone())));
            let point = badge_position(bounds, size, owner.bounds().size(), large);
            Canvas::set_left(&badge, point.x);
            Canvas::set_top(&badge, point.y);
            overlay.canvas.children().add(badge.clone());
            self.keyboard.badges.add(Some(badge));
        }
        drop(previous);
        self.keyboard.overlays.replace(overlays.clone());
        for overlay in overlays {
            overlay.popup.set_is_open(true);
        }
        self.update_key_tip_badge_visibility();
    }
    fn update_key_tip_badge_visibility(&self) {
        let typed = self
            .keyboard
            .navigator
            .current()
            .map(|s| s.typed())
            .unwrap_or_default();
        for badge in self.keyboard.badges.iter() {
            let badge = badge.expect("null badge");
            badge.set_is_visible(
                badge
                    .text()
                    .expect("null badge Text")
                    .encode_utf16()
                    .collect::<Vec<_>>()
                    .starts_with(typed.as_utf16()),
            );
        }
    }
    pub(super) fn on_root_key_down(&self, e: &KeyEventArgs) {
        if matches!(e.key, Key::LeftAlt | Key::RightAlt) {
            self.keyboard.alt_alone.set(true);
            return;
        }
        self.keyboard.alt_alone.set(false);
        if self.is_key_tip_mode() {
            self.handle_key_tip_key(e);
            return;
        }
        if e.handled() || !self.is_effectively_visible_visibility() {
            return;
        }
        if e.key == Key::F1 && e.key_modifiers == KeyModifiers::CONTROL && self.is_collapsible() {
            self.toggle_minimized();
            e.set_handled(true);
            return;
        }
        if e.key == Key::F10 && e.key_modifiers == KeyModifiers::NONE && self.is_key_tips_enabled()
        {
            self.show_key_tips();
            e.set_handled(true);
            return;
        }
        if e.key == Key::Q && e.key_modifiers == KeyModifiers::ALT {
            let h = self.keyboard.search_requested.borrow().clone();
            if let Some(h) = h {
                h.try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(ferroui_base::utilities::EventArgs::EMPTY)),
                ])
                .expect("SearchRequested handler");
                e.set_handled(true);
                return;
            }
        }
        if e.key == Key::Escape && self.is_backstage_open() {
            self.set_is_backstage_open(false);
            e.set_handled(true);
            return;
        }
        self.on_shortcut_key_down(e);
    }
    pub(super) fn on_root_key_up(&self, e: &KeyEventArgs) {
        if matches!(e.key, Key::LeftAlt | Key::RightAlt)
            && self.keyboard.alt_alone.get()
            && self.is_key_tips_enabled()
            && !e.handled()
        {
            self.keyboard.alt_alone.set(false);
            if self.is_key_tip_mode() {
                self.cancel_key_tips();
                e.set_handled(true);
            } else if self.is_loaded() && self.is_effectively_visible_visibility() {
                self.show_key_tips();
                e.set_handled(true);
            }
        }
    }
    pub(super) fn on_key_tip_pointer_pressed(&self) {
        self.keyboard.alt_alone.set(false);
        if self.is_key_tip_mode() {
            self.cancel_key_tips();
        }
    }
    pub(crate) fn attach_popup_keyboard(&self, child: Option<&Control>) {
        let Some(child) = child else { return };
        if self
            .keyboard
            .popup_hooks
            .borrow()
            .iter()
            .any(|h| h.root.points_to(&child.to_ref()))
        {
            return;
        }
        let weak = self.to_ref().downgrade();
        let down = child.add_handler_with(
            InputElement::key_down_event(),
            move |_, e| {
                if let Some(root) = weak.upgrade() {
                    root.on_root_key_down(e);
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        let weak = self.to_ref().downgrade();
        let up = child.add_handler_with(
            InputElement::key_up_event(),
            move |_, e| {
                if let Some(root) = weak.upgrade() {
                    root.on_root_key_up(e);
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        self.keyboard.popup_hooks.borrow_mut().push(PopupKeyHook {
            root: child.to_ref().downgrade(),
            down,
            up,
        });
    }
    pub(super) fn detach_popup_keyboard(&self) {
        let hooks = std::mem::take(&mut *self.keyboard.popup_hooks.borrow_mut());
        drop(hooks);
    }
}
fn key_to_char(key: Key) -> Option<u16> {
    if (Key::A..=Key::Z).contains(&key) {
        Some(b'A' as u16 + (key as u16 - Key::A as u16))
    } else if (Key::D0..=Key::D9).contains(&key) {
        Some(b'0' as u16 + (key as u16 - Key::D0 as u16))
    } else if (Key::NumPad0..=Key::NumPad9).contains(&key) {
        Some(b'0' as u16 + (key as u16 - Key::NumPad0 as u16))
    } else {
        None
    }
}
fn simplified_item(target: &Control) -> bool {
    target
        .to_ref()
        .cast::<RibbonButton>()
        .is_some_and(|b| b.is_simplified())
        || target
            .to_ref()
            .cast::<RibbonToggleButton>()
            .is_some_and(|b| b.is_simplified())
        || target
            .to_ref()
            .cast::<RibbonControlBase>()
            .is_some_and(|b| b.is_simplified())
}
fn flyout_for(target: &Control) -> Option<Ref<FlyoutBase>> {
    target
        .to_ref()
        .cast::<Button>()
        .and_then(|b| b.flyout())
        .or_else(|| {
            target
                .to_ref()
                .cast::<RibbonSplitButton>()
                .and_then(|b| b.flyout())
        })
}
fn popup_flyout(flyout: Option<Ref<FlyoutBase>>) -> Option<Ref<PopupFlyoutBase>> {
    flyout
        .filter(|flyout| flyout.is_open())
        .and_then(|flyout| flyout.cast())
}
fn invoke_menu_entry(target: &Control) {
    if !target.is_effectively_enabled() {
        return;
    }
    if let Some(item) = target.to_ref().cast::<MenuFlyoutItem>() {
        // The native peer runs the menu interaction handler, including closure
        // after the adapter's source click/command/toggle invocation.
        ferroui_controls::automation::peers::MenuItemAutomationPeer::new(&item)
            .invoke()
            .expect("KeyTip menu invocation");
    } else if target.is::<MenuFlyoutSubItem>() {
        target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
    }
}

#[cfg(test)]
#[path = "../keyboard_tests.rs"]
mod tests;
fn bounds_in_window(target: &Control, window: &TopLevel) -> Option<Rect> {
    let rect = Rect::new(0., 0., target.bounds().width, target.bounds().height);
    if let Some(matrix) = target.transform_to_visual(window) {
        return Some(rect.transform_to_aabb(matrix));
    }
    // Popup roots share a source XamlRoot but native presentation sources differ.
    // Translate their root origins through screen pixels; retain fractional
    // target bounds inside the root instead of rounding every target point.
    let mut root = target.to_ref().upcast::<Visual>();
    while let Some(parent) = root.visual_parent() {
        root = parent;
    }
    let matrix = target.transform_to_visual(&root)?;
    let origin = window.point_to_client(root.point_to_screen(Point::default()));
    let bounds = rect.transform_to_aabb(matrix);
    Some(Rect::new(
        bounds.x + origin.x,
        bounds.y + origin.y,
        bounds.width,
        bounds.height,
    ))
}

// Original RenderBadges placement and clamping, isolated only to compare exact
// fractional coordinates against the unchanged C# whole-method reference.
fn badge_position(bounds: Rect, size: Size, window: Size, large: bool) -> Point {
    use ferroui_base::utilities::{MathUtilities, math_utilities::max};
    let (x, y) = if large {
        (
            bounds.x + (bounds.width - size.width) / 2.,
            bounds.bottom() - size.height / 2. - 2.,
        )
    } else {
        (bounds.x + 4., bounds.y + bounds.height / 2. - 1.)
    };
    Point::new(
        MathUtilities::clamp(x, 0., max(0., window.width - size.width)),
        MathUtilities::clamp(y, 0., max(0., window.height - size.height)),
    )
}
