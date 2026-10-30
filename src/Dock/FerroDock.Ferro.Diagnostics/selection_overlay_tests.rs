use crate::{
    controls::SelectionOverlayAdorner, helpers::selection_overlay_helper::SelectionOverlayHelper,
};
use ferrodock_ferro::controls::DockableControl;
use ferrodock_model::core::{
    DockableControlMap, DockableObjectMap, IDockable, IDockableControl, IFactory,
};
use ferrodock_model_reactiveui::{controls::Tool, Factory};
use ferroui_base::{
    media::{Colors, DrawingGroup, GeometryDrawing},
    threading::{CancellationToken, Dispatcher},
    Rect, Ref,
};
use ferroui_controls::{
    primitives::{AdornerLayer, VisualLayerManager},
    AppBuilder, Application, Border, Canvas, Control, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;

fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-selection-overlay",
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
struct Scene {
    window: Ref<Window>,
    root: Ref<Canvas>,
    target: Ref<Border>,
}
impl Scene {
    fn new() -> Self {
        let root = Canvas::new();
        let target = Border::new();
        target.set_width(80.0);
        target.set_height(35.0);
        Canvas::set_left(&target, 17.0);
        Canvas::set_top(&target, 23.0);
        root.children().add(target.clone().upcast::<Control>());
        let layers = VisualLayerManager::new();
        layers.set_child(Some(root.clone().upcast::<Control>()));
        let window = Window::new();
        window.set_width(400.0);
        window.set_height(300.0);
        window.set_content(Some(Rc::new(layers)));
        window.show();
        Self::drain();
        Self {
            window,
            root,
            target,
        }
    }
    fn drain() {
        for _ in 0..3 {
            Dispatcher::ui_thread().run_jobs(None);
        }
    }
    fn layer(&self) -> Ref<AdornerLayer> {
        AdornerLayer::get_adorner_layer(&self.root).expect("actual native adorner layer")
    }
    fn adorner(&self) -> Ref<SelectionOverlayAdorner> {
        self.layer()
            .children()
            .snapshot()
            .iter()
            .find_map(|child| child.cast())
            .expect("source selection adorner")
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
fn draw(adorner: &SelectionOverlayAdorner) -> Ref<DrawingGroup> {
    let group = DrawingGroup::new();
    let mut drawing = group.open();
    adorner.render(&mut drawing);
    drawing.dispose();
    group
}

#[test]
fn original_adorner_draws_only_the_optional_yellow_two_pixel_outline() {
    run(|| {
        let adorner = SelectionOverlayAdorner::new();
        assert!(adorner.is_hit_test_visible());
        assert_eq!(adorner.highlight_rect(), None);
        assert_eq!(draw(&adorner).children().len(), 0);
        adorner.set_highlight_rect(Some(Rect::new(17.0, 23.0, 80.0, 35.0)));
        let drawing = draw(&adorner);
        assert_eq!(drawing.children().len(), 1);
        let outline = drawing.children().get(0).cast::<GeometryDrawing>().unwrap();
        assert!(outline.brush().is_none());
        let pen = outline.pen().unwrap();
        assert_eq!(pen.thickness(), 2.0);
        assert_eq!(
            pen.brush().unwrap().as_solid_color_brush().unwrap().color(),
            Colors::YELLOW
        );
        assert_eq!(
            outline.geometry().unwrap().bounds(),
            Rect::new(17.0, 23.0, 80.0, 35.0)
        );
        adorner.set_highlight_rect(None);
        assert_eq!(draw(&adorner).children().len(), 0);
    });
}
#[test]
fn source_attach_same_root_and_remove_preserve_identity_parent_and_adorned_element() {
    run(|| {
        let scene = Scene::new();
        let layer = scene.layer();
        let baseline = layer.children().count();
        let helper = SelectionOverlayHelper::new();
        helper.attach_overlay(scene.root.clone().upcast());
        let adorner = scene.adorner();
        assert_eq!(layer.children().count(), baseline + 1);
        assert!(adorner.parent().unwrap().ptr_eq(&scene.root));
        assert!(AdornerLayer::get_adorned_element(&adorner)
            .unwrap()
            .ptr_eq(&scene.root));
        helper.attach_overlay(scene.root.clone().upcast());
        assert!(scene.adorner().ptr_eq(&adorner));
        assert_eq!(layer.children().count(), baseline + 1);
        helper.remove_overlay();
        assert_eq!(layer.children().count(), baseline);
        assert!(adorner.parent().is_none());
        // The original clears the logical parent, but retains its attached adorned value.
        assert!(AdornerLayer::get_adorned_element(&adorner)
            .unwrap()
            .ptr_eq(&scene.root));
        helper.remove_overlay();
    });
}
#[test]
fn original_highlight_uses_root_coordinates_and_live_target_bounds_then_clears() {
    run(|| {
        let scene = Scene::new();
        let helper = SelectionOverlayHelper::new();
        helper.attach_overlay(scene.root.clone().upcast());
        helper.highlight(Some(scene.target.clone().upcast()));
        let adorner = scene.adorner();
        assert_eq!(
            adorner.highlight_rect(),
            Some(Rect::new(17.0, 23.0, 80.0, 35.0))
        );
        Canvas::set_left(&scene.target, 42.0);
        scene.target.set_width(105.0);
        Scene::drain();
        assert_eq!(
            adorner.highlight_rect(),
            Some(Rect::new(42.0, 23.0, 105.0, 35.0))
        );
        helper.highlight(None);
        assert_eq!(adorner.highlight_rect(), None);
        helper.highlight(Some(Border::new().upcast()));
        assert_eq!(adorner.highlight_rect(), None);
        helper.remove_overlay();
    });
}
#[test]
fn missing_layer_switch_and_remove_follow_source_target_reset_and_subscription_order() {
    run(|| {
        let helper = SelectionOverlayHelper::new();
        let detached = Canvas::new();
        helper.attach_overlay(detached.upcast());
        helper.remove_overlay();
        let first = Scene::new();
        helper.attach_overlay(first.root.clone().upcast());
        helper.highlight(Some(first.target.clone().upcast()));
        let old = first.adorner();
        let second = Scene::new();
        helper.attach_overlay(second.root.clone().upcast());
        assert!(old.parent().is_none());
        assert_eq!(first.layer().children().count(), 0);
        assert_eq!(second.adorner().highlight_rect(), None);
        helper.highlight(Some(second.target.clone().upcast()));
        let current = second.adorner();
        helper.remove_overlay();
        Canvas::set_left(&second.target, 73.0);
        Scene::drain();
        assert_eq!(
            current.highlight_rect(),
            Some(Rect::new(17.0, 23.0, 80.0, 35.0))
        );
        assert!(current.parent().is_none());
        assert_eq!(second.layer().children().count(), 0);
        helper.attach_overlay(Border::new().upcast());
    });
}
#[test]
fn original_pre_attach_highlight_survives_the_early_remove_and_updates_on_layout() {
    run(|| {
        let scene = Scene::new();
        let helper = SelectionOverlayHelper::new();
        helper.highlight(Some(scene.target.clone().upcast()));
        helper.remove_overlay();
        helper.attach_overlay(scene.root.clone().upcast());
        assert_eq!(scene.adorner().highlight_rect(), None);
        Scene::drain();
        assert_eq!(
            scene.adorner().highlight_rect(),
            Some(Rect::new(17.0, 23.0, 80.0, 35.0))
        );
        helper.remove_overlay();
    });
}

#[test]
fn source_layout_subscription_retains_its_owner_until_explicit_removal() {
    run(|| {
        let scene = Scene::new();
        let helper = SelectionOverlayHelper::new();
        let weak = Rc::downgrade(&helper);
        helper.attach_overlay(scene.root.clone().upcast());
        drop(helper);
        let retained = weak.upgrade().expect("source strong event subscription");
        retained.remove_overlay();
        drop(retained);
        assert!(weak.upgrade().is_none());
        assert_eq!(scene.layer().children().count(), 0);
    });
}

#[test]
fn source_removal_after_root_detach_does_not_guess_the_previous_layer() {
    run(|| {
        let scene = Scene::new();
        let helper = SelectionOverlayHelper::new();
        helper.attach_overlay(scene.root.clone().upcast());
        let layer = scene.layer();
        let adorner = scene.adorner();
        let manager = scene
            .root
            .visual_parent()
            .unwrap()
            .cast::<VisualLayerManager>()
            .unwrap();
        manager.set_child(None::<Ref<Control>>);
        assert!(AdornerLayer::get_adorner_layer(&scene.root).is_none());
        helper.remove_overlay();
        assert!(adorner.parent().is_none());
        assert!(layer
            .children()
            .snapshot()
            .iter()
            .any(|child| child.ptr_eq(&adorner)));
        assert!(AdornerLayer::get_adorned_element(&adorner)
            .unwrap()
            .ptr_eq(&scene.root));
        // The detached source adorner is left in that layer. Clean up only in the test.
        layer.children().remove(adorner.upcast::<Control>());
    });
}

#[test]
fn original_collection_callback_nested_removal_fails_after_source_fields_are_cleared() {
    run(|| {
        let scene = Scene::new();
        let helper = SelectionOverlayHelper::new();
        helper.attach_overlay(scene.root.clone().upcast());
        let owner = helper.clone();
        let subscription = scene
            .layer()
            .children()
            .add_collection_changed(Rc::new(move |event| {
                if event.action == ferroui_base::collections::NotifyCollectionChangedAction::Remove
                {
                    owner.remove_overlay();
                }
            }));
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| helper.remove_overlay()));
        scene
            .layer()
            .children()
            .remove_collection_changed(subscription);
        assert!(result.is_err());
        assert_eq!(scene.layer().children().count(), 0);
        helper.remove_overlay();
    });
}

#[test]
fn native_metadata_retains_original_plain_highlight_property_and_class_name() {
    run(|| {
        use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
        assert!(std::ptr::eq(
            MarkupAssembly::find("FerroDock.Ferro.Diagnostics").unwrap(),
            &crate::ASSEMBLY,
        ));
        assert_eq!(
            crate::ASSEMBLY.xmlns_definitions,
            &[
                XmlnsDefinition {
                    xml_namespace: FERRO_XML_NAMESPACE,
                    namespace: "FerroDock.Ferro.Diagnostics"
                },
                XmlnsDefinition {
                    xml_namespace: FERRO_XML_NAMESPACE,
                    namespace: "FerroDock.Ferro.Diagnostics.Controls"
                },
            ]
        );
        let class = SelectionOverlayAdorner::TYPE;
        assert_eq!(class.name(), "SelectionOverlayAdorner");
        let metadata = class.markup().unwrap();
        let property = metadata.find_property("HighlightRect").unwrap();
        let adorner = SelectionOverlayAdorner::new();
        let boxed: ferroui_base::BoxedValue = Rc::new(adorner.clone());
        let rect = Some(Rect::new(7.0, 8.0, 9.0, 10.0));
        (property.set.unwrap())(&[Some(boxed.clone()), Some(Rc::new(rect))]).unwrap();
        assert_eq!(adorner.highlight_rect(), rect);
        let read = (property.get.unwrap())(&[Some(boxed)]).unwrap().unwrap();
        assert_eq!(read.downcast_ref::<Rect>(), rect.as_ref());
    });
}

struct CustomControl(Ref<Border>);
impl IDockableControl for CustomControl {
    fn boxed_reference(self: Rc<Self>) -> ferrodock_model::BoxedValue {
        Rc::new(self.0.clone())
    }
}
#[test]
fn native_class_projection_accepts_custom_controls_and_preserves_actual_reference_identity() {
    run(|| {
        let factory = Factory::new();
        let model: Rc<dyn IDockable> = Tool::new();
        let root = Border::new();
        let custom = Border::new();
        let (controls, roots) = maps(&*factory, 2);
        controls
            .borrow_mut()
            .insert(model.clone(), Rc::new(CustomControl(custom.clone())));
        roots
            .borrow_mut()
            .insert(model.clone(), Rc::new(root.clone().upcast::<Control>()));
        let (actual_root, actual_control) =
            SelectionOverlayHelper::try_get_control(&*factory, &model).unwrap();
        assert!(actual_root.ptr_eq(&root));
        assert!(actual_control.unwrap().ptr_eq(&custom));
        // The engine's marker can still be implemented by a framework-free object.
        let marker = Rc::new(NonControl);
        let value = marker.clone().boxed_reference();
        assert!(Rc::ptr_eq(
            value.downcast_ref::<Rc<NonControl>>().unwrap(),
            &marker
        ));
    });
}

struct NonControl;
impl IDockableControl for NonControl {}
fn maps(factory: &dyn IFactory, mode: usize) -> (DockableControlMap, DockableObjectMap) {
    match mode {
        0 => (
            factory.visible_dockable_controls(),
            factory.visible_root_controls(),
        ),
        1 => (
            factory.pinned_dockable_controls(),
            factory.pinned_root_controls(),
        ),
        2 => (factory.tab_dockable_controls(), factory.tab_root_controls()),
        _ => unreachable!(),
    }
}
#[test]
fn original_lookup_keeps_visible_pinned_tab_precedence_and_nullable_control_success() {
    run(|| {
        let factory = Factory::new();
        let model: Rc<dyn IDockable> = Tool::new();
        let root = Border::new();
        let expected = DockableControl::new();
        let (controls, roots) = maps(&*factory, 1);
        controls.borrow_mut().insert(
            model.clone(),
            DockableControl::as_dockable_control(expected.clone()),
        );
        roots
            .borrow_mut()
            .insert(model.clone(), Rc::new(root.clone()));
        let (actual_root, actual_control) =
            SelectionOverlayHelper::try_get_control(&*factory, &model).unwrap();
        assert!(actual_root.ptr_eq(&root));
        assert!(actual_control.unwrap().ptr_eq(&expected));
        let (controls, roots) = maps(&*factory, 0);
        controls
            .borrow_mut()
            .insert(model.clone(), Rc::new(NonControl));
        roots.borrow_mut().insert(
            model.clone(),
            Rc::new(root.clone().upcast::<ferroui_base::FerroObject>()),
        );
        let (actual_root, actual_control) =
            SelectionOverlayHelper::try_get_control(&*factory, &model).unwrap();
        assert!(actual_root.ptr_eq(&root));
        assert!(actual_control.is_none());
        roots
            .borrow_mut()
            .insert(model.clone(), Rc::new("wrong root".to_string()));
        let (_, actual_control) =
            SelectionOverlayHelper::try_get_control(&*factory, &model).unwrap();
        assert!(actual_control.unwrap().ptr_eq(&expected));
        controls.borrow_mut().clear();
        let unknown: Rc<dyn IDockable> = Tool::new();
        unknown.set_id(model.id());
        assert!(SelectionOverlayHelper::try_get_control(&*factory, &unknown).is_none());
    });
}

#[test]
fn actual_original_dotnet_lookup_corpus_matches_all_216_factory_map_combinations() {
    run(|| {
        let corpus = include_str!("../../../tests/Dock/data/selection-overlay-lookup.tsv");
        let mut rows = 0;
        for line in corpus
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
        {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 5);
            let factory = Factory::new();
            let model: Rc<dyn IDockable> = Tool::new();
            for (mode, state) in fields[..3].iter().enumerate() {
                let (controls, roots) = maps(&*factory, mode);
                let state: u8 = state.parse().unwrap();
                let control = DockableControl::new();
                control.set_name(Some(mode.to_string()));
                let root = Border::new();
                root.set_name(Some(mode.to_string()));
                if matches!(state, 1 | 3 | 4) {
                    controls
                        .borrow_mut()
                        .insert(model.clone(), DockableControl::as_dockable_control(control));
                } else if state == 5 {
                    controls
                        .borrow_mut()
                        .insert(model.clone(), Rc::new(NonControl));
                }
                if state == 3 {
                    roots
                        .borrow_mut()
                        .insert(model.clone(), Rc::new("not a control".to_string()));
                } else if matches!(state, 2 | 4 | 5) {
                    roots.borrow_mut().insert(model.clone(), Rc::new(root));
                }
            }
            let result = SelectionOverlayHelper::try_get_control(&*factory, &model);
            let names = result
                .map(|(root, control)| {
                    (
                        root.name().unwrap(),
                        control
                            .and_then(|control| control.name())
                            .unwrap_or_else(|| "-".into()),
                    )
                })
                .unwrap_or_else(|| ("-".into(), "-".into()));
            assert_eq!(
                (&*names.0, &*names.1),
                (fields[3], fields[4]),
                "source row {line}"
            );
            rows += 1;
        }
        assert_eq!(rows, 216);
    });
}
