//! Source Ribbon.Commands.cs, using the existing pinned CLR ordinal comparer and
//! Core catalog and actual QAT presentations.
use super::*;
use crate::controls::{IRibbonCheckable, IRibbonItem, RibbonItemHelper};
use ferroribbon_core::{model::RibbonItemsSource, text::RibbonOrdinalIgnoreCaseKey};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    threading::{Dispatcher, DispatcherPriority},
};
#[derive(Clone, Copy, Default)]
pub(super) struct CommandState {
    enabled: Option<bool>,
    checked: Option<bool>,
}
pub(super) struct CatalogSubscription {
    catalog: Rc<RibbonCommandCatalog>,
    token: u64,
}
impl Drop for CatalogSubscription {
    fn drop(&mut self) {
        self.catalog.commands_changed().remove(self.token);
    }
}
fn cast<T: ferroui_base::PropertyValue>(item: &Control) -> Option<T> {
    ValueTypes::try_cast(&Control::boxed(item.to_ref()), ValueType::of::<T>())
        .and_then(|v| v.downcast_ref::<T>().cloned())
}
impl Ribbon {
    pub fn set_command_enabled(&self, id: Option<RibbonString>, enabled: bool) {
        let id = id.expect("ArgumentNull: commandId");
        let key = RibbonOrdinalIgnoreCaseKey(id.clone());
        let state = {
            let old = self
                .command_states
                .borrow()
                .get(&key)
                .copied()
                .unwrap_or_default();
            CommandState {
                enabled: Some(enabled),
                checked: old.checked,
            }
        };
        self.command_states.borrow_mut().insert(key, state);
        for item in self.find_items_by_command(Some(id)).iter() {
            self.apply_command_state(&item);
        }
    }
    pub fn set_command_checked(&self, id: Option<RibbonString>, checked: bool) {
        let id = id.expect("ArgumentNull: commandId");
        let key = RibbonOrdinalIgnoreCaseKey(id.clone());
        let state = {
            let old = self
                .command_states
                .borrow()
                .get(&key)
                .copied()
                .unwrap_or_default();
            CommandState {
                enabled: old.enabled,
                checked: Some(checked),
            }
        };
        self.command_states.borrow_mut().insert(key, state);
        for item in self.find_items_by_command(Some(id)).iter() {
            self.apply_command_state(&item);
        }
    }
    pub fn get_all_items(&self) -> RibbonItemsSource<Ref<Control>> {
        let tabs = self.tabs();
        let strip = self.tab_strip_items();
        RibbonItemsSource::deferred(move || {
            let strip = strip.clone();
            let body = tabs
                .iter()
                .flat_map(|tab| {
                    let tab = tab.expect("NullReference: tab.Groups");
                    tab.groups().iter().inspect(move |_| {
                        let _owner = &tab;
                    })
                })
                .flat_map(|group| {
                    let group = group.expect("NullReference: group.GetAllItems");
                    group.get_all_items().iter().inspect(move |_| {
                        let _owner = &group;
                    })
                });
            let tail = std::iter::once_with(move || {
                strip
                    .iter()
                    .flat_map(|item| super::super::RibbonGroup::flatten(item).iter())
            })
            .flatten();
            Box::new(body.chain(tail))
        })
    }
    pub(super) fn command_presentations(&self) -> RibbonItemsSource<Ref<Control>> {
        let items = self.get_all_items();
        let qat = self.quick_access_tool_bar().map(|bar| bar.items());
        RibbonItemsSource::deferred(move || {
            let qat = qat.clone();
            let tail = std::iter::once_with(move || {
                qat.into_iter().flat_map(|items| items.iter().flatten())
            })
            .flatten();
            Box::new(items.iter().chain(tail))
        })
    }
    pub fn find_items_by_command(
        &self,
        id: Option<RibbonString>,
    ) -> RibbonItemsSource<Ref<Control>> {
        let items = self.command_presentations();
        RibbonItemsSource::deferred(move || {
            let id = id.clone();
            Box::new(items.iter().filter(move |item| {
                let command = cast::<Rc<dyn IRibbonItem>>(item).and_then(|i| i.command_id());
                command.as_ref().zip(id.as_ref()).is_some_and(|(a, b)| {
                    RibbonOrdinalIgnoreCaseKey(a.clone()) == RibbonOrdinalIgnoreCaseKey(b.clone())
                })
            }))
        })
    }
    pub fn find_item(&self, id: Option<RibbonString>) -> Option<Ref<Control>> {
        self.get_all_items()
            .iter()
            .find(|item| cast::<Rc<dyn IRibbonItem>>(item).is_some_and(|i| i.id() == id))
    }
    pub fn find_location(
        &self,
        item: &Control,
    ) -> (
        Option<Ref<RibbonTab>>,
        Option<Ref<super::super::RibbonGroup>>,
    ) {
        for tab in self.tabs.iter() {
            let tab = tab.expect("NullReference: tab.Groups");
            for group in tab.groups().iter() {
                let group = group.expect("NullReference: group.GetAllItems");
                if group
                    .get_all_items()
                    .iter()
                    .any(|candidate| candidate.ptr_eq(&item.to_ref()))
                {
                    return (Some(tab), Some(group));
                }
            }
        }
        (None, None)
    }
    pub(crate) fn apply_command_state(&self, item: &Control) {
        let Some(id) = cast::<Rc<dyn IRibbonItem>>(item).and_then(|i| i.command_id()) else {
            return;
        };
        let state = self
            .command_states
            .borrow()
            .get(&RibbonOrdinalIgnoreCaseKey(id))
            .copied();
        let Some(state) = state else { return };
        if let Some(enabled) = state.enabled {
            item.set_is_enabled(enabled);
        }
        if let Some(checked) = state.checked
            && let Some(checkable) = cast::<Rc<dyn IRibbonCheckable>>(item)
        {
            checkable.set_is_checked(Some(checked));
        }
    }
    pub(super) fn on_command_catalog_changed(&self) {
        self.detach_command_catalog();
        if self.is_loaded() {
            self.attach_command_catalog();
        }
        for item in self.get_all_items().iter() {
            RibbonItemHelper::resolve_command(&item)
                .expect("catalog replacement command resolution");
        }
    }
    pub(crate) fn attach_command_catalog(&self) {
        let catalog = self.command_catalog();
        if self
            .catalog_subscription
            .borrow()
            .as_ref()
            .map(|s| &s.catalog)
            == catalog.as_ref()
        {
            return;
        }
        self.detach_command_catalog();
        if let Some(catalog) = catalog {
            let weak = self.to_ref().downgrade();
            let token = catalog.commands_changed().add(Rc::new(move |(_, id)| {
                if let Some(ribbon) = weak.upgrade() {
                    let id = id.clone();
                    Dispatcher::ui_thread().post_local(
                        move || {
                            for item in ribbon.find_items_by_command(Some(id)).iter() {
                                RibbonItemHelper::resolve_command(&item)
                                    .expect("deferred catalog command resolution");
                            }
                        },
                        DispatcherPriority::NORMAL,
                    );
                }
            }));
            self.catalog_subscription
                .replace(Some(CatalogSubscription { catalog, token }));
        }
    }
    pub(crate) fn detach_command_catalog(&self) {
        let old = self.catalog_subscription.replace(None);
        drop(old);
    }
}
