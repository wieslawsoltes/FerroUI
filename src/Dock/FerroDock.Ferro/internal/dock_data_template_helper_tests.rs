//! Original default-template order, membership, construction and live binding behavior.
use super::*;
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_reactiveui::{
    controls::*,
    core::{DockableBase, DockableBaseImpl},
    impl_dockable,
};
use ferroui_base::{data::BindingOperations, threading::CancellationToken};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_reactiveui::reactive_object;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::RefCell;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-default-templates",
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
struct ContentModel {
    base: DockableBase,
    content: RefCell<Option<ferrodock_model::BoxedValue>>,
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
        Rc::new_cyclic(|weak| Self {
            base: DockableBase::construct(weak.clone()),
            content: RefCell::new(None),
        })
    }
}
impl IDocumentContent for ContentModel {
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.content.borrow().clone()
    }
    fn set_content(&self, value: Option<ferrodock_model::BoxedValue>) {
        self.content.replace(value);
    }
}
impl IToolContent for ContentModel {
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.content.borrow().clone()
    }
    fn set_content(&self, value: Option<ferrodock_model::BoxedValue>) {
        self.content.replace(value);
    }
}
#[test]
fn original_templates_have_all_fourteen_types_in_order_and_build_fresh_nonrecycled_controls() {
    run(|| {
        let expected = [
            DocumentContentControl::TYPE,
            ToolContentControl::TYPE,
            ProportionalStackPanelSplitter::TYPE,
            GridSplitter::TYPE,
            DocumentDockControl::TYPE,
            ToolDockControl::TYPE,
            SplitViewDockControl::TYPE,
            ProportionalDockControl::TYPE,
            StackDockControl::TYPE,
            GridDockControl::TYPE,
            WrapDockControl::TYPE,
            UniformGridDockControl::TYPE,
            DockDockControl::TYPE,
            RootDockControl::TYPE,
        ];
        let sequence = DockDataTemplateHelper::create_default_data_templates();
        let templates: Vec<_> = (&sequence).into_iter().collect();
        let second: Vec<_> = (&sequence).into_iter().collect();
        assert_eq!(templates.len(), expected.len());
        for ((template, other), kind) in templates.iter().zip(&second).zip(expected) {
            assert!(!Rc::ptr_eq(template, other));
            // Original FuncDataTemplate<T> reference types accept null.
            assert!(template.match_(None));
            let control = template.build(&None).unwrap();
            assert_eq!(control.get_type(), kind);
            assert!(control.data_context().is_none());
            let replacement = template
                .as_recycling_data_template()
                .unwrap()
                .build_with_existing(None, Some(control.clone()))
                .unwrap();
            assert_ne!(replacement, control);
            assert_eq!(replacement.get_type(), kind);
        }
    });
}
#[test]
fn templates_match_original_interfaces_and_do_not_treat_every_document_or_tool_as_content() {
    run(|| {
        let content = ContentModel::new();
        let models: Vec<Rc<dyn IDockable>> = vec![
            content.clone(),
            content,
            ProportionalDockSplitter::new(),
            GridDockSplitter::new(),
            DocumentDock::new(),
            ToolDock::new(),
            SplitViewDock::new(),
            ProportionalDock::new(),
            StackDock::new(),
            GridDock::new(),
            WrapDock::new(),
            UniformGridDock::new(),
            DockDock::new(),
            RootDock::new(),
        ];
        let templates: Vec<_> = DockDataTemplateHelper::create_default_data_templates()
            .into_iter()
            .collect();
        for (i, model) in models.iter().enumerate() {
            let value = model_projection::to_dockable_value(model.clone());
            for (j, template) in templates.iter().enumerate() {
                assert_eq!(
                    template.match_(Some(&value)),
                    i == j || (i < 2 && j < 2),
                    "model {i}, template {j}"
                );
            }
            assert!(templates[i]
                .build(&Some(value))
                .unwrap()
                .data_context()
                .is_none());
        }
        for value in [
            model_projection::to_dockable_value(Document::new()),
            model_projection::to_dockable_value(Tool::new()),
            Rc::new(42_i32) as BoxedValue,
        ] {
            assert!(templates
                .iter()
                .all(|template| !template.match_(Some(&value))));
        }
        let document: Rc<dyn IDocumentContent> = ContentModel::new();
        assert!(templates[0].match_(Some(&(Rc::new(document) as BoxedValue))));
        let tool: Rc<dyn IToolContent> = ContentModel::new();
        assert!(templates[1].match_(Some(&(Rc::new(tool) as BoxedValue))));
        // A raw model is supported through the shared framework registration.
        ferroui_base::data::core::ValueTypes::register_cast::<
            Rc<ProportionalDockSplitter>,
            Rc<dyn IDockable>,
        >(|m| m.clone());
        let raw: BoxedValue = Rc::new(ProportionalDockSplitter::new());
        assert!(templates[2].match_(Some(&raw)));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || templates[0].build(&Some(raw))
        ))
        .is_err());
    });
}
#[test]
fn splitter_compiled_bindings_follow_original_changes_and_replace_and_unsubscribe_contexts() {
    run(|| {
        let template = DockDataTemplateHelper::create_default_data_templates()
            .into_iter()
            .nth(2)
            .unwrap();
        let first = ProportionalDockSplitter::new();
        first.set_can_resize(false);
        first.set_resize_preview(true);
        let second = ProportionalDockSplitter::new();
        let first_handlers = first.property_changed().handler_count();
        let second_handlers = second.property_changed().handler_count();
        let control = template
            .build(&Some(model_projection::to_dockable_value(first.clone())))
            .unwrap()
            .cast::<ProportionalStackPanelSplitter>()
            .unwrap();
        // Build ignores the model; the presenter supplies DataContext, as in source.
        assert!(control.data_context().is_none());
        control.set_data_context(Some(model_projection::to_dockable_value(first.clone())));
        assert!(!control.is_resizing_enabled());
        assert!(control.preview_resize());
        assert_eq!(first.property_changed().handler_count(), first_handlers + 2);
        first.set_can_resize(true);
        first.set_resize_preview(false);
        assert!(control.is_resizing_enabled());
        assert!(!control.preview_resize());
        control.set_is_resizing_enabled(false);
        assert!(first.can_resize()); // Original default binding is one way.
        let reference: Rc<dyn IProportionalDockSplitter> = second.clone();
        control.set_data_context(Some(Rc::new(reference)));
        assert!(control.is_resizing_enabled());
        assert_eq!(first.property_changed().handler_count(), first_handlers);
        assert_eq!(
            second.property_changed().handler_count(),
            second_handlers + 2
        );
        first.set_resize_preview(true);
        assert!(!control.preview_resize());
        second.set_resize_preview(true);
        assert!(control.preview_resize());
        let expression = BindingOperations::get_binding_expression_base(
            &control,
            ProportionalStackPanelSplitter::preview_resize_property().as_property(),
        );
        assert!(expression.is_some());
        control.set_data_context(None);
        assert_eq!(second.property_changed().handler_count(), second_handlers);
    });
}
#[test]
fn original_grid_enum_binding_keeps_shared_default_conversion_failure_and_live_subscription() {
    run(|| {
        let template = DockDataTemplateHelper::create_default_data_templates()
            .into_iter()
            .nth(3)
            .unwrap();
        let model = GridDockSplitter::new();
        let handlers = model.property_changed().handler_count();
        let control = template
            .build(&None)
            .unwrap()
            .cast::<GridSplitter>()
            .unwrap();
        let reference: Rc<dyn IGridDockSplitter> = model.clone();
        control.set_data_context(Some(Rc::new(reference)));
        assert_eq!(model.property_changed().handler_count(), handlers + 1);
        // The source's distinct model/native enums have no cross-enum converter.
        // Preserve its error/default behavior instead of adding a name/number map.
        assert_eq!(
            control.resize_direction(),
            ferroui_controls::GridResizeDirection::Auto
        );
        model.set_resize_direction(GridResizeDirection::Rows);
        assert_eq!(
            control.resize_direction(),
            ferroui_controls::GridResizeDirection::Auto
        );
        control.set_data_context(None);
        assert_eq!(model.property_changed().handler_count(), handlers);
    });
}

#[test]
fn native_presenter_selects_defaults_and_supplies_original_model_to_live_splitter_bindings() {
    run(|| {
        let presenter = ferroui_controls::presenters::ContentPresenter::new();
        for template in DockDataTemplateHelper::create_default_data_templates() {
            presenter.data_templates().add(template);
        }
        let window = ferroui_controls::Window::new();
        window.set_content(Some(Rc::new(presenter.clone())));
        window.show();
        let model = ProportionalDockSplitter::new();
        model.set_can_resize(false);
        presenter.set_content(Some(model_projection::to_dockable_value(model.clone())));
        presenter.update_child();
        let child = presenter
            .child()
            .unwrap()
            .cast::<ProportionalStackPanelSplitter>()
            .unwrap();
        assert!(!child.is_resizing_enabled());
        model.set_can_resize(true);
        model.set_resize_preview(true);
        assert!(child.is_resizing_enabled());
        assert!(child.preview_resize());
        let received = model_projection::dockable(child.data_context().as_ref()).unwrap();
        let original: Rc<dyn IDockable> = model;
        assert!(Rc::ptr_eq(&received, &original));
        let content = ContentModel::new();
        presenter.set_content(Some(model_projection::to_dockable_value(content)));
        presenter.update_child();
        // Both content contracts match; the original document template wins by order.
        assert!(presenter
            .child()
            .unwrap()
            .cast::<DocumentContentControl>()
            .is_some());
        presenter.set_content(Some(model_projection::to_dockable_value(GridDock::new())));
        presenter.update_child();
        assert!(presenter
            .child()
            .unwrap()
            .cast::<GridDockControl>()
            .is_some());
        window.close();
    });
}
#[test]
fn every_canonical_container_interface_is_accepted_without_replacing_model_identity() {
    run(|| {
        let templates: Vec<_> = DockDataTemplateHelper::create_default_data_templates()
            .into_iter()
            .collect();
        macro_rules! check {
            ($index:expr, $interface:ty, $model:expr) => {{
                let model: Rc<$interface> = $model;
                let value: BoxedValue = Rc::new(model.clone());
                assert!(templates[$index].match_(Some(&value)));
                let resolved = model_projection::dockable(Some(&value)).unwrap();
                let original: Rc<dyn IDockable> = model;
                assert!(Rc::ptr_eq(&resolved, &original));
                let event = model_projection::notifier(&*value)
                    .unwrap()
                    .property_changed();
                assert!(std::ptr::eq(
                    event,
                    original
                        .as_notify_property_changed()
                        .unwrap()
                        .property_changed()
                ));
            }};
        }
        check!(0, dyn IDocumentContent, ContentModel::new());
        check!(1, dyn IToolContent, ContentModel::new());
        check!(
            2,
            dyn IProportionalDockSplitter,
            ProportionalDockSplitter::new()
        );
        check!(3, dyn IGridDockSplitter, GridDockSplitter::new());
        check!(4, dyn IDocumentDock, DocumentDock::new());
        check!(5, dyn IToolDock, ToolDock::new());
        check!(6, dyn ISplitViewDock, SplitViewDock::new());
        check!(7, dyn IProportionalDock, ProportionalDock::new());
        check!(8, dyn IStackDock, StackDock::new());
        check!(9, dyn IGridDock, GridDock::new());
        check!(10, dyn IWrapDock, WrapDock::new());
        check!(11, dyn IUniformGridDock, UniformGridDock::new());
        check!(12, dyn IDockDock, DockDock::new());
        check!(13, dyn IRootDock, RootDock::new());
        check!(2, dyn ISplitter, ProportionalDockSplitter::new());
        check!(9, dyn IDock, GridDock::new());
    });
}
