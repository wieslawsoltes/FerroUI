use crate::automation::peers::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::RootDockControl;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock, Tool, ToolDock},
    Factory,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::input::{InputElement, MouseButton, RawInputModifiers};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::{
    AppBuilder, Application, Border, ContentControl, Control, ControlImpl, ItemsControl, TextBlock,
    Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::cell::RefCell;
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-root-control",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                register_root_model();
                register_dockable_model::<Document>();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
// Concrete application composition registrations use the shared value system.
fn register_dockable_model<T: IDockable + PartialEq + 'static>() {
    ValueTypes::register_reference::<T>();
    ValueTypes::register_nullable::<Rc<dyn IDockable>>();
    ValueTypes::register_boxed_cast::<T, Rc<dyn IDockable>>(|value| {
        let any: Rc<dyn std::any::Any> = value.clone();
        any.downcast::<T>()
            .ok()
            .map(|model| model as Rc<dyn IDockable>)
    });
    ValueTypes::register_cast::<Rc<T>, Rc<dyn IDockable>>(|model| model.clone());
}
fn register_root_model() {
    register_dockable_model::<RootDock>();
    ValueTypes::register_nullable::<Rc<dyn IRootDock>>();
    ValueTypes::register_boxed_cast::<RootDock, Rc<dyn IRootDock>>(|value| {
        let any: Rc<dyn std::any::Any> = value.clone();
        any.downcast::<RootDock>()
            .ok()
            .map(|model| model as Rc<dyn IRootDock>)
    });
    ValueTypes::register_cast::<Rc<RootDock>, Rc<dyn IRootDock>>(|model| model.clone());
}
struct Scene(Ref<Window>);
impl Scene {
    fn new(control: Ref<Control>) -> Self {
        let w = Window::new();
        w.set_width(640.0);
        w.set_height(360.0);
        w.set_content(Some(Control::boxed(control)));
        w.show();
        let s = Self(w);
        s.update();
        s
    }
    fn update(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.0.update_layout();
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
struct Layout {
    factory: Rc<Factory>,
    root: Rc<RootDock>,
    tools: Rc<ToolDock>,
    documents: Rc<DocumentDock>,
    a: Rc<Document>,
    b: Rc<Document>,
    tool: Rc<Tool>,
}
impl Layout {
    fn new() -> Self {
        let factory = Factory::new();
        let root = RootDock::new();
        root.set_id("Root".into());
        root.set_title("Root title".into());
        root.set_factory(Some(factory.clone()));
        let documents = DocumentDock::new();
        documents.set_id("Documents".into());
        documents.set_title("Documents".into());
        documents.set_factory(Some(factory.clone()));
        documents.set_owner(Some(root.clone()));
        let tools = ToolDock::new();
        tools.set_id("Tools".into());
        tools.set_factory(Some(factory.clone()));
        tools.set_owner(Some(root.clone()));
        let a = Document::new();
        a.set_id("DocA".into());
        a.set_title("Document A".into());
        a.set_factory(Some(factory.clone()));
        a.set_owner(Some(documents.clone()));
        let b = Document::new();
        b.set_id("DocB".into());
        b.set_title("Document B".into());
        b.set_factory(Some(factory.clone()));
        b.set_owner(Some(documents.clone()));
        let tool = Tool::new();
        tool.set_id("ToolA".into());
        tool.set_title("Tool A".into());
        tool.set_factory(Some(factory.clone()));
        tool.set_owner(Some(tools.clone()));
        documents
            .set_visible_dockables(Some(factory.create_list::<Option<Rc<dyn IDockable>>>(
                vec![Some(a.clone()), Some(b.clone())],
            )));
        documents.set_active_dockable(Some(a.clone()));
        documents.set_focused_dockable(Some(a.clone()));
        tools.set_visible_dockables(Some(
            factory.create_list::<Option<Rc<dyn IDockable>>>(vec![Some(tool.clone())]),
        ));
        tools.set_active_dockable(Some(tool.clone()));
        root.set_visible_dockables(Some(factory.create_list::<Option<Rc<dyn IDockable>>>(
            vec![Some(tools.clone()), Some(documents.clone())],
        )));
        root.set_active_dockable(Some(documents.clone()));
        root.set_focused_dockable(Some(a.clone()));
        Self {
            factory,
            root,
            tools,
            documents,
            a,
            b,
            tool,
        }
    }
    fn preview(&self) {
        let pinned = ToolDock::new();
        pinned.set_id("Pinned".into());
        pinned.set_factory(Some(self.factory.clone()));
        pinned.set_owner(Some(self.root.clone()));
        pinned.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(
            self.tool.clone() as Rc<dyn IDockable>
        )])));
        pinned.set_active_dockable(Some(self.tool.clone()));
        self.root.set_pinned_dock(Some(pinned));
    }
}
fn template(control: &RootDockControl, name: &str) -> Rc<RefCell<Option<Ref<ContentControl>>>> {
    let part = Rc::new(RefCell::new(None));
    let out = part.clone();
    let name = name.to_owned();
    control.set_template(Some(FuncControlTemplate::for_type::<RootDockControl>(
        move |_, scope| {
            let content = ContentControl::new();
            content.set_background(Some(Rc::new(
                ferroui_base::media::immutable::ImmutableSolidColorBrush::new(
                    ferroui_base::media::Color::from_argb(0, 0, 0, 0),
                ),
            )));
            scope.register(&name, content.clone().upcast());
            *out.borrow_mut() = Some(content.clone());
            content.upcast()
        },
    )));
    part
}
fn click(scene: &Scene) {
    scene.0.mouse_down(
        Point::new(20.0, 20.0),
        MouseButton::Left,
        RawInputModifiers::empty(),
    );
    scene.0.mouse_up(
        Point::new(20.0, 20.0),
        MouseButton::Left,
        RawInputModifiers::empty(),
    );
    scene.update();
}
#[test]
fn original_root_peer_case_reports_source_role_class_id_and_pinned_state() {
    run(|| {
        let layout = Layout::new();
        layout.preview();
        let control = RootDockControl::new();
        control.set_data_context(Some(layout.root.clone()));
        let _scene = Scene::new(control.clone().upcast());
        control.apply_template();
        let peer = ControlAutomationPeer::create_peer_for_element(&control);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert_eq!(peer.get_class_name(), "RootDockControl");
        assert_eq!(peer.get_automation_id().as_deref(), Some("Root"));
        assert_eq!(peer.get_name(), "Root title");
        assert!(peer.get_help_text().contains("HasRoot=true"));
        assert!(peer.get_help_text().contains("PinnedVisible=1"));
        assert_eq!(peer.get_help_text(),"HasRoot=true; FocusableRoot=true; VisibleDockables=2; HiddenDockables=0; PinnedLeft=0; PinnedRight=0; PinnedTop=0; PinnedBottom=0; PinnedVisible=1; ActiveDockable=Documents");
    });
}
#[test]
fn peer_priorities_keep_base_label_name_explicit_properties_and_non_root_context_fallbacks() {
    run(|| {
        let layout = Layout::new();
        let control = RootDockControl::new();
        control.set_data_context(Some(layout.root.clone()));
        let peer = ControlAutomationPeer::create_peer_for_element(&control);
        AutomationProperties::set_name(&control, Some("Explicit"));
        assert_eq!(peer.get_name(), "Explicit");
        AutomationProperties::set_name(&control, Some(" \t"));
        let label = TextBlock::new();
        label.set_text(Some("Label"));
        AutomationProperties::set_labeled_by(&control, Some(label.clone().upcast()));
        assert_eq!(peer.get_name(), "Label");
        AutomationProperties::set_labeled_by(&control, None);
        assert_eq!(peer.get_name(), "Root title");
        AutomationProperties::set_automation_id(&control, Some("ID"));
        assert_eq!(peer.get_automation_id().as_deref(), Some("ID"));
        AutomationProperties::set_automation_id(&control, Some(" \n"));
        assert_eq!(peer.get_automation_id().as_deref(), Some("Root"));
        control.set_data_context(Some(layout.a.clone()));
        assert_eq!(peer.get_name(), "Document A");
        assert_eq!(peer.get_automation_id().as_deref(), Some("DocA"));
        assert_eq!(peer.get_help_text(), "HasRoot=false");
        control.set_data_context(None);
        assert_eq!(peer.get_name(), "Root dock");
        AutomationProperties::set_automation_id(&control, None);
        control.set_name(Some("ElementName".into()));
        assert_eq!(peer.get_automation_id().as_deref(), Some("ElementName"));
    });
}
#[test]
fn raw_erased_and_handle_root_contexts_preserve_original_models_and_live_state() {
    run(|| {
        let layout = Layout::new();
        let control = RootDockControl::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&control);
        for value in [
            layout.root.clone() as ferroui_base::BoxedValue,
            Rc::new(layout.root.clone()),
            Rc::new(layout.root.clone() as Rc<dyn IRootDock>),
            Rc::new(layout.root.clone() as Rc<dyn IDockable>),
        ] {
            control.set_data_context(Some(value));
            assert_eq!(peer.get_automation_id().as_deref(), Some("Root"));
            assert!(peer.get_help_text().starts_with("HasRoot=true"));
        }
        layout.root.set_is_focusable_root(false);
        layout
            .root
            .set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(
                layout.b.clone() as Rc<dyn IDockable>
            )])));
        layout.preview();
        assert!(peer
            .get_help_text()
            .contains("FocusableRoot=false; VisibleDockables=2; HiddenDockables=1"));
        layout.root.set_active_dockable(Some(layout.a.clone()));
        assert!(peer.get_help_text().ends_with("ActiveDockable=Document A"));
    });
}
#[test]
fn actual_tunnel_pointer_input_uses_current_model_factory_and_preserves_keep_visible() {
    run(|| {
        let layout = Layout::new();
        let control = RootDockControl::new();
        control.set_data_context(Some(layout.root.clone()));
        template(&control, "PART_MainContent");
        let scene = Scene::new(control.clone().upcast());
        layout.preview();
        click(&scene);
        assert!(layout.root.pinned_dock().is_none());
        layout.preview();
        layout.tool.set_keep_pinned_dockable_visible(true);
        click(&scene);
        assert!(layout.root.pinned_dock().is_some());
        layout.tool.set_keep_pinned_dockable_visible(false);
        layout.root.set_factory(None);
        click(&scene);
        assert!(layout.root.pinned_dock().is_some());
        let other = Layout::new();
        other.preview();
        control.set_data_context(Some(other.root.clone()));
        click(&scene);
        assert!(other.root.pinned_dock().is_none());
        assert!(layout.root.pinned_dock().is_some());
        control.set_data_context(Some(Rc::new("ordinary".to_owned())));
        click(&scene);
    });
}
#[test]
fn missing_or_renamed_template_part_is_optional_and_tunnel_handler_skips_already_handled_input() {
    run(|| {
        let layout = Layout::new();
        let control = RootDockControl::new();
        control.set_data_context(Some(layout.root.clone()));
        template(&control, "wrong_name");
        let scene = Scene::new(control.clone().upcast());
        layout.preview();
        click(&scene);
        assert!(layout.root.pinned_dock().is_some());
        template(&control, "PART_MainContent");
        scene.update();
        let token = control.add_handler_with(
            InputElement::pointer_pressed_event(),
            |_, e| e.set_handled(true),
            RoutingStrategies::TUNNEL,
            false,
        );
        click(&scene);
        assert!(layout.root.pinned_dock().is_some());
        control.remove_handler(InputElement::pointer_pressed_event(), token);
        click(&scene);
        assert!(layout.root.pinned_dock().is_none());
    });
}
#[test]
fn helper_activation_runs_original_factory_and_selection_and_container_peers_keep_identity() {
    run(|| {
        let layout = Layout::new();
        assert!(!Helper::try_activate_dockable(None));
        let orphan = Document::new();
        assert!(!Helper::try_activate_dockable(Some(orphan.clone())));
        orphan.set_owner(Some(layout.a.clone()));
        assert!(!Helper::try_activate_dockable(Some(orphan)));
        layout.tools.set_factory(None);
        assert!(!Helper::try_activate_dockable(Some(layout.tool.clone())));
        layout.tools.set_factory(Some(layout.factory.clone()));
        assert!(Helper::try_activate_dockable(Some(layout.b.clone())));
        assert!(Rc::ptr_eq(
            &layout.documents.active_dockable().unwrap(),
            &(layout.b.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &layout.root.focused_dockable().unwrap(),
            &(layout.b.clone() as Rc<dyn IDockable>)
        ));
        let items = ItemsControl::new();
        let child = Border::new();
        let item = Control::boxed(child.clone());
        items.items().add(Some(item.clone()));
        let _scene = Scene::new(items.clone().upcast());
        assert!(Helper::try_get_container_peer(&items, None).is_none());
        let peer = Helper::try_get_container_peer(&items, Some(&item))
            .expect("native Fluent ItemsControl creates a container");
        assert_eq!(peer, ControlAutomationPeer::create_peer_for_element(&child));
        assert_eq!(Helper::to_selection_list(Some(peer.clone())), vec![peer]);
        let peer = ControlAutomationPeer::create_peer_for_element(&child);
        assert_eq!(Helper::to_selection_list(Some(peer.clone())), vec![peer]);
        assert!(Helper::to_selection_list(None).is_empty());
    });
}
#[test]
fn helper_name_id_and_state_format_retain_source_precedence_null_bool_and_enum_text() {
    run(|| {
        let layout = Layout::new();
        let owner = Border::new();
        owner.set_data_context(Some(layout.a.clone()));
        assert_eq!(
            Helper::resolve_name(&owner, "fallback", Some(layout.b.clone())),
            "Document B"
        );
        layout.b.set_title(" \u{85}".into());
        assert_eq!(
            Helper::resolve_name(&owner, "fallback", Some(layout.b.clone())),
            "Document A"
        );
        layout.b.set_id(" \t".into());
        assert_eq!(
            Helper::resolve_automation_id(&owner, Some(layout.b.clone())),
            "DocA"
        );
        let text = Helper::format_state(&[
            ("Null", None),
            ("Bool", Some(Rc::new(false))),
            ("Integer", Some(Rc::new(12_i32))),
            ("Operation", Some(Rc::new(DockOperation::OuterLeft))),
            ("Text", Some(Rc::new("value".to_owned()))),
        ]);
        assert_eq!(
            text,
            "Null=null; Bool=false; Integer=12; Operation=OuterLeft; Text=value"
        );
        assert_eq!(Helper::format_state(&[]), "");
    });
}

#[repr(C)]
struct DerivedRoot {
    base: RootDockControl,
}
ferro_class!(DerivedRoot:RootDockControl);
ferro_impl_classes!(DerivedRoot:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
#[test]
fn derived_root_inherits_template_and_peer_contract_without_overriding_source_role() {
    run(|| {
        let layout = Layout::new();
        let derived: Ref<DerivedRoot> = instantiate(DerivedRoot {
            base: RootDockControl::construct(),
        });
        derived.set_data_context(Some(layout.root.clone()));
        template(&derived, "PART_MainContent");
        let scene = Scene::new(derived.clone().upcast());
        layout.preview();
        click(&scene);
        assert!(layout.root.pinned_dock().is_none());
        let peer = ControlAutomationPeer::create_peer_for_element(&derived);
        assert_eq!(peer.get_class_name(), "RootDockControl");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
    });
}
#[test]
fn retemplating_unsubscribes_old_part_and_detach_clears_the_registered_part() {
    run(|| {
        let layout = Layout::new();
        let control = RootDockControl::new();
        control.set_data_context(Some(layout.root.clone()));
        let old = template(&control, "PART_MainContent");
        let scene = Scene::new(control.clone().upcast());
        let old = old.borrow().clone().unwrap();
        template(&control, "PART_MainContent");
        scene.update();
        assert!(old.visual_parent().is_none());
        let old_scene = Scene::new(old.upcast());
        layout.preview();
        click(&old_scene);
        assert!(layout.root.pinned_dock().is_some());
        click(&scene);
        assert!(layout.root.pinned_dock().is_none());
        drop(old_scene);
        layout.preview();
        scene.0.set_content(None);
        scene.update();
        scene.0.set_content(Some(Control::boxed(control.clone())));
        scene.update();
        // Source detachment clears the part and handler; it has no attach-time
        // registration. The existing template alone does not re-add that handler.
        click(&scene);
        assert!(layout.root.pinned_dock().is_some());
        template(&control, "PART_MainContent");
        scene.update();
        click(&scene);
        assert!(layout.root.pinned_dock().is_none());
    });
}

struct ItemsWithMissingIndex {
    item: ferroui_base::BoxedValue,
    reads: Rc<RefCell<Vec<usize>>>,
}
impl ferroui_controls::IItemsList for ItemsWithMissingIndex {
    fn count(&self) -> usize {
        2
    }
    fn get_at(&self, index: usize) -> Option<ferroui_base::BoxedValue> {
        self.reads.borrow_mut().push(index);
        Some(self.item.clone())
    }
    fn index_of(&self, _: &Option<ferroui_base::BoxedValue>) -> i32 {
        -1
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
#[test]
fn helper_container_fallback_reads_live_items_and_stops_at_first_unrealized_match() {
    run(|| {
        let reads = Rc::new(RefCell::new(Vec::new()));
        let item: ferroui_base::BoxedValue = Rc::new("item".to_owned());
        let owner = ItemsControl::new();
        owner.set_items_source(Some(ferroui_controls::ItemsSource::new(Rc::new(
            ItemsWithMissingIndex {
                item: item.clone(),
                reads: reads.clone(),
            },
        ))));
        reads.borrow_mut().clear();
        assert!(Helper::try_get_container_peer(&owner, Some(&item)).is_none());
        assert_eq!(&*reads.borrow(), &[0]);
        let scene = Scene::new(owner.clone().upcast());
        reads.borrow_mut().clear();
        let peer = Helper::try_get_container_peer(&owner, Some(&item))
            .expect("fallback locates realized index zero");
        assert_eq!(
            peer,
            ControlAutomationPeer::create_peer_for_element(&owner.container_from_index(0).unwrap())
        );
        assert_eq!(&*reads.borrow(), &[0]);
        assert!(Helper::try_get_container_peer(
            &owner,
            Some(&(Rc::new("missing".to_owned()) as ferroui_base::BoxedValue))
        )
        .is_none());
        drop(scene);
    });
}
#[derive(Clone, Copy, PartialEq)]
enum NamedValue {
    Original = 1,
}
impl std::fmt::Display for NamedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("custom display")
    }
}
ferroui_base::ferro_markup_enum!(NamedValue {Original},{namespace:"Tests.RootControl"});
#[test]
fn helper_enum_formatting_precedes_general_registered_object_display() {
    run(|| {
        use ferroui_base::metadata::{MarkupType, MarkupTyped};
        MarkupType::register(NamedValue::MARKUP);
        ValueTypes::register_display::<NamedValue>();
        assert_eq!(
            Helper::format_state(&[("Value", Some(Rc::new(NamedValue::Original)))]),
            "Value=Original"
        );
    });
}

struct RelayFactory {
    component: Factory,
    label: &'static str,
    calls: Rc<RefCell<Vec<(&'static str, &'static str)>>>,
    owner: std::rc::Weak<dyn IDock>,
    root: std::rc::Weak<dyn IRootDock>,
    next: RefCell<Option<Rc<dyn IFactory>>>,
}
impl RelayFactory {
    fn new(
        label: &'static str,
        layout: &Layout,
        calls: Rc<RefCell<Vec<(&'static str, &'static str)>>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            component: Factory::construct(runtime.clone()),
            label,
            calls,
            owner: Rc::downgrade(&(layout.documents.clone() as Rc<dyn IDock>)),
            root: Rc::downgrade(&(layout.root.clone() as Rc<dyn IRootDock>)),
            next: RefCell::new(None),
        })
    }
    fn record_and_replace(&self, operation: &'static str) {
        self.calls.borrow_mut().push((self.label, operation));
        let next = self.next.borrow().clone();
        if let Some(next) = next {
            self.owner.upgrade().unwrap().set_factory(Some(next));
        }
    }
}
impl ferrodock_model::FactoryBaseImpl for RelayFactory {
    fn factory_base(&self) -> &ferrodock_model::FactoryBase {
        self.component.base()
    }
}
impl ferrodock_model_reactiveui::FactoryImpl for RelayFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn set_active_dockable(this: &Self, _dockable: Option<Rc<dyn IDockable>>) {
        this.record_and_replace("active");
    }
    fn find_root(
        this: &Self,
        _dockable: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        if let Some(predicate) = predicate {
            let root = this.root.upgrade().unwrap();
            assert!(predicate(root.clone()));
            this.record_and_replace("root");
            Some(root)
        } else {
            // The original nonvirtual ActivateWindow calls virtual FindRoot(None).
            this.record_and_replace("window");
            None
        }
    }
    fn set_focused_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
    ) {
        assert!(Rc::ptr_eq(
            &(dock.expect("nonnull focus fixture") as Rc<dyn IDockable>),
            &(this.root.upgrade().unwrap() as Rc<dyn IDockable>)
        ));
        assert_eq!(dockable.unwrap().id(), "DocB");
        this.record_and_replace("focus");
    }
}
ferrodock_model_reactiveui::impl_factory!(RelayFactory);
#[test]
fn helper_activation_rereads_original_owner_factory_after_each_callback() {
    run(|| {
        let layout = Layout::new();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let a = RelayFactory::new("a", &layout, calls.clone());
        let b = RelayFactory::new("b", &layout, calls.clone());
        let c = RelayFactory::new("c", &layout, calls.clone());
        let d = RelayFactory::new("d", &layout, calls.clone());
        a.next.replace(Some(b.clone()));
        b.next.replace(Some(c.clone()));
        c.next.replace(Some(d.clone()));
        layout.documents.set_factory(Some(a.clone()));
        assert!(Helper::try_activate_dockable(Some(layout.b.clone())));
        assert_eq!(
            &*calls.borrow(),
            &[
                ("a", "active"),
                ("b", "root"),
                ("c", "focus"),
                ("d", "window")
            ]
        );
    });
}
