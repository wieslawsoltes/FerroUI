//! Ribbon.QuickAccess.cs on the same native Ribbon, including model/state and native
//! customization entry integration.
use super::*;
use crate::compat::{MenuFlyoutItem, MenuFlyoutSeparator, ToggleMenuFlyoutItem};
use crate::controls::ribbon_quick_access_tool_bar::as_item;
use crate::controls::{RibbonItemHelper, RibbonMenu, RibbonQuickAccessToolBar};
use crate::metadata::quick_access_collections::{
    RibbonQuickAccessCandidates, RibbonQuickAccessItemIds,
};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonQuickAccessPosition, collections::RibbonReadOnlyList, localization::RibbonStrings,
};
use ferroui_controls::{Border, MenuFlyout, presenters::ContentPresenter};
pub(super) struct QuickAccessState {
    candidates: RibbonQuickAccessCandidates,
    defaults: RefCell<Option<RibbonQuickAccessItemIds>>,
    default_position: Cell<RibbonQuickAccessPosition>,
    changed: RefCell<Option<MarkupDelegate>>,
}
impl Default for QuickAccessState {
    fn default() -> Self {
        Self {
            candidates: RibbonQuickAccessCandidates::new_managed(),
            defaults: RefCell::default(),
            default_position: Cell::new(RibbonQuickAccessPosition::AboveRibbon),
            changed: RefCell::default(),
        }
    }
}
ferroui_base::ferro_thread_local! {static EMPTY_IDS:RibbonQuickAccessItemIds=RibbonFixedItems::new(vec![]) as RibbonReadOnlyList<_>;}
impl Ribbon {
    pub fn quick_access_tool_bar(&self) -> Option<Ref<RibbonQuickAccessToolBar>> {
        self.get_value(Self::quick_access_tool_bar_property())
    }
    pub fn set_quick_access_tool_bar(&self, v: Option<Ref<RibbonQuickAccessToolBar>>) {
        self.set_value(Self::quick_access_tool_bar_property(), v)
    }
    pub fn quick_access_position(&self) -> RibbonQuickAccessPosition {
        self.get_value(Self::quick_access_position_property())
    }
    pub fn set_quick_access_position(&self, v: RibbonQuickAccessPosition) {
        self.set_value(Self::quick_access_position_property(), v)
    }
    pub fn is_quick_access_visible(&self) -> bool {
        self.get_value(Self::is_quick_access_visible_property())
    }
    pub fn set_is_quick_access_visible(&self, v: bool) {
        self.set_value(Self::is_quick_access_visible_property(), v)
    }
    pub fn show_quick_access_labels(&self) -> bool {
        self.get_value(Self::show_quick_access_labels_property())
    }
    pub fn set_show_quick_access_labels(&self, v: bool) {
        self.set_value(Self::show_quick_access_labels_property(), v)
    }
    pub fn is_quick_access_hosted_externally(&self) -> bool {
        self.get_value(Self::is_quick_access_hosted_externally_property())
    }
    pub fn set_is_quick_access_hosted_externally(&self, v: bool) {
        self.set_value(Self::is_quick_access_hosted_externally_property(), v)
    }
    pub fn quick_access_candidate_ids(&self) -> RibbonQuickAccessCandidates {
        self.quick_access.candidates.clone()
    }
    pub fn default_quick_access_item_ids(&self) -> Option<RibbonQuickAccessItemIds> {
        self.quick_access.defaults.borrow().clone()
    }
    pub fn default_quick_access_position(&self) -> RibbonQuickAccessPosition {
        self.quick_access.default_position.get()
    }
    pub(super) fn capture_quick_access_defaults(&self) {
        if self.quick_access.defaults.borrow().is_none() {
            let ids = self.get_quick_access_item_ids();
            self.quick_access.defaults.replace(Some(ids));
            self.quick_access
                .default_position
                .set(self.quick_access_position());
        }
    }
    pub(super) fn on_quick_access_tool_bar_changed(
        &self,
        old: Option<Ref<RibbonQuickAccessToolBar>>,
        new: Option<Ref<RibbonQuickAccessToolBar>>,
    ) {
        if let Some(old) = &old {
            old.set_ribbon(None);
        }
        if let Some(new) = &new {
            new.set_ribbon(Some(self.to_ref()));
            for item in new.items().iter() {
                RibbonItemHelper::set_owner(
                    &item.expect("ArgumentNull: QAT item"),
                    Some(self.item_owner()),
                )
                .expect("QAT replacement owner");
            }
        }
        self.update_quick_access_placement();
        self.apply_quick_access_layout();
        if old.is_some() || new.is_some_and(|q| q.items().count() > 0) {
            self.on_quick_access_items_changed();
        }
    }
    pub(super) fn on_quick_access_options_changed(&self) {
        self.update_quick_access_placement();
        self.apply_quick_access_layout();
        self.raise_state_changed();
        self.on_model_quick_access_changed();
        self.raise_quick_access_changed();
    }
    pub(crate) fn on_quick_access_items_changed(&self) {
        self.sync_quick_access_to_model();
        self.invalidate_shortcuts();
        self.raise_state_changed();
        self.raise_quick_access_changed();
    }
    pub fn add_quick_access_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.quick_access.changed.borrow().as_ref(), h.as_ref());
        self.quick_access.changed.replace(next);
    }
    pub fn remove_quick_access_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.quick_access.changed.borrow().as_ref(), h.as_ref());
        self.quick_access.changed.replace(next);
    }
    fn raise_quick_access_changed(&self) {
        let h = self.quick_access.changed.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                Some(Rc::new(ferroui_base::utilities::EventArgs::EMPTY)),
            ])
            .expect("QuickAccessChanged handler");
        }
    }
    pub(super) fn apply_quick_access_layout(&self) {
        if let Some(q) = self.quick_access_tool_bar() {
            q.set_metrics(Some(self.metrics()));
            q.set_show_labels(self.show_quick_access_labels());
            q.apply_layouts();
        }
    }
    pub(crate) fn update_quick_access_placement(&self) {
        let (Some(q), Some(above), Some(below)) = (
            self.quick_access_tool_bar(),
            self.parts.borrow().quick_access_above.clone(),
            self.parts.borrow().quick_access_below.clone(),
        ) else {
            return;
        };
        let target = if !self.is_quick_access_visible()
            || (self.visibility_mode() == RibbonVisibilityMode::FullScreen
                && !self.is_full_screen_revealed())
        {
            None
        } else if self.quick_access_position() == RibbonQuickAccessPosition::AboveRibbon {
            (!self.is_quick_access_hosted_externally()).then_some(above.clone())
        } else {
            Some(below.clone())
        };
        for host in [above, below] {
            if target.as_ref() != Some(&host) && host.child().is_some_and(|c| c.ptr_eq(&q)) {
                RibbonQuickAccessToolBar::detach_from_border(&host);
            }
            host.set_is_visible(target.as_ref() == Some(&host));
        }
        if let Some(target) = target
            && !target.child().is_some_and(|c| c.ptr_eq(&q))
        {
            if let Some(p) = q.visual_parent().and_then(|p| p.cast::<Border>()) {
                RibbonQuickAccessToolBar::detach_from_border(&p);
            } else if let Some(p) = q.visual_parent().and_then(|p| p.cast::<ContentPresenter>()) {
                p.set_content(None);
            }
            target.set_child(q);
        }
        self.apply_quick_access_layout();
    }
    pub fn find_quick_access_copy(&self, item: Option<Ref<Control>>) -> Option<Ref<Control>> {
        self.quick_access_tool_bar()?
            .items()
            .iter()
            .flatten()
            .find(|i| Some(i) == item.as_ref() || RibbonItemHelper::get_source_item(i) == item)
    }
    pub fn is_in_quick_access(&self, item: &Control) -> bool {
        self.find_quick_access_copy(Some(
            RibbonItemHelper::get_source_item(item).unwrap_or_else(|| item.to_ref()),
        ))
        .is_some()
            || self
                .quick_access_tool_bar()
                .is_some_and(|q| q.items().contains(&Some(item.to_ref())))
    }
    pub fn add_to_quick_access(&self, item: &Control) -> bool {
        let source = RibbonItemHelper::get_source_item(item).unwrap_or_else(|| item.to_ref());
        if self.quick_access_tool_bar().is_none()
            || self.is_in_quick_access(&source)
            || !as_item(&source).is_some_and(|i| i.can_add_to_quick_access())
        {
            return false;
        }
        let Some(copy) = self.create_quick_access_copy(&source) else {
            return false;
        };
        self.quick_access_tool_bar()
            .expect("NullReference: QAT replaced during copy creation")
            .items()
            .add(Some(copy));
        true
    }
    pub(super) fn create_quick_access_copy(&self, source: &Control) -> Option<Ref<Control>> {
        let item = as_item(source).filter(|i| i.can_add_to_quick_access())?;
        let copy = item.create_linked_copy()?;
        RibbonItemHelper::set_source_item(&copy, source.to_ref());
        RibbonItemHelper::set_owner(&copy, Some(self.item_owner())).expect("QAT copy owner");
        Some(copy)
    }
    pub fn add_to_quick_access_id(&self, id: Option<RibbonString>) -> bool {
        self.find_item(id)
            .is_some_and(|i| self.add_to_quick_access(&i))
    }
    pub fn remove_from_quick_access(&self, item: &Control) -> bool {
        let Some(q) = self.quick_access_tool_bar() else {
            return false;
        };
        let target = if q.items().contains(&Some(item.to_ref())) {
            Some(item.to_ref())
        } else {
            self.find_quick_access_copy(Some(
                RibbonItemHelper::get_source_item(item).unwrap_or_else(|| item.to_ref()),
            ))
        };
        target.is_some_and(|t| {
            self.quick_access_tool_bar()
                .expect("NullReference: QAT replaced during lookup")
                .items()
                .remove(&Some(t))
        })
    }
    pub fn get_quick_access_item_ids(&self) -> RibbonQuickAccessItemIds {
        let ids = self
            .quick_access_tool_bar()
            .into_iter()
            .flat_map(|q| q.items().iter())
            .flatten()
            .filter_map(|i| {
                as_item(&RibbonItemHelper::get_source_item(&i).unwrap_or(i)).and_then(|i| i.id())
            })
            .map(Some)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            EMPTY_IDS.with(Clone::clone)
        } else {
            RibbonFixedItems::new(ids) as RibbonReadOnlyList<_>
        }
    }
    pub fn toggle_quick_access_position(&self) {
        self.set_quick_access_position(
            if self.quick_access_position() == RibbonQuickAccessPosition::AboveRibbon {
                RibbonQuickAccessPosition::BelowRibbon
            } else {
                RibbonQuickAccessPosition::AboveRibbon
            },
        );
    }
    pub fn build_quick_access_customize_menu(&self) -> Ref<MenuFlyout> {
        let strings = RibbonStrings::current();
        let menu = MenuFlyout::new();
        let title = MenuFlyoutItem::new();
        title.set_text(utf8(strings.customize_quick_access_toolbar()));
        title.set_is_enabled(false);
        menu.items().add(Some(Control::boxed(title)));
        let mut candidates = Vec::new();
        for candidate in self
            .quick_access_candidate_ids()
            .iter()
            .filter_map(|id| self.find_item(id))
            .chain(
                self.quick_access_tool_bar()
                    .into_iter()
                    .flat_map(|q| q.items().iter())
                    .flatten()
                    .map(|i| RibbonItemHelper::get_source_item(&i).unwrap_or(i)),
            )
        {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
        for candidate in &candidates {
            let Some(item) = as_item(candidate) else {
                continue;
            };
            let toggle = ToggleMenuFlyoutItem::new();
            toggle.set_text(utf8(Some(
                item.label().or_else(|| item.id()).unwrap_or_default(),
            )));
            toggle.set_is_checked(self.is_in_quick_access(candidate));
            let weak = self.to_ref().downgrade();
            let candidate = candidate.clone();
            toggle.add_click_action(move |sender, _| {
                if let Some(root) = weak.upgrade() {
                    let checked = sender
                        .to_ref()
                        .cast::<ToggleMenuFlyoutItem>()
                        .expect("QAT menu toggle sender")
                        .is_checked();
                    if checked {
                        root.add_to_quick_access(&candidate);
                    } else {
                        root.remove_from_quick_access(&candidate);
                    }
                }
            });
            menu.items().add(Some(Control::boxed(toggle)));
        }
        if !candidates.is_empty() {
            menu.items()
                .add(Some(Control::boxed(MenuFlyoutSeparator::new())));
        }
        let weak = self.to_ref().downgrade();
        menu.items().add(Some(Control::boxed(action_item(
            utf8(strings.more_commands()),
            move || {
                if let Some(ribbon) = weak.upgrade() {
                    ribbon.request_customize_dialog(
                        super::super::RibbonCustomizePage::QuickAccessToolbar,
                    );
                }
            },
        ))));
        let weak = self.to_ref().downgrade();
        menu.items().add(Some(Control::boxed(action_item(
            utf8(
                if self.quick_access_position() == RibbonQuickAccessPosition::AboveRibbon {
                    strings.show_below_ribbon()
                } else {
                    strings.show_above_ribbon()
                },
            ),
            move || {
                if let Some(r) = weak.upgrade() {
                    r.toggle_quick_access_position();
                }
            },
        ))));
        let labels = ToggleMenuFlyoutItem::new();
        labels.set_text(utf8(strings.show_command_labels()));
        labels.set_is_checked(self.show_quick_access_labels());
        let weak = self.to_ref().downgrade();
        labels.add_click_action(move |s, _| {
            if let Some(r) = weak.upgrade() {
                r.set_show_quick_access_labels(
                    s.to_ref()
                        .cast::<ToggleMenuFlyoutItem>()
                        .expect("label toggle")
                        .is_checked(),
                );
            }
        });
        menu.items().add(Some(Control::boxed(labels)));
        let weak = self.to_ref().downgrade();
        menu.items().add(Some(Control::boxed(action_item(
            utf8(strings.hide_quick_access_toolbar()),
            move || {
                if let Some(r) = weak.upgrade() {
                    r.set_is_quick_access_visible(false);
                }
            },
        ))));
        RibbonMenu::apply_theme(Some(&menu));
        menu
    }
}
fn utf8(s: Option<RibbonString>) -> Option<String> {
    s.map(|s| s.to_utf8().expect("native QAT text"))
}
struct Action(Box<dyn Fn()>);
impl PartialEq for Action {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class Action{this:Rc<Action>,handles:[Rc<Action>],methods:[fn Invoke()=>|a:&Rc<Action>|(a.0)()]});
fn action_item(text: Option<String>, f: impl Fn() + 'static) -> Ref<MenuFlyoutItem> {
    use ferroui_base::{data::core::ValueTypes, metadata::MarkupTyped};
    ValueTypes::register_reference::<Action>();
    let h = MarkupDelegate::for_method(
        Some(Rc::new(Action(Box::new(f)))),
        Action::MARKUP,
        &Action::MARKUP.methods[0],
    );
    RibbonMenu::item(text, Some(h), None, None).expect("QAT menu action")
}
#[cfg(test)]
#[path = "../quick_access_tests.rs"]
mod tests;
