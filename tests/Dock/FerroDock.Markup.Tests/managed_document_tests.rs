// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

//! Compiled inherited properties and the original managed document recycling template.
use ferrodock_ferro::controls::{ManagedDockWindowDocument, MdiDocumentWindow};
use ferrodock_model::core::{IDockable, MdiWindowState};
use ferrodock_model_reactiveui::{
    controls::{Document, RootDock},
    core::DockWindow,
};
use ferroui_base::{
    controls::ResourceKey,
    threading::{CancellationToken, Dispatcher},
    BoxedValue, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, Button, ContentControl, Control, StackPanel, TextBlock, TextBox,
    Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_markup_xaml::templates::DataTemplate;
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
    name: "managed-document-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn compiled_consumer_uses_original_managed_document_notifications_recycling_and_shared_deferred_loader(
) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/ManagedDocumentConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
                let root = RootDock::new();
                let focused = Document::new();
                focused.set_title("Original focused".into());
                root.set_focused_dockable(Some(focused.clone()));
                let model_window = DockWindow::new();
                model_window.set_layout(Some(root));
                let model = ManagedDockWindowDocument::new(model_window.clone());
                model.set_mdi_z_index(4);
                let first = Button::new();
                first.set_content(Some(Rc::new("First".to_owned())));
                model.set_content(Some(ferrodock_controls_recycling::to_recycling_value(
                    Control::boxed(first.clone()),
                )));
                let context: BoxedValue =
                    ferrodock_ferro::model_projection::to_dockable_value(model.clone());
                assert!(
                    ferroui_base::metadata::from_markup_value::<
                        Option<Rc<dyn ferroui_controls::templates::IDataTemplate>>,
                    >(&Some(context.clone()))
                    .is_some(),
                    "nullable template conversion"
                );
                panel.set_data_context(Some(context));
                let window = Window::new();
                window.set_width(500.0);
                window.set_height(400.0);
                window.set_content(Some(Control::boxed(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                let title = panel
                    .find_descendant_of_type_where::<TextBlock>(false, |c| {
                        c.name().as_deref() == Some("ManagedTitle")
                    })
                    .unwrap();
                let editor = panel
                    .find_descendant_of_type_where::<TextBox>(false, |c| {
                        c.name().as_deref() == Some("ManagedEditor")
                    })
                    .unwrap();
                let z = panel
                    .find_descendant_of_type_where::<TextBlock>(false, |c| {
                        c.name().as_deref() == Some("ManagedZIndex")
                    })
                    .unwrap();
                let frame = panel
                    .find_descendant_of_type::<MdiDocumentWindow>(false)
                    .unwrap();
                assert!(frame.header_template().is_some());
                assert_eq!(frame.mdi_state(), MdiWindowState::Normal);
                model.set_mdi_state(MdiWindowState::Maximized);
                assert_eq!(frame.mdi_state(), MdiWindowState::Maximized);
                assert!(frame.pseudo_classes().contains(":maximized"));
                frame.set_mdi_state(MdiWindowState::Minimized);
                assert_eq!(model.mdi_state(), MdiWindowState::Minimized);
                frame.set_is_active(true);
                assert!(frame.is_active());
                assert_eq!(title.text().as_deref(), Some("Original focused"));
                assert_eq!(z.text().as_deref(), Some("4"));
                focused.set_title("Live focus".into());
                assert_eq!(title.text().as_deref(), Some("Live focus"));
                editor.set_text(Some("Edited"));
                assert_eq!(model.title(), "Edited");
                assert_eq!(model_window.title(), "Edited");
                model.set_mdi_z_index(9);
                assert_eq!(z.text().as_deref(), Some("9"));
                let content = panel
                    .find_descendant_of_type_where::<ContentControl>(false, |c| {
                        c.name().as_deref() == Some("ManagedBody")
                    })
                    .unwrap();
                assert!(
                    content.content_template().is_some(),
                    "original document template binding"
                );
                window.update_layout();
                assert_eq!(content.presenter().unwrap().child(), Some(first.upcast()));
                let body = panel.find_resource(&ResourceKey::from("DeferredManagedBody"));
                let template = ferroui_base::metadata::from_markup_value::<
                    Rc<dyn ferroui_controls::templates::IDataTemplate>,
                >(&body)
                .expect("compiled data template resource");
                let template = template
                    .as_any()
                    .and_then(|t| t.downcast_ref::<DataTemplate>())
                    .expect("native DataTemplate content");
                panel.set_data_context(None);
                window.update_layout();
                model.set_content(
                    template
                        .content()
                        .map(ferrodock_controls_recycling::to_recycling_value),
                );
                panel.set_data_context(Some(
                    ferrodock_ferro::model_projection::to_mdi_document_value(model.clone()),
                ));
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                let built = content
                    .presenter()
                    .unwrap()
                    .child()
                    .unwrap()
                    .cast::<Button>()
                    .unwrap();
                assert_eq!(
                    built.content().unwrap().downcast_ref::<String>().unwrap(),
                    "Compiled managed body"
                );
                focused.set_title("Live MDI interface".into());
                assert_eq!(title.text().as_deref(), Some("Live MDI interface"));
                model.set_mdi_z_index(13);
                assert_eq!(z.text().as_deref(), Some("13"));
                model.set_content(None);
                content.set_content(None);
                window.close();
                model.dispose();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
