//! Ribbon.State.cs on the actual native Ribbon. Core state and JSON remain UI-free.
use super::*;
use crate::controls::{RibbonGroup, RibbonItemHelper, ribbon_quick_access_tool_bar::as_item};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{
    collections::{RibbonList, RibbonReadOnlyList},
    model::RibbonItemsSource,
    state::{
        RibbonCustomGroup, RibbonCustomization, RibbonFloatingGroupState, RibbonState,
        RibbonStateFloatingGroups, RibbonStateSerializer,
    },
};

pub(super) struct RootState {
    customization: RefCell<Rc<RibbonCustomization>>,
    custom_tabs: RibbonList<Option<Ref<RibbonTab>>>,
    custom_groups: RibbonList<Option<(Ref<RibbonTab>, Ref<RibbonGroup>)>>,
    pub(super) applying: Cell<bool>,
}
impl Default for RootState {
    fn default() -> Self {
        Self {
            customization: RefCell::new(RibbonCustomization::new()),
            custom_tabs: RibbonItems::new(),
            custom_groups: RibbonItems::new(),
            applying: Cell::new(false),
        }
    }
}
/// C# finally sets false even during nested calls; it does not restore an old flag.
struct Applying<'a>(&'a Cell<bool>);
impl Drop for Applying<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
fn native_string(value: Option<RibbonString>) -> Result<Option<String>, String> {
    value
        .map(|v| {
            v.to_utf8().map_err(|_| {
                "FerroUI String cannot represent unpaired UTF-16 in ribbon state".into()
            })
        })
        .transpose()
}
pub(super) fn clone_customization(
    value: Rc<RibbonCustomization>,
) -> Result<Rc<RibbonCustomization>, String> {
    let state = RibbonState::new();
    state.set_customization(Some(value));
    let json = RibbonStateSerializer::serialize_default(Some(&state)).map_err(|e| e.to_string())?;
    Ok(RibbonStateSerializer::deserialize(Some(&json))
        .map_err(|e| e.to_string())?
        .and_then(|s| s.customization())
        .unwrap_or_else(RibbonCustomization::new))
}
impl Ribbon {
    /// The source exposes the actual shared customization object, despite its
    /// summary calling this a read-only snapshot.
    pub fn customization(&self) -> Rc<RibbonCustomization> {
        self.state.customization.borrow().clone()
    }
    pub fn get_state(&self) -> Result<Rc<RibbonState>, String> {
        let state = RibbonState::new();
        state.set_selected_tab_id(
            self.selected_tab()
                .and_then(|t| t.effective_id())
                .map(Into::into),
        );
        state.set_display_mode(self.display_mode());
        state.set_visibility_mode(self.visibility_mode());
        state.set_density(self.density());
        state.set_quick_access_position(self.quick_access_position());
        state.set_is_quick_access_visible(self.is_quick_access_visible());
        state.set_show_quick_access_labels(self.show_quick_access_labels());
        state.set_quick_access_item_ids(Some(RibbonItems::from_values(
            self.get_quick_access_item_ids().iter().collect(),
        )));
        state.set_customization(Some(clone_customization(self.customization())?));
        state.set_recent_search_ids(Some(RibbonItems::from_values(
            self.search_engine().recent().iter().collect(),
        )));
        state.set_minimize_behavior(self.minimize_behavior());
        state.set_show_group_captions(self.show_group_captions());
        state.set_floating_groups(Some(self.get_floating_group_states()));
        Ok(state)
    }
    pub fn apply_saved_state(&self, state: Option<Rc<RibbonState>>) -> Result<(), String> {
        let state = state.ok_or("Value cannot be null. (Parameter 'state')")?;
        state.normalize();
        self.apply_customization(state.customization())?;
        if let Some(recent) = state.recent_search_ids() {
            // Preserve aliasing: SetRecent clears its destination before acquiring
            // this live enumerable. State may refer to that very same list.
            self.search_engine()
                .set_recent(Some(RibbonItemsSource::new(move || {
                    Box::new(recent.iter())
                })));
        }
        self.set_display_mode(state.display_mode());
        self.set_minimize_behavior(state.minimize_behavior());
        self.set_show_group_captions(state.show_group_captions());
        self.set_visibility_mode(state.visibility_mode());
        self.set_density(state.density());
        self.set_quick_access_position(state.quick_access_position());
        self.set_is_quick_access_visible(state.is_quick_access_visible());
        self.set_show_quick_access_labels(state.show_quick_access_labels());
        if let Some(ids) = state.quick_access_item_ids()
            && let Some(qat) = self.quick_access_tool_bar()
        {
            let existing = qat.items().iter().flatten().collect::<Vec<_>>();
            let mut desired = Vec::<Ref<Control>>::new();
            for id in ids.iter() {
                let own = existing
                    .iter()
                    .find(|i| {
                        as_item(i).is_some_and(|r| r.id() == id)
                            || RibbonItemHelper::get_source_item(i)
                                .and_then(|s| as_item(&s))
                                .is_some_and(|r| r.id() == id)
                    })
                    .cloned();
                let element = own.or_else(|| {
                    self.find_item(id)
                        .filter(|i| !self.is_in_quick_access(i))
                        .and_then(|i| self.create_quick_access_copy(&i))
                });
                if let Some(element) = element
                    && !desired.contains(&element)
                {
                    desired.push(element);
                }
            }
            let departed = qat
                .items()
                .iter()
                .filter(|i| i.as_ref().is_none_or(|i| !desired.contains(i)))
                .collect::<Vec<_>>();
            for item in departed {
                qat.items().remove(&item);
            }
            for (i, item) in desired.into_iter().enumerate() {
                let index = qat.items().index_of(&Some(item.clone()));
                match index {
                    None => qat.items().insert(i, Some(item)),
                    Some(index) if index != i => qat.items().move_item(index, i),
                    _ => {}
                }
            }
        }
        if let Some(id) = state.selected_tab_id() {
            // This is an ordinal lookup, not a native text setter. An unknown
            // unpaired UTF-16 ID remains an unmatched ID, as it does in C#.
            if let Some(tab) = self
                .tabs
                .iter()
                .map(|t| t.expect("NullReference: tab"))
                .find(|t| {
                    t.effective_id().map(RibbonString::from).as_ref() == Some(&id)
                        || t.header().map(RibbonString::from).as_ref() == Some(&id)
                })
            {
                self.select_tab(tab);
            }
        }
        if let Some(floating) = state.floating_groups() {
            self.apply_floating_group_states(&floating);
        }
        Ok(())
    }
    pub fn save_state_to_json(&self) -> Result<RibbonString, String> {
        let state = self.get_state()?;
        RibbonStateSerializer::serialize_default(Some(&state)).map_err(|e| e.to_string())
    }
    pub(super) fn save_state_json_markup(&self) -> Result<String, String> {
        self.save_state_to_json()?
            .to_utf8()
            .map_err(|e| e.to_string())
    }
    pub fn load_state_from_json(&self, json: Option<RibbonString>) -> Result<bool, String> {
        let Some(state) =
            RibbonStateSerializer::deserialize(json.as_ref()).map_err(|e| e.to_string())?
        else {
            return Ok(false);
        };
        self.apply_saved_state(Some(state))?;
        Ok(true)
    }
    pub fn apply_customization(
        &self,
        customization: Option<Rc<RibbonCustomization>>,
    ) -> Result<(), String> {
        let customization =
            customization.ok_or("Value cannot be null. (Parameter 'customization')")?;
        customization.normalize();
        let selected_id = self.selected_tab().and_then(|t| t.effective_id());
        self.state.applying.set(true);
        {
            let _finally = Applying(&self.state.applying);
            self.remove_custom_elements();
            self.state
                .customization
                .replace(clone_customization(customization)?);
            for tab in self.tabs.iter() {
                Self::restore_original(&tab.expect("NullReference: tab"));
            }
            for custom in self
                .customization()
                .custom_tabs()
                .expect("NullReference: CustomTabs")
                .iter()
            {
                let custom = custom.expect("NullReference: custom tab");
                let tab = RibbonTab::new();
                tab.set_id(native_string(custom.id())?);
                tab.set_header(native_string(custom.label())?);
                tab.is_custom.set(true);
                for group in custom
                    .groups()
                    .expect("NullReference: custom.Groups")
                    .iter()
                {
                    tab.groups().add(Some(
                        self.create_custom_group(&group.expect("NullReference: custom group"))?,
                    ));
                }
                self.state.custom_tabs.add(Some(tab.clone()));
                self.tabs.add(Some(tab));
            }
            for group in self
                .customization()
                .custom_groups()
                .expect("NullReference: CustomGroups")
                .iter()
            {
                let group = group.expect("NullReference: custom group");
                if let Some(id) = native_string(group.tab_id())?
                    && let Some(host) = self.find_tab(&id)
                {
                    let element = self.create_custom_group(&group)?;
                    host.groups().add(Some(element.clone()));
                    self.state.custom_groups.add(Some((host, element)));
                }
            }
            for tab in self.tabs.iter() {
                self.apply_customization_to_tab(&tab.expect("NullReference: tab"))?;
            }
        }
        self.on_tabs_changed();
        if let Some(id) = selected_id
            && self.selected_tab().and_then(|t| t.effective_id()).as_ref() != Some(&id)
        {
            self.select_tab_id(&id);
        }
        self.raise_state_changed();
        Ok(())
    }
    pub fn reset_customization(&self) -> Result<(), String> {
        self.apply_customization(Some(RibbonCustomization::new()))
    }
    pub fn reset_quick_access(&self) -> Result<(), String> {
        self.capture_quick_access_defaults();
        let state = self.get_state()?;
        state.set_quick_access_item_ids(Some(RibbonItems::from_values(
            self.default_quick_access_item_ids()
                .expect("NullReference: defaults")
                .iter()
                .collect(),
        )));
        state.set_quick_access_position(self.default_quick_access_position());
        state.set_is_quick_access_visible(true);
        self.apply_saved_state(Some(state))
    }
    fn create_custom_group(
        &self,
        definition: &RibbonCustomGroup,
    ) -> Result<Ref<RibbonGroup>, String> {
        let group = RibbonGroup::new();
        group.set_id(native_string(definition.id())?);
        group.set_header(native_string(definition.label())?);
        group.set_is_custom(true);
        for id in definition
            .item_ids()
            .expect("NullReference: ItemIds")
            .iter()
        {
            if let Some(element) = self.find_item(id)
                && let Some(item) = as_item(&element)
                && let Some(copy) = item.create_linked_copy()
            {
                RibbonItemHelper::set_source_item(&copy, element);
                group.items().add(Some(copy));
            }
        }
        Ok(group)
    }
    fn remove_custom_elements(&self) {
        for tab in self.state.custom_tabs.iter() {
            let tab = tab.expect("null private custom tab");
            self.tabs.remove(&Some(tab.clone()));
            for group in tab.groups().iter() {
                for item in group.expect("NullReference: group").items().iter() {
                    RibbonItemHelper::unlink_tree(item.map(|i| i.upcast()));
                }
            }
        }
        for pair in self.state.custom_groups.iter() {
            let (tab, group) = pair.expect("null private custom group");
            tab.groups().remove(&Some(group.clone()));
            for item in group.items().iter() {
                RibbonItemHelper::unlink_tree(item.map(|i| i.upcast()));
            }
        }
        self.state.custom_tabs.clear();
        self.state.custom_groups.clear();
    }
    fn restore_original(tab: &RibbonTab) {
        let original = tab.original_header.borrow().clone();
        if let Some(original) = original {
            tab.set_header(Some(original));
        }
        tab.set_hidden_by_customization(false);
        for group in tab.groups().iter() {
            let group = group.expect("NullReference: group");
            let original = group.original_header.borrow().clone();
            if let Some(original) = original {
                group.set_header(Some(original));
            }
            group.set_hidden_by_customization(false);
        }
    }
    pub(super) fn apply_customization_to_tab(&self, tab: &RibbonTab) -> Result<(), String> {
        let c = self.customization();
        if let Some(id) = tab.effective_id() {
            let id = RibbonString::from(id);
            if let Some(label) = c.labels().expect("NullReference: Labels").get(&id) {
                if tab.original_header.borrow().is_none() {
                    tab.original_header.replace(tab.header());
                }
                tab.set_header(native_string(label)?);
            }
            let hidden = c
                .hidden_tab_ids()
                .expect("NullReference: HiddenTabIds")
                .iter()
                .any(|v| v.as_ref() == Some(&id));
            if tab.hidden_by_customization() != hidden {
                tab.set_hidden_by_customization(hidden);
            }
            let order = c
                .group_order()
                .expect("NullReference: GroupOrder")
                .get(&id)
                .map(|v| v.expect("NullReference: groupOrder.Count"));
            tab.set_group_order(
                order
                    .filter(|o| !o.is_empty())
                    .map(|o| RibbonFixedItems::new(o.iter().collect()) as RibbonReadOnlyList<_>),
            );
        }
        for group in tab.groups().iter() {
            let group = group.expect("NullReference: group");
            let Some(id) = group.effective_id().map(RibbonString::from) else {
                continue;
            };
            if let Some(label) = c.labels().expect("NullReference: Labels").get(&id) {
                if group.original_header.borrow().is_none() {
                    group.original_header.replace(group.header());
                }
                group.set_header(native_string(label)?);
            }
            group.set_hidden_by_customization(
                c.hidden_group_ids()
                    .expect("NullReference: HiddenGroupIds")
                    .iter()
                    .any(|v| v.as_ref() == Some(&id)),
            );
        }
        if !self.state.applying.get() {
            self.on_tabs_changed();
        }
        Ok(())
    }
    pub(super) fn apply_tab_order(&self, tabs: Vec<Ref<RibbonTab>>) -> Vec<Ref<RibbonTab>> {
        let order = self
            .customization()
            .tab_order()
            .expect("NullReference: TabOrder");
        if order.is_empty() {
            return tabs;
        }
        let count = i32::try_from(order.len()).expect("Int32 order.Count");
        let mut ranked = tabs
            .into_iter()
            .enumerate()
            .map(|(index, tab)| {
                let id = tab
                    .effective_id()
                    .map(RibbonString::from)
                    .unwrap_or_default();
                let rank = order.iter().position(|v| v.as_ref() == Some(&id));
                let rank = rank
                    .map(|r| i32::try_from(r).expect("Int32 rank"))
                    .unwrap_or_else(|| {
                        count.wrapping_add(i32::try_from(index).expect("Int32 index"))
                    });
                (tab.is_contextual(), rank, tab)
            })
            .collect::<Vec<_>>();
        ranked.sort_by_key(|(contextual, rank, _)| (*contextual, *rank));
        ranked.into_iter().map(|(_, _, t)| t).collect()
    }
    fn get_floating_group_states(&self) -> RibbonStateFloatingGroups {
        let states = RibbonItems::new();
        for group in self.floating_groups().iter() {
            let group = group.expect("null floating group");
            if let Some(id) = group.effective_id() {
                let state = RibbonFloatingGroupState::new();
                state.set_group_id(Some(id.into()));
                state.set_x(group.floating_position().x);
                state.set_y(group.floating_position().y);
                states.add(Some(state));
            }
        }
        states
    }
    fn apply_floating_group_states(&self, states: &RibbonStateFloatingGroups) {
        for tab in self.tabs.iter() {
            for group in tab.expect("NullReference: tab").groups().iter() {
                let group = group.expect("NullReference: group");
                let id = group.effective_id().map(RibbonString::from);
                let state = states
                    .iter()
                    .find(|s| {
                        s.as_ref()
                            .expect("NullReference: floating state")
                            .group_id()
                            == id
                    })
                    .flatten();
                if let Some(state) = state {
                    group.float(Some(ferroui_base::Point::new(state.x(), state.y())));
                } else if group.is_floating() {
                    group.return_to_ribbon();
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../root_state_tests.rs"]
mod tests;
