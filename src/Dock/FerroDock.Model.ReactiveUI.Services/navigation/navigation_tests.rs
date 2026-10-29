use super::controls::*;
use super::services::*;
use super::view_models::DockViewModel;
use crate::overlays::controls::BusyRootDock;
use crate::overlays::hosting::hosting_test_support::{Overlays, Screen};
use crate::overlays::hosting::HostTypeRegistry;
use crate::test_environment::Environment;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::services::*;
use ferrodock_model::{FactoryBase, FactoryBaseImpl};
use ferrodock_model_reactiveui::controls::{Document, DocumentDock, RootDock};
use ferrodock_model_reactiveui::{impl_factory, Factory, FactoryImpl};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IRoutableViewModel, IScreen};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type AddedDockable = (Rc<dyn IDock>, Rc<dyn IDockable>);
type FocusedDockable = (Rc<dyn IDock>, Option<Rc<dyn IDockable>>);
type OverlayContext = (Rc<dyn IScreen>, Rc<dyn IDockable>);

pub(crate) struct TestFactory {
    component: Factory,
    pub added: RefCell<Vec<AddedDockable>>,
    active: RefCell<Option<Rc<dyn IDockable>>>,
    focused: RefCell<Option<FocusedDockable>>,
    floated: RefCell<Vec<Rc<dyn IDockable>>>,
    pub layout: RefCell<Option<Rc<dyn IRootDock>>>,
    pub creates: Cell<usize>,
    pub inits: Cell<usize>,
}
impl TestFactory {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            added: RefCell::new(Vec::new()),
            active: RefCell::new(None),
            focused: RefCell::new(None),
            floated: RefCell::new(Vec::new()),
            layout: RefCell::new(None),
            creates: Cell::new(0),
            inits: Cell::new(0),
        })
    }
}
impl FactoryBaseImpl for TestFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for TestFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn create_layout(this: &Self) -> Option<Rc<dyn IRootDock>> {
        this.creates.set(this.creates.get() + 1);
        this.layout.borrow().clone()
    }
    fn init_layout(this: &Self, layout: Rc<dyn IDockable>) {
        this.inits.set(this.inits.get() + 1);
        this.component.base().init_layout(layout);
    }
    fn add_dockable(this: &Self, dock: Option<Rc<dyn IDock>>, item: Option<Rc<dyn IDockable>>) {
        let dock = dock.expect("nonnull recording fixture");
        let item = item.expect("nonnull recording fixture");
        this.added.borrow_mut().push((dock.clone(), item.clone()));
        this.component.base().add_dockable(Some(dock), Some(item));
    }
    fn set_active_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        let item = item.expect("nonnull recording fixture");
        this.active.replace(Some(item.clone()));
        this.component.base().set_active_dockable(Some(item));
    }
    fn set_focused_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
    ) {
        this.focused.replace(Some((
            dock.clone().expect("nonnull navigation fixture"),
            item.clone(),
        )));
        this.component.base().set_focused_dockable(dock, item);
    }
    fn float_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        this.floated
            .borrow_mut()
            .push(item.clone().expect("nonnull navigation fixture"));
        this.component.base().float_dockable(item);
    }
}
impl_factory!(TestFactory);
struct Scene {
    factory: Rc<TestFactory>,
    root: Rc<RootDock>,
    dock: Rc<DocumentDock>,
    host: Rc<RoutableDocument>,
    _screen: Rc<Screen>,
    service: Rc<DockNavigationService>,
}
impl Scene {
    fn new() -> Self {
        let factory = TestFactory::new();
        let root = RootDock::new();
        root.set_visible_dockables(Some(ListHandle::new()));
        root.set_factory(Some(factory.clone()));
        let dock = DocumentDock::new();
        dock.set_visible_dockables(Some(ListHandle::new()));
        root.visible_dockables().unwrap().add(Some(dock.clone()));
        root.set_default_dockable(Some(dock.clone()));
        dock.set_owner(Some(root.clone()));
        dock.set_factory(Some(factory.clone()));
        let screen = Screen::new("outer");
        let host = RoutableDocument::new(screen.clone(), None);
        host.set_owner(Some(dock.clone()));
        host.set_factory(Some(factory.clone()));
        let types = HostTypeRegistry::new();
        host.register_types(&types);
        let service = DockNavigationService::with_types(types);
        service.attach_factory(factory.clone(), host.clone());
        Self {
            factory,
            root,
            dock,
            host,
            _screen: screen,
            service,
        }
    }
}
fn same<A: ?Sized, B: ?Sized>(a: &Rc<A>, b: &Rc<B>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}
#[test]
fn try_navigate_back_returns_false_when_stack_has_single_item() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let first = RoutableDocument::new(host.clone(), Some("first".into()));
    let _execution = host
        .router()
        .navigate()
        .execute(Some(first.clone()))
        .subscribe_next(|_| {});
    assert!(!DockNavigationHelpers::try_navigate_back(host.clone()));
    assert_eq!(host.router().navigation_stack().count(), 1);
    assert!(same(&host.router().navigation_stack().get(0), &first));
}
#[test]
fn try_navigate_back_navigates_when_stack_has_multiple_items() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let first = RoutableDocument::new(host.clone(), Some("first".into()));
    let second = RoutableTool::new(host.clone(), Some("second".into()));
    let _a = host
        .router()
        .navigate()
        .execute(Some(first.clone()))
        .subscribe_next(|_| {});
    let _b = host
        .router()
        .navigate()
        .execute(Some(second))
        .subscribe_next(|_| {});
    assert!(DockNavigationHelpers::try_navigate_back(host.clone()));
    assert_eq!(host.router().navigation_stack().count(), 1);
    assert!(same(&host.router().navigation_stack().get(0), &first));
    assert!(!DockNavigationHelpers::try_navigate_back(Screen::new(
        "empty"
    )));
}
#[test]
fn open_document_uses_document_dock_from_owner_chain() {
    let _env = Environment::new();
    let s = Scene::new();
    let document = Document::new();
    s.service
        .open_document(s.host.clone(), document.clone(), false);
    let added = s.factory.added.borrow();
    assert_eq!(added.len(), 1);
    assert!(same(&added[0].0, &s.dock));
    assert!(same(&added[0].1, &document));
    assert!(same(s.factory.active.borrow().as_ref().unwrap(), &document));
    let focused = s.factory.focused.borrow();
    let (dock, item) = focused.as_ref().unwrap();
    assert!(same(dock, &s.dock));
    assert!(same(item.as_ref().unwrap(), &document));
    assert!(s.factory.floated.borrow().is_empty());
}
#[test]
fn open_document_floats_when_requested() {
    let _env = Environment::new();
    let s = Scene::new();
    let document = Document::new();
    s.service
        .open_document(s.host.clone(), document.clone(), true);
    assert_eq!(s.factory.floated.borrow().len(), 1);
    assert!(same(&s.factory.floated.borrow()[0], &document));
    assert!(s.root.windows().is_some());
}
#[test]
fn open_document_uses_document_already_in_dock() {
    let _env = Environment::new();
    let s = Scene::new();
    let document = Document::new();
    s.factory
        .add_dockable(Some(s.dock.clone()), Some(document.clone()));
    s.factory.added.borrow_mut().clear();
    s.service
        .open_document(s.host.clone(), document.clone(), false);
    assert!(s.factory.added.borrow().is_empty());
    assert_eq!(s.dock.visible_dockables().unwrap().count(), 1);
    assert!(same(&s.dock.active_dockable().unwrap(), &document));
    assert!(same(&s.root.focused_dockable().unwrap(), &document));
}
#[test]
fn open_document_readds_document_when_removed_from_owner() {
    let _env = Environment::new();
    let s = Scene::new();
    let document = Document::new();
    s.factory
        .add_dockable(Some(s.dock.clone()), Some(document.clone()));
    s.factory.remove_dockable(Some(document.clone()), true);
    assert_eq!(s.dock.visible_dockables().unwrap().count(), 0);
    s.service
        .open_document(s.host.clone(), document.clone(), false);
    assert_eq!(s.dock.visible_dockables().unwrap().count(), 1);
    assert!(same(
        (s.dock.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &document
    ));
}
#[test]
fn open_document_adds_new_document_when_id_matches_existing() {
    let _env = Environment::new();
    let s = Scene::new();
    let old = Document::new();
    old.set_id("Doc1".into());
    s.factory
        .add_dockable(Some(s.dock.clone()), Some(old.clone()));
    let new = Document::new();
    new.set_id("Doc1".into());
    s.service.open_document(s.host.clone(), new.clone(), false);
    let items = s.dock.visible_dockables().unwrap();
    assert_eq!(items.count(), 2);
    assert!(same(
        (items.get(0)).as_ref().expect("nonnull identity fixture"),
        &old
    ));
    assert!(same(
        (items.get(1)).as_ref().expect("nonnull identity fixture"),
        &new
    ));
}
#[test]
fn context_resolves_factory_and_document_dock_independently_and_falls_back_to_attached_host() {
    let _env = Environment::new();
    let s = Scene::new();
    let nearest = TestFactory::new();
    s.host.set_factory(Some(nearest.clone()));
    s.dock.set_factory(None);
    let (factory, dock) = s
        .service
        .try_get_document_context(Some(s.host.clone()))
        .unwrap();
    assert!(same(&factory, &nearest));
    assert!(same(&dock, &s.dock));
    let (factory, dock) = s
        .service
        .try_get_document_context(Some(Screen::new("plain")))
        .unwrap();
    assert!(same(&factory, &nearest));
    assert!(same(&dock, &s.dock));
    assert!(s
        .service
        .try_resolve_document_dock(Some(Screen::new("plain")))
        .is_none());
    s.host.set_factory(None);
    s.root.set_factory(None);
    let (factory, dock) = s
        .service
        .try_get_document_context(Some(s.host.clone()))
        .unwrap();
    assert!(same(&factory, &s.factory));
    assert!(same(&dock, &s.dock));
}
#[test]
fn root_fallback_prefers_unlisted_default_then_first_direct_child_never_nested_children() {
    let _env = Environment::new();
    let s = Scene::new();
    s.host.set_owner(Some(s.root.clone()));
    let unlisted = DocumentDock::new();
    s.root.set_default_dockable(Some(unlisted.clone()));
    let (_, found) = s
        .service
        .try_resolve_document_dock(Some(s.host.clone()))
        .unwrap();
    assert!(same(&found, &unlisted));
    s.root.set_default_dockable(Some(Document::new()));
    let (_, found) = s
        .service
        .try_resolve_document_dock(Some(s.host.clone()))
        .unwrap();
    assert!(same(&found, &s.dock));
    let nested = RootDock::new();
    nested.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        s.dock.clone() as Rc<dyn IDockable>
    )])));
    s.root
        .set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            nested as Rc<dyn IDockable>,
        )])));
    assert!(s
        .service
        .try_resolve_document_dock(Some(s.host.clone()))
        .is_none());
}
#[test]
fn visible_foreign_owner_is_retained_and_resolved_factory_receives_activation() {
    let _env = Environment::new();
    let s = Scene::new();
    let foreign = Scene::new();
    let document = Document::new();
    foreign
        .factory
        .add_dockable(Some(foreign.dock.clone()), Some(document.clone()));
    s.service
        .open_document(s.host.clone(), document.clone(), false);
    assert!(s.factory.added.borrow().is_empty());
    assert!(same(&document.owner().unwrap(), &foreign.dock));
    assert!(same(s.factory.active.borrow().as_ref().unwrap(), &document));
    assert!(same(
        &s.factory.focused.borrow().as_ref().unwrap().0,
        &foreign.dock
    ));
}
#[test]
fn missing_context_is_noop_and_close_uses_nearest_factory_even_when_close_is_cancelled() {
    let _env = Environment::new();
    let plain = Screen::new("plain");
    let document = Document::new();
    DockNavigationService::new().open_document(plain, document.clone(), true);
    assert!(document.owner().is_none());
    assert!(!DockNavigationHelpers::try_close_dockable(document.clone()));
    let s = Scene::new();
    s.factory
        .add_dockable(Some(s.dock.clone()), Some(document.clone()));
    document.set_factory(None);
    document.set_can_close(false);
    assert!(same(
        &DockNavigationHelpers::find_factory(document.clone()).unwrap(),
        &s.factory
    ));
    assert!(DockNavigationHelpers::try_close_dockable(document.clone()));
    assert!(s
        .dock
        .visible_dockables()
        .unwrap()
        .contains(&Some((document as Rc<dyn IDockable>).clone())));
}
#[test]
fn routable_models_keep_independent_routers_host_urls_and_original_model_identity() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let doc = RoutableDocument::new(host.clone(), None);
    let tool = RoutableTool::new(host.clone(), Some(String::new()));
    let root = RoutableRootDock::new(host.clone(), None);
    assert_eq!(doc.url_path_segment(), "RoutableDocument");
    assert_eq!(tool.url_path_segment(), "");
    assert_eq!(root.url_path_segment(), "RoutableRootDock");
    for screen in [doc.clone() as Rc<dyn IScreen>, tool.clone(), root.clone()] {
        assert!(!same(&screen.router(), &host.router()));
        assert_eq!(screen.router().navigation_stack().count(), 0);
    }
    assert!(!same(&doc.router(), &tool.router()));
    assert!(same(&doc.host_screen().unwrap(), &host));
    let types = HostTypeRegistry::new();
    doc.register_types(&types);
    tool.register_types(&types);
    root.register_types(&types);
    assert!(same(
        &types
            .screen::<dyn IDockable>(&(doc.clone() as Rc<dyn IScreen>))
            .unwrap(),
        &doc
    ));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| seen.borrow_mut().push(name.to_owned())));
    doc.set_title("changed".into());
    doc.set_title("changed".into());
    assert_eq!(&*changes.borrow(), &["Title"]);
    // The boxed payload stores the canonical outer model handle.
    assert!(same(
        doc.clone()
            .boxed_reference()
            .downcast_ref::<Rc<RoutableDocument>>()
            .unwrap(),
        &doc
    ));
}
#[test]
fn dock_view_model_initializes_only_created_layout_and_forwards_current_overlays() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let services = overlays.host.clone();
    let root = BusyRootDock::new(
        host.clone(),
        Rc::new(move || {
            counted.set(counted.get() + 1);
            services.clone()
        }),
        None,
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(root.url_path_segment(), "BusyRootDock");
    let factory = TestFactory::new();
    factory.layout.replace(Some(root.clone()));
    let vm = DockViewModel::new(host.clone(), factory.clone());
    assert_eq!(factory.creates.get(), 1);
    assert_eq!(factory.inits.get(), 1);
    assert!(same(&vm.layout().unwrap(), &root));
    assert!(same(&vm.host_screen().unwrap(), &host));
    assert_eq!(vm.url_path_segment().as_deref(), Some("dock"));
    assert!(same(&vm.busy(), &overlays.host.busy()));
    assert!(same(&vm.dialogs(), &overlays.host.dialogs()));
    assert!(same(&vm.confirmations(), &overlays.host.confirmations()));
    assert!(same(
        &vm.global_busy_service(),
        &overlays.host.global_busy_service()
    ));
    assert!(same(
        &vm.global_dialog_service(),
        &overlays.host.global_dialog_service()
    ));
    assert!(same(
        &vm.global_confirmation_service(),
        &overlays.host.global_confirmation_service()
    ));
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    vm.property_changed().add(Rc::new(move |name| {
        assert_eq!(name, "Layout");
        seen.set(seen.get() + 1)
    }));
    vm.set_layout(Some(root.clone()));
    assert_eq!(changes.get(), 0);
    let other = Overlays::new();
    let services = other.host.clone();
    let next = BusyRootDock::new(host, Rc::new(move || services.clone()), None);
    vm.set_layout(Some(next.clone()));
    assert_eq!(changes.get(), 1);
    assert_eq!(factory.inits.get(), 1);
    assert!(next.factory().is_none());
    assert!(same(&vm.busy(), &other.host.busy()));
    vm.set_layout(None);
    assert_eq!(changes.get(), 2);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| vm.busy())).is_err());
}
#[test]
fn dock_view_model_with_null_or_nonoverlay_layout_preserves_source_service_failure() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let factory = TestFactory::new();
    let vm = DockViewModel::new(host, factory.clone());
    assert!(vm.layout().is_none());
    assert_eq!(factory.inits.get(), 0);
    vm.set_layout(Some(RootDock::new()));
    assert_eq!(factory.inits.get(), 0);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| vm.dialogs())).is_err());
}

use crate::overlays::hosting::IHostOverlayServicesProvider;
use ferrodock_model_reactiveui::core::DockableBaseImpl;
use ferrodock_model_reactiveui::{impl_dock, impl_dockable};
use ferroui_reactiveui::reactive_object;
struct RecordingProvider {
    services: Rc<dyn IHostOverlayServices>,
    screens: RefCell<Vec<Rc<dyn IScreen>>>,
    contexts: RefCell<Vec<OverlayContext>>,
}
impl IHostOverlayServicesProvider for RecordingProvider {
    fn get_services(&self, screen: Rc<dyn IScreen>) -> Rc<dyn IHostOverlayServices> {
        self.screens.borrow_mut().push(screen);
        self.services.clone()
    }
    fn get_services_for_dockable(
        &self,
        screen: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
    ) -> Rc<dyn IHostOverlayServices> {
        self.contexts.borrow_mut().push((screen, dockable));
        self.services.clone()
    }
}
macro_rules! application_routable {
    ($name:ident,$base:ident) => {
        struct $name {base:$base}
        reactive_object!($name,base);
        impl DockableBaseImpl for $name {}
        impl_dockable!(@base $name,$base,
            fn as_document(self:Rc<Self>)->Option<Rc<dyn IDocument>> {Some(self)},
            fn as_mdi_document(self:Rc<Self>)->Option<Rc<dyn IMdiDocument>> {Some(self)},
            fn as_docking_window_state(&self)->Option<&dyn IDockingWindowState> {Some(self)}
        );
        document_contracts!($name);
        impl IDocument for $name {}
        impl ILocalTarget for $name {}
        impl IScreen for $name {fn router(&self)->Rc<ferroui_reactiveui::RoutingState> {self.base.router()}}
        impl IRoutableViewModel for $name {
            fn url_path_segment(&self)->Option<String> {Some(self.base.url_path_segment())}
            fn host_screen(&self)->Option<Rc<dyn IScreen>> {self.base.host_screen()}
        }
        impl $name {
            fn new(host:Rc<dyn IScreen>,provider:Option<Rc<dyn IHostOverlayServicesProvider>>)->Rc<Self> {
                Rc::new_cyclic(|weak|Self {base:$base::construct(weak.clone(),host,provider,None)})
            }
        }
    }
}
application_routable!(ApplicationDocument, RoutableDocumentBase);
application_routable!(ApplicationTool, RoutableToolBase);
impl ITool for ApplicationTool {}
struct ApplicationDock {
    base: super::core::RoutableDockBase,
}
reactive_object!(ApplicationDock, base);
impl DockableBaseImpl for ApplicationDock {}
impl_dock!(@base ApplicationDock,super::core::RoutableDockBase);
impl IRoutableViewModel for ApplicationDock {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.base.url_path_segment())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.base.host_screen()
    }
}
#[test]
fn abstract_document_and_tool_helpers_pass_outer_self_context_and_use_their_own_router() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let provider = Rc::new(RecordingProvider {
        services: overlays.host.clone(),
        screens: RefCell::new(Vec::new()),
        contexts: RefCell::new(Vec::new()),
    });
    let doc = ApplicationDocument::new(host.clone(), Some(provider.clone()));
    let tool = ApplicationTool::new(host.clone(), Some(provider.clone()));
    assert_eq!(doc.base.url_path_segment(), "ApplicationDocument");
    assert_eq!(tool.base.url_path_segment(), "ApplicationTool");
    assert!(same(&doc.base.get_busy_service(), &overlays.host.busy()));
    assert!(same(
        &doc.base.get_dialog_service(),
        &overlays.host.dialogs()
    ));
    assert!(same(
        &doc.base.get_confirmation_service(),
        &overlays.host.confirmations()
    ));
    assert!(same(&tool.base.get_busy_service(), &overlays.host.busy()));
    assert!(same(
        &tool.base.get_dialog_service(),
        &overlays.host.dialogs()
    ));
    assert!(same(
        &tool.base.get_confirmation_service(),
        &overlays.host.confirmations()
    ));
    assert!(provider.screens.borrow().is_empty());
    for (index, (screen, dockable)) in provider.contexts.borrow().iter().enumerate() {
        if index < 3 {
            assert!(same(screen, &doc));
            assert!(same(dockable, &doc));
        } else {
            assert!(same(screen, &tool));
            assert!(same(dockable, &tool));
        }
    }
    for screen in [doc.clone() as Rc<dyn IScreen>, tool.clone()] {
        let _first = screen
            .router()
            .navigate()
            .execute(Some(RoutableDocument::new(host.clone(), None)))
            .subscribe_next(|_| {});
        let _second = screen
            .router()
            .navigate()
            .execute(Some(RoutableDocument::new(host.clone(), None)))
            .subscribe_next(|_| {});
    }
    assert!(doc.base.navigate_back());
    assert!(tool.base.navigate_back());
    assert_eq!(host.router().navigation_stack().count(), 0);
    assert!(!doc.base.navigate_back());
    assert!(!tool.base.navigate_back());
    assert!(!doc.base.close_dockable());
    assert!(!tool.base.close_dockable());
    let factory = Factory::new();
    doc.set_factory(Some(factory.clone()));
    tool.set_factory(Some(factory.clone()));
    assert!(doc.base.close_dockable());
    assert!(tool.base.close_dockable());
}
#[test]
fn missing_provider_errors_match_source_and_abstract_dock_preserves_inherited_commands_and_outer_identity(
) {
    let _env = Environment::new();
    let host = Screen::new("host");
    let doc = ApplicationDocument::new(host.clone(), None);
    let tool = ApplicationTool::new(host.clone(), None);
    for action in [
        Box::new(|| {
            doc.base.get_busy_service();
        }) as Box<dyn Fn()>,
        Box::new(|| {
            tool.base.get_dialog_service();
        }),
        Box::new(|| {
            tool.base.get_confirmation_service();
        }),
    ] {
        let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).unwrap_err();
        assert_eq!(
            error.downcast_ref::<String>().unwrap(),
            "Overlay services provider is not available."
        );
    }
    let dock = Rc::new_cyclic(|weak| ApplicationDock {
        base: super::core::RoutableDockBase::construct(weak.clone(), host.clone(), None),
    });
    dock.base.initialize(&dock);
    assert_eq!(dock.url_path_segment().as_deref(), Some("ApplicationDock"));
    assert!(same(&dock.host_screen().unwrap(), &host));
    let child = Document::new();
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        child.clone() as Rc<dyn IDockable>
    )])));
    dock.navigate()
        .execute(Some(&(child.clone() as Rc<dyn IDockable>)));
    assert!(same(&dock.active_dockable().unwrap(), &child));
    assert!(same(
        dock.clone()
            .boxed_reference()
            .downcast_ref::<Rc<ApplicationDock>>()
            .unwrap(),
        &dock
    ));
}
#[test]
fn static_overlay_helpers_preserve_screen_and_dockable_overload_arguments() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let doc = Document::new();
    let overlays = Overlays::new();
    let provider = RecordingProvider {
        services: overlays.host.clone(),
        screens: RefCell::new(Vec::new()),
        contexts: RefCell::new(Vec::new()),
    };
    assert!(same(
        &DockNavigationHelpers::get_overlay_services(host.clone(), &provider),
        &overlays.host
    ));
    assert!(same(
        &DockNavigationHelpers::get_busy_service(host.clone(), &provider),
        &overlays.host.busy()
    ));
    assert!(same(
        &DockNavigationHelpers::get_dialog_service(host.clone(), &provider),
        &overlays.host.dialogs()
    ));
    assert!(same(
        &DockNavigationHelpers::get_confirmation_service(host.clone(), &provider),
        &overlays.host.confirmations()
    ));
    assert!(same(
        &DockNavigationHelpers::get_overlay_services_for_dockable(
            host.clone(),
            doc.clone(),
            &provider
        ),
        &overlays.host
    ));
    assert!(same(
        &DockNavigationHelpers::get_busy_service_for_dockable(host.clone(), doc.clone(), &provider),
        &overlays.host.busy()
    ));
    assert!(same(
        &DockNavigationHelpers::get_dialog_service_for_dockable(
            host.clone(),
            doc.clone(),
            &provider
        ),
        &overlays.host.dialogs()
    ));
    assert!(same(
        &DockNavigationHelpers::get_confirmation_service_for_dockable(
            host.clone(),
            doc.clone(),
            &provider
        ),
        &overlays.host.confirmations()
    ));
    assert_eq!(provider.screens.borrow().len(), 4);
    assert_eq!(provider.contexts.borrow().len(), 4);
    for screen in provider.screens.borrow().iter() {
        assert!(same(screen, &host));
    }
    for (screen, dockable) in provider.contexts.borrow().iter() {
        assert!(same(screen, &host));
        assert!(same(dockable, &doc));
    }
}

#[test]
fn original_routable_models_expose_inherited_local_targets_with_same_identity() {
    let scene = Scene::new();
    let tool = RoutableTool::new(scene.host.clone(), None);
    for model in [
        scene.host.clone() as Rc<dyn IDockable>,
        tool as Rc<dyn IDockable>,
    ] {
        let target = model.clone().as_local_target().unwrap();
        assert_eq!(
            Rc::as_ptr(&model) as *const (),
            Rc::as_ptr(&target) as *const ()
        );
        assert!(model.as_global_target().is_none());
    }
}
