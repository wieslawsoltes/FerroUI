// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DefaultDockCommandBarAdapter, IDockCommandBarAdapter};
use crate::controls::DockCommandBarHost;
use ferrodock_model::{
    collections::ReadOnlyListHandle,
    command_bars::{
        DockCommandBarDefinition, DockCommandBarItem, DockCommandBarKind, DockCommandBarMergeMode,
        IDockCommandBarProvider,
    },
    core::{IDock, IDockable, IFactory},
};
use ferrodock_settings::{DockCommandBarMergingScope, DockSettings};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    Ref,
};
use ferroui_system::IDisposable;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
/// Original command bar merge manager owned by DockControl.
pub struct DockCommandBarManager {
    host: Ref<DockCommandBarHost>,
    adapter: Rc<dyn IDockCommandBarAdapter>,
    layout: RefCell<Option<Rc<dyn IDock>>>,
    factory: RefCell<Option<Rc<dyn IFactory>>>,
    provider: RefCell<Option<Rc<dyn IDockCommandBarProvider>>>,
    factory_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    provider_subscription: Cell<Option<u64>>,
}
impl PartialEq for DockCommandBarManager {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DockCommandBarManager {
    /// Creates the manager with the supplied host and original default adapter fallback.
    pub fn new(
        host: Ref<DockCommandBarHost>,
        adapter: Option<Rc<dyn IDockCommandBarAdapter>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            host,
            adapter: adapter.unwrap_or_else(|| DefaultDockCommandBarAdapter::new()),
            layout: RefCell::new(None),
            factory: RefCell::new(None),
            provider: RefCell::new(None),
            factory_subscription: RefCell::new(None),
            provider_subscription: Cell::new(None),
        })
    }
    /// Attaches to the layout, retaining original event subscriptions until Detach.
    pub fn attach(self: &Rc<Self>, layout: Rc<dyn IDock>) {
        let same = self
            .layout
            .borrow()
            .as_ref()
            .is_some_and(|old| Rc::ptr_eq(old, &layout));
        if same {
            self.update_bars();
            return;
        }
        self.detach();
        self.layout.replace(Some(layout.clone()));
        self.factory.replace(layout.factory());
        let factory = self.factory.borrow().clone();
        if let Some(factory) = factory {
            let this = self.clone();
            let subscription =
                factory.active_dockable_changed(Rc::new(move |_| this.update_bars()));
            self.factory_subscription.replace(Some(subscription));
        }
        self.update_bars();
    }
    /// Detaches original factory/provider events and clears the host.
    pub fn detach(&self) {
        let subscription = self.factory_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let provider = self.provider.borrow().clone();
        if let Some(provider) = provider {
            if let Some(token) = self.provider_subscription.take() {
                provider.command_bars_changed().remove(token);
            }
            self.provider.replace(None);
        }
        self.factory.replace(None);
        self.layout.replace(None);
        self.clear_host();
    }
    fn update_bars(self: &Rc<Self>) {
        let layout = self.layout.borrow().clone();
        if !DockSettings::command_bar_merging_enabled()
            || layout
                .as_ref()
                .and_then(|layout| layout.factory())
                .is_none()
        {
            self.clear_host();
            return;
        }
        let layout = layout.expect("source layout with factory");
        let active =
            Self::resolve_active_dockable(layout, DockSettings::command_bar_merging_scope());
        let provider = active.and_then(Self::find_provider);
        let old = self.provider.borrow().clone();
        let same = match (&old, &provider) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            if let Some(old) = old {
                if let Some(token) = self.provider_subscription.take() {
                    old.command_bars_changed().remove(token);
                }
            }
            self.provider.replace(provider.clone());
            if let Some(provider) = provider {
                let this = self.clone();
                self.provider_subscription.set(Some(
                    provider
                        .command_bars_changed()
                        .add(Rc::new(move |_| this.update_bars())),
                ));
            }
        }
        let base = self
            .host
            .base_command_bars()
            .unwrap_or_else(|| ReadOnlyListHandle::from_items(Vec::new()));
        let provider = self.provider.borrow().clone();
        let active = provider
            .map(|provider| provider.get_command_bars())
            .unwrap_or_else(|| ReadOnlyListHandle::from_items(Vec::new()));
        let merged = Self::merge_bars(&base, &active);
        let menus = self.adapter.build_bars(
            DockCommandBarKind::Menu,
            &ReadOnlyListHandle::from_items(
                merged
                    .iter()
                    .filter(|bar| bar.kind() == DockCommandBarKind::Menu)
                    .cloned()
                    .collect(),
            ),
        );
        let tools = self.adapter.build_bars(
            DockCommandBarKind::ToolBar,
            &ReadOnlyListHandle::from_items(
                merged
                    .iter()
                    .filter(|bar| bar.kind() == DockCommandBarKind::ToolBar)
                    .cloned()
                    .collect(),
            ),
        );
        let ribbons = self.adapter.build_bars(
            DockCommandBarKind::Ribbon,
            &ReadOnlyListHandle::from_items(
                merged
                    .iter()
                    .filter(|bar| bar.kind() == DockCommandBarKind::Ribbon)
                    .cloned()
                    .collect(),
            ),
        );
        self.host
            .set_menu_bars((menus.count() > 0).then(|| menus.clone()));
        self.host
            .set_tool_bars((tools.count() > 0).then(|| tools.clone()));
        self.host
            .set_ribbon_bars((ribbons.count() > 0).then(|| ribbons.clone()));
        self.host
            .set_is_visible(menus.count() > 0 || tools.count() > 0 || ribbons.count() > 0);
    }
    fn clear_host(&self) {
        self.host.set_menu_bars(None);
        self.host.set_tool_bars(None);
        self.host.set_ribbon_bars(None);
        self.host.set_is_visible(false);
    }
    fn find_provider(dockable: Rc<dyn IDockable>) -> Option<Rc<dyn IDockCommandBarProvider>> {
        if let Some(provider) = dockable.clone().as_command_bar_provider() {
            return Some(provider);
        }
        if let Some(value) = dockable
            .clone()
            .boxed_interface(std::any::TypeId::of::<Rc<dyn IDockCommandBarProvider>>())
        {
            if let Some(provider) = value.downcast_ref::<Rc<dyn IDockCommandBarProvider>>() {
                return Some(provider.clone());
            }
        }
        let context = dockable.context()?;
        if let Some(provider) = context.downcast_ref::<Rc<dyn IDockCommandBarProvider>>() {
            return Some(provider.clone());
        }
        let native = ferrodock_controls_recycling::from_recycling_value(&context);
        ValueTypes::try_cast(&native, ValueType::of::<Rc<dyn IDockCommandBarProvider>>())?
            .downcast_ref::<Rc<dyn IDockCommandBarProvider>>()
            .cloned()
    }
    fn resolve_active_dockable(
        layout: Rc<dyn IDock>,
        scope: DockCommandBarMergingScope,
    ) -> Option<Rc<dyn IDockable>> {
        let active = layout.active_dockable();
        if scope == DockCommandBarMergingScope::ActiveDockable {
            if let Some(child) = active
                .clone()
                .and_then(|a| a.as_dock())
                .and_then(|d| d.active_dockable())
            {
                return Some(child);
            }
            return active;
        }
        Self::find_active_document(layout)
    }
    fn find_active_document(dock: Rc<dyn IDock>) -> Option<Rc<dyn IDockable>> {
        if dock.clone().as_document_dock().is_some() {
            if let Some(document) = dock.active_dockable().and_then(|d| d.as_document()) {
                return Some(document);
            }
        }
        if let Some(children) = dock.visible_dockables() {
            for child in children.iter() {
                if child.clone().and_then(|item| item.as_document()).is_some()
                    && dock.active_dockable().as_ref().is_some_and(|active| {
                        child
                            .as_ref()
                            .is_some_and(|child| Rc::ptr_eq(active, child))
                    })
                {
                    return child;
                }
                if let Some(child_dock) = child.and_then(|item| item.as_dock()) {
                    if let Some(document) = Self::find_active_document(child_dock) {
                        return Some(document);
                    }
                }
            }
        }
        if let Some(split) = dock.as_split_view_dock() {
            let pane = split.pane_dockable().and_then(|d| d.as_dock());
            let content = split.content_dockable().and_then(|d| d.as_dock());
            if let Some(pane) = pane {
                if let Some(document) = Self::find_active_document(pane) {
                    return Some(document);
                }
            }
            if let Some(content) = content {
                if let Some(document) = Self::find_active_document(content) {
                    return Some(document);
                }
            }
        }
        None
    }
    fn merge_bars(
        base: &ReadOnlyListHandle<Rc<DockCommandBarDefinition>>,
        active: &ReadOnlyListHandle<Rc<DockCommandBarDefinition>>,
    ) -> Vec<Rc<DockCommandBarDefinition>> {
        let mut merged = Vec::new();
        for kind in [
            DockCommandBarKind::Menu,
            DockCommandBarKind::ToolBar,
            DockCommandBarKind::Ribbon,
        ] {
            merged.extend(Self::merge_bars_for_kind(kind, base, active));
        }
        merged
    }
    fn merge_bars_for_kind(
        kind: DockCommandBarKind,
        base: &ReadOnlyListHandle<Rc<DockCommandBarDefinition>>,
        active: &ReadOnlyListHandle<Rc<DockCommandBarDefinition>>,
    ) -> Vec<Rc<DockCommandBarDefinition>> {
        let mut base: Vec<_> = base.iter().filter(|bar| bar.kind() == kind).collect();
        base.sort_by_key(|bar| bar.order());
        let base: Vec<_> = base.iter().map(Self::clone_bar).collect();
        let mut active: Vec<_> = active.iter().filter(|bar| bar.kind() == kind).collect();
        active.sort_by_key(|bar| bar.order());
        if active.is_empty() {
            return base;
        }
        let replace: Vec<_> = active
            .iter()
            .filter(|bar| bar.merge_mode() == DockCommandBarMergeMode::Replace)
            .collect();
        if !replace.is_empty() {
            let mut result: Vec<_> = replace.into_iter().map(Self::clone_bar).collect();
            result.sort_by_key(|bar| bar.order());
            return result;
        }
        let mut merged = base;
        for bar in active {
            match bar.merge_mode() {
                DockCommandBarMergeMode::Append => Self::append_to_bar(&mut merged, &bar),
                DockCommandBarMergeMode::MergeByGroup => Self::merge_by_group(&mut merged, &bar),
                DockCommandBarMergeMode::Replace => merged.push(Self::clone_bar(&bar)),
            }
        }
        merged.sort_by_key(|bar| bar.order());
        merged
    }
    fn append_to_bar(
        merged: &mut Vec<Rc<DockCommandBarDefinition>>,
        bar: &Rc<DockCommandBarDefinition>,
    ) {
        if bar.items().is_none_or(|items| items.count() == 0) {
            merged.push(Self::clone_bar(bar));
            return;
        }
        let target = merged
            .iter()
            .find(|candidate| candidate.id() == bar.id())
            .or_else(|| merged.last())
            .cloned();
        if target
            .as_ref()
            .is_none_or(|target| target.items().is_none_or(|items| items.count() == 0))
        {
            merged.push(Self::clone_bar(bar));
            return;
        }
        let target = target.expect("source target checked");
        let mut items: Vec<_> = target
            .items()
            .expect("source target items checked")
            .iter()
            .collect();
        let mut added: Vec<_> = bar
            .items()
            .expect("source bar items checked")
            .iter()
            .collect();
        added.sort_by_key(|item| item.order());
        items.extend(added);
        target.set_items(Some(ReadOnlyListHandle::from_items(items)));
    }
    fn merge_by_group(
        merged: &mut Vec<Rc<DockCommandBarDefinition>>,
        bar: &Rc<DockCommandBarDefinition>,
    ) {
        if bar.items().is_none_or(|items| items.count() == 0)
            || bar.group_id().is_none_or(|id| id.trim().is_empty())
        {
            merged.push(Self::clone_bar(bar));
            return;
        }
        let target = merged
            .iter()
            .find(|candidate| {
                candidate
                    .items()
                    .is_some_and(|items| items.iter().any(|item| item.group_id() == bar.group_id()))
            })
            .cloned();
        let Some(target) = target else {
            merged.push(Self::clone_bar(bar));
            return;
        };
        let mut items: Vec<_> = target
            .items()
            .map(|items| items.iter().collect())
            .unwrap_or_default();
        let index =
            Self::find_group_insert_index(&items, &bar.group_id().expect("source nonblank group"));
        let mut added: Vec<_> = bar.items().expect("source items checked").iter().collect();
        added.sort_by_key(|item| item.order());
        items.splice(index..index, added);
        target.set_items(Some(ReadOnlyListHandle::from_items(items)));
    }
    fn find_group_insert_index(items: &[Rc<DockCommandBarItem>], group: &str) -> usize {
        let mut index = None;
        for (i, item) in items.iter().enumerate() {
            if item.group_id().as_deref() == Some(group) {
                index = Some(i);
            }
        }
        index.map_or(items.len(), |index| index + 1)
    }
    fn clone_bar(bar: &Rc<DockCommandBarDefinition>) -> Rc<DockCommandBarDefinition> {
        let copy = DockCommandBarDefinition::new(bar.id(), bar.kind());
        copy.set_merge_mode(bar.merge_mode());
        copy.set_order(bar.order());
        copy.set_group_id(bar.group_id());
        copy.set_content(bar.content());
        copy.set_items(
            bar.items()
                .map(|items| ReadOnlyListHandle::from_items(items.iter().collect())),
        );
        copy
    }
}
ferroui_base::ferro_markup_type!(class DockCommandBarManager {
    namespace:"FerroDock.Ferro.CommandBars",this:Rc<DockCommandBarManager>,handles:[Rc<DockCommandBarManager>,Option<Rc<DockCommandBarManager>>],
    constructors:[(Ref<DockCommandBarHost>,Option<Rc<dyn IDockCommandBarAdapter>>)=>DockCommandBarManager::new],
    methods:[fn Attach(Rc<dyn IDock>)=>(|this:&Rc<DockCommandBarManager>,layout|this.attach(layout)),fn Detach()=>(|this:&Rc<DockCommandBarManager>|this.detach())],
});

#[cfg(test)]
#[path = "manager_tests.rs"]
pub(super) mod tests;
