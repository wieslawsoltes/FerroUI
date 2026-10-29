//! Reload integration with application-owned view models and shared selection.
use ferroui_base::{threading::Dispatcher, Ref};
use ferroui_controls::{selection::ISelectionModel, ContentPage, Control, Window};
use ferroui_hot_reload::{
    ComponentReloadTarget, HotReloadSession, ReloadError, ReloadStatus, ReloadTarget,
};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use std::rc::Rc;

#[test]
fn catalog_inline_styles_refresh_existing_page_and_preserve_application_styles() {
    use ferroui_controls::Border;
    use ferroui_base::styling::{IStyle, style_ptr_eq};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderDocument};
    let _application = super::support::start_catalog_application();
    let page = crate::pages::BorderPage::new();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.show();
    let uri = "ferres://controlcatalog/reload-tests/InlineStyles.xaml";
    let baseline = include_str!("../Pages/BorderPage.xaml");
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::hot_reload::component_target(
        &root, uri.into(), baseline.into(), Vec::new(),
    ));
    let app_style = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::new(
            r#"<Style xmlns="https://github.com/ferroui" Selector="Border"><Setter Property="MinHeight" Value="23"/></Style>"#,
        ), None,
    ).unwrap();
    let app_style = Rc::<dyn IStyle>::from_xaml_object(&app_style).unwrap();
    let styles = root.styles();
    styles.add(app_style.clone());
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), baseline.into()).unwrap();
    session.register_target(&target);
    for width in [240, 280] {
        let update = baseline.replace("  <ScrollViewer", &format!(r#"<ContentPage.Styles><Styles>
<Styles.Resources><x:Double x:Key="InlineWidth">{width}</x:Double></Styles.Resources>
<Style Selector="Border"><Setter Property="Width" Value="{{StaticResource InlineWidth}}"/></Style>
</Styles></ContentPage.Styles>
  <ScrollViewer"#));
        session.submit(uri.into(), update);
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        let border = page.get_logical_descendants().find_map(|control| control.cast::<Border>()).unwrap();
        assert_eq!(border.width(), width as f64);
        assert_eq!(border.min_height(), 23.0);
        assert!(root.styles().ptr_eq(&styles));
        assert!(style_ptr_eq(&styles.get(1), &app_style));
    }
    session.submit(uri.into(), baseline.into());
    let result = session.apply_pending().unwrap();
    assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
    assert_eq!(styles.count(), 1);
    assert!(style_ptr_eq(&styles.get(0), &app_style));
    assert!(styles.try_get_resource(&"InlineWidth".into(), None).is_none());
    window.close();
}

#[test]
fn catalog_root_resources_reload_without_replacing_page_dictionary_or_application_keys() {
    use ferroui_controls::Border;
    let _application = super::support::start_catalog_application();
    let page = crate::pages::BorderPage::new();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.show();
    let uri = "ferres://controlcatalog/reload-tests/InlineResources.xaml";
    let baseline = include_str!("../Pages/BorderPage.xaml");
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::hot_reload::component_target(
        &root, uri.into(), baseline.into(), Vec::new(),
    ));
    let dictionary = root.resources();
    dictionary.add_value("ApplicationKey", "preserved".to_string());
    let app_value = dictionary.get(&"ApplicationKey".into()).unwrap();
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), baseline.into()).unwrap();
    session.register_target(&target);
    for width in [240, 280] {
        let update = baseline.replace(
            "  <ScrollViewer", &format!(r#"<ContentPage.Resources><x:Double x:Key="RootWidth">{width}</x:Double></ContentPage.Resources>
  <ScrollViewer"#),
        ).replacen("<Border BorderBrush=", r#"<Border Width="{StaticResource RootWidth}" BorderBrush="#, 1);
        session.submit(uri.into(), update);
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        assert!(root.resources().ptr_eq(&dictionary));
        assert!(Rc::ptr_eq(&app_value, &dictionary.get(&"ApplicationKey".into()).unwrap()));
        let border = page.get_logical_descendants().find_map(|control| control.cast::<Border>()).unwrap();
        assert_eq!(border.width(), width as f64);
    }
    session.submit(uri.into(), baseline.into());
    let result = session.apply_pending().unwrap();
    assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
    assert!(!dictionary.contains_key(&"RootWidth".into()));
    assert!(Rc::ptr_eq(&app_value, &dictionary.get(&"ApplicationKey".into()).unwrap()));
    window.close();
}

#[test]
fn catalog_mixed_merged_resources_preserve_application_precedence_and_rollback() {
    use ferroui_base::controls::{IResourceProvider, ResourceDictionary};
    use ferroui_controls::Border;
    let _application = super::support::start_catalog_application();
    let page = crate::pages::BorderPage::new();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.show();
    let uri = "ferres://controlcatalog/reload-tests/MixedResources.xaml";
    let baseline = include_str!("../Pages/BorderPage.xaml");
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::hot_reload::component_target(
        &root, uri.into(), baseline.into(), Vec::new(),
    ));
    let dictionary = root.resources();
    let application = ResourceDictionary::new();
    application.add_value("RootWidth", 333.0f64);
    dictionary.merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(application.clone()));
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), baseline.into()).unwrap();
    session.register_target(&target);
    for width in [240, 280] {
        let update = baseline.replace("  <ScrollViewer", &format!(r#"<ContentPage.Resources><ResourceDictionary>
<ResourceDictionary.MergedDictionaries><ResourceDictionary><x:Double x:Key="RootWidth">{width}</x:Double></ResourceDictionary></ResourceDictionary.MergedDictionaries>
</ResourceDictionary></ContentPage.Resources>
  <ScrollViewer"#)).replacen("<Border BorderBrush=", r#"<Border Width="{StaticResource RootWidth}" BorderBrush="#, 1);
        let old = root.content();
        session.submit(uri.into(), update.clone());
        let reached = std::cell::Cell::new(false);
        let result = session.apply_pending_with(|| {
            reached.set(true);
            Err(ReloadError::Failed("catalog rollback".into()))
        }).unwrap();
        assert!(reached.get(), "{result:?}");
        assert_eq!(root.content(), old);
        session.submit(uri.into(), update);
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        let border = page.get_logical_descendants().find_map(|control| control.cast::<Border>()).unwrap();
        assert_eq!(border.width(), 333.0);
        assert!(root.resources().ptr_eq(&dictionary));
        assert!(application.owner().is_some());
    }
    session.submit(uri.into(), baseline.into());
    assert_eq!(session.apply_pending().unwrap().status, ReloadStatus::Applied);
    assert_eq!(dictionary.merged_dictionaries().count(), 1);
    assert!(dictionary.merged_dictionaries().get(0).as_ref() == Rc::<dyn IResourceProvider>::from(application).as_ref());
    window.close();
}

#[test]
fn generated_manifest_allows_binding_element_path_edits() {
    use ferroui_controls::{CheckBox, ContentControl, TextBlock};
    use ferroui_markup_xaml::FromXamlObject;
    let _application = super::support::start_catalog_application();
    let uri = "ferres://catalog-binding-contract/View.xaml";
    let source = r#"<TextBlock xmlns="https://github.com/ferroui"><TextBlock.Text><Binding Path="Tag"/></TextBlock.Text></TextBlock>"#;
    let mut manifest = ferroui_hot_reload::ReloadManifest::parse(
        include_str!(env!("FERROUI_HOT_RELOAD_MANIFEST")),
    ).unwrap();
    manifest.sources.clear();
    manifest.sources.insert(uri.into(), source.into());
    let mut session = manifest.session().unwrap();
    let model = CheckBox::new();
    model.set_tag(Some(Rc::new("tag value".to_string())));
    model.set_content(Some(Rc::new("content value".to_string())));
    let host = ContentControl::new();
    host.set_data_context(Some(Rc::new(model)));
    let target: Rc<dyn ReloadTarget> = Rc::new(ferroui_hot_reload::ContentReloadTarget::new(
        &host, uri.into(), Vec::new(), RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&target);
    for (source, expected) in [
        (source.to_owned(), "tag value"),
        (source.replace("Path=\"Tag\"", "Path=\"Content\""), "content value"),
    ] {
        session.submit(uri.into(), source);
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        Dispatcher::ui_thread().run_jobs(None);
        let text = Ref::<TextBlock>::from_xaml_object(&host.content().unwrap()).unwrap();
        assert_eq!(text.text().as_deref(), Some(expected));
    }
}

#[test]
fn checkbox_application_state_adapter_preserves_input_and_respects_new_markup() {
    use ferroui_base::controls::NameScope;
    use ferroui_controls::CheckBox;
    let _application = super::support::start_catalog_application();
    let page = crate::pages::CheckBoxPage::new();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    let find = || {
        NameScope::get_name_scope(&root).unwrap()
            .find("ReloadCheckBox").unwrap().cast::<CheckBox>().unwrap()
    };
    let uri = "ferres://catalog-state-tests/CheckBoxPage.xaml";
    let source = include_str!("../Pages/CheckBoxPage.xaml");
    // Exercise the actual build metadata while isolating this session's document
    // URI from the application's own development session.
    let mut manifest = ferroui_hot_reload::ReloadManifest::parse(
        include_str!(env!("FERROUI_HOT_RELOAD_MANIFEST")),
    ).unwrap();
    manifest.sources.clear();
    manifest.sources.insert(uri.into(), source.into());
    let mut session = manifest.session().unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::hot_reload::component_target(
        &root, uri.into(), source.into(), Vec::new(),
    ));
    session.register_target(&target);
    for revision in 1..=3 {
        let old = find();
        old.set_is_checked(Some(true));
        let retired = old.downgrade();
        let update = source.replace("A check box control", &format!("Check box revision {revision}"));
        session.submit(uri.into(), update.clone());
        let failed = session.apply_pending_with(|| Err(ReloadError::Failed("rollback".into()))).unwrap();
        assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
        Dispatcher::ui_thread().run_jobs(None);
        assert!(find().ptr_eq(&old));
        assert_eq!(old.is_checked(), Some(true));
        session.submit(uri.into(), update);
        let applied = session.apply_pending().unwrap();
        assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!find().ptr_eq(&old));
        assert_eq!(find().is_checked(), Some(true));
        drop(old);
        assert!(retired.upgrade().is_none(), "custom state retained an old check box");
    }
    session.submit(uri.into(), source.replace(
        "x:Name=\"ReloadCheckBox\"", "x:Name=\"ReloadCheckBox\" IsChecked=\"False\"",
    ));
    let applied = session.apply_pending().unwrap();
    assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(find().is_checked(), Some(false));
    find().set_is_checked(Some(true));
    session.submit(uri.into(), source.into());
    let applied = session.apply_pending().unwrap();
    assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(find().is_checked(), Some(false), "removed authored value must not restore stale input");
    window.close();
}

#[test]
fn registered_catalog_root_binding_uses_preserved_page_and_window() {
    let _application = super::support::start_catalog_application();
    let page = crate::pages::BorderPage::new();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.show();
    let uri = "ferres://controlcatalog/reload-tests/BorderPage.xaml";
    let source = include_str!("../Pages/BorderPage.xaml");
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::hot_reload::component_target(
        &root, uri.into(), source.into(), Vec::new(),
    ));
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), source.into()).unwrap();
    session.register_target(&target);
    root.set_height(320.0);
    let edited = source.replacen(
        "<ContentPage ", r#"<ContentPage Width="{Binding Height, RelativeSource={RelativeSource Self}}" "#, 1,
    );
    assert_ne!(edited, source);
    let old = root.content();
    session.submit(uri.into(), edited.clone());
    let finalized = std::cell::Cell::new(false);
    let outcome = session.apply_pending_with(|| {
        finalized.set(true);
        assert_eq!(root.width(), 320.0);
        Err(ReloadError::Failed("later target failed".into()))
    }).unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed, "{outcome:?}");
    assert!(finalized.get(), "fixture failed before testing transaction rollback: {outcome:?}");
    assert_eq!(root.content(), old);
    assert!(root.width().is_nan());
    session.submit(uri.into(), edited);
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    assert_eq!(root.width(), 320.0);
    root.set_height(400.0);
    assert_eq!(root.width(), 400.0);
    assert_eq!(window.content(), Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.close();
}

#[test]
fn application_scope_releases_catalog_watcher_before_thread_teardown() {
    let released = {
        let _application = super::support::start_catalog_application();
        let _page = crate::pages::ListBoxPage::new();
        crate::hot_reload::session_release_check()
    };
    assert!(released(), "application scope retained the catalog reader");
}

#[test]
fn list_box_reload_preserves_shared_selection_on_commit_and_rollback() {
    let _application = super::support::start_catalog_application();
    let page = crate::pages::ListBoxPage::new();
    let context = page.data_context().unwrap();
    let model = context
        .downcast_ref::<crate::view_models::ListBoxPageViewModel>()
        .unwrap();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new();
    window.set_width(900.0);
    window.set_height(650.0);
    window.set_content(Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    model.selection().select(42);
    assert_eq!(model.selection().selected_index(), 42);
    // The catalog watcher owns its original URI. This transaction fixture uses
    // a separate document identity while exercising the actual page and model.
    let uri = "ferres://catalog-reload-tests/ListBoxPage.xaml".to_string();
    let source = include_str!("../Pages/ListBoxPage.xaml");
    let mut configuration = RuntimeXamlLoaderConfiguration::default();
    configuration.local_assembly = Some(&crate::register_types::ASSEMBLY);
    configuration.use_compiled_bindings_by_default = true;
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.clone(), source.into())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_page(
        &root,
        uri.clone(),
        source.into(),
        configuration,
    ));
    session.register_target(&target);
    for index in 0..3 {
        let before = root.content();
        let retired = ferroui_base::metadata::from_markup_value::<Ref<Control>>(&before)
            .unwrap()
            .downgrade();
        let update = source.replace(
            "Hosts a collection",
            &format!("Revision {index}: Hosts a collection"),
        );
        session.submit(uri.clone(), update.clone());
        assert_eq!(
            session
                .apply_pending_with(|| Err(ReloadError::Failed("rollback fixture".into())))
                .unwrap()
                .status,
            ReloadStatus::Failed
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(root.content(), before);
        assert_eq!(
            model.selection().selected_index(),
            42,
            "rollback changed view-model selection"
        );
        session.submit(uri.clone(), update);
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Applied
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert_ne!(root.content(), before);
        assert!(Rc::ptr_eq(&root.data_context().unwrap(), &context));
        assert_eq!(
            model.selection().selected_index(),
            42,
            "reload changed view-model selection"
        );
        drop(before);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            retired.upgrade().is_none(),
            "retired tree retained after reload"
        );
    }
    window.close();
}

#[test]
fn application_resource_include_is_discovered_and_replaced() {
    use ferroui_controls::Application;
    let _application = super::support::start_catalog_app();
    let application = Application::current().unwrap();
    let resources = application.resources();
    let mut configuration = RuntimeXamlLoaderConfiguration::default();
    configuration.local_assembly = Some(&crate::register_types::ASSEMBLY);
    let uri = "ferres://controlcatalog/CustomThemes.xaml";
    let source = include_str!("../CustomThemes.xaml");
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source.into())
        .unwrap();
    session.track_resource_includes(&resources, configuration);
    let old = resources.merged_dictionaries().to_vec();
    session.submit(uri.into(), source.replace("#2B2B33", "#2C2C34"));
    let result = session.apply_pending().unwrap();
    assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
    let current = resources.merged_dictionaries().to_vec();
    assert_eq!(current.len(), old.len());
    assert!(!Rc::ptr_eq(&current[0], &old[0]));
    assert!(old[0].owner().is_none());
    assert!(current[0].owner().is_some());
    assert!(application.resources().ptr_eq(&resources));
}

#[test]
fn catalog_xaml_handlers_refresh_cached_adorner_and_retire_old_subscriptions() {
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::{Button, primitives::AdornerLayer};
    use std::cell::Cell;
    let _application = super::support::start_catalog_application();
    // This constructor uses generated markup by default. Replacements use the
    // interpreter, exercising ownership across the two actual loading paths.
    let page = crate::pages::AdornerLayerPage::new();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&page.clone().upcast::<Control>())));
    window.show();
    let uri = "ferres://controlcatalog/reload-tests/Handlers.xaml";
    let baseline = include_str!("../Pages/AdornerLayerPage.xaml");
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::hot_reload::component_target(
        &root, uri.into(), baseline.into(), Vec::new(),
    ));
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), baseline.into()).unwrap();
    session.register_target(&target);
    let click = |button: &Button| button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    let application = Rc::new(Cell::new(0));
    for width in [240, 280, 320] {
        let old = page.get_control::<Button>("AdornerButton");
        let remove = page.get_control::<Button>("RemoveAdorner");
        let add = page.get_control::<Button>("AddAdorner");
        let old_adorner = AdornerLayer::get_adorner(&old).unwrap();
        let adorner_weak = old_adorner.downgrade();
        let remove_weak = remove.downgrade();
        let app = application.clone(); remove.click(move |_, _| app.set(app.get() + 1));
        click(&remove);
        assert!(AdornerLayer::get_adorner(&old).is_none());
        let update = baseline.replace("Width=\"200\"", &format!("Width=\"{width}\""));
        session.submit(uri.into(), update.clone());
        let reached = Cell::new(false);
        let result = session.apply_pending_with(|| {
            reached.set(true);
            Err(ReloadError::Failed("adorner rollback".into()))
        }).unwrap();
        assert!(reached.get(), "{result:?}");
        assert_eq!(result.status, ReloadStatus::Failed);
        assert!(page.get_control::<Button>("AdornerButton").ptr_eq(&old));
        click(&add);
        assert!(AdornerLayer::get_adorner(&old).unwrap().ptr_eq(&old_adorner));
        click(&remove);
        session.submit(uri.into(), update);
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        let new = page.get_control::<Button>("AdornerButton");
        assert!(!new.ptr_eq(&old));
        assert_eq!(new.width(), width as f64);
        let new_adorner = AdornerLayer::get_adorner(&new).unwrap();
        // Old XAML subscriptions must not act on the new namescope, even while
        // the application deliberately holds the old buttons alive.
        click(&remove);
        click(&add);
        assert!(AdornerLayer::get_adorner(&new).unwrap().ptr_eq(&new_adorner));
        click(&page.get_control::<Button>("AddAdorner"));
        assert!(AdornerLayer::get_adorner(&new).unwrap().ptr_eq(&new_adorner));
        click(&page.get_control::<Button>("RemoveAdorner"));
        assert!(AdornerLayer::get_adorner(&new).is_none());
        click(&page.get_control::<Button>("AddAdorner"));
        assert!(AdornerLayer::get_adorner(&new).unwrap().ptr_eq(&new_adorner));
        drop(old); drop(remove); drop(add); drop(old_adorner);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(remove_weak.upgrade().is_none());
        assert!(adorner_weak.upgrade().is_none());
    }
    assert_eq!(application.get(), 9);
    window.close();
}

#[test]
fn compiled_catalog_template_scroll_survives_included_theme_replacement() {
    use ferroui_base::Vector;
    use ferroui_controls::{Application, ScrollViewer};
    let _application = super::support::start_catalog_app();
    let page = crate::pages::BorderPage::new();
    // Use the real compiled page/theme, then own an isolated session explicitly.
    crate::hot_reload::shutdown();
    let root: Ref<ContentPage> = page.clone().upcast();
    let window = Window::new(); window.set_width(640.0); window.set_height(180.0);
    window.set_content(Some(Control::boxed(&root.clone().upcast::<Control>())));
    window.show(); Dispatcher::ui_thread().run_jobs(None);
    let find = || root.get_visual_descendants().find_map(|node| node.cast::<ScrollViewer>()
        .filter(|scroll| ferroui_markup_xaml::reload_identity::key(scroll).as_deref() == Some("PageScroll"))).unwrap();
    let old = find(); assert!(old.name().is_none(), "development keys are not public names"); old.set_offset(Vector::new(0.0, 80.0));
    assert_eq!(old.offset().y, 80.0, "fixture must actually scroll");
    let view_uri = "ferres://controlcatalog/Pages/BorderPage.xaml";
    let theme_uri = "ferres://controlcatalog/CustomThemes.xaml";
    let view = include_str!("../Pages/BorderPage.xaml");
    let theme = include_str!("../CustomThemes.xaml");
    let mut session = HotReloadSession::new();
    session.register_document(view_uri.into(), view.into()).unwrap();
    session.register_document(theme_uri.into(), theme.into()).unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::hot_reload::component_target(
        &root, view_uri.into(), view.into(), vec![theme_uri.into()],
    ));
    session.register_target(&target);
    let mut configuration = RuntimeXamlLoaderConfiguration::default();
    configuration.local_assembly = Some(&crate::register_types::ASSEMBLY);
    session.track_resource_includes(&Application::current().unwrap().resources(), configuration);
    for padding in [21, 22] {
        let previous = find(); let weak = previous.downgrade();
        session.submit(theme_uri.into(), theme.replace("hr:Key=\"PageScroll\" Padding=\"20\"", &format!("hr:Key=\"PageScroll\" Padding=\"{padding}\"")));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        let current = find(); assert!(!current.ptr_eq(&previous));
        assert_eq!(current.offset().y, 80.0);
        drop(previous);
        if padding == 22 { assert!(weak.upgrade().is_none()); }
    }
    drop(old); window.close();
}

#[test]
fn reload_overlay_needs_the_source_directory_of_the_build() {
    use std::{collections::BTreeMap, path::Path};
    let original_root = Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize().unwrap();
    let page = original_root.join("Pages/BorderPage.xaml");
    let directory =
        std::env::temp_dir().join(format!("ferroui-catalog-overlay-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("Pages")).unwrap();
    std::fs::write(directory.join("Pages/BorderPage.xaml"), "<ContentPage/>").unwrap();
    let files = BTreeMap::from([
        ("border".to_owned(), page.clone()),
        ("slider".to_owned(), original_root.join("Pages/SliderPage.xaml")),
    ]);

    // A copy in the overlay takes the place of the source; the others stay.
    let mut overlaid = files.clone();
    crate::hot_reload::overlay_sources(&mut overlaid, &original_root, &directory).unwrap();
    assert_eq!(overlaid["border"], directory.join("Pages/BorderPage.xaml"));
    assert_eq!(overlaid["slider"], files["slider"]);

    // A binary that runs where the source directory of its build does not exist.
    let mut missing = files.clone();
    let error =
        crate::hot_reload::overlay_sources(&mut missing, &directory.join("missing"), &directory)
            .unwrap_err();
    assert!(error.contains("is not available"), "{error}");
    assert_eq!(missing, files);

    // A source outside the source directory of the build.
    let mut outside =
        BTreeMap::from([("other".to_owned(), directory.join("Pages/BorderPage.xaml"))]);
    let error =
        crate::hot_reload::overlay_sources(&mut outside, &original_root, &directory).unwrap_err();
    assert!(error.contains("is not under"), "{error}");
    std::fs::remove_dir_all(&directory).unwrap();
}
