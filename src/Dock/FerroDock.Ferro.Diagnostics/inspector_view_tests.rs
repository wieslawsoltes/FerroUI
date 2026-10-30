//! The actual compiled inspector views edit original models and follow notifications.
use crate::controls::*;
use ferrodock_model::{collections::ListHandle, controls::*, core::*};
use ferrodock_model_reactiveui::{controls::*, core::DockWindow};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    threading::{CancellationToken, Dispatcher},
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, TypeInfo, VisualImpl,
};
use ferroui_controls::{
    primitives::TemplatedControlImpl, AppBuilder, Application, CheckBox, ContentControlImpl,
    Control, ControlImpl, Grid, TextBox, UserControl, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    use ferroui_fonts_inter::AppBuilderExtension;
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with_inter_font()
        .after_setup(|_| {
            let app = Application::current().unwrap();
            app.styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style());
            app.styles()
                .add(ferrodock_ferro_themes_fluent::DockFluentTheme::new().as_style());
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-inspector-property-views",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn drain(window: &Window) {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
    }
}
fn show(view: &Ref<UserControl>, value: BoxedValue) -> Ref<Window> {
    view.set_data_context(Some(value));
    let window = Window::new();
    window.set_width(600.0);
    window.set_height(900.0);
    window.set_content(Some(Control::boxed(view.clone())));
    window.show();
    drain(&window);
    window
}
fn grid(view: &UserControl) -> Ref<Grid> {
    ferroui_base::data::core::ValueTypes::as_object(&*view.content().unwrap())
        .unwrap()
        .cast::<Grid>()
        .expect("source Grid")
}
fn field(view: &UserControl, row: i32) -> Ref<Control> {
    grid(view)
        .children()
        .snapshot()
        .iter()
        .find(|child| Grid::get_row(child) == row && Grid::get_column(child) == 1)
        .cloned()
        .unwrap()
}
fn text(view: &UserControl, row: i32) -> Ref<TextBox> {
    field(view, row).cast().unwrap()
}
fn check(view: &UserControl, row: i32) -> Ref<CheckBox> {
    field(view, row).cast().unwrap()
}
fn dockable(model: Rc<dyn IDockable>) -> BoxedValue {
    Rc::new(model)
}
#[test]
fn public_source_view_classes_construct_the_complete_original_grids() {
    run(|| {
        let cases: [(&str, usize); 18] = [
            ("DockableDebugView", 29),
            ("DockDebugView", 16),
            ("DockWindowDebugView", 11),
            ("RootDockInfoDebugView", 8),
            ("DockDockDebugView", 1),
            ("GridDockDebugView", 2),
            ("GridDockSplitterDebugView", 1),
            ("ProportionalDockDebugView", 1),
            ("ProportionalDockSplitterDebugView", 1),
            ("StackDockDebugView", 2),
            ("WrapDockDebugView", 1),
            ("UniformGridDockDebugView", 2),
            ("DocumentDockDebugView", 4),
            ("ToolDockDebugView", 4),
            ("DocumentContentDebugView", 1),
            ("ToolContentDebugView", 1),
            ("DocumentTemplateDebugView", 1),
            ("DocumentDockContentDebugView", 1),
        ];
        for (name, rows) in cases {
            let ty = TypeInfo::find("FerroDock.Ferro.Diagnostics.Controls", name).expect(name);
            let value = ty.default_constructor().expect("public source constructor")();
            let view = value.cast::<UserControl>().expect(name);
            assert_eq!(grid(&view).children().count(), rows * 2, "{name}");
            let expected = include_str!("../../../tests/Dock/data/inspector-view-grids.tsv")
                .lines()
                .find(|line| line.starts_with(&format!("{name}\t")))
                .unwrap();
            let source: Vec<_> = expected.split('\t').collect();
            let actual_grid = grid(&view);
            assert_eq!(
                actual_grid.row_definitions().count().to_string(),
                source[1],
                "{name} rows"
            );
            assert_eq!(
                actual_grid.column_definitions().count().to_string(),
                source[2],
                "{name} columns"
            );
            let actual = actual_grid
                .children()
                .snapshot()
                .iter()
                .map(|child| {
                    format!(
                        "{},{},{},{},{}",
                        child.get_type().name(),
                        Grid::get_row(child),
                        Grid::get_column(child),
                        i32::from(child.is_enabled()),
                        i32::from(
                            child
                                .cast::<TextBox>()
                                .is_some_and(|text| text.is_read_only())
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(actual, source[3], "original {name} fields");
            for row in 0..rows {
                assert!(
                    field(&view, row as i32).is::<TextBox>()
                        || field(&view, row as i32).is::<CheckBox>(),
                    "{name} row {row}"
                );
            }
        }
    });
}
#[repr(C)]
struct DerivedInspector {
    base: DockableDebugView,
}
ferro_class!(DerivedInspector:DockableDebugView);
ferro_class_info!(DerivedInspector {
    new: DerivedInspector::new
});
ferro_impl_classes!(DerivedInspector:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl DerivedInspector {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: DockableDebugView::construct(),
        })
    }
}
#[test]
fn inherited_constructor_initializes_the_source_view_once() {
    run(|| {
        let view = DerivedInspector::new();
        assert_eq!(grid(&view).children().count(), 58);
        let model = Tool::new();
        model.set_title("Derived original model".into());
        let base = view.upcast::<UserControl>();
        let window = show(&base, dockable(model));
        assert_eq!(
            text(&base, 1).text().as_deref(),
            Some("Derived original model")
        );
        window.close();
    });
}
#[test]
fn dockable_inspector_edits_original_values_and_rebinds_nested_owners() {
    run(|| {
        let model = Tool::new();
        model.set_id("tool-id".into());
        model.set_title("tool-title".into());
        let owner = ToolDock::new();
        owner.set_title("first owner".into());
        model.set_owner(Some(owner.clone()));
        let view = DockableDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        assert_eq!(text(&view, 0).text().as_deref(), Some("tool-id"));
        assert_eq!(text(&view, 3).text().as_deref(), Some("first owner"));
        for row in [2, 3, 4, 5] {
            assert!(text(&view, row).is_read_only());
        }
        text(&view, 1).set_text(Some("edited title"));
        check(&view, 20).set_is_checked(Some(false));
        drain(&window);
        assert_eq!(model.title(), "edited title");
        assert!(!model.can_close());
        model.set_column(7);
        model.set_is_modified(true);
        drain(&window);
        assert_eq!(text(&view, 10).text().as_deref(), Some("7"));
        assert_eq!(check(&view, 28).is_checked(), Some(true));
        let replacement = ToolDock::new();
        replacement.set_title("second owner".into());
        model.set_owner(Some(replacement.clone()));
        drain(&window);
        assert_eq!(text(&view, 3).text().as_deref(), Some("second owner"));
        owner.set_title("detached owner".into());
        replacement.set_title("updated owner".into());
        drain(&window);
        assert_eq!(text(&view, 3).text().as_deref(), Some("updated owner"));
        model.set_owner(None);
        drain(&window);
        assert!(text(&view, 3).text().is_none_or(|value| value.is_empty()));
        window.close();
    });
}
#[test]
fn dock_inspector_preserves_read_only_navigation_and_editable_open_count() {
    run(|| {
        let model = DocumentDock::new();
        let active = Document::new();
        active.set_title("active".into());
        model.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            active.clone() as Rc<dyn IDockable>
        )])));
        model.set_active_dockable(Some(active.clone()));
        let view = DockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        assert_eq!(text(&view, 4).text().as_deref(), Some("active"));
        for row in [10, 11] {
            assert!(!check(&view, row).is_enabled());
        }
        text(&view, 12).set_text(Some("12"));
        check(&view, 13).set_is_checked(Some(false));
        drain(&window);
        assert_eq!(model.opened_dockables_count(), 12);
        assert!(!model.can_close_last_dockable());
        active.set_title("renamed active".into());
        drain(&window);
        assert_eq!(text(&view, 4).text().as_deref(), Some("renamed active"));
        window.close();
    });
}
#[test]
fn root_inspector_observes_collection_mutation_replacement_and_pinned_title() {
    run(|| {
        let model = RootDock::new();
        let hidden = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        model.set_hidden_dockables(Some(hidden.clone()));
        let pinned = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        model.set_left_pinned_dockables(Some(pinned.clone()));
        let view = RootDockInfoDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        assert_eq!(text(&view, 1).text().as_deref(), Some("0"));
        hidden.add(Some(Tool::new()));
        pinned.add(Some(Tool::new()));
        drain(&window);
        assert_eq!(text(&view, 1).text().as_deref(), Some("1"));
        assert_eq!(text(&view, 2).text().as_deref(), Some("1"));
        let replacement = ListHandle::from_items(vec![
            Some(Tool::new() as Rc<dyn IDockable>),
            Some(Tool::new() as Rc<dyn IDockable>),
        ]);
        model.set_hidden_dockables(Some(replacement));
        hidden.add(Some(Tool::new()));
        drain(&window);
        assert_eq!(text(&view, 1).text().as_deref(), Some("2"));
        let dock = ToolDock::new();
        dock.set_title("pinned".into());
        model.set_pinned_dock(Some(dock.clone()));
        drain(&window);
        assert_eq!(text(&view, 6).text().as_deref(), Some("pinned"));
        dock.set_title("renamed pinned".into());
        check(&view, 0).set_is_checked(Some(false));
        drain(&window);
        assert_eq!(text(&view, 6).text().as_deref(), Some("renamed pinned"));
        assert!(!model.is_focusable_root());
        window.close();
    });
}
#[test]
fn dock_window_inspector_edits_geometry_and_observes_layout_title() {
    run(|| {
        let model = DockWindow::new();
        model.set_x(31.0);
        let root = RootDock::new();
        root.set_title("layout".into());
        model.set_layout(Some(root.clone()));
        let view = DockWindowDebugView::new().upcast::<UserControl>();
        let window = show(&view, Rc::new(model.clone() as Rc<dyn IDockWindow>));
        assert_eq!(text(&view, 1).text().as_deref(), Some("31"));
        text(&view, 3).set_text(Some("720"));
        check(&view, 5).set_is_checked(Some(true));
        drain(&window);
        assert_eq!(model.width(), 720.0);
        assert!(model.topmost());
        root.set_title("renamed layout".into());
        drain(&window);
        assert_eq!(text(&view, 9).text().as_deref(), Some("renamed layout"));
        for row in [7, 8, 9, 10] {
            assert!(text(&view, row).is_read_only());
        }
        window.close();
    });
}
#[test]
fn splitter_inspectors_edit_live_original_models_through_erased_contracts() {
    run(|| {
        let proportional = ProportionalDockSplitter::new();
        let view = ProportionalDockSplitterDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(proportional.clone()));
        assert_eq!(check(&view, 0).is_checked(), Some(true));
        check(&view, 0).set_is_checked(Some(false));
        assert!(!proportional.can_resize());
        proportional.set_can_resize(true);
        drain(&window);
        assert_eq!(check(&view, 0).is_checked(), Some(true));
        window.close();
        let model = GridDockSplitter::new();
        let view = GridDockSplitterDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        assert_eq!(text(&view, 0).text().as_deref(), Some("Columns"));
        text(&view, 0).set_text(Some("Rows"));
        drain(&window);
        assert_eq!(model.resize_direction(), GridResizeDirection::Rows);
        model.set_resize_direction(GridResizeDirection::Columns);
        drain(&window);
        assert_eq!(text(&view, 0).text().as_deref(), Some("Columns"));
        window.close();
    });
}
#[test]
fn container_inspectors_edit_each_original_layout_contract() {
    run(|| {
        let model = DockDock::new();
        let view = DockDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        check(&view, 0).set_is_checked(Some(false));
        assert!(!model.last_child_fill());
        window.close();
        let model = GridDock::new();
        let view = GridDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        text(&view, 0).set_text(Some("2*,Auto"));
        text(&view, 1).set_text(Some("Auto,*"));
        drain(&window);
        assert_eq!(model.column_definitions().as_deref(), Some("2*,Auto"));
        assert_eq!(model.row_definitions().as_deref(), Some("Auto,*"));
        window.close();
        let model = StackDock::new();
        let view = StackDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        text(&view, 0).set_text(Some("Vertical"));
        text(&view, 1).set_text(Some("8.5"));
        drain(&window);
        assert_eq!(model.orientation(), Orientation::Vertical);
        assert_eq!(model.spacing(), 8.5);
        window.close();
        let model = WrapDock::new();
        let view = WrapDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        text(&view, 0).set_text(Some("Vertical"));
        drain(&window);
        assert_eq!(model.orientation(), Orientation::Vertical);
        window.close();
        let model = ProportionalDock::new();
        let view = ProportionalDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        text(&view, 0).set_text(Some("Vertical"));
        drain(&window);
        assert_eq!(model.orientation(), Orientation::Vertical);
        window.close();
        let model = UniformGridDock::new();
        let view = UniformGridDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        text(&view, 0).set_text(Some("3"));
        text(&view, 1).set_text(Some("4"));
        drain(&window);
        assert_eq!(model.rows(), 3);
        assert_eq!(model.columns(), 4);
        window.close();
    });
}
#[test]
fn document_and_tool_inspectors_edit_original_presentation_properties() {
    run(|| {
        let model = DocumentDock::new();
        let view = DocumentDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        check(&view, 0).set_is_checked(Some(true));
        check(&view, 2).set_is_checked(Some(true));
        text(&view, 3).set_text(Some("Right"));
        drain(&window);
        assert!(model.can_create_document());
        assert!(model.enable_window_drag());
        assert_eq!(model.tabs_layout(), DocumentTabLayout::Right);
        assert!(text(&view, 1).is_read_only());
        window.close();
        let model = ToolDock::new();
        let view = ToolDockDebugView::new().upcast::<UserControl>();
        let window = show(&view, dockable(model.clone()));
        text(&view, 0).set_text(Some("Right"));
        check(&view, 1).set_is_checked(Some(true));
        check(&view, 2).set_is_checked(Some(false));
        text(&view, 3).set_text(Some("Hidden"));
        drain(&window);
        assert_eq!(model.alignment(), Alignment::Right);
        assert!(model.is_expanded());
        assert!(!model.auto_hide());
        assert_eq!(model.grip_mode(), GripMode::Hidden);
        window.close();
    });
}

use ferrodock_model_reactiveui::{
    core::{DockBase, DockableBase, DockableBaseImpl, ReactiveBase},
    impl_dock, impl_dockable,
};
use ferroui_reactiveui::{reactive_object, IReactiveObject};
use std::{cell::RefCell, rc::Weak};
struct ContentModel {
    base: DockableBase,
    value: RefCell<Option<ferrodock_model::BoxedValue>>,
}
reactive_object!(ContentModel, base);
impl DockableBaseImpl for ContentModel {}
impl_dockable!(
    ContentModel,
    fn as_document_content(self: Rc<Self>) -> Option<Rc<dyn IDocumentContent>> {
        Some(self)
    },
    fn as_tool_content(self: Rc<Self>) -> Option<Rc<dyn IToolContent>> {
        Some(self)
    }
);
impl ContentModel {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DockableBase::construct(weak.clone()),
            value: RefCell::new(None),
        })
    }
}
impl IDocumentContent for ContentModel {
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.value.borrow().clone()
    }
    fn set_content(&self, value: Option<ferrodock_model::BoxedValue>) {
        self.value.replace(value);
        self.raise_property_changed("Content");
    }
}
impl IToolContent for ContentModel {
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        IDocumentContent::content(self)
    }
    fn set_content(&self, value: Option<ferrodock_model::BoxedValue>) {
        IDocumentContent::set_content(self, value);
    }
}
struct Template {
    base: ReactiveBase,
    value: RefCell<Option<ferrodock_model::BoxedValue>>,
}
reactive_object!(Template, base);
impl Template {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ReactiveBase::default(),
            value: RefCell::new(None),
        })
    }
}
impl IDocumentTemplate for Template {
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.value.borrow().clone()
    }
    fn set_content(&self, value: Option<ferrodock_model::BoxedValue>) {
        self.value.replace(value);
        self.raise_property_changed("Content");
    }
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        Some(self)
    }
}
struct ContentDock {
    base: DockBase,
    template: RefCell<Option<Rc<dyn IDocumentTemplate>>>,
}
reactive_object!(ContentDock, base);
impl DockableBaseImpl for ContentDock {}
impl_dock!(
    ContentDock,
    fn as_document_dock_content(self: Rc<Self>) -> Option<Rc<dyn IDocumentDockContent>> {
        Some(self)
    }
);
impl ContentDock {
    fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DockBase::construct(weak.clone()),
            template: RefCell::new(None),
        });
        model.base.initialize(&model);
        model
    }
}
impl IDocumentDockContent for ContentDock {
    fn document_template(&self) -> Option<Rc<dyn IDocumentTemplate>> {
        self.template.borrow().clone()
    }
    fn set_document_template(&self, value: Option<Rc<dyn IDocumentTemplate>>) {
        self.template.replace(value);
        self.raise_property_changed("DocumentTemplate");
    }
    fn create_document_from_template(&self) -> Option<ferrodock_model::BoxedValue> {
        self.document_template()
            .and_then(|template| template.content())
    }
}
#[test]
fn content_inspectors_show_actual_objects_and_follow_optional_notifications() {
    run(|| {
        let model = ContentModel::new();
        for view in [
            DocumentContentDebugView::new().upcast::<UserControl>(),
            ToolContentDebugView::new().upcast::<UserControl>(),
        ] {
            IDocumentContent::set_content(&*model, Some(Rc::new("source content".to_owned())));
            let window = show(&view, dockable(model.clone()));
            assert!(text(&view, 0).is_read_only());
            assert_eq!(text(&view, 0).text().as_deref(), Some("source content"));
            IDocumentContent::set_content(&*model, Some(Rc::new("changed content".to_owned())));
            drain(&window);
            assert_eq!(text(&view, 0).text().as_deref(), Some("changed content"));
            window.close();
        }
        let model = Template::new();
        model.set_content(Some(Rc::new("template content".to_owned())));
        let view = DocumentTemplateDebugView::new().upcast::<UserControl>();
        let window = show(&view, Rc::new(model.clone() as Rc<dyn IDocumentTemplate>));
        assert_eq!(text(&view, 0).text().as_deref(), Some("template content"));
        assert!(text(&view, 0).is_read_only());
        model.set_content(Some(Rc::new("updated template".to_owned())));
        drain(&window);
        assert_eq!(text(&view, 0).text().as_deref(), Some("updated template"));
        model.set_content(None);
        drain(&window);
        assert!(text(&view, 0).text().is_none_or(|s| s.is_empty()));
        window.close();
    });
}
#[test]
fn document_template_metadata_retains_identity_nulls_and_original_factory_dispatch() {
    run(|| {
        use ferroui_base::{
            data::core::{ValueType, ValueTypes},
            metadata::MarkupType,
        };
        use std::any::TypeId;
        let model = ContentDock::new();
        let receiver = dockable(model.clone());
        let metadata =
            MarkupType::find_by_handle(TypeId::of::<Rc<dyn IDocumentDockContent>>()).unwrap();
        assert_eq!(metadata.base_type().unwrap().name, "IDock");
        let property = metadata.find_property("DocumentTemplate").unwrap();
        let template = Template::new();
        let handle: Rc<dyn IDocumentTemplate> = template.clone();
        let boxed: BoxedValue = Rc::new(handle.clone());
        property.set.unwrap()(&[Some(receiver.clone()), Some(boxed)]).unwrap();
        assert!(Rc::ptr_eq(&model.document_template().unwrap(), &handle));
        let actual = property.get.unwrap()(&[Some(receiver.clone())])
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(
            actual.downcast_ref::<Rc<dyn IDocumentTemplate>>().unwrap(),
            &handle
        ));
        let template_metadata =
            MarkupType::find_by_handle(TypeId::of::<Rc<dyn IDocumentTemplate>>()).unwrap();
        let content = template_metadata.find_property("Content").unwrap();
        let value: BoxedValue = Rc::new("original content identity".to_owned());
        content.set.unwrap()(&[Some(actual.clone()), Some(value.clone())]).unwrap();
        let result = metadata
            .find_methods("CreateDocumentFromTemplate")
            .next()
            .unwrap();
        let created = (result.invoke)(&[Some(receiver.clone())]).unwrap().unwrap();
        assert!(Rc::ptr_eq(&created, &value));
        assert!(
            ValueTypes::try_cast(&receiver, ValueType::of::<Rc<dyn IDocumentDockContent>>())
                .is_some()
        );
        assert!(ValueTypes::try_cast(
            &dockable(Tool::new()),
            ValueType::of::<Rc<dyn IDocumentDockContent>>()
        )
        .is_none());
        let view = DocumentDockContentDebugView::new().upcast::<UserControl>();
        let window = show(&view, receiver.clone());
        assert!(text(&view, 0).is_read_only());
        property.set.unwrap()(&[Some(receiver.clone()), None]).unwrap();
        drain(&window);
        assert!(model.document_template().is_none());
        assert!(text(&view, 0).text().is_none_or(|s| s.is_empty()));
        assert!((result.invoke)(&[Some(receiver)]).unwrap().is_none());
        window.close();
    });
}
#[test]
fn splitter_metadata_keeps_inherited_contracts_and_preview_setter() {
    run(|| {
        use ferroui_base::{
            data::core::{ValueType, ValueTypes},
            metadata::MarkupType,
        };
        use std::any::TypeId;
        let model = ProportionalDockSplitter::new();
        let receiver = dockable(model.clone());
        let metadata =
            MarkupType::find_by_handle(TypeId::of::<Rc<dyn IProportionalDockSplitter>>()).unwrap();
        assert_eq!(metadata.base_type().unwrap().name, "ISplitter");
        assert_eq!(
            metadata.base_type().unwrap().base_type().unwrap().name,
            "IDockable"
        );
        let property = metadata.find_property("ResizePreview").unwrap();
        property.set.unwrap()(&[Some(receiver.clone()), Some(Rc::new(true))]).unwrap();
        assert!(model.resize_preview());
        assert_eq!(
            property.get.unwrap()(&[Some(receiver.clone())])
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>(),
            Some(&true)
        );
        let splitter =
            ValueTypes::try_cast(&receiver, ValueType::of::<Rc<dyn ISplitter>>()).unwrap();
        assert!(ValueTypes::try_cast(
            &splitter,
            ValueType::of::<Rc<dyn IProportionalDockSplitter>>()
        )
        .is_some());
        assert!(
            ValueTypes::try_cast(&dockable(Tool::new()), ValueType::of::<Rc<dyn ISplitter>>())
                .is_none()
        );
        assert!(
            ValueTypes::try_cast(&receiver, ValueType::of::<Rc<dyn IGridDockSplitter>>()).is_none()
        );
    });
}

#[test]
fn enum_text_edits_match_the_24_unchanged_original_view_cases() {
    run(|| {
        let grid_model = GridDockSplitter::new();
        let grid = GridDockSplitterDebugView::new().upcast::<UserControl>();
        let doc_model = DocumentDock::new();
        let doc = DocumentDockDebugView::new().upcast::<UserControl>();
        let stack_model = StackDock::new();
        let stack = StackDockDebugView::new().upcast::<UserControl>();
        let grid_window = show(&grid, dockable(grid_model.clone()));
        let doc_window = show(&doc, dockable(doc_model.clone()));
        let stack_window = show(&stack, dockable(stack_model.clone()));
        let mut count = 0;
        for line in include_str!("../../../tests/Dock/data/inspector-view-grids.tsv")
            .lines()
            .filter(|line| line.starts_with("# edit\t"))
        {
            let fields: Vec<_> = line.split('\t').collect();
            let expected = fields[3].parse::<i32>().unwrap();
            let actual = match fields[1] {
                "GridSplitter" => {
                    grid_model.set_resize_direction(GridResizeDirection::Columns);
                    drain(&grid_window);
                    text(&grid, 0).set_text(Some(fields[2]));
                    drain(&grid_window);
                    grid_model.resize_direction() as i32
                }
                "DocumentDock" => {
                    doc_model.set_tabs_layout(DocumentTabLayout::Top);
                    drain(&doc_window);
                    text(&doc, 3).set_text(Some(fields[2]));
                    drain(&doc_window);
                    doc_model.tabs_layout() as i32
                }
                "StackDock" => {
                    stack_model.set_orientation(Orientation::Horizontal);
                    drain(&stack_window);
                    text(&stack, 0).set_text(Some(fields[2]));
                    drain(&stack_window);
                    stack_model.orientation() as i32
                }
                other => panic!("Unknown source fixture {other}"),
            };
            assert_eq!(actual, expected, "{line}");
            count += 1;
        }
        assert_eq!(count, 24);
        grid_window.close();
        doc_window.close();
        stack_window.close();
    });
}
