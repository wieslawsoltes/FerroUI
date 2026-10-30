//! An application compilation boundary for the original MDI layout API.
use ferrodock_ferro::{
    controls::MdiLayoutPanel,
    mdi::{ClassicMdiLayoutManager, IMdiLayoutManager, MdiResizeDirection},
    model_projection,
};
use ferrodock_model::{
    controls::IMdiDocument,
    core::{DockRect, IDockable, MdiWindowState},
};
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Rect, Ref};
use ferroui_controls::{AppBuilder, Application, Border, Control, Panel, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-mdi-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn compiled_consumer_selects_original_managers_flags_and_arranges_original_reactive_documents() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/MdiLayoutConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let root = value.downcast_ref::<Ref<Panel>>().unwrap();
                let panels: Vec<_> = (0..4)
                    .map(|index| root.children().get(index).cast::<MdiLayoutPanel>().unwrap())
                    .collect();
                let shared = ClassicMdiLayoutManager::instance() as Rc<dyn IMdiLayoutManager>;
                assert!(Rc::ptr_eq(&panels[0].layout_manager().unwrap(), &shared));
                assert!(!Rc::ptr_eq(&panels[1].layout_manager().unwrap(), &shared));
                assert!(Rc::ptr_eq(&panels[2].layout_manager().unwrap(), &shared));
                assert!(panels[3].layout_manager().is_none());
                let direction = root.find_resource(&"CornerDirection".into()).unwrap();
                assert_eq!(
                    *direction.downcast_ref::<MdiResizeDirection>().unwrap(),
                    MdiResizeDirection::LEFT | MdiResizeDirection::TOP
                );
                let documents: Vec<_> = panels
                    .iter()
                    .map(|panel| {
                        let normal = Document::new();
                        let minimized = Document::new();
                        minimized.set_mdi_state(MdiWindowState::Minimized);
                        for (index, document) in
                            [normal.clone(), minimized.clone()].iter().enumerate()
                        {
                            let child = Border::new();
                            child.set_use_layout_rounding(false);
                            let context = if index == 0 {
                                model_projection::to_dockable_value(
                                    document.clone() as Rc<dyn IDockable>
                                )
                            } else {
                                model_projection::to_mdi_document_value(
                                    document.clone() as Rc<dyn IMdiDocument>
                                )
                            };
                            child.set_data_context(Some(context));
                            panel.children().add(child);
                        }
                        (normal, minimized)
                    })
                    .collect();
                let window = Window::new();
                window.set_width(600.0);
                window.set_height(432.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                for (panel, (normal, minimized)) in panels.iter().zip(documents.iter()) {
                    assert_eq!(normal.mdi_bounds(), DockRect::new(0.0, 0.0, 360.0, 240.0));
                    assert_eq!(
                        panel.children().get(0).bounds(),
                        Rect::new(0.0, 0.0, 360.0, 240.0)
                    );
                    assert_eq!(
                        panel.children().get(1).bounds(),
                        Rect::new(0.0, 400.0, 220.0, 28.0)
                    );
                    assert_eq!(minimized.mdi_bounds(), DockRect::default());
                    normal.set_mdi_state(MdiWindowState::Maximized);
                    panel.invalidate_arrange();
                }
                window.update_layout();
                for panel in &panels {
                    assert_eq!(
                        panel.children().get(0).bounds(),
                        Rect::new(0.0, 0.0, 600.0, 400.0)
                    );
                }
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .expect("external MDI markup case completed");
}
