//! Shared Ribbon.Model.cs algorithms. Copyright Wiesław Šoltés, MIT.
//! Native parent/context ownership uses weak backlinks without a managed runtime.
use super::super::{RibbonBackstage, RibbonItemHelper};
use super::*;
use crate::{
    controls::ribbon_quick_access_tool_bar::as_item,
    mvvm::{RibbonAction, RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{collections::RibbonList, model::*};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::threading::{DispatcherOperationStatus, DispatcherPriority};
use std::collections::HashMap;

// Only QAT control ownership lives here. Source extension disposal callbacks
// observe reused cleanup positions (see qat-scope-order.json); retain that small
// ordering contract without a general Dictionary or CLR enumerator protocol.
#[derive(Default)]
struct Scopes {
    entries: RefCell<ScopeEntries>,
}
#[derive(Default)]
struct ScopeEntries {
    by_control: HashMap<Ref<Control>, usize>,
    slots: Vec<Option<(Ref<Control>, Rc<RibbonModelBindings>)>>,
    vacant: Vec<usize>,
}
impl Scopes {
    fn insert(&self, key: Ref<Control>, value: Rc<RibbonModelBindings>) {
        let old = {
            let mut entries = self.entries.borrow_mut();
            if let Some(&index) = entries.by_control.get(&key) {
                let (_, current) = entries.slots[index].as_mut().expect("tracked QAT scope");
                Some(std::mem::replace(current, value))
            } else {
                let index = if let Some(index) = entries.vacant.pop() {
                    entries.slots[index] = Some((key.clone(), value));
                    index
                } else {
                    let index = entries.slots.len();
                    entries.slots.push(Some((key.clone(), value)));
                    index
                };
                entries.by_control.insert(key, index);
                None
            }
        };
        // A scope's last Rust owner can run extension code during Drop.
        drop(old);
    }
    fn remove(&self, key: &Ref<Control>) -> Option<Rc<RibbonModelBindings>> {
        let (stored_key, (control, scope)) = {
            let mut entries = self.entries.borrow_mut();
            let (stored_key, index) = entries.by_control.remove_entry(key)?;
            let entry = entries.slots[index].take().expect("tracked QAT scope");
            entries.vacant.push(index);
            (stored_key, entry)
        };
        drop(stored_key);
        drop(control);
        Some(scope)
    }
    fn clear(&self) {
        let old = std::mem::take(&mut *self.entries.borrow_mut());
        drop(old);
    }
    fn keys(&self) -> Vec<Ref<Control>> {
        self.entries
            .borrow()
            .slots
            .iter()
            .flatten()
            .map(|(control, _)| control.clone())
            .collect()
    }
    fn values(&self) -> Vec<Rc<RibbonModelBindings>> {
        // Both bulk callers dispose the parent binding scope first. These
        // children are already disposed; no live collection iterator is needed.
        self.entries
            .borrow()
            .slots
            .iter()
            .flatten()
            .map(|(_, scope)| scope.clone())
            .collect()
    }
}
pub(super) struct ModelState {
    quick_items: RibbonList<Option<Ref<Control>>>,
    quick_scopes: Scopes,
    bindings: RefCell<Option<Rc<RibbonModelBindings>>>,
    factory: RefCell<Ref<RibbonElementFactory>>,
    syncing: Cell<bool>,
    pub(super) elements_change_pending: Cell<bool>,
    unload_pending: Cell<bool>,
    detached: Cell<bool>,
    catalog_token: Cell<Option<u64>>,
    backstage: RefCell<Option<Ref<RibbonBackstage>>>,
    backstage_before: RefCell<Option<Ref<RibbonBackstage>>>,
    catalog: RefCell<Option<Rc<RibbonCommandCatalog>>>,
    catalog_before: RefCell<Option<Rc<RibbonCommandCatalog>>>,
}
impl Default for ModelState {
    fn default() -> Self {
        Self {
            quick_items: RibbonItems::new(),
            quick_scopes: Scopes::default(),
            bindings: RefCell::default(),
            factory: RefCell::new(RibbonElementFactory::new()),
            syncing: Cell::new(false),
            elements_change_pending: Cell::new(false),
            unload_pending: Cell::new(false),
            detached: Cell::new(false),
            catalog_token: Cell::new(None),
            backstage: RefCell::default(),
            backstage_before: RefCell::default(),
            catalog: RefCell::default(),
            catalog_before: RefCell::default(),
        }
    }
}
impl Drop for ModelState {
    fn drop(&mut self) {
        if let Some(bindings) = self.bindings.get_mut().take() {
            bindings.dispose();
        }
        for scope in self.quick_scopes.values() {
            scope.dispose();
        }
        self.quick_scopes.clear();
    }
}
struct Reset<'a>(&'a Cell<bool>);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
fn text(v: Option<RibbonString>) -> Option<String> {
    v.map(|s| {
        s.to_utf8()
            .expect("Native text cannot represent an unpaired UTF-16 surrogate")
    })
}
fn node_matches(element: &Control, model: &dyn RibbonNodeModelImpl) -> bool {
    RibbonElementFactory::get_model(element)
        .is_some_and(|m| std::ptr::addr_eq(Rc::as_ptr(&m), model as *const dyn RibbonNodeModelImpl))
}
impl Ribbon {
    pub(super) fn initialize_model() {
        ValueTypes::register_nullable::<Rc<dyn RibbonModelImpl>>();
        Self::model_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (old, new) = e.get_old_and_new_value::<Option<Rc<dyn RibbonModelImpl>>>();
                this.on_model_changed(old, new);
            })
            .detach();
    }
    pub fn model(&self) -> Option<Rc<dyn RibbonModelImpl>> {
        self.get_value(Self::model_property())
    }
    pub fn set_model(&self, value: Option<Rc<dyn RibbonModelImpl>>) {
        self.set_value(Self::model_property(), value);
    }
    pub fn item_factory(&self) -> Ref<RibbonElementFactory> {
        let factory = self.model.factory.borrow().clone();
        factory.add_ribbon_owner(&self.to_ref());
        factory
    }
    pub fn set_item_factory(&self, value: Option<Ref<RibbonElementFactory>>) {
        let value = value.expect("ArgumentNullException: value");
        value.add_ribbon_owner(&self.to_ref());
        let same = self.model.factory.borrow().ptr_eq(&value);
        let old = self.model.factory.replace(value);
        if !same {
            old.remove_ribbon_owner(&self.to_ref());
        }
        drop(old);
        let model = self.model();
        self.on_model_changed(model.clone(), model);
    }
    pub fn get_model(element: &FerroObject) -> Option<Rc<dyn RibbonNodeModelImpl>> {
        RibbonElementFactory::get_model(element)
    }
    pub(crate) fn on_model_loaded(&self) {
        self.model.unload_pending.set(false);
        self.model.detached.set(false);
        let bindings = self.model.bindings.borrow().clone();
        if let Some(bindings) = bindings {
            if bindings.dispatcher().is_none() {
                bindings.set_dispatcher(Some(self.dispatcher().clone()));
            }
            bindings.resume();
        }
    }
    pub(crate) fn on_model_unloaded(&self) {
        self.model.unload_pending.set(true);
        let ribbon = self.to_ref();
        let operation = self.dispatcher().invoke_async_local_with_priority(
            move || ribbon.detach_model_after_unload(),
            DispatcherPriority::NORMAL,
        );
        if operation.status() == DispatcherOperationStatus::Aborted {
            self.detach_model_after_unload();
        }
    }
    fn detach_model_after_unload(&self) {
        if !self.model.unload_pending.get() || self.is_loaded() {
            return;
        }
        self.model.unload_pending.set(false);
        self.model.detached.set(true);
        let bindings = self.model.bindings.borrow().clone();
        if let Some(bindings) = bindings {
            bindings.suspend();
        }
    }
    pub(crate) fn on_model_elements_changed(&self) {
        if self.model.elements_change_pending.get() {
            return;
        }
        self.model.elements_change_pending.set(true);
        let ribbon = self.to_ref();
        let operation = self.dispatcher().invoke_async_local_with_priority(
            move || ribbon.apply_model_elements_changed(),
            DispatcherPriority::NORMAL,
        );
        if operation.status() == DispatcherOperationStatus::Aborted {
            self.apply_model_elements_changed();
        }
    }
    fn apply_model_elements_changed(&self) {
        self.model.elements_change_pending.set(false);
        let bindings = self.model.bindings.borrow().clone();
        if let Some(model) = self.model()
            && let Some(bindings) = bindings.filter(|b| !b.is_disposed())
        {
            self.sync_model_quick_access(model, &bindings);
        }
        self.invalidate_shortcuts();
    }
    fn on_model_changed(
        &self,
        _old: Option<Rc<dyn RibbonModelImpl>>,
        new: Option<Rc<dyn RibbonModelImpl>>,
    ) {
        let old = self.model.bindings.borrow().clone();
        if let Some(old) = old {
            old.dispose();
        }
        let old = self.model.bindings.replace(None);
        drop(old);
        self.remove_model_quick_access_items();
        self.release_model_backstage();
        self.release_model_catalog();
        let Some(model) = new else {
            return;
        };
        RibbonModelBindings::set_fallback_dispatcher(Some(self.dispatcher().clone()));
        // A parent-owned factory context is weak under native ownership.
        self.item_factory().set_composed_ribbon(Some(self.to_ref()));
        if self.model.catalog_token.get().is_none() {
            let weak = self.to_ref().downgrade();
            let token = self.add_property_changed_handler(Rc::new(move |e| {
                if e.property() == Self::command_catalog_property().as_property()
                    && e.is_effective_value_change()
                    && let Some(r) = weak.upgrade()
                {
                    r.item_factory().invalidate_commands();
                }
            }));
            self.model.catalog_token.set(Some(token));
        }
        let bindings = RibbonModelBindings::with_dispatcher(Some(self.dispatcher().clone()));
        let old = self.model.bindings.replace(Some(bindings.clone()));
        drop(old);
        let weak = self.to_ref().downgrade();
        let m = model.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                if let Some(r) = weak.upgrade() {
                    r.apply_model_catalog(m.clone());
                }
            })),
            Some(vec![Some("CommandCatalog".into())]),
        );
        let f = self.item_factory();
        bindings.items(
            model.contextual_groups(),
            Rc::new(self.contextual_groups()),
            Rc::new(move |m, b| Some(f.create_contextual_group(m, Some(b)))),
            true,
            None,
            None,
        );
        let selected_id = Rc::new(RefCell::new(None));
        let weak = self.to_ref().downgrade();
        let m = model.clone();
        let before = selected_id.clone();
        let changing: RibbonAction = Rc::new(move || {
            if let Some(r) = weak.upgrade() {
                let old = before.replace(
                    text(m.selected_tab_id())
                        .or_else(|| r.selected_tab().and_then(|t| t.effective_id())),
                );
                drop(old);
            }
        });
        let weak = self.to_ref().downgrade();
        let after = selected_id;
        let changed: RibbonAction = Rc::new(move || {
            if let Some(r) = weak.upgrade() {
                let id = after.borrow().clone();
                if let Some(id) = id
                    && r.selected_tab().and_then(|t| t.effective_id()).as_deref() != Some(&id)
                {
                    r.select_tab_id(&id);
                }
                r.on_model_elements_changed();
            }
        });
        let f = self.item_factory();
        bindings.items(
            model.tabs(),
            Rc::new(self.tabs()),
            Rc::new(move |m, b| Some(f.create_tab(m.expect("NullReference: tab model"), b))),
            true,
            Some(changing),
            Some(changed),
        );
        let weak = self.to_ref().downgrade();
        let changed: RibbonAction = Rc::new(move || {
            if let Some(r) = weak.upgrade() {
                r.on_model_elements_changed();
            }
        });
        let f = self.item_factory();
        bindings.items(
            model.tab_strip_items(),
            Rc::new(self.tab_strip_items()),
            Rc::new(move |m, b| f.create_item(m, b)),
            true,
            None,
            Some(changed),
        );
        let weak = self.to_ref().downgrade();
        let scope = Rc::downgrade(&bindings);
        let m = model.clone();
        bindings.collection(
            Some(Rc::new(model.quick_access_items())),
            Some(Rc::new(move || {
                if let Some(r) = weak.upgrade()
                    && let Some(b) = scope.upgrade()
                {
                    r.sync_model_quick_access(m.clone(), &b);
                }
            })),
        );
        let weak = self.to_ref().downgrade();
        let m = model.clone();
        bindings.collection(
            Some(Rc::new(model.quick_access_candidates())),
            Some(Rc::new(move || {
                if let Some(r) = weak.upgrade() {
                    r.quick_access_candidate_ids().clear();
                    for candidate in m.quick_access_candidates().iter() {
                        r.quick_access_candidate_ids()
                            .add(candidate.expect("NullReference: candidate").id());
                    }
                }
            })),
        );
        let backstage = self
            .item_factory()
            .create_backstage(Some(model.backstage()), Some(bindings.clone()));
        let old = self.model.backstage.replace(Some(backstage.clone()));
        drop(old);
        let weak = self.to_ref().downgrade();
        let m = model.clone();
        bindings.collection(
            Some(Rc::new(model.backstage().items())),
            Some(Rc::new(move || {
                if let Some(r) = weak.upgrade() {
                    r.apply_model_backstage(m.clone(), backstage.clone());
                }
            })),
        );
        let weak = self.to_ref().downgrade();
        let m = model.clone();
        bindings.one_way(
            Some(model.backstage()),
            Some(Rc::new(move || {
                if let Some(r) = weak.upgrade()
                    && r.is_backstage_open() != m.backstage().is_open()
                {
                    r.set_is_backstage_open(m.backstage().is_open());
                }
            })),
            Some(vec![Some("IsOpen".into())]),
        );
        let weak = self.to_ref().downgrade();
        let m = model.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                if let Some(r) = weak.upgrade() {
                    r.model.syncing.set(true);
                    let _reset = Reset(&r.model.syncing);
                    if let Some(id) = text(m.selected_tab_id())
                        && r.selected_tab().and_then(|t| t.effective_id()).as_deref() != Some(&id)
                    {
                        r.select_tab_id(&id);
                    }
                    r.set_display_mode(m.display_mode());
                    r.set_visibility_mode(m.visibility_mode());
                    r.set_density(m.density());
                    r.set_quick_access_position(m.quick_access_position());
                    r.set_is_quick_access_visible(m.is_quick_access_visible());
                    r.set_show_quick_access_labels(m.show_quick_access_labels());
                    r.set_minimize_behavior(m.minimize_behavior());
                    r.set_show_group_captions(m.show_group_captions());
                    r.set_application_button_label(text(m.application_button_label()));
                    r.set_is_application_button_visible(m.is_application_button_visible());
                }
            })),
            Some(
                [
                    "SelectedTabId",
                    "DisplayMode",
                    "VisibilityMode",
                    "Density",
                    "QuickAccessPosition",
                    "IsQuickAccessVisible",
                    "ShowQuickAccessLabels",
                    "MinimizeBehavior",
                    "ShowGroupCaptions",
                    "ApplicationButtonLabel",
                    "IsApplicationButtonVisible",
                ]
                .map(|p| Some(p.into()))
                .to_vec(),
            ),
        );
        if model.selected_tab_id().is_none()
            && let Some(selected) = self.selected_tab()
        {
            model.set_selected_tab_id(selected.effective_id().map(Into::into));
        }
        self.invalidate_shortcuts();
        if self.model.detached.get() && !self.is_loaded() {
            bindings.suspend();
        }
    }
    fn apply_model_catalog(&self, model: Rc<dyn RibbonModelImpl>) {
        if let Some(catalog) = model.command_catalog() {
            if self.model.catalog.borrow().is_none() {
                let old = self.model.catalog_before.replace(self.command_catalog());
                drop(old);
            }
            let old = self.model.catalog.replace(Some(catalog.clone()));
            drop(old);
            self.set_command_catalog(Some(catalog));
        } else {
            self.release_model_catalog();
        }
    }
    fn release_model_catalog(&self) {
        let applied = self.model.catalog.borrow().clone();
        let Some(applied) = applied else {
            return;
        };
        if self
            .command_catalog()
            .as_ref()
            .is_some_and(|v| Rc::ptr_eq(v, &applied))
        {
            let previous = self.model.catalog_before.borrow().clone();
            self.set_command_catalog(previous);
        }
        let old = self.model.catalog.replace(None);
        drop(old);
        let old = self.model.catalog_before.replace(None);
        drop(old);
    }
    fn apply_model_backstage(
        &self,
        model: Rc<dyn RibbonModelImpl>,
        backstage: Ref<RibbonBackstage>,
    ) {
        if model.backstage().items().count() > 0 {
            if self.backstage() != Some(backstage.clone()) {
                let old = self.model.backstage_before.replace(self.backstage());
                drop(old);
                self.set_backstage(Some(backstage));
            }
        } else if self.backstage() == Some(backstage) {
            let previous = self.model.backstage_before.borrow().clone();
            self.set_backstage(previous);
            let old = self.model.backstage_before.replace(None);
            drop(old);
        }
    }
    fn release_model_backstage(&self) {
        let current = self.model.backstage.borrow().clone();
        if let Some(current) = current
            && self.backstage() == Some(current)
        {
            let previous = self.model.backstage_before.borrow().clone();
            self.set_backstage(previous);
        }
        let old = self.model.backstage.replace(None);
        drop(old);
        let old = self.model.backstage_before.replace(None);
        drop(old);
    }
    fn remove_model_quick_access_items(&self) {
        self.model.syncing.set(true);
        {
            let _reset = Reset(&self.model.syncing);
            for item in self.model.quick_items.iter() {
                if let Some(qat) = self.quick_access_tool_bar() {
                    qat.items().remove(&item);
                }
            }
        }
        for scope in self.model.quick_scopes.values() {
            scope.dispose();
        }
        self.model.quick_items.clear();
        self.model.quick_scopes.clear();
    }
    fn sync_model_quick_access(
        &self,
        model: Rc<dyn RibbonModelImpl>,
        bindings: &Rc<RibbonModelBindings>,
    ) {
        let Some(qat) = self
            .quick_access_tool_bar()
            .filter(|_| !self.model.syncing.get())
        else {
            return;
        };
        self.model.syncing.set(true);
        let _reset = Reset(&self.model.syncing);
        let sources = self
            .get_all_items()
            .iter()
            .filter(|i| {
                RibbonItemHelper::get_source_item(i).is_none() && Self::get_model(i).is_some()
            })
            .collect::<Vec<_>>();
        let mut available = self
            .model
            .quick_items
            .iter()
            .filter(|e| qat.items().contains(e))
            .collect::<Vec<_>>();
        let mut desired = Vec::new();
        for item_model in model.quick_access_items().iter() {
            let source = sources
                .iter()
                .find(|i| {
                    item_model
                        .as_ref()
                        .is_some_and(|m| node_matches(i, m.as_ref()))
                })
                .cloned();
            if let Some(index) = available.iter().position(|e| {
                Self::is_quick_access_copy_of(
                    e.as_ref().expect("NullReference: tracked QAT element"),
                    item_model.as_ref(),
                    source.as_ref(),
                )
            }) {
                desired.push(available.remove(index));
                continue;
            }
            let mut element = None;
            if let Some(item) = source.as_ref().and_then(|s| as_item(s))
                && let Some(copy) = item.create_linked_copy()
            {
                RibbonItemHelper::set_source_item(&copy, source.clone());
                element = Some(copy);
            } else if source.is_none() {
                let scope = bindings.child();
                element = self.item_factory().create_item(item_model, scope.clone());
                if let Some(element) = &element {
                    self.model.quick_scopes.insert(element.clone(), scope);
                } else {
                    scope.dispose();
                }
            }
            if let Some(element) = element
                && !desired.contains(&Some(element.clone()))
            {
                RibbonItemHelper::set_owner(&element, Some(self.item_owner()))
                    .expect("model QAT owner");
                desired.push(Some(element));
            }
        }
        for stale in available {
            qat.items().remove(&stale);
            self.dispose_quick_access_scope(
                stale.as_ref().expect("NullReference: tracked QAT element"),
            );
        }
        RibbonModelBindings::arrange_contiguous(&qat.items(), &desired);
        self.model.quick_items.clear();
        for element in desired {
            self.model.quick_items.add(element);
        }
    }
    fn is_quick_access_copy_of(
        element: &Control,
        model: Option<&Rc<dyn RibbonItemModelImpl>>,
        source: Option<&Ref<Control>>,
    ) -> bool {
        if let Some(linked) = RibbonItemHelper::get_source_item(element) {
            source.is_some_and(|s| s == &linked)
        } else {
            source.is_none() && model.is_some_and(|m| node_matches(element, m.as_ref()))
        }
    }
    fn dispose_quick_access_scope(&self, element: &Ref<Control>) {
        if let Some(scope) = self.model.quick_scopes.remove(element) {
            scope.dispose();
        }
    }
    pub(crate) fn on_model_selected_tab_changed(&self, tab: Option<Ref<RibbonTab>>) {
        if let Some(model) = self.model().filter(|_| !self.model.syncing.get())
            && let Some(tab) = tab
        {
            model.set_selected_tab_id(tab.effective_id().map(Into::into));
        }
    }
    pub(crate) fn on_model_display_mode_changed(&self) {
        if let Some(model) = self.model().filter(|_| !self.model.syncing.get()) {
            model.set_display_mode(self.display_mode());
            model.set_density(self.density());
        }
    }
    pub(crate) fn on_model_visibility_mode_changed(&self) {
        if let Some(model) = self.model().filter(|_| !self.model.syncing.get()) {
            model.set_visibility_mode(self.visibility_mode());
        }
    }
    pub(crate) fn on_model_quick_access_changed(&self) {
        if let Some(model) = self.model().filter(|_| !self.model.syncing.get()) {
            model.set_quick_access_position(self.quick_access_position());
            model.set_is_quick_access_visible(self.is_quick_access_visible());
            model.set_show_quick_access_labels(self.show_quick_access_labels());
        }
    }
    pub(crate) fn on_model_panel_options_changed(&self) {
        if let Some(model) = self.model().filter(|_| !self.model.syncing.get()) {
            model.set_minimize_behavior(self.minimize_behavior());
            model.set_show_group_captions(self.show_group_captions());
        }
    }
    pub(crate) fn on_model_backstage_changed(&self, open: bool) {
        if let Some(model) = self.model()
            && model.backstage().is_open() != open
        {
            model.backstage().set_is_open(open);
        }
    }
    pub(crate) fn sync_quick_access_to_model(&self) {
        let Some(model) = self.model().filter(|_| !self.model.syncing.get()) else {
            return;
        };
        let Some(qat) = self.quick_access_tool_bar() else {
            return;
        };
        self.model.syncing.set(true);
        let _reset = Reset(&self.model.syncing);
        let models = qat
            .items()
            .iter()
            .flatten()
            .filter_map(|i| {
                let source = RibbonItemHelper::get_source_item(&i).unwrap_or(i);
                try_cast_node::<dyn RibbonItemModelImpl>(Self::get_model(&source))
                    .expect("registered node cast")
            })
            .map(Some)
            .collect::<Vec<_>>();
        if model.quick_access_items().iter().collect::<Vec<_>>() != models {
            model.quick_access_items().clear();
            for m in models {
                model.quick_access_items().add(m);
            }
        }
        self.model.quick_items.clear();
        for item in qat.items().iter().flatten() {
            let source = RibbonItemHelper::get_source_item(&item).unwrap_or_else(|| item.clone());
            if Self::get_model(&source).is_some() {
                self.model.quick_items.add(Some(item));
            }
        }
        for element in self
            .model
            .quick_scopes
            .keys()
            .into_iter()
            .filter(|e| !qat.items().contains(&Some(e.clone())))
            .collect::<Vec<_>>()
        {
            self.dispose_quick_access_scope(&element);
        }
    }
    #[cfg(test)]
    pub(crate) fn model_bindings(&self) -> Option<Rc<RibbonModelBindings>> {
        self.model.bindings.borrow().clone()
    }
}

#[cfg(test)]
impl Ribbon {
    pub(crate) fn generated_model_backstage(&self) -> Option<Ref<RibbonBackstage>> {
        self.model.backstage.borrow().clone()
    }
}

#[cfg(test)]
mod scope_registry_tests {
    use super::*;
    #[test]
    fn removing_replacing_and_clearing_release_owners_outside_the_registry_borrow() {
        crate::group_tests::run(|| {
            let registry = Rc::new(Scopes::default());
            let a = crate::controls::RibbonButton::new().upcast::<Control>();
            let b = crate::controls::RibbonButton::new().upcast::<Control>();
            let c = crate::controls::RibbonButton::new().upcast::<Control>();
            let retired = Rc::new(Cell::new(0));
            let first = RibbonModelBindings::new();
            let weak_first = Rc::downgrade(&first);
            let owner = Rc::downgrade(&registry);
            let target = b.clone();
            let last = c.clone();
            let count = retired.clone();
            first.add(Some(Rc::new(move || {
                let next = RibbonModelBindings::new();
                let owner2 = owner.clone();
                let target2 = last.clone();
                let count = count.clone();
                next.add(Some(Rc::new(move || {
                    let final_scope = RibbonModelBindings::new();
                    let count = count.clone();
                    final_scope.add(Some(Rc::new(move || count.set(count.get() + 1))));
                    owner2
                        .upgrade()
                        .unwrap()
                        .insert(target2.clone(), final_scope);
                })));
                owner.upgrade().unwrap().insert(target.clone(), next);
            })));
            registry.insert(a.clone(), first);
            let replacement = RibbonModelBindings::new();
            let weak_replacement = Rc::downgrade(&replacement);
            registry.insert(a.clone(), replacement);
            assert!(
                weak_first.upgrade().is_none(),
                "replacement releases the previous owner"
            );
            assert_eq!(registry.keys(), vec![a.clone(), b.clone()]);
            let removed = registry.remove(&a).unwrap();
            assert!(
                weak_replacement.upgrade().is_some(),
                "the caller owns the removed scope"
            );
            drop(removed);
            assert!(weak_replacement.upgrade().is_none());
            registry.clear();
            assert_eq!(
                registry.keys(),
                vec![c],
                "a Drop callback can install a new scope during clear"
            );
            registry.clear();
            assert!(registry.keys().is_empty());
            assert_eq!(retired.get(), 1);
        });
    }
}
