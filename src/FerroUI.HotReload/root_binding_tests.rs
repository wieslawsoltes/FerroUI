use super::*;
use ferroui_base::{
    data::{BindingOperations, BindingPriority},
    layout::Layoutable,
    threading::Dispatcher,
    utilities::{Uri, UriKind},
    BoxedValue, Ref,
};
use ferroui_controls::{Border, ContentControl, Panel, StackPanel};
use ferroui_markup_xaml::{
    FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
};

fn load(uri: &str, source: &str) -> BoxedValue {
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_base_uri(
            Some(Uri::try_create(uri, UriKind::Absolute).unwrap()),
            source,
        ),
        None,
    )
    .unwrap()
}

fn component_target(root: &Ref<ContentControl>, uri: &str, source: &str) -> Rc<dyn ReloadTarget> {
    Rc::new(
        ComponentReloadTarget::content_control(
            root,
            uri.into(),
            source.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_root_binding_property(Layoutable::width_property()),
    )
}

#[test]
fn component_root_bindings_preserve_models_and_rollback_original_expression() {
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/View.xaml";
    let source = |attribute: &str| {
        format!(
            r#"<ContentControl xmlns="https://github.com/ferroui" {attribute}><Border/></ContentControl>"#
        )
    };
    let initial = source(r#"Width="{Binding Width, Mode=TwoWay}""#);
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, &initial)).unwrap();
    let model = Border::new();
    model.set_width(10.0);
    model.set_height(20.0);
    let context: BoxedValue = Rc::new(model.clone());
    root.set_data_context(Some(context.clone()));
    let _style =
        root.set_value_with_priority(Layoutable::width_property(), 5.0, BindingPriority::Style);
    let target = component_target(&root, uri, &initial);
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), initial.clone())
        .unwrap();
    session.register_target(&target);
    let original =
        BindingOperations::get_binding_expression_base(&root, Layoutable::width_property())
            .unwrap();
    let original_weak = Rc::downgrade(&original);
    let content = root.content().unwrap();
    let edited = source(r#"Width="{Binding Height, Mode=TwoWay}""#);
    session.submit(uri.into(), edited.clone());
    let result = session
        .apply_pending_with(|| {
            assert_eq!(root.width(), 20.0);
            model.set_width(11.0);
            assert_eq!(root.width(), 20.0);
            Err(ReloadError::Failed("later target rejected commit".into()))
        })
        .unwrap();
    assert_eq!(result.status, ReloadStatus::Failed, "{result:?}");
    assert_eq!(root.width(), 11.0);
    assert!(Rc::ptr_eq(&content, &root.content().unwrap()));
    assert!(Rc::ptr_eq(
        &original,
        &BindingOperations::get_binding_expression_base(&root, Layoutable::width_property())
            .unwrap()
    ));
    drop(content);
    drop(original);
    session.submit(uri.into(), edited);
    let result = session.apply_pending().unwrap();
    assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
    assert!(original_weak.upgrade().is_none());
    assert_eq!(root.width(), 20.0);
    assert!(Rc::ptr_eq(&root.data_context().unwrap(), &context));
    root.set_current_value(Layoutable::width_property(), 40.0);
    assert_eq!(model.height(), 40.0);
    session.submit(uri.into(), source(r#"Width="30""#));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(root.width(), 30.0);
    assert_eq!(
        model.height(),
        40.0,
        "literal assignment wrote to retired model binding"
    );
    session.submit(uri.into(), source(""));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(root.width(), 5.0);
    session.submit(uri.into(), initial.clone());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(root.width(), 11.0);
    root.set_width(11.0);
    session.submit(uri.into(), source(r#"Width="{Binding Height}""#));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.width(), 11.0);
}

#[test]
fn root_name_bindings_follow_fresh_scopes_without_retaining_detached_children() {
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/Names.xaml";
    let source = |width| {
        format!(
            r##"<ContentControl xmlns="https://github.com/ferroui" Width="{{Binding #Part.Width}}"><Border Name="Part" Width="{width}"/></ContentControl>"##
        )
    };
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, &source(10))).unwrap();
    assert_eq!(root.width(), 10.0);
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), source(10)).unwrap();
    let target = component_target(&root, uri, &source(10));
    session.register_target(&target);
    for width in 20..25 {
        let old = Ref::<Border>::from_xaml_object(&root.content().unwrap()).unwrap();
        let old_weak = old.downgrade();
        session.submit(uri.into(), source(width));
        let outcome = session
            .apply_pending_with(|| Err(ReloadError::Failed("rollback".into())))
            .unwrap();
        assert_eq!(outcome.status, ReloadStatus::Failed, "{outcome:?}");
        assert!(old.ptr_eq(&Ref::<Border>::from_xaml_object(&root.content().unwrap()).unwrap()));
        old.set_width(17.0);
        assert_eq!(root.width(), 17.0);
        session.submit(uri.into(), source(width));
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        assert_eq!(root.width(), width as f64);
        old.set_width(999.0);
        assert_eq!(
            root.width(),
            width as f64,
            "old namescope still drives root"
        );
        drop(old);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            old_weak.upgrade().is_none(),
            "captured descriptor retained old child"
        );
    }
}

#[test]
fn panel_compiled_root_bindings_preserve_application_children() {
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/Panel.xaml";
    let source = |path| {
        format!(
            r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:DataType="Border" Spacing="{{CompiledBinding {path}}}"><Border/></StackPanel>"#
        )
    };
    let root = Ref::<StackPanel>::from_xaml_object(&load(uri, &source("Width"))).unwrap();
    let model = Border::new();
    model.set_width(10.0);
    model.set_height(20.0);
    root.set_data_context(Some(Rc::new(model.clone())));
    assert_eq!(root.spacing(), 10.0);
    let application_child = Border::new();
    root.children().add(
        application_child
            .clone()
            .upcast::<ferroui_controls::Control>(),
    );
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source("Width"))
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        PanelReloadTarget::new(
            &root.clone().upcast::<Panel>(),
            0..1,
            uri.into(),
            source("Width"),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap()
        .with_owned_root_binding_property(StackPanel::spacing_property()),
    );
    session.register_target(&target);
    session.submit(uri.into(), source("Height"));
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    assert_eq!(root.spacing(), 20.0);
    assert!(root.children().get(1).ptr_eq(&application_child));
    model.set_height(30.0);
    assert_eq!(root.spacing(), 30.0);
    model.set_width(99.0);
    assert_eq!(root.spacing(), 30.0);
}

#[test]
fn root_binding_admission_precedes_source_activation() {
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application,
    };
    let _application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let model = Border::new();
    model.set_width(17.0);
    Application::current()
        .unwrap()
        .resources()
        .add_value("VM", model.clone());
    let uri = "ferres://root-binding/Admission.xaml";
    let initial = r#"<ContentControl xmlns="https://github.com/ferroui" Width="10"><Border/></ContentControl>"#;
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, initial)).unwrap();
    let content = root.content();
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), initial.into())
        .unwrap();
    let target = component_target(&root, uri, initial);
    session.register_target(&target);
    for mode in ["OneWayToSource", "OneTime"] {
        let source = initial.replace(
            "Width=\"10\"",
            &format!(r#"Width="{{Binding Width, Source={{StaticResource VM}}, Mode={mode}}}""#),
        );
        session.submit(uri.into(), source);
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::RestartRequired, "{outcome:?}");
        assert_eq!(
            model.width(),
            17.0,
            "candidate binding wrote before admission"
        );
        assert_eq!(root.width(), 10.0);
        assert_eq!(root.content(), content);
    }
    for bad in [
        r#"Width="not a number""#,
        r#"Width="{Binding Width, Source={StaticResource Missing}}""#,
    ] {
        session.submit(uri.into(), initial.replace("Width=\"10\"", bad));
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Failed, "{outcome:?}");
        assert_eq!(root.width(), 10.0);
        assert_eq!(model.width(), 17.0);
        assert_eq!(root.content(), content);
    }
}

#[test]
fn root_binding_capture_scopes_restore_after_unwind_and_do_not_affect_normal_loads() {
    use ferroui_markup_xaml_loader::binding_capture::BindingCaptureScope;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/Capture.xaml";
    let source = r#"<ContentControl xmlns="https://github.com/ferroui" Height="30" Width="{Binding Height, RelativeSource={RelativeSource Self}}"/>"#;
    let outer = BindingCaptureScope::new(uri.into(), [Layoutable::width_property().as_property()]);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _inner = BindingCaptureScope::new("ferres://root-binding/Other.xaml".into(), []);
        panic!("failed nested loader");
    }))
    .is_err());
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, source)).unwrap();
    let weak = root.downgrade();
    assert!(
        root.width().is_nan(),
        "capture activated the temporary root binding"
    );
    assert!(outer
        .binding(&root.clone().upcast(), Layoutable::width_property())
        .is_some());
    drop(root);
    drop(outer);
    assert!(weak.upgrade().is_none());
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, source)).unwrap();
    assert_eq!(
        root.width(),
        30.0,
        "capture leaked into ordinary runtime loading"
    );
}

#[test]
fn root_self_bindings_use_each_preserved_instance() {
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/Instances.xaml";
    let initial = r#"<ContentControl xmlns="https://github.com/ferroui"><Border Width="1"/></ContentControl>"#;
    let roots: Vec<_> = (0..2)
        .map(|_| Ref::<ContentControl>::from_xaml_object(&load(uri, initial)).unwrap())
        .collect();
    roots[0].set_height(10.0);
    roots[1].set_height(20.0);
    let targets: Vec<_> = roots
        .iter()
        .map(|root| component_target(root, uri, initial))
        .collect();
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), initial.into())
        .unwrap();
    for target in &targets {
        session.register_target(target);
    }
    let edited = initial.replace(
        "<ContentControl ",
        r#"<ContentControl Width="{Binding Height, RelativeSource={RelativeSource Self}}" "#,
    );
    session.submit(uri.into(), edited);
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    assert_eq!(roots[0].width(), 10.0);
    assert_eq!(roots[1].width(), 20.0);
    roots[0].set_height(30.0);
    assert_eq!(roots[0].width(), 30.0);
    assert_eq!(roots[1].width(), 20.0);
}

#[test]
fn unchanged_context_bindings_keep_expression_and_user_values_on_child_edits() {
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    for kind in ["Binding", "CompiledBinding"] {
        let uri = format!("ferres://root-binding/{kind}.xaml");
        let initial = format!(
            r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:DataType="Border" Width="{{{kind} Width, Mode=TwoWay}}"><Border Height="1"/></ContentControl>"#
        );
        let root = Ref::<ContentControl>::from_xaml_object(&load(&uri, &initial)).unwrap();
        let model = Border::new();
        model.set_width(10.0);
        root.set_data_context(Some(Rc::new(model.clone())));
        let target = component_target(&root, &uri, &initial);
        let mut session = HotReloadSession::new();
        session
            .register_document(uri.clone(), initial.clone())
            .unwrap();
        session.register_target(&target);
        let expression =
            BindingOperations::get_binding_expression_base(&root, Layoutable::width_property())
                .unwrap();
        root.set_current_value(Layoutable::width_property(), 40.0);
        assert_eq!(model.width(), 40.0);
        session.submit(uri.clone(), initial.replace("Height=\"1\"", "Height=\"2\""));
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        assert_eq!(root.width(), 40.0);
        assert!(
            Rc::ptr_eq(
                &expression,
                &BindingOperations::get_binding_expression_base(
                    &root,
                    Layoutable::width_property()
                )
                .unwrap()
            ),
            "unchanged data-context binding was needlessly recreated"
        );
    }
}

#[test]
fn root_binding_resource_options_join_the_resource_revision() {
    use ferroui_base::controls::{IResourceProvider, ResourceDictionary};
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application,
    };
    let _application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/ResourcesView.xaml";
    let resource_uri = "ferres://root-binding/Values.xaml";
    let values = |number| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="Fallback">{number}</x:Double></ResourceDictionary>"#
        )
    };
    let resources = Application::current().unwrap().resources();
    let dictionary =
        Ref::<ResourceDictionary>::from_xaml_object(&load(resource_uri, &values(10))).unwrap();
    resources.add_merged_dictionary(Rc::<dyn IResourceProvider>::from(dictionary));
    let source = r#"<ContentControl xmlns="https://github.com/ferroui" Width="{Binding Missing, FallbackValue={StaticResource Fallback}}"><Border/></ContentControl>"#;
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, source)).unwrap();
    assert_eq!(root.width(), 10.0);
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root,
            uri.into(),
            source.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_root_binding_property(Layoutable::width_property())
        .with_dependencies([resource_uri.into()]),
    );
    let owner: Rc<dyn ReloadTarget> = Rc::new(
        ResourceReloadTarget::merged(
            &resources,
            0,
            resource_uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source.into())
        .unwrap();
    session
        .register_document(resource_uri.into(), values(10))
        .unwrap();
    session.register_target(&target);
    session.register_target(&owner);
    let content = root.content();
    session.submit(resource_uri.into(), values(20));
    let outcome = session
        .apply_pending_with(|| {
            assert_eq!(root.width(), 20.0);
            Err(ReloadError::Failed("reject resource revision".into()))
        })
        .unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed, "{outcome:?}");
    assert_eq!(root.width(), 10.0);
    assert_eq!(root.content(), content);
    session.submit(resource_uri.into(), values(20));
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    assert_eq!(
        root.width(),
        20.0,
        "unchanged binding retained an old static fallback"
    );
    session.submit(
        resource_uri.into(),
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"/>"#.into(),
    );
    let content = root.content();
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed, "{outcome:?}");
    assert_eq!(root.width(), 20.0);
    assert_eq!(root.content(), content);
}

#[test]
fn resource_supplied_binding_path_is_not_treated_as_unchanged_context_binding() {
    use ferroui_base::controls::{IResourceProvider, ResourceDictionary};
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application,
    };
    let _application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/ResourcePath.xaml";
    let resource_uri = "ferres://root-binding/Paths.xaml";
    let values = |path| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:String x:Key="PathName">{path}</x:String></ResourceDictionary>"#
        )
    };
    let resources = Application::current().unwrap().resources();
    let dictionary =
        Ref::<ResourceDictionary>::from_xaml_object(&load(resource_uri, &values("Width"))).unwrap();
    resources.add_merged_dictionary(Rc::<dyn IResourceProvider>::from(dictionary));
    let source = r#"<ContentControl xmlns="https://github.com/ferroui" Width="{Binding Path={StaticResource PathName}}"><Border/></ContentControl>"#;
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, source)).unwrap();
    let model = Border::new();
    model.set_width(10.0);
    model.set_height(20.0);
    root.set_data_context(Some(Rc::new(model.clone())));
    assert_eq!(root.width(), 10.0);
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root,
            uri.into(),
            source.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_root_binding_property(Layoutable::width_property())
        .with_dependencies([resource_uri.into()]),
    );
    let owner: Rc<dyn ReloadTarget> = Rc::new(
        ResourceReloadTarget::merged(
            &resources,
            0,
            resource_uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source.into())
        .unwrap();
    session
        .register_document(resource_uri.into(), values("Width"))
        .unwrap();
    session.register_target(&target);
    session.register_target(&owner);
    session.submit(resource_uri.into(), values("Height"));
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    assert_eq!(
        root.width(),
        20.0,
        "resource-selected path retained the old expression"
    );
}

#[test]
fn failed_binding_commit_does_not_adopt_an_application_assignment() {
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://root-binding/Ownership.xaml";
    let initial = r#"<ContentControl xmlns="https://github.com/ferroui" Width="10"><Border/></ContentControl>"#;
    let root = Ref::<ContentControl>::from_xaml_object(&load(uri, initial)).unwrap();
    let target = component_target(&root, uri, initial);
    let sources = |width| {
        Rc::new(
            [(
                uri.to_owned(),
                initial.replace("Width=\"10\"", &format!("Width=\"{width}\"")),
            )]
            .into_iter()
            .collect(),
        )
    };
    let mut change = target.prepare(sources(20)).unwrap();
    root.set_width(10.0);
    assert!(matches!(
        change.commit(),
        Err(ReloadError::RestartRequired(_))
    ));
    change.rollback();
    assert_eq!(root.width(), 10.0);
    assert!(
        matches!(
            target.prepare(sources(30)),
            Err(ReloadError::RestartRequired(_))
        ),
        "rollback adopted an intervening application assignment as XAML-owned"
    );
}
