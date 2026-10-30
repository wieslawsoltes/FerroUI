//! A separate compilation boundary for native FerroDock library consumers.
extern crate self as ferrodock_markup_tests;
#[cfg(test)]
mod native_json_model_tests;
#[cfg(test)]
mod native_json_property_tests;
#[cfg(test)]
mod numeric_enum_sdk_tests;
// The shared 0.3.0 emitter produces duplicate nullable setter branches and
// Clone::clone for direct-property literals. Keep these two lint allowances
// confined to its generated code; handwritten port/test code remains checked.
#[allow(clippy::if_same_then_else, clippy::clone_on_copy)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};

/// Plain, named source-model fixture for the shared XAML compiler boundary.
pub mod named_document;

/// External-constructor metadata fixture for the shared compiler boundary.
pub mod carrier_model;

#[cfg(test)]
mod tests {
    use ferrodock_controls_proportional_stack_panel::{
        ProportionalStackPanel, ProportionalStackPanelSplitter,
    };
    use ferroui_base::data::core::ValueTypes;
    use ferroui_base::layout::Orientation;
    use ferroui_base::threading::CancellationToken;
    use ferroui_base::{Rect, Ref};
    use ferroui_controls::{AppBuilder, Application, Border, Control, Window};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
        FerroTestApplicationAttribute, FerroTestAssembly, HeadlessUnitTestSession,
    };

    #[test]
    fn external_consumer_constructs_all_original_diagnostic_property_views() {
        HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
            .dispatch(
                || {
                    ferrodock_ferro_diagnostics::register_types();
                    crate::compiled_markup::register();
                    let value = crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferrodock_markup_tests/InspectorConsumer.xaml",
                    )
                    .unwrap()
                    .unwrap();
                    let panel = value
                        .downcast_ref::<Ref<ferroui_controls::StackPanel>>()
                        .unwrap();
                    assert_eq!(panel.children().count(), 19);
                    for child in panel.children().snapshot().iter() {
                        let view = child
                            .cast::<ferroui_controls::UserControl>()
                            .expect("original inspector");
                        let content = view.content().unwrap();
                        let content = ValueTypes::as_object(&*content).unwrap();
                        if let Some(inspector) =
                            view.cast::<ferrodock_ferro_diagnostics::controls::RootDockDebug>()
                        {
                            assert_eq!(inspector.filter().as_deref(), Some("added"));
                            assert!(content.is::<ferroui_controls::DockPanel>());
                        } else {
                            assert!(content.is::<ferroui_controls::Grid>());
                        }
                    }
                },
                CancellationToken::none(),
            )
            .wait()
            .unwrap();
    }
    fn build() -> AppBuilder {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    }
    pub(super) const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
        name: "ferrodock-markup-headless",
        test_application: Some(FerroTestApplicationAttribute::new(build)),
        test_isolation: None,
    };

    #[test]
    fn external_overlay_host_markup_resolves_collections_contracts_and_content_properties() {
        HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
            .dispatch(
                || {
                    use ferrodock_ferro::controls::overlays::{OverlayHost, OverlayLayerRegistry};
                    ferrodock_ferro::register_types();
                    ferroui_markup_xaml::register_types();
                    OverlayLayerRegistry::use_service_provider(None);
                    crate::compiled_markup::register();
                    let value = crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferrodock_markup_tests/OverlayHostConsumer.xaml",
                    )
                    .unwrap()
                    .unwrap();
                    let host = value.downcast_ref::<Ref<OverlayHost>>().unwrap();
                    assert!(!host.use_service_layers());
                    assert_eq!(host.overlays().unwrap().count(), 1);
                    let list = host.overlay_layers();
                    assert_eq!(list.count(), 2);
                    assert_eq!(
                        list.get(0)
                            .unwrap()
                            .style_key()
                            .unwrap()
                            .downcast_ref::<String>()
                            .unwrap(),
                        "DockOverlayBusyLayerTheme"
                    );
                    let plain = list.get(0).unwrap().overlay().unwrap();
                    let outer = list.get(1).unwrap().overlay().unwrap();
                    let content = Control::from_boxed(&host.content().unwrap()).unwrap();
                    let window = Window::new();
                    window.set_content(Some(Control::boxed(host.clone())));
                    window.show();
                    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                    window.update_layout();
                    assert_eq!(host.child(), Some(outer));
                    assert!(plain.visual_parent().is_some());
                    assert_eq!(plain.z_index(), 4);
                    assert!(!plain.is_hit_test_visible());
                    assert!(content.visual_parent().is_some());
                    assert!(host
                        .overlays()
                        .unwrap()
                        .get(0)
                        .unwrap()
                        .visual_parent()
                        .is_none());
                    window.close();
                },
                CancellationToken::none(),
            )
            .wait()
            .expect("external overlay host markup case completed");
    }

    #[test]
    fn external_recycling_markup_assigns_contracts_and_compiles_parent_binding() {
        HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
            .dispatch(
                || {
                    use ferrodock_controls_recycling::{
                        from_recycling_value, to_recycling_value, ControlRecyclingDataTemplate,
                    };
                    use ferroui_controls::presenters::ContentPresenter;
                    use std::rc::Rc;
                    ferrodock_controls_recycling::register_types();
                    ferroui_markup_xaml::register_types();
                    crate::compiled_markup::register();
                    let value = crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferrodock_markup_tests/RecyclingConsumer.xaml",
                    )
                    .unwrap()
                    .unwrap();
                    let presenter = value.downcast_ref::<Ref<ContentPresenter>>().unwrap();
                    let window = Window::new();
                    window.set_content(Some(Control::boxed(presenter.clone())));
                    window.show();
                    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                    window.update_layout();
                    let cache =
                        ControlRecyclingDataTemplate::get_control_recycling(presenter).unwrap();
                    assert!(cache.try_to_use_id_as_key());
                    let first = presenter.child().unwrap();
                    let key = to_recycling_value(Rc::new("first".to_owned()));
                    assert_eq!(
                        Control::from_boxed(&from_recycling_value(
                            &cache.try_get_value(Some(key)).unwrap()
                        )),
                        Some(first.clone())
                    );
                    presenter.set_content(Some(Rc::new("second".to_owned())));
                    window.update_layout();
                    assert_ne!(presenter.child(), Some(first.clone()));
                    presenter.set_content(Some(Rc::new("first".to_owned())));
                    window.update_layout();
                    assert_eq!(presenter.child(), Some(first));
                    window.close();
                },
                CancellationToken::none(),
            )
            .wait()
            .expect("external recycling markup case completed");
    }

    #[test]
    fn external_compiled_markup_resolves_library_types_theme_and_two_way_attached_binding() {
        HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
            .dispatch(
                || {
                    ferrodock_controls_proportional_stack_panel::register_types();
                    crate::compiled_markup::register();
                    let value = crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferrodock_markup_tests/ProportionalConsumer.xaml",
                    )
                    .unwrap()
                    .unwrap();
                    let panel = value.downcast_ref::<Ref<ProportionalStackPanel>>().unwrap();
                    let first = panel.children().get(0);
                    let splitter = panel
                        .children()
                        .get(1)
                        .cast::<ProportionalStackPanelSplitter>()
                        .unwrap();
                    let last = panel.children().get(2);
                    let first_model = ValueTypes::as_object(&*first.data_context().unwrap())
                        .unwrap()
                        .cast::<Border>()
                        .unwrap();
                    let last_model = ValueTypes::as_object(&*last.data_context().unwrap())
                        .unwrap()
                        .cast::<Border>()
                        .unwrap();
                    let window = Window::new();
                    window.set_width(604.0);
                    window.set_height(100.0);
                    window.set_content(Some(Control::boxed(panel.clone())));
                    window.show();
                    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                    window.update_layout();
                    assert_eq!(panel.orientation(), Orientation::Horizontal);
                    assert_eq!(first.bounds(), Rect::new(0.0, 0.0, 150.0, 100.0));
                    assert_eq!(splitter.bounds(), Rect::new(150.0, 0.0, 4.0, 100.0));
                    assert_eq!(last.bounds(), Rect::new(154.0, 0.0, 450.0, 100.0));
                    splitter.apply_template();
                    assert!(splitter.visual_children().get(0).cast::<Border>().is_some());
                    panel.resize_proportions(first.clone(), last.clone(), 60.4, 75.0);
                    assert!(
                        (ProportionalStackPanel::get_proportion(&first_model) - 0.35).abs() < 1e-9
                    );
                    assert!(
                        (ProportionalStackPanel::get_proportion(&last_model) - 0.65).abs() < 1e-9
                    );
                    ProportionalStackPanel::set_proportion(&first_model, 0.5);
                    ProportionalStackPanel::set_proportion(&last_model, 0.5);
                    window.update_layout();
                    assert_eq!(first.bounds(), Rect::new(0.0, 0.0, 300.0, 100.0));
                    assert_eq!(last.bounds(), Rect::new(304.0, 0.0, 300.0, 100.0));
                    window.close();
                },
                CancellationToken::none(),
            )
            .wait()
            .expect("external compiled markup case completed");
    }
}

#[cfg(test)]
mod service_overlay_tests;

#[cfg(test)]
mod request_dialog_tests;

#[cfg(test)]
mod lifecycle_tests;

#[cfg(test)]
mod root_control_tests;

#[cfg(test)]
mod deferred_tests;

#[cfg(test)]
mod dock_target_tests;

#[cfg(test)]
mod mdi_tests;

#[cfg(test)]
mod managed_dockable_tests;

#[cfg(test)]
mod container_tests;

#[cfg(test)]
mod drag_preview_tests;

#[cfg(test)]
mod dockable_contract_tests;
#[cfg(test)]
mod selector_tests;

#[cfg(test)]
mod command_bar_tests;

#[cfg(test)]
mod pinned_window_tests;

#[cfg(test)]
mod managed_document_tests;

#[cfg(test)]
mod mdi_host_tests;

#[cfg(test)]
mod mdi_theme_tests;

#[cfg(test)]
mod dock_model_tests;

#[cfg(test)]
mod capability_tests;

#[cfg(test)]
mod managed_layer_tests;

#[cfg(test)]
mod host_control_tests;

#[cfg(test)]
mod tab_theme_tests;

#[cfg(test)]
mod docking_graph_tests;

#[cfg(test)]
mod full_theme_tests;

#[cfg(test)]
mod markup_extension_tests;

#[cfg(test)]
mod name_scope_tests;

#[cfg(test)]
mod carrier_model_tests;

#[cfg(test)]
mod reactive_sdk_tests;

#[cfg(test)]
mod reactive_layout_sdk_tests;

#[cfg(test)]
mod factory_window_sdk_tests;

/// Source event owner at the application compilation boundary.
pub mod factory_event_consumer;
#[cfg(test)]
mod factory_event_sdk_tests;

/// Application source factory used to exercise the real locator API.
pub mod locator_factory;

#[cfg(test)]
mod factory_locator_sdk_tests;

#[cfg(test)]
mod core_policy_sdk_tests;

#[cfg(test)]
mod core_contract_sdk_tests;

#[cfg(test)]
mod core_host_sdk_tests;

#[cfg(test)]
mod core_controls_sdk_tests;

#[cfg(test)]
mod native_document_factory_sdk_tests;

#[cfg(test)]
mod null_engine_sdk_tests;

#[cfg(test)]
mod null_list_sdk_tests;

#[cfg(test)]
mod nullable_move_sdk_tests;

#[cfg(test)]
mod nullable_preview_sdk_tests;

#[cfg(test)]
mod nullable_hidden_sdk_tests;

#[cfg(test)]
mod nullable_floating_sdk_tests;

#[cfg(test)]
mod nullable_focus_sdk_tests;

#[cfg(test)]
mod nullable_window_options_sdk_tests;

#[cfg(test)]
mod nullable_windows_sdk_tests;

#[cfg(test)]
mod nullable_search_sdk_tests;

#[cfg(test)]
mod query_cursor_sdk_tests;
