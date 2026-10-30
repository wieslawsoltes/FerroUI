// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original host state, subscriptions and automation with native Headless and real models.
use super::*;
use crate::mdi::{MdiLayoutEntry, MdiResizeDirection};
use ferrodock_model::{controls::IMdiDocument, core::*};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock, StackDock, Tool},
    Factory,
};
use ferroui_base::{
    metadata::MarkupTypeKind,
    threading::{CancellationToken, Dispatcher},
    Rect, Size, Vector,
};
use ferroui_controls::{
    automation::{
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::IInvokeProvider,
        AutomationProperties,
    },
    templates::FuncDataTemplate,
    AppBuilder, Application, Border, Control, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_system::component_model::INotifyPropertyChanged;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "mdi-document-host",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene(Ref<Window>);
impl Scene {
    fn new(c: &MdiDocumentControl) -> Self {
        let w = Window::new();
        w.set_content(Some(Control::boxed(c.to_ref())));
        w.show();
        w.update_layout();
        Self(w)
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
fn document(id: &str) -> Rc<Document> {
    let d = Document::new();
    d.set_id(id.into());
    d.set_title(format!("Title {id}"));
    d
}
fn control(dock: Rc<dyn IDock>) -> Ref<MdiDocumentControl> {
    let c = MdiDocumentControl::new();
    c.set_data_context(Some(model_projection::to_dockable_value(dock)));
    c
}
#[derive(Default)]
struct RecordingManager {
    updates: RefCell<Vec<(Vec<String>, Option<String>)>>,
    hook: RefCell<Option<Rc<dyn Fn()>>>,
}
impl IMdiLayoutManager for RecordingManager {
    fn arrange(&self, _: &ListHandle<MdiLayoutEntry>, _: Size) {}
    fn get_drag_bounds(
        &self,
        _: Rc<dyn IMdiDocument>,
        bounds: Rect,
        _: Vector,
        _: Size,
        _: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        bounds
    }
    fn get_resize_bounds(
        &self,
        _: Rc<dyn IMdiDocument>,
        bounds: Rect,
        _: Vector,
        _: MdiResizeDirection,
        _: Size,
        _: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        bounds
    }
    fn update_z_order(
        &self,
        documents: &ListHandle<Rc<dyn IMdiDocument>>,
        active: Option<Rc<dyn IMdiDocument>>,
    ) {
        let ids = documents.iter().map(|d| d.id()).collect();
        self.updates
            .borrow_mut()
            .push((ids, active.map(|d| d.id())));
        let hook = self.hook.borrow().clone();
        if let Some(hook) = hook {
            hook();
        }
    }
}
#[test]
fn original_properties_native_metadata_defaults_and_readonly_state_are_preserved() {
    run(|| {
        let c = MdiDocumentControl::new();
        assert!(!c.is_active());
        assert!(!c.has_visible_documents());
        assert!(!c.pseudo_classes().contains(":active"));
        assert!(
            c.icon_template().is_none()
                && c.header_template().is_none()
                && c.modified_template().is_none()
                && c.close_template().is_none()
                && c.empty_content_template().is_none()
                && c.close_button_theme().is_none()
        );
        let expected: Rc<dyn IMdiLayoutManager> = ClassicMdiLayoutManager::instance();
        assert!(Rc::ptr_eq(&c.layout_manager().unwrap(), &expected));
        assert_eq!(expected.runtime_type_name(), "ClassicMdiLayoutManager");
        let icon: BoxedValue = Rc::new("original".to_owned());
        c.set_icon_template(Some(icon.clone()));
        assert!(Rc::ptr_eq(&c.icon_template().unwrap(), &icon));
        let template: Rc<dyn IDataTemplate> =
            FuncDataTemplate::new(|_| true, |_, _| Some(Border::new().upcast()), true);
        c.set_header_template(Some(template.clone()));
        c.set_modified_template(Some(template.clone()));
        c.set_close_template(Some(template.clone()));
        c.set_empty_content_template(Some(template.clone()));
        for actual in [
            c.header_template(),
            c.modified_template(),
            c.close_template(),
            c.empty_content_template(),
        ] {
            assert!(Rc::ptr_eq(&actual.unwrap(), &template));
        }
        let theme = ControlTheme::new();
        c.set_close_button_theme(Some(theme.clone()));
        assert_eq!(c.close_button_theme(), Some(theme));
        c.set_is_active(true);
        assert!(c.pseudo_classes().contains(":active"));
        c.set_is_active(false);
        assert!(!c.pseudo_classes().contains(":active"));
        let descriptor = MdiDocumentControl::__MARKUP.unwrap();
        assert_eq!(descriptor.kind, MarkupTypeKind::Class);
        assert!(MdiDocumentControl::has_visible_documents_property()
            .setter()
            .is_none());
        assert!(MdiDocumentControl::has_visible_documents_property()
            .as_property()
            .is_read_only());
        assert_eq!(
            c.get_value_untyped(MdiDocumentControl::has_visible_documents_property().as_property())
                .downcast_ref::<bool>(),
            Some(&false)
        );
        assert_eq!(c.get_type().namespace(), "FerroDock.Ferro.Controls");
    });
}
#[test]
fn observable_collections_preserve_any_mdi_member_state_and_change_only_notifications() {
    run(|| {
        let dock = DocumentDock::new();
        let list: ListHandle<Option<Rc<dyn IDockable>>> =
            ListHandle::from_items(vec![Some(StackDock::new())]);
        dock.set_visible_dockables(Some(list.clone()));
        let c = control(dock.clone());
        let scene = Scene::new(&c);
        assert!(!c.has_visible_documents());
        let values = Rc::new(RefCell::new(Vec::new()));
        let seen = values.clone();
        let sub = c.property_changed(move |e| {
            if e.property() == MdiDocumentControl::has_visible_documents_property().as_property() {
                seen.borrow_mut().push(e.get_new_value::<bool>());
            }
        });
        let first = document("first");
        let second = Tool::new();
        list.add(Some(first.clone()));
        assert!(c.has_visible_documents());
        list.add(Some(second.clone()));
        assert_eq!(&*values.borrow(), &[true]);
        list.remove(&Some((first as Rc<dyn IDockable>).clone()));
        assert!(c.has_visible_documents());
        list.remove(&Some((second as Rc<dyn IDockable>).clone()));
        assert!(!c.has_visible_documents());
        assert_eq!(&*values.borrow(), &[true, false]);
        list.set(0, Some(document("replacement")));
        assert!(c.has_visible_documents());
        list.clear();
        assert!(!c.has_visible_documents());
        assert_eq!(&*values.borrow(), &[true, false, true, false]);
        drop(scene);
        sub.dispose();
    });
}
#[test]
fn list_replacement_and_context_rebinding_remove_original_stale_sources() {
    run(|| {
        let first = DocumentDock::new();
        let old: ListHandle<Option<Rc<dyn IDockable>>> = ListHandle::new();
        first.set_visible_dockables(Some(old.clone()));
        let c = control(first.clone());
        let scene = Scene::new(&c);
        assert_eq!(
            old.as_notify_collection_changed()
                .unwrap()
                .collection_changed()
                .handler_count(),
            1
        );
        let replacement: ListHandle<Option<Rc<dyn IDockable>>> =
            ListHandle::from_items(vec![Some(document("replacement"))]);
        first.set_visible_dockables(Some(replacement.clone()));
        assert!(c.has_visible_documents());
        assert_eq!(
            old.as_notify_collection_changed()
                .unwrap()
                .collection_changed()
                .handler_count(),
            0
        );
        old.add(Some(document("stale")));
        replacement.clear();
        assert!(!c.has_visible_documents());
        let second = DocumentDock::new();
        let list: ListHandle<Option<Rc<dyn IDockable>>> =
            ListHandle::from_items(vec![Some(document("second"))]);
        second.set_visible_dockables(Some(list.clone()));
        let before = first.property_changed().handler_count();
        c.set_data_context(Some(model_projection::to_dockable_value(second.clone())));
        assert_eq!(first.property_changed().handler_count(), before - 1);
        assert_eq!(
            replacement
                .as_notify_collection_changed()
                .unwrap()
                .collection_changed()
                .handler_count(),
            0
        );
        assert!(c.has_visible_documents());
        first.set_visible_dockables(None);
        assert!(c.has_visible_documents());
        c.set_data_context(None);
        assert!(!c.has_visible_documents());
        assert!(c.dock().is_none());
        assert_eq!(
            list.as_notify_collection_changed()
                .unwrap()
                .collection_changed()
                .handler_count(),
            0
        );
        drop(scene);
    });
}
#[test]
fn source_nonnotifying_lists_and_named_property_dispatch_remain_unchanged() {
    run(|| {
        let dock = DocumentDock::new();
        let list: ListHandle<Option<Rc<dyn IDockable>>> = ListHandle::from_plain(vec![]);
        dock.set_visible_dockables(Some(list.clone()));
        let c = control(dock.clone());
        let manager = Rc::new(RecordingManager::default());
        c.set_layout_manager(Some(manager.clone()));
        let scene = Scene::new(&c);
        assert_eq!(manager.updates.borrow().len(), 1);
        list.add(Some(document("plain")));
        assert!(!c.has_visible_documents());
        dock.property_changed().raise("");
        dock.property_changed().raise("IsActive");
        assert!(!c.has_visible_documents());
        assert_eq!(manager.updates.borrow().len(), 1);
        dock.property_changed().raise("ActiveDockable");
        assert!(!c.has_visible_documents());
        assert_eq!(manager.updates.borrow().len(), 2);
        dock.property_changed().raise("VisibleDockables");
        assert!(c.has_visible_documents());
        assert_eq!(manager.updates.borrow().len(), 3);
        dock.set_visible_dockables(None);
        assert!(!c.has_visible_documents());
        assert_eq!(manager.updates.borrow().len(), 3);
        drop(scene);
    });
}
#[test]
fn custom_layout_manager_receives_source_document_order_active_and_update_triggers() {
    run(|| {
        let dock = DocumentDock::new();
        let first = document("first");
        let second = document("second");
        let list: ListHandle<Option<Rc<dyn IDockable>>> = ListHandle::from_items(vec![
            Some(first.clone()),
            Some(StackDock::new()),
            Some(second.clone()),
        ]);
        dock.set_visible_dockables(Some(list.clone()));
        dock.set_active_dockable(Some(second.clone()));
        let c = control(dock.clone());
        let manager = Rc::new(RecordingManager::default());
        assert_eq!(manager.runtime_type_name(), "RecordingManager");
        c.set_layout_manager(Some(manager.clone()));
        let scene = Scene::new(&c);
        assert_eq!(
            &*manager.updates.borrow(),
            &[(vec!["first".into(), "second".into()], Some("second".into()))]
        );
        dock.set_active_dockable(Some(first.clone()));
        assert_eq!(
            manager.updates.borrow().last().unwrap().1.as_deref(),
            Some("first")
        );
        let before = manager.updates.borrow().len();
        c.set_is_active(true);
        c.set_layout_manager(None);
        assert_eq!(manager.updates.borrow().len(), before);
        dock.set_active_dockable(Some(second.clone()));
        assert!(second.mdi_z_index() > first.mdi_z_index());
        c.set_layout_manager(Some(manager.clone()));
        assert_eq!(manager.updates.borrow().len(), before);
        list.add(Some(document("third")));
        assert_eq!(
            manager.updates.borrow().last().unwrap().0,
            vec!["first", "second", "third"]
        );
        list.clear();
        assert_eq!(
            manager.updates.borrow().last().unwrap().0,
            Vec::<String>::new()
        );
        drop(scene);
    });
}
#[test]
fn state_callback_reentrancy_uses_current_dock_before_source_zorder_dispatch() {
    run(|| {
        let first = DocumentDock::new();
        let visible: ListHandle<Option<Rc<dyn IDockable>>> = ListHandle::new();
        first.set_visible_dockables(Some(visible.clone()));
        let second = DocumentDock::new();
        second.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(document("second"))])));
        let c = control(first);
        let manager = Rc::new(RecordingManager::default());
        c.set_layout_manager(Some(manager.clone()));
        let scene = Scene::new(&c);
        manager.updates.borrow_mut().clear();
        let owner = c.downgrade();
        let next = second.clone();
        let switch = Rc::new(Cell::new(false));
        let once = switch.clone();
        let sub = c.property_changed(move |e| {
            if e.property() == MdiDocumentControl::has_visible_documents_property().as_property()
                && e.get_new_value::<bool>()
                && !once.replace(true)
            {
                owner
                    .upgrade()
                    .unwrap()
                    .set_data_context(Some(model_projection::to_dockable_value(next.clone())));
            }
        });
        visible.add(Some(document("old")));
        assert!(c.has_visible_documents());
        assert!(Rc::ptr_eq(&c.dock().unwrap(), &(second as Rc<dyn IDock>)));
        assert!(manager
            .updates
            .borrow()
            .iter()
            .all(|(ids, _)| ids == &["second"]));
        assert!(manager.updates.borrow().len() >= 2);
        sub.dispose();
        drop(scene);
    });
}
#[test]
fn source_detach_reattach_unsubscribes_and_releases_control_with_all_dependencies_alive() {
    run(|| {
        let factory = Factory::new();
        let dock = DocumentDock::new();
        dock.set_factory(Some(factory.clone()));
        let list: ListHandle<Option<Rc<dyn IDockable>>> =
            ListHandle::from_items(vec![Some(document("alive"))]);
        dock.set_visible_dockables(Some(list.clone()));
        let original = dock.property_changed().handler_count();
        let c = control(dock.clone());
        let scene = Scene::new(&c);
        assert_eq!(dock.property_changed().handler_count(), original + 1);
        scene.0.set_content(None);
        assert!(!c.has_visible_documents());
        assert!(c.dock().is_none());
        assert!(c.data_context_subscription.borrow().is_none());
        assert_eq!(dock.property_changed().handler_count(), original);
        assert_eq!(
            list.as_notify_collection_changed()
                .unwrap()
                .collection_changed()
                .handler_count(),
            0
        );
        scene.0.set_content(Some(Control::boxed(c.clone())));
        assert!(c.has_visible_documents());
        assert_eq!(dock.property_changed().handler_count(), original + 1);
        assert_eq!(
            list.as_notify_collection_changed()
                .unwrap()
                .collection_changed()
                .handler_count(),
            1
        );
        scene.0.set_content(None);
        let weak = c.downgrade();
        drop(c);
        Dispatcher::ui_thread().run_jobs(None);
        scene.0.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        assert_eq!(dock.property_changed().handler_count(), original);
        drop((factory, dock, list));
    });
}
struct Layout {
    factory: Rc<Factory>,
    root: Rc<RootDock>,
    dock: Rc<DocumentDock>,
    first: Rc<Document>,
    second: Rc<Document>,
}
fn layout() -> Layout {
    let factory = Factory::new();
    let root = RootDock::new();
    let dock = DocumentDock::new();
    let first = document("first");
    let second = document("second");
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(first.clone()),
        Some(second.clone()),
    ])));
    factory.init_layout(root.clone());
    Layout {
        factory,
        root,
        dock,
        first,
        second,
    }
}
#[test]
fn original_host_automation_pane_role_names_live_state_and_invoke_use_real_factory() {
    run(|| {
        let Layout {
            factory: _factory,
            root: _root,
            dock,
            first,
            second,
        } = layout();
        dock.set_id("dock-id".into());
        let c = control(dock.clone());
        let scene = Scene::new(&c);
        dock.set_active_dockable(None);
        let peer = ControlAutomationPeer::create_peer_for_element(&c);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert_eq!(peer.get_class_name(), "MdiDocumentControl");
        assert_eq!(peer.get_automation_id().as_deref(), Some("dock-id"));
        assert!(peer.get_help_text().contains("HasVisibleDocuments=true"));
        assert!(peer
            .get_help_text()
            .contains("LayoutManager=ClassicMdiLayoutManager"));
        assert!(peer.get_help_text().contains("VisibleDockables=2"));
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert!(Rc::ptr_eq(
            &dock.active_dockable().unwrap(),
            &(first.clone() as Rc<dyn IDockable>)
        ));
        assert_eq!(peer.get_name(), first.title());
        dock.set_active_dockable(Some(second.clone()));
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert!(Rc::ptr_eq(
            &dock.active_dockable().unwrap(),
            &(second as Rc<dyn IDockable>)
        ));
        c.set_is_active(true);
        c.set_layout_manager(None);
        assert!(peer.get_help_text().contains("Active=true"));
        assert!(peer.get_help_text().contains("LayoutManager=none"));
        let custom = Rc::new(RecordingManager::default());
        c.set_layout_manager(Some(custom));
        assert!(peer
            .get_help_text()
            .contains("LayoutManager=RecordingManager"));
        AutomationProperties::set_name(&c, Some("Explicit host"));
        assert_eq!(peer.get_name(), "Explicit host");
        AutomationProperties::set_automation_id(&c, Some("explicit-id"));
        assert_eq!(peer.get_automation_id().as_deref(), Some("explicit-id"));
        drop(scene);
    });
}
#[test]
fn host_invoke_uses_first_dockable_even_when_nonmdi_and_focuses_when_activation_is_unavailable() {
    run(|| {
        let Layout {
            factory,
            root: _root,
            dock,
            ..
        } = layout();
        let nonmdi = StackDock::new();
        nonmdi.set_id("nonmdi".into());
        nonmdi.set_owner(Some(dock.clone()));
        nonmdi.set_factory(Some(factory.clone()));
        dock.set_visible_dockables(Some(ListHandle::from_plain(vec![
            Some(nonmdi.clone()),
            Some(document("later")),
        ])));
        dock.set_active_dockable(None);
        let c = control(dock.clone());
        c.set_focusable(true);
        let scene = Scene::new(&c);
        let peer = ControlAutomationPeer::create_peer_for_element(&c);
        let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        invoke.invoke().unwrap();
        assert!(Rc::ptr_eq(
            &dock.active_dockable().unwrap(),
            &(nonmdi as Rc<dyn IDockable>)
        ));
        dock.set_active_dockable(None);
        dock.set_visible_dockables(None);
        invoke.invoke().unwrap();
        assert!(c.is_focused());
        c.set_data_context(None);
        invoke.invoke().unwrap();
        assert!(c.is_focused());
        assert_eq!(peer.get_name(), "MDI document host");
        assert!(peer.get_help_text().contains("ActiveDockable=none"));
        assert!(peer.get_help_text().contains("VisibleDockables=0"));
        drop(scene);
    });
}

struct TracedList {
    items: ListHandle<Option<Rc<dyn IDockable>>>,
    log: Rc<RefCell<Vec<&'static str>>>,
}
impl ferrodock_model::collections::IList<Option<Rc<dyn IDockable>>> for TracedList {
    fn count(&self) -> usize {
        self.log.borrow_mut().push("count");
        self.items.count()
    }
    fn get(&self, i: usize) -> Option<Rc<dyn IDockable>> {
        self.items.get(i)
    }
    fn set(&self, i: usize, v: Option<Rc<dyn IDockable>>) {
        self.items.set(i, v);
    }
    fn insert(&self, i: usize, v: Option<Rc<dyn IDockable>>) {
        self.items.insert(i, v);
    }
    fn remove_at(&self, i: usize) {
        self.items.remove_at(i);
    }
    fn clear(&self) {
        self.items.clear();
    }
    fn index_of(&self, v: &Option<Rc<dyn IDockable>>) -> Option<usize> {
        self.items.index_of(v)
    }
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = Option<Rc<dyn IDockable>>>> {
        self.log.borrow_mut().push("enumerate");
        let iterator = self.items.iter();
        Box::new(TracedIterator {
            owner: self,
            iterator,
        })
    }
}
struct TracedIterator {
    owner: Rc<TracedList>,
    iterator: Box<dyn Iterator<Item = Option<Rc<dyn IDockable>>>>,
}
impl Iterator for TracedIterator {
    type Item = Option<Rc<dyn IDockable>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.owner.log.borrow_mut().push("next");
        self.iterator.next()
    }
}
impl Drop for TracedIterator {
    fn drop(&mut self) {
        self.owner.log.borrow_mut().push("dispose");
    }
}
#[test]
fn original_any_and_snapshot_enumerators_finish_before_state_and_manager_callbacks() {
    run(|| {
        let dock = DocumentDock::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        let list = ListHandle::from_list(Rc::new(TracedList {
            items: ListHandle::from_plain(vec![Some(document("first")), Some(document("second"))]),
            log: log.clone(),
        }));
        dock.set_visible_dockables(Some(list));
        let c = MdiDocumentControl::new();
        let manager = Rc::new(RecordingManager::default());
        let events = log.clone();
        manager
            .hook
            .replace(Some(Rc::new(move || events.borrow_mut().push("manager"))));
        c.set_layout_manager(Some(manager));
        let events = log.clone();
        let subscription = c.property_changed(move |e| {
            if e.property() == MdiDocumentControl::has_visible_documents_property().as_property() {
                events.borrow_mut().push("visible");
            }
        });
        log.borrow_mut().clear();
        c.on_dock_changed(Some(model_projection::to_dockable_value(dock)));
        assert_eq!(
            &*log.borrow(),
            &[
                "enumerate",
                "next",
                "dispose",
                "visible",
                "enumerate",
                "next",
                "next",
                "next",
                "dispose",
                "manager"
            ]
        );
        subscription.dispose();
        c.detach_dock_subscriptions();
    });
}
