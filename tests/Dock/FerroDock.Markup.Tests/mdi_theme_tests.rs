// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original MDI host dictionary and its four source empty-content scenarios.
use ferrodock_ferro::{
    controls::{MdiDocumentControl, MdiDocumentWindow},
    model_projection,
};
use ferrodock_model::{collections::ListHandle, controls::IDocumentDock, core::*};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock},
    Factory,
};
use ferroui_base::{
    controls::ResourceKey,
    styling::ControlTheme,
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{
    templates::FuncDataTemplate, AppBuilder, Application, ContentControl, Control, ItemsControl,
    Panel, TextBlock, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
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
    name: "mdi-original-theme",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene {
    window: Ref<Window>,
    control: Ref<MdiDocumentControl>,
    dock: Rc<DocumentDock>,
    _factory: Rc<Factory>,
}
impl Scene {
    fn new(with_document: bool) -> Self {
        Self::with_template(with_document, None)
    }
    fn with_template(
        with_document: bool,
        template: Option<Rc<dyn ferroui_controls::templates::IDataTemplate>>,
    ) -> Self {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/MdiThemeConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
        let c = panel
            .find_descendant_of_type::<MdiDocumentControl>(false)
            .unwrap();
        let factory = Factory::new();
        let dock = DocumentDock::new();
        dock.set_factory(Some(factory.clone()));
        dock.set_layout_mode(DocumentLayoutMode::Mdi);
        dock.set_empty_content(Some(Rc::new("No documents are open.".to_owned())));
        let list: ListHandle<Option<Rc<dyn IDockable>>> = ListHandle::new();
        if with_document {
            let d = Document::new();
            d.set_id("Document1".into());
            d.set_title("Document 1".into());
            list.add(Some(d));
        }
        dock.set_visible_dockables(Some(list));
        let model: Rc<dyn IDocumentDock> = dock.clone();
        c.set_data_context(Some(Rc::new(model)));
        let theme = ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(
            &panel.find_resource(&ResourceKey::from(MdiDocumentControl::TYPE)),
        )
        .unwrap();
        c.set_theme(Some(theme));
        c.set_empty_content_template(template);
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        c.apply_template();
        window.update_layout();
        c.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        Self {
            window,
            control: c,
            dock,
            _factory: factory,
        }
    }
    fn empty(&self) -> Ref<ContentControl> {
        self.control
            .find_descendant_of_type_where::<ContentControl>(false, |c| {
                c.name().as_deref() == Some("PART_EmptyContentHost")
            })
            .unwrap()
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
#[test]
fn original_mdi_host_shows_empty_content_with_no_documents() {
    run(|| {
        let s = Scene::new(false);
        let empty = s.empty();
        assert!(empty.is_visible());
        assert!(!s.control.has_visible_documents());
        assert_eq!(
            empty.content().unwrap().downcast_ref::<String>().unwrap(),
            "No documents are open."
        );
    });
}
#[test]
fn original_mdi_host_hides_empty_content_with_documents() {
    run(|| {
        let s = Scene::new(true);
        assert!(!s.empty().is_visible());
        assert!(s.control.has_visible_documents());
        let frame = s
            .control
            .find_descendant_of_type::<MdiDocumentWindow>(false)
            .unwrap();
        assert_eq!(
            model_projection::to_dockable_value(
                s.dock
                    .visible_dockables()
                    .unwrap()
                    .get(0)
                    .expect("nonnull MDI fixture")
            )
            .downcast_ref::<Rc<dyn IDockable>>()
            .unwrap()
            .id(),
            "Document1"
        );
        assert_eq!(frame.mdi_state(), MdiWindowState::Normal);
        let document = s
            .dock
            .visible_dockables()
            .unwrap()
            .get(0)
            .and_then(|item| item.as_mdi_document())
            .unwrap();
        document.set_mdi_state(MdiWindowState::Maximized);
        document.set_mdi_z_index(12);
        assert_eq!(frame.mdi_state(), MdiWindowState::Maximized);
        assert_eq!(frame.z_index(), 12);
        document.set_mdi_state(MdiWindowState::Minimized);
        assert_eq!(frame.mdi_state(), MdiWindowState::Minimized);
    });
}
#[test]
fn original_mdi_host_updates_empty_visibility_and_real_items_on_add_remove() {
    run(|| {
        let s = Scene::new(false);
        let empty = s.empty();
        let items = s
            .control
            .find_descendant_of_type::<ItemsControl>(false)
            .unwrap();
        assert!(empty.is_visible());
        let d = Document::new();
        s.dock.visible_dockables().unwrap().add(Some(d.clone()));
        s.window.update_layout();
        s.control.update_layout();
        assert!(!empty.is_visible());
        assert!(s.control.has_visible_documents());
        assert_eq!(items.items().count(), 1);
        assert!(s
            .control
            .find_descendant_of_type::<MdiDocumentWindow>(false)
            .is_some());
        s.dock
            .visible_dockables()
            .unwrap()
            .remove(&Some((d as Rc<dyn IDockable>).clone()));
        s.window.update_layout();
        s.control.update_layout();
        assert!(empty.is_visible());
        assert!(!s.control.has_visible_documents());
        assert_eq!(items.items().count(), 0);
    });
}
#[test]
fn original_mdi_host_applies_empty_content_template() {
    run(|| {
        let template = FuncDataTemplate::for_type::<String>(
            |text, _| {
                let t = TextBlock::new();
                t.set_text(Some(&format!("Template: {text}")));
                Some(t.upcast())
            },
            true,
        );
        let s = Scene::with_template(false, Some(template.clone()));
        let empty = s.empty();
        let expected: Rc<dyn ferroui_controls::templates::IDataTemplate> = template;
        assert!(Rc::ptr_eq(&empty.content_template().unwrap(), &expected));
        assert!(empty
            .find_descendant_of_type_where::<TextBlock>(false, |t| t.text().as_deref()
                == Some("Template: No documents are open."))
            .is_some());
    });
}
