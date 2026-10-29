//! Original container/content control behavior with actual native templates and models.
use crate::{controls::*, model_projection};
use ferrodock_controls_recycling::{from_recycling_value, to_recycling_value};
use ferrodock_model::{core::*, BoxedValue as ModelValue};
use ferrodock_model_reactiveui::{
    controls::{Document, GridDock, SplitViewDock},
    Factory,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::metadata::{MarkupTypeKind, MarkupTyped};
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{
    presenters::ContentPresenter,
    primitives::TemplatedControl,
    templates::{FuncControlTemplate, FuncTemplate},
    AppBuilder, Application, Control, Grid, ItemsControl, Panel, SplitView, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-original-containers",
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
    fn new(content: Ref<Control>) -> Self {
        let w = Window::new();
        w.set_width(640.0);
        w.set_height(480.0);
        w.set_content(Some(Control::boxed(content)));
        w.show();
        Dispatcher::ui_thread().run_jobs(None);
        w.update_layout();
        Self(w)
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
#[derive(Default)]
struct Metadata {
    theme: RefCell<Option<ModelValue>>,
    selector: RefCell<Option<ModelValue>>,
    reads: Cell<usize>,
    read_hook: RefCell<Option<Rc<dyn Fn()>>>,
}
impl IDockItemContainerMetadata for Metadata {
    fn item_container_theme(&self) -> Option<ModelValue> {
        self.reads.set(self.reads.get() + 1);
        let hook = self.read_hook.borrow().clone();
        if let Some(hook) = hook {
            hook();
        }
        self.theme.borrow().clone()
    }
    fn set_item_container_theme(&self, value: Option<ModelValue>) {
        self.theme.replace(value);
    }
    fn item_template_selector(&self) -> Option<ModelValue> {
        self.selector.borrow().clone()
    }
    fn set_item_template_selector(&self, value: Option<ModelValue>) {
        self.selector.replace(value);
    }
}
struct Generated {
    base: ManagedDockableBase,
    metadata: Metadata,
}
crate::impl_managed_dockable!(
    Generated,
    fn as_dock_item_container_metadata(&self) -> Option<&dyn IDockItemContainerMetadata> {
        Some(self)
    }
);
impl ManagedDockableBaseImpl for Generated {}
impl Generated {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
            metadata: Metadata::default(),
        })
    }
}
impl IDockItemContainerMetadata for Generated {
    fn as_dockable(self: Rc<Self>) -> Option<Rc<dyn IDockable>> {
        Some(self)
    }
    fn item_container_theme(&self) -> Option<ModelValue> {
        self.metadata.item_container_theme()
    }
    fn set_item_container_theme(&self, value: Option<ModelValue>) {
        self.metadata.set_item_container_theme(value);
    }
    fn item_template_selector(&self) -> Option<ModelValue> {
        self.metadata.item_template_selector()
    }
    fn set_item_template_selector(&self, value: Option<ModelValue>) {
        self.metadata.set_item_template_selector(value);
    }
}
fn content_control(document: bool) -> Ref<TemplatedControl> {
    if document {
        DocumentContentControl::new().upcast()
    } else {
        ToolContentControl::new().upcast()
    }
}
fn theme() -> Ref<ControlTheme> {
    let theme = ControlTheme::new();
    theme.set_target_type(Some(Control::TYPE));
    theme
}
#[test]
fn original_container_constructors_export_class_inheritance_and_required_template_metadata() {
    run(|| {
        let types: Vec<Ref<TemplatedControl>> = vec![
            DocumentDockControl::new().upcast(),
            DockDockControl::new().upcast(),
            StackDockControl::new().upcast(),
            WrapDockControl::new().upcast(),
            UniformGridDockControl::new().upcast(),
            ProportionalDockControl::new().upcast(),
            ToolDockControl::new().upcast(),
            GridDockControl::new().upcast(),
            SplitViewDockControl::new().upcast(),
            DocumentContentControl::new().upcast(),
            ToolContentControl::new().upcast(),
        ];
        for control in types {
            assert_eq!(control.get_type().namespace(), "FerroDock.Ferro.Controls");
            assert!(control.get_type().is_subclass_of(TemplatedControl::TYPE));
            assert!(control.template().is_none());
        }
        let descriptor = ProportionalDockControl::__MARKUP.unwrap();
        let part = descriptor
            .attributes
            .iter()
            .find(|attribute| attribute.name == "TemplatePart")
            .unwrap();
        assert!(!part.arguments.is_empty());
    });
}
#[test]
fn optional_generated_metadata_contract_and_native_descriptor_preserve_both_original_object_properties(
) {
    run(|| {
        let model = Rc::new(Metadata::default());
        let descriptor = <dyn crate::IDockItemContainerMetadataDescriptor>::MARKUP;
        assert_eq!(descriptor.kind, MarkupTypeKind::Interface);
        assert_eq!(descriptor.namespace(), "FerroDock.Model.Core");
        assert_eq!(descriptor.properties.len(), 2);
        let reference = model_projection::to_item_container_metadata_value(model.clone());
        assert!(model_projection::dockable(Some(&reference)).is_none());
        for name in ["ItemContainerTheme", "ItemTemplateSelector"] {
            let original = Rc::new(String::from(name)) as BoxedValue;
            (descriptor.find_property(name).unwrap().set.unwrap())(&[
                Some(reference.clone()),
                Some(Rc::new(Some(original.clone()))),
            ])
            .unwrap();
            let result =
                (descriptor.find_property(name).unwrap().get.unwrap())(&[Some(reference.clone())])
                    .unwrap()
                    .unwrap();
            let value = result
                .downcast_ref::<Option<BoxedValue>>()
                .unwrap()
                .as_ref()
                .unwrap();
            assert!(Rc::ptr_eq(value, &original));
            (descriptor.find_property(name).unwrap().set.unwrap())(&[
                Some(reference.clone()),
                None,
            ])
            .unwrap();
        }
        assert!(model.item_container_theme().is_none());
        assert!(model.item_template_selector().is_none());
        let generated = Generated::new();
        let value = model_projection::to_item_container_metadata_value(generated.clone());
        assert!(Rc::ptr_eq(
            &model_projection::dockable(Some(&value)).unwrap(),
            &(generated as Rc<dyn IDockable>)
        ));
    });
}
#[test]
fn content_hosts_apply_direct_theme_twice_on_attach_and_register_original_native_controls() {
    run(|| {
        for document in [true, false] {
            let control = content_control(document);
            let generated = Generated::new();
            let factory = Factory::new();
            generated.set_factory(Some(factory.clone()));
            let expected = theme();
            generated.set_item_container_theme(Some(Rc::new(expected.clone())));
            control.set_data_context(Some(model_projection::to_managed_dockable_value(
                generated.clone(),
            )));
            assert_eq!(generated.metadata.reads.get(), 0);
            let scene = Scene::new(control.clone().upcast());
            assert_eq!(control.theme(), Some(expected));
            assert_eq!(generated.metadata.reads.get(), 2);
            let registry = if document {
                factory.document_controls()
            } else {
                factory.tool_controls()
            };
            let value = registry
                .borrow()
                .get(&(generated.clone() as Rc<dyn IDockable>))
                .unwrap()
                .clone();
            let native = from_recycling_value(&value);
            assert_eq!(
                ValueTypes::as_object(&*native).unwrap(),
                control.clone().upcast::<ferroui_base::FerroObject>()
            );
            drop(scene);
            assert!(control.theme().is_none());
            assert!(!registry
                .borrow()
                .contains_key(&(generated as Rc<dyn IDockable>)));
        }
    });
}
#[test]
fn generated_content_theme_resolves_resource_keys_and_clears_missing_non_theme_or_absent_metadata()
{
    run(|| {
        for document in [true, false] {
            let control = content_control(document);
            let metadata = Rc::new(Metadata::default());
            let expected = theme();
            control
                .resources()
                .add_value("GeneratedTheme", expected.clone());
            metadata.set_item_container_theme(Some(Rc::new(String::from("GeneratedTheme"))));
            control.set_data_context(Some(model_projection::to_item_container_metadata_value(
                metadata.clone(),
            )));
            let scene = Scene::new(control.clone().upcast());
            assert_eq!(control.theme(), Some(expected.clone()));
            // Source observes DataContext, not in-place metadata edits.
            metadata.set_item_container_theme(Some(Rc::new(String::from("Missing"))));
            assert_eq!(control.theme(), Some(expected.clone()));
            control.set_data_context(None);
            assert!(control.theme().is_none());
            control.set_data_context(Some(model_projection::to_item_container_metadata_value(
                metadata.clone(),
            )));
            assert!(control.theme().is_none());
            control
                .resources()
                .add_value("WrongType", String::from("text"));
            metadata.set_item_container_theme(Some(Rc::new(String::from("WrongType"))));
            control.set_data_context(None);
            control.set_data_context(Some(model_projection::to_item_container_metadata_value(
                metadata.clone(),
            )));
            assert!(control.theme().is_none());
            metadata.set_item_container_theme(Some(Rc::new(expected.clone())));
            control.set_data_context(None);
            control.set_data_context(Some(model_projection::to_item_container_metadata_value(
                metadata,
            )));
            assert_eq!(control.theme(), Some(expected));
            control.set_data_context(Some(Rc::new(String::from("ordinary"))));
            assert!(control.theme().is_none());
            drop(scene);
        }
    });
}
#[test]
fn generated_theme_supported_native_key_forms_keep_the_original_resource_identity() {
    run(|| {
        for key in [
            Rc::new(ResourceKey::from("key")) as BoxedValue,
            Rc::new("key") as BoxedValue,
            Rc::new(Control::TYPE) as BoxedValue,
            Rc::new(ValueType::of::<Ref<Control>>()) as BoxedValue,
            Rc::new(ThemeVariant::light()) as BoxedValue,
        ] {
            let control = DocumentContentControl::new();
            let metadata = Rc::new(Metadata::default());
            let expected = theme();
            let actual_key = if key.downcast_ref::<ResourceKey>().is_some()
                || key.downcast_ref::<&'static str>().is_some()
            {
                ResourceKey::from("key")
            } else if key.downcast_ref::<ThemeVariant>().is_some() {
                ThemeVariant::light().key().clone()
            } else {
                ResourceKey::from(Control::TYPE)
            };
            control.resources().add_value(actual_key, expected.clone());
            metadata.set_item_container_theme(Some(to_recycling_value(key)));
            control.set_data_context(Some(model_projection::to_item_container_metadata_value(
                metadata,
            )));
            let scene = Scene::new(control.clone().upcast());
            assert_eq!(control.theme(), Some(expected));
            drop(scene);
        }
    });
}
#[test]
fn content_host_context_changes_preserve_source_attach_only_registration_and_current_context_detach_removal(
) {
    run(|| {
        for document in [true, false] {
            let first = Document::new();
            let second = Document::new();
            let first_factory = Factory::new();
            let second_factory = Factory::new();
            first.set_factory(Some(first_factory.clone()));
            second.set_factory(Some(second_factory.clone()));
            let control = content_control(document);
            control.set_data_context(Some(model_projection::to_dockable_value(first.clone())));
            let scene = Scene::new(control.clone().upcast());
            let old = if document {
                first_factory.document_controls()
            } else {
                first_factory.tool_controls()
            };
            let current = if document {
                second_factory.document_controls()
            } else {
                second_factory.tool_controls()
            };
            control.set_data_context(Some(model_projection::to_dockable_value(second.clone())));
            assert!(old
                .borrow()
                .contains_key(&(first.clone() as Rc<dyn IDockable>)));
            assert!(!current
                .borrow()
                .contains_key(&(second.clone() as Rc<dyn IDockable>)));
            current
                .borrow_mut()
                .insert(second.clone(), Rc::new(content_control(document)));
            drop(scene);
            assert!(!current
                .borrow()
                .contains_key(&(second as Rc<dyn IDockable>)));
            assert!(old.borrow().contains_key(&(first as Rc<dyn IDockable>)));
            old.borrow_mut().clear();
        }
    });
}
fn split_template(control: &SplitViewDockControl, split: Ref<SplitView>) {
    control.set_template(Some(FuncControlTemplate::for_type::<SplitViewDockControl>(
        move |_, scope| {
            scope.register("PART_SplitView", split.clone().upcast());
            split.clone().upcast()
        },
    )));
    control.apply_template();
}
#[test]
fn split_view_control_preserves_original_model_factory_defaults_and_source_constructor_scenarios() {
    run(|| {
        let control = SplitViewDockControl::new();
        let factory = Factory::new();
        let model = factory.create_split_view_dock();
        assert_eq!(model.compact_pane_length(), 48.0);
        assert_eq!(model.open_pane_length(), 320.0);
        assert_eq!(model.display_mode(), SplitViewDisplayMode::Overlay);
        assert!(!model.is_pane_open());
        assert_eq!(model.pane_placement(), SplitViewPanePlacement::Left);
        model.set_id(String::from("Test"));
        model.set_title(String::from("Test SplitViewDock"));
        model.set_display_mode(SplitViewDisplayMode::CompactOverlay);
        control.set_data_context(Some(Rc::new(model.clone())));
        assert!(Rc::ptr_eq(
            &model_projection::dockable(control.data_context().as_ref()).unwrap(),
            &(model as Rc<dyn IDockable>)
        ));
    });
}
#[test]
fn split_view_events_follow_original_light_dismiss_policy_for_all_modes_and_flags() {
    run(|| {
        for mode in [
            SplitViewDisplayMode::Overlay,
            SplitViewDisplayMode::CompactOverlay,
            SplitViewDisplayMode::Inline,
            SplitViewDisplayMode::CompactInline,
        ] {
            for allow in [false, true] {
                for open in [false, true] {
                    let model = SplitViewDock::new();
                    model.set_display_mode(mode);
                    model.set_use_light_dismiss_overlay_mode(allow);
                    let control = SplitViewDockControl::new();
                    control
                        .set_data_context(Some(model_projection::to_dockable_value(model.clone())));
                    let split = SplitView::new();
                    split_template(&control, split.clone());
                    split.set_is_pane_open(true);
                    model.set_is_pane_open(open);
                    split.set_is_pane_open(false);
                    let reopen = !allow
                        && open
                        && matches!(
                            mode,
                            SplitViewDisplayMode::Overlay | SplitViewDisplayMode::CompactOverlay
                        );
                    assert_eq!(split.is_pane_open(), reopen);
                    assert_eq!(model.is_pane_open(), reopen);
                    model.set_is_pane_open(false);
                    split.set_is_pane_open(false);
                    split.set_is_pane_open(true);
                    assert!(model.is_pane_open());
                }
            }
        }
    });
}
#[test]
fn split_view_retemplate_unsubscribes_old_parts_and_missing_parts_and_null_models_are_valid() {
    run(|| {
        let control = SplitViewDockControl::new();
        let model = SplitViewDock::new();
        control.set_data_context(Some(model_projection::to_dockable_value(model.clone())));
        let old = SplitView::new();
        split_template(&control, old.clone());
        old.set_is_pane_open(true);
        assert!(model.is_pane_open());
        let current = SplitView::new();
        split_template(&control, current.clone());
        model.set_is_pane_open(false);
        old.set_is_pane_open(false);
        old.set_is_pane_open(true);
        assert!(!model.is_pane_open());
        current.set_is_pane_open(true);
        assert!(model.is_pane_open());
        control.set_template(Some(FuncControlTemplate::for_type::<SplitViewDockControl>(
            |_, _| Panel::new().upcast(),
        )));
        control.apply_template();
        model.set_is_pane_open(false);
        current.set_is_pane_open(false);
        current.set_is_pane_open(true);
        assert!(!model.is_pane_open());
        control.set_data_context(None);
        let unbound = SplitView::new();
        split_template(&control, unbound.clone());
        unbound.set_is_pane_open(true);
        unbound.set_is_pane_open(false);
    });
}
fn install_items_template(items: &ItemsControl) {
    items.set_template(Some(FuncControlTemplate::for_type::<ItemsControl>(
        |_, scope| {
            let presenter = ferroui_controls::presenters::ItemsPresenter::new();
            presenter.set_name(Some(String::from("PART_ItemsPresenter")));
            presenter.bind_binding(
                ItemsControl::items_panel_property().as_property(),
                &ferroui_base::data::TemplateBinding::new(
                    ItemsControl::items_panel_property().as_property(),
                ),
            );
            scope.register("PART_ItemsPresenter", presenter.clone().upcast());
            presenter.upcast()
        },
    )));
}
fn grid_template(control: &GridDockControl, items: Ref<ItemsControl>) {
    control.set_template(Some(FuncControlTemplate::for_type::<GridDockControl>(
        move |_, scope| {
            scope.register("PART_ItemsControl", items.clone().upcast());
            items.clone().upcast()
        },
    )));
}
#[test]
fn grid_loaded_event_applies_original_column_then_row_definitions_and_preserves_nulls() {
    run(|| {
        let model = GridDock::new();
        model.set_column_definitions(Some(String::from("2*,Auto,40")));
        model.set_row_definitions(Some(String::from("Auto,3*")));
        let control = GridDockControl::new();
        let items = ItemsControl::new();
        let grid = Grid::new();
        install_items_template(&items);
        let panel = grid.clone();
        items.set_items_panel(FuncTemplate::new(move || {
            Some(panel.clone().upcast::<Panel>())
        }));
        items.set_data_context(Some(Rc::new(
            model.clone() as Rc<dyn ferrodock_model::controls::IGridDock>
        )));
        grid_template(&control, items.clone());
        let scene = Scene::new(control.clone().upcast());
        assert_eq!(grid.column_definitions().to_string(), "2*,Auto,40");
        assert_eq!(grid.row_definitions().to_string(), "Auto,3*");
        let columns = grid.column_definitions();
        let rows = grid.row_definitions();
        model.set_column_definitions(None);
        model.set_row_definitions(None);
        items.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()));
        assert!(grid.column_definitions() == columns);
        assert!(grid.row_definitions() == rows);
        drop(scene);
    });
}
#[test]
fn private_tool_dock_panel_lookup_preserves_direct_parent_and_presenter_branch_order() {
    run(|| {
        use ferrodock_controls_proportional_stack_panel::ProportionalStackPanel;
        let control = ToolDockControl::new();
        assert!(control.get_panel().is_none());
        let panel = ProportionalStackPanel::new();
        panel.children().add(control.clone());
        assert_eq!(control.get_panel(), Some(panel.clone()));
        panel.children().clear();
        let presenter = ContentPresenter::new();
        presenter.set_content(Some(Control::boxed(control.clone())));
        panel.children().add(presenter.clone());
        let scene = Scene::new(panel.clone().upcast());
        presenter.update_child();
        assert_eq!(control.get_panel(), Some(panel));
        drop(scene);
    });
}

#[test]
fn source_duplicate_content_hosts_last_attach_wins_and_any_current_context_detach_removes_mapping()
{
    run(|| {
        for document in [true, false] {
            let factory = Factory::new();
            let model = Document::new();
            model.set_factory(Some(factory.clone()));
            let first = content_control(document);
            let second = content_control(document);
            for control in [&first, &second] {
                control.set_data_context(Some(model_projection::to_dockable_value(model.clone())));
            }
            let a = Scene::new(first.clone().upcast());
            let b = Scene::new(second.clone().upcast());
            let registry = if document {
                factory.document_controls()
            } else {
                factory.tool_controls()
            };
            let saved = registry
                .borrow()
                .get(&(model.clone() as Rc<dyn IDockable>))
                .unwrap()
                .clone();
            assert_eq!(
                ValueTypes::as_object(&*from_recycling_value(&saved)).unwrap(),
                second.clone().upcast::<ferroui_base::FerroObject>()
            );
            drop(a);
            assert!(!registry
                .borrow()
                .contains_key(&(model as Rc<dyn IDockable>)));
            drop(b);
        }
    });
}
#[test]
fn generated_theme_getter_can_replace_context_before_explicit_apply_and_current_factory_registration(
) {
    run(|| {
        let control = DocumentContentControl::new();
        let first = Generated::new();
        let second = Generated::new();
        let factory = Factory::new();
        second.set_factory(Some(factory.clone()));
        let expected = theme();
        second.set_item_container_theme(Some(Rc::new(expected.clone())));
        let weak = control.downgrade();
        let next = second.clone();
        *first.metadata.read_hook.borrow_mut() = Some(Rc::new(move || {
            weak.upgrade().unwrap().set_data_context(Some(
                model_projection::to_managed_dockable_value(next.clone()),
            ));
        }));
        control.set_data_context(Some(model_projection::to_managed_dockable_value(
            first.clone(),
        )));
        let scene = Scene::new(control.clone().upcast());
        assert_eq!(control.theme(), Some(expected));
        assert!(first.metadata.reads.get() >= 1);
        assert!(second.metadata.reads.get() >= 1);
        assert!(factory
            .document_controls()
            .borrow()
            .contains_key(&(second as Rc<dyn IDockable>)));
        drop(scene);
    });
}
#[test]
fn split_pane_reopen_guard_resets_after_source_event_failure_and_detach_retains_part_handlers() {
    run(|| {
        let model = SplitViewDock::new();
        model.set_is_pane_open(true);
        let control = SplitViewDockControl::new();
        control.set_data_context(Some(model_projection::to_dockable_value(model.clone())));
        let split = SplitView::new();
        split_template(&control, split.clone());
        split.set_is_pane_open(true);
        let token = split.pane_opened(|_, _| panic!("source open handler failed"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            split.set_is_pane_open(false)
        }));
        assert!(result.is_err());
        split.remove_handler(SplitView::pane_opened_event(), token);
        split.set_is_pane_open(false);
        assert!(split.is_pane_open());
        assert!(model.is_pane_open());
        let scene = Scene::new(control.clone().upcast());
        scene.0.set_content(None);
        model.set_is_pane_open(false);
        split.raise_event(&RoutedEventArgs::with_event(SplitView::pane_opened_event()));
        assert!(model.is_pane_open());
        drop(scene);
    });
}
#[test]
fn grid_definition_event_failure_preserves_source_column_then_row_partial_assignment_order() {
    run(|| {
        let model = GridDock::new();
        model.set_column_definitions(Some(String::from("*,20")));
        model.set_row_definitions(Some(String::from("30,*")));
        let control = GridDockControl::new();
        let items = ItemsControl::new();
        install_items_template(&items);
        let grid = Grid::new();
        let panel = grid.clone();
        items.set_items_panel(FuncTemplate::new(move || {
            Some(panel.clone().upcast::<Panel>())
        }));
        items.set_data_context(Some(model_projection::to_dockable_value(model.clone())));
        grid_template(&control, items.clone());
        let scene = Scene::new(control.clone().upcast());
        let columns = grid.column_definitions();
        let rows = grid.row_definitions();
        model.set_column_definitions(Some(String::from("bad")));
        model.set_row_definitions(Some(String::from("50")));
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            items.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()))
        }));
        assert!(failed.is_err());
        assert!(grid.column_definitions() == columns);
        assert!(grid.row_definitions() == rows);
        model.set_column_definitions(Some(String::from("40")));
        model.set_row_definitions(Some(String::from("bad")));
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            items.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()))
        }));
        assert!(failed.is_err());
        assert_eq!(grid.column_definitions().to_string(), "40");
        assert!(grid.row_definitions() == rows);
        model.set_column_definitions(Some(String::new()));
        model.set_row_definitions(Some(String::new()));
        items.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()));
        assert_eq!(grid.column_definitions().count(), 0);
        assert_eq!(grid.row_definitions().count(), 0);
        drop(scene);
    });
}
#[test]
fn generated_theme_numeric_and_explicit_custom_native_resource_keys_use_shared_resource_lookup() {
    run(|| {
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct Key(i32);
        for (key, metadata) in [
            (ResourceKey::object(42i32), Rc::new(42i32) as BoxedValue),
            (
                ResourceKey::object(Key(1)),
                Rc::new(ResourceKey::object(Key(1))) as BoxedValue,
            ),
        ] {
            let control = ToolContentControl::new();
            let model = Rc::new(Metadata::default());
            let expected = theme();
            control.resources().add_value(key, expected.clone());
            model.set_item_container_theme(Some(to_recycling_value(metadata)));
            control.set_data_context(Some(model_projection::to_item_container_metadata_value(
                model,
            )));
            let scene = Scene::new(control.clone().upcast());
            assert_eq!(control.theme(), Some(expected));
            drop(scene);
        }
    });
}
