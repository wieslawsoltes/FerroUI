use super::*;
use std::cell::{Cell, RefCell};

#[test]
fn uncommon_handler_contract_edits_restart_before_preparing_targets() {
    ferroui_controls::register_types();
    let uri = "ferres://test/View.xaml";
    let source = r#"<Button xmlns="https://github.com/ferroui" PointerMoved="Before"/>"#;
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source.into())
        .unwrap();
    let prepares = Rc::new(Cell::new(0));
    let live: Rc<dyn ReloadTarget> = Rc::new(Target {
        value: Rc::new(RefCell::new("before".into())),
        fail: Rc::new(Cell::new(false)),
        prepares: prepares.clone(),
    });
    session.register_target(&live);
    for changed in [
        source.replace("Before", "After"),
        source.replace(" PointerMoved=\"Before\"", ""),
    ] {
        session.submit(uri.into(), changed);
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::RestartRequired
        );
        assert_eq!(prepares.get(), 0);
    }
}

struct Target {
    value: Rc<RefCell<String>>,
    fail: Rc<Cell<bool>>,
    prepares: Rc<Cell<usize>>,
}

#[test]
fn watcher_revision_changed_during_preparation_discards_candidate_before_commit() {
    use std::time::{Duration, Instant};
    struct SupersedingTarget {
        inner: Target,
        path: std::path::PathBuf,
        revision: WatchRevision,
        supersede: Cell<bool>,
    }
    impl ReloadTarget for SupersedingTarget {
        fn documents(&self) -> Vec<String> {
            self.inner.documents()
        }
        fn prepare(&self, sources: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
            let candidate = self.inner.prepare(sources)?;
            if self.supersede.replace(false) {
                std::fs::write(&self.path, "<View Width=\"3\"/>").unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while self.revision.is_current() {
                    assert!(
                        Instant::now() < deadline,
                        "watcher did not observe newer source"
                    );
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            Ok(candidate)
        }
    }
    let directory = std::env::temp_dir().join(format!(
        "ferroui-superseded-preparation-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("View.xaml");
    let uri = "ferres://test/View.xaml";
    std::fs::write(&path, "<View/>").unwrap();
    let watcher = FileWatcher::with_baseline(
        BTreeMap::from([(uri.into(), path.clone())]),
        Duration::from_millis(10),
        BTreeMap::from([(uri.into(), "<View/>".into())]),
    );
    std::fs::write(&path, "<View Width=\"2\"/>").unwrap();
    let batch = watcher
        .changes
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let revision = batch.revision.clone().unwrap();
    let value = Rc::new(RefCell::new("before".into()));
    let prepares = Rc::new(Cell::new(0));
    let target: Rc<dyn ReloadTarget> = Rc::new(SupersedingTarget {
        inner: Target {
            value: value.clone(),
            fail: Rc::new(Cell::new(false)),
            prepares: prepares.clone(),
        },
        path: path.clone(),
        revision,
        supersede: Cell::new(true),
    });
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), "<View/>".into())
        .unwrap();
    session.register_target(&target);
    session.submit_files(batch);
    let finalized = Cell::new(false);
    let outcome = session
        .apply_pending_with(|| {
            finalized.set(true);
            Ok(())
        })
        .unwrap();
    assert_eq!(outcome.status, ReloadStatus::Superseded);
    assert_eq!(prepares.get(), 1);
    assert!(!finalized.get());
    assert_eq!(&*value.borrow(), "before");
    assert_eq!(session.sources()[uri], "<View/>");
    assert_eq!(session.applied_revision(), 0);
    assert!(
        session.apply_pending().is_none(),
        "stale drafts must not spin"
    );

    session.submit_files(
        watcher
            .changes
            .recv_timeout(Duration::from_secs(5))
            .unwrap(),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(&*value.borrow(), "<View Width=\"3\"/>");
    assert!(!session.has_pending_changes());
    let applied_read = session.watch_revision.clone().unwrap();
    std::fs::write(&path, "<View Width=\"4\"/>").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while applied_read.is_current() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        session.has_pending_changes(),
        "observed reads await debounce"
    );
    assert!(session
        .apply_pending_with(|| {
            panic!("must not finalize code while a newer read awaits admission")
        })
        .is_none());
    drop(watcher);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn watcher_reissues_a_current_revision_after_an_observed_edit_is_reverted() {
    use std::time::{Duration, Instant};
    let directory = std::env::temp_dir().join(format!(
        "ferroui-reverted-watch-revision-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("View.xaml");
    let uri = "ferres://test/View.xaml";
    std::fs::write(&path, "<View/>").unwrap();
    let watcher = FileWatcher::with_baseline(
        BTreeMap::from([(uri.into(), path.clone())]),
        Duration::from_millis(50),
        BTreeMap::from([(uri.into(), "<View/>".into())]),
    );
    let accepted_text = "<View Width=\"2\"/>";
    std::fs::write(&path, accepted_text).unwrap();
    let batch = watcher
        .changes
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let revision = batch.revision.clone().unwrap();
    std::fs::write(&path, "<").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while revision.is_current() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    std::fs::write(&path, accepted_text).unwrap();
    let mut session = session();
    let (_target, value) = target(&mut session, false);
    session.submit_files(batch);
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Superseded
    );
    assert_eq!(&*value.borrow(), "before");
    // Even if the partial edit never survives debounce, the previously delivered
    // token is stale. Returning to its text must deliver a fresh usable token.
    loop {
        assert!(Instant::now() < deadline);
        let batch = watcher
            .changes
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        if batch
            .files
            .iter()
            .any(|file| file.contents.as_deref() == Ok(accepted_text))
            && batch.revision.as_ref().unwrap().is_current()
        {
            session.submit_files(batch);
            break;
        }
    }
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(&*value.borrow(), accepted_text);
    drop(watcher);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn builder_reapplication_retains_drafts_and_read_errors() {
    let mut session = session();
    let (_target, value) = target(&mut session, false);
    let uri = "ferres://test/View.xaml";
    session.submit(uri.into(), "<View Width=\"23\"/>".into());
    session.reapply_document(uri).unwrap();
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(&*value.borrow(), "<View Width=\"23\"/>");
    let before = session.applied_revision();
    session.reapply_document(uri).unwrap();
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert!(session.applied_revision() > before);

    session.submit_files(FileBatch {
        revision: None,
        files: vec![FileChange {
            uri: uri.into(),
            contents: Err("unreadable".into()),
        }],
        diagnostics: Vec::new(),
    });
    session.reapply_document(uri).unwrap();
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(&*value.borrow(), "<View Width=\"23\"/>");
    assert!(matches!(
        session.reapply_document("ferres://test/Missing.xaml"),
        Err(ReloadError::RestartRequired(_))
    ));
}

#[test]
fn text_snapshots_skip_accepted_sources_and_preserve_other_failed_drafts() {
    let mut session = session();
    let (_target, value) = target(&mut session, false);
    let view = "ferres://test/View.xaml";
    let other = "ferres://test/Other.xaml";
    session
        .register_document(other.into(), "<Other/>".into())
        .unwrap();
    let accepted = session.sources();
    let revision = session.applied_revision();
    session.submit_texts((*accepted).clone());
    assert!(!session.has_pending_changes());
    assert!(session.apply_pending().is_none());
    assert_eq!(session.applied_revision(), revision);
    assert_eq!(&*value.borrow(), "before");

    session.submit_texts([(view.into(), "<".into()), (other.into(), "<".into())]);
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    // Reverting one document must not discard its neighbor's invalid draft.
    session.submit_texts([(view.into(), accepted[view].clone())]);
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(&*value.borrow(), "before");
    session.submit_texts([(other.into(), accepted[other].clone())]);
    assert!(!session.has_pending_changes());
    assert!(session.apply_pending().is_none());

    session.submit_texts([(view.into(), "<View Width=\"17\"/>".into())]);
    let failed = session
        .apply_pending_with(|| Err(ReloadError::Failed("patch failed".into())))
        .unwrap();
    assert_eq!(failed.status, ReloadStatus::Failed);
    assert_eq!(&*value.borrow(), "before");
    session.submit_texts([(view.into(), "<View Width=\"17\"/>".into())]);
    let applied = session.apply_pending().unwrap();
    assert_eq!(applied.status, ReloadStatus::Applied);
    assert!(applied.revision > failed.revision);
    assert_eq!(&*value.borrow(), "<View Width=\"17\"/>");
}

#[test]
fn resource_owner_reload_notifies_dynamic_consumers_and_rolls_back_ownership() {
    use ferroui_base::{
        controls::{IResourceProvider, ResourceDictionary},
        threading::Dispatcher,
        Ref,
    };
    use ferroui_controls::Border;
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let load = |source: &str| {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(None, source),
            None,
        )
        .unwrap()
    };
    let view =
        load(r#"<Border xmlns="https://github.com/ferroui" Width="{DynamicResource width}"/>"#);
    let root = Ref::<Border>::from_xaml_object(&view).unwrap();
    let resource_source = |value: i32| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">{value}</x:Double></ResourceDictionary>"#
        )
    };
    let container = root.resources();
    container.add_value("application", 42_i32);
    let application = container.get(&"application".into());
    let other = ResourceDictionary::new();
    other.add_value("untouched", 7_i32);
    let other_provider: Rc<dyn IResourceProvider> = other.into();
    container.add_merged_dictionary(other_provider.clone());
    let initial = Ref::<ResourceDictionary>::from_xaml_object(&load(&resource_source(12))).unwrap();
    container.add_merged_dictionary(Rc::<dyn IResourceProvider>::from(initial));
    assert_eq!(root.width(), 12.0);
    let uri = "ferres://resources/Values.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), resource_source(12))
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ResourceReloadTarget::merged(
            &container,
            1,
            uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&target);
    for number in 20..30 {
        let old = container.merged_dictionaries().get(1);
        let weak = Rc::downgrade(&old);
        session.submit(uri.into(), resource_source(number));
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        assert_eq!(root.width(), f64::from(number));
        assert!(old.owner().is_none());
        let new = container.merged_dictionaries().get(1);
        assert!(new.owner() == Some(root.clone().into()));
        drop(old);
        assert!(
            weak.upgrade().is_none(),
            "detached resource provider retained"
        );
        assert_eq!(container.get(&"application".into()), application);
        assert!(ferroui_base::controls::resource_provider_ptr_eq(
            &container.merged_dictionaries().get(0),
            &other_provider
        ));
    }
    let old = container.merged_dictionaries().get(1);
    let accepted = session.sources();
    session.submit(uri.into(), resource_source(99));
    let outcome = session
        .apply_pending_with(|| Err(ReloadError::Failed("patch rejected".into())))
        .unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed);
    assert_eq!(root.width(), 29.0);
    assert!(old.owner() == Some(root.clone().into()));
    assert!(ferroui_base::controls::resource_provider_ptr_eq(
        &container.merged_dictionaries().get(1),
        &old
    ));
    assert_eq!(session.sources(), accepted);
    session.submit(
        uri.into(),
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"/>"#.into(),
    );
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    assert!(
        root.width().is_nan(),
        "removed resource must not leave its old dynamic value"
    );
    assert_eq!(container.get(&"application".into()), application);
}

#[test]
fn forward_static_resource_is_validated_after_candidate_construction() {
    use ferroui_base::{threading::Dispatcher, Ref};
    use ferroui_controls::{Border, ContentControl};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://forward/View.xaml";
    let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Width="{StaticResource width}"><Border.Resources><x:Double x:Key="width">42</x:Double></Border.Resources></Border>"#;
    let host = ContentControl::new();
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source.into())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        uri.into(),
        vec![],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&target);
    session.submit(uri.into(), source.into());
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    let child = Ref::<Border>::from_xaml_object(&host.content().unwrap()).unwrap();
    child.initialize_if_needed();
    assert_eq!(child.width(), 42.0);
}

#[test]
fn resource_owner_and_static_consumer_accept_or_reject_one_dependency_revision() {
    use ferroui_base::{
        controls::{IResourceProvider, ResourceDictionary},
        threading::Dispatcher,
        Ref,
    };
    use ferroui_controls::{Border, ContentControl};
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let load = |source: &str| {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(None, source),
            None,
        )
        .unwrap()
    };
    let dynamic = Ref::<Border>::from_xaml_object(&load(
        r#"<Border xmlns="https://github.com/ferroui" Width="{DynamicResource width}"/>"#,
    ))
    .unwrap();
    let values = |number| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">{number}</x:Double></ResourceDictionary>"#
        )
    };
    let initial = Ref::<ResourceDictionary>::from_xaml_object(&load(&values(7))).unwrap();
    dynamic
        .resources()
        .add_merged_dictionary(Rc::<dyn IResourceProvider>::from(initial));
    let view = r#"<Border xmlns="https://github.com/ferroui"><Border.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="Values.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Border.Resources><Border Width="{StaticResource width}"/></Border>"#;
    let mut session = HotReloadSession::new();
    let resource_uri = "ferres://owners/Values.xaml";
    let view_uri = "ferres://owners/View.xaml";
    session
        .register_document(resource_uri.into(), values(7))
        .unwrap();
    session
        .register_document(view_uri.into(), view.into())
        .unwrap();
    let owner: Rc<dyn ReloadTarget> = Rc::new(
        ResourceReloadTarget::merged(
            &dynamic.resources(),
            0,
            resource_uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    let host = ContentControl::new();
    let consumer: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        view_uri.into(),
        vec![resource_uri.into()],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&owner);
    session.register_target(&consumer);
    session.submit(view_uri.into(), view.into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    session.submit(resource_uri.into(), values(8));
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    let outer = Ref::<Border>::from_xaml_object(&host.content().unwrap()).unwrap();
    let child = outer.child().unwrap().cast::<Border>().unwrap();
    assert_eq!(child.width(), 8.0);
    assert_eq!(dynamic.width(), 8.0);
    let old_content = host.content();
    let accepted = session.sources();
    session.submit(
        resource_uri.into(),
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"/>"#.into(),
    );
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed, "{outcome:?}");
    assert_eq!(host.content(), old_content);
    assert_eq!(dynamic.width(), 8.0);
    assert_eq!(session.sources(), accepted);
}

#[test]
fn panel_reload_retains_root_and_application_children_and_rolls_back() {
    use ferroui_base::{controls::NameScope, threading::Dispatcher, Ref};
    use ferroui_controls::{Border, Panel, StackPanel, TextBox};
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let source = r#"<StackPanel xmlns="https://github.com/ferroui" Spacing="7"><TextBox Name="edit"/></StackPanel>"#;
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_base_uri(None, source),
        None,
    )
    .unwrap();
    let root = Ref::<StackPanel>::from_xaml_object(&loaded).unwrap();
    let panel: Ref<Panel> = root.clone().upcast();
    let header = Border::new();
    let footer = Border::new();
    root.children().insert(0, header.clone());
    root.children().add(footer.clone());
    root.set_spacing(88.0); // An application assignment on the preserved root.
    let model: ferroui_base::BoxedValue = Rc::new(42_i32);
    root.set_data_context(Some(model.clone()));
    let uri = "ferres://panel/View.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source.into())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        PanelReloadTarget::new(
            &panel,
            1..2,
            uri.into(),
            source.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&target);
    for index in 0..10 {
        let old = root.children().get(1).cast::<TextBox>().unwrap();
        old.set_text(Some("unsaved edit"));
        let weak = old.downgrade();
        drop(old);
        let mut changed = source.replace("<TextBox", &format!("<TextBox Width=\"{}\"", index + 30));
        if index % 2 == 0 {
            // The inherited property owner is Panel, even though the component
            // is a StackPanel. Switching syntax must preserve the same ownership.
            changed = changed
                .replace("<TextBox", "<Panel.Children><TextBox")
                .replace("/></StackPanel>", "/></Panel.Children></StackPanel>");
        }
        session.submit(uri.into(), changed);
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Applied
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none(), "detached owned child retained");
        assert_eq!(
            root.children().get(0),
            header.clone().upcast::<ferroui_controls::Control>()
        );
        assert_eq!(
            root.children().get(2),
            footer.clone().upcast::<ferroui_controls::Control>()
        );
        let edit = root.children().get(1).cast::<TextBox>().unwrap();
        assert_eq!(edit.text().as_deref(), Some("unsaved edit"));
        assert_eq!(edit.width(), f64::from(index + 30));
        assert_eq!(root.spacing(), 88.0);
        assert_eq!(root.data_context(), Some(model.clone()));
        assert_eq!(
            NameScope::get_name_scope(&root).unwrap().find("edit"),
            Some(edit.upcast())
        );
    }
    let children = root.children().to_vec();
    let scope = NameScope::get_name_scope(&root);
    let accepted = session.sources();
    session.submit(
        uri.into(),
        source.replace("<TextBox", "<TextBox Width=\"99\""),
    );
    assert_eq!(
        session
            .apply_pending_with(|| Err(ReloadError::Failed("patch failed".into())))
            .unwrap()
            .status,
        ReloadStatus::Failed
    );
    assert_eq!(root.children().to_vec(), children);
    assert!(NameScope::get_name_scope(&root) == scope);
    assert_eq!(session.sources(), accepted);
}

#[test]
fn panel_reload_rejects_externally_interleaved_owned_children() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{Border, Panel};
    use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
    let _scope = Dispatcher::unit_test_scope();
    let root = Panel::new();
    root.children().add(Border::new());
    root.children().add(Border::new());
    let source = r#"<Panel xmlns="https://github.com/ferroui"><Border/><Border/></Panel>"#;
    let uri = "ferres://panel/Interleaved.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), source.into())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        PanelReloadTarget::new(
            &root,
            0..2,
            uri.into(),
            source.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&target);
    root.children().insert(1, Border::new());
    let before = root.children().to_vec();
    session.submit(
        uri.into(),
        source.replace("<Border/>", "<Border Width=\"2\"/>"),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.children().to_vec(), before);
}

#[test]
fn panel_owned_range_can_shrink_and_grow_without_removing_application_children() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{Border, Control, Panel};
    use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let root = Panel::new();
    let header = Border::new();
    let footer = Border::new();
    root.children().add(header.clone());
    root.children().add(Border::new());
    root.children().add(footer.clone());
    let source =
        |children: &str| format!(r#"<Panel xmlns="https://github.com/ferroui">{children}</Panel>"#);
    let initial = source("<Border/>");
    let uri = "ferres://panel/Range.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), initial.clone())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        PanelReloadTarget::new(
            &root,
            1..2,
            uri.into(),
            initial,
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&target);
    for (text, count) in [("", 0), ("<Border/><Border/>", 2), ("<Border/>", 1)] {
        session.submit(uri.into(), source(text));
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(root.children().count(), count + 2);
        assert_eq!(root.children().get(0), header.clone().upcast::<Control>());
        assert_eq!(
            root.children().get(count + 1),
            footer.clone().upcast::<Control>()
        );
    }
    let before = root.children().to_vec();
    session.submit(
        uri.into(),
        source("<Border/>").replace("<Panel ", "<Panel Background=\"Red\" "),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.children().to_vec(), before);
}

#[test]
fn external_commit_failure_rolls_back_markup_and_does_not_publish_sources() {
    let mut session = session();
    let (_target, value) = target(&mut session, false);
    let before = session.sources();
    submit(&mut session, "<Border Width=\"91\"/>");
    let outcome = session
        .apply_pending_with(|| {
            assert_eq!(&*value.borrow(), "<Border Width=\"91\"/>");
            Err(ReloadError::Failed("patch could not load".into()))
        })
        .unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed);
    assert_eq!(&*value.borrow(), "before");
    assert_eq!(session.sources(), before);
    assert_eq!(session.applied_revision(), 0);
    let invoked = Cell::new(false);
    assert!(session
        .apply_pending_with(|| {
            invoked.set(true);
            Ok(())
        })
        .is_none());
    assert!(!invoked.get());
    submit(&mut session, "<Border Width=\"91\"/>");
    let applied = session
        .apply_pending_with(|| {
            invoked.set(true);
            Ok(())
        })
        .unwrap();
    assert_eq!(applied.status, ReloadStatus::Applied);
    assert!(invoked.get());
    assert_eq!(
        session.sources()["ferres://test/View.xaml"],
        "<Border Width=\"91\"/>"
    );
}

#[test]
fn invalid_markup_or_failed_adapter_never_installs_external_update() {
    for (source, fail) in [("<", false), ("<Border Width=\"2\"/>", true)] {
        let mut session = session();
        let (_target, value) = target(&mut session, fail);
        submit(&mut session, source);
        let invoked = Cell::new(false);
        let outcome = session
            .apply_pending_with(|| {
                invoked.set(true);
                Ok(())
            })
            .unwrap();
        assert_eq!(outcome.status, ReloadStatus::Failed);
        assert!(!invoked.get());
        assert_eq!(&*value.borrow(), "before");
    }
}

#[test]
fn external_commit_panic_restores_all_target_graphs() {
    let mut session = session();
    let (_one, first) = target(&mut session, false);
    let (_two, second) = target(&mut session, false);
    submit(&mut session, "<Border Width=\"5\"/>");
    let outcome = session
        .apply_pending_with(|| panic!("before external state changes"))
        .unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed);
    assert_eq!(&*first.borrow(), "before");
    assert_eq!(&*second.borrow(), "before");
}

#[test]
fn manifest_directory_watcher_discovers_nested_files_and_survives_directory_replacement() {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let directory = std::env::temp_dir().join(format!(
        "ferroui-discovery-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(directory.join("Includes")).unwrap();
    let original = directory.join("View.xaml");
    std::fs::write(&original, "<Border/>").unwrap();
    let manifest = ReloadManifest::parse(
        &serde_json::json!({
            "version": 1, "documents": [{ "uri": "ferres://Example/Views/View.xaml",
                "source_path": original, "source": "<Border/>", "includes": [] }]
        })
        .to_string(),
    )
    .unwrap();
    let watcher = manifest.watcher(Duration::from_millis(10));
    std::fs::write(
        directory.join("Includes/New Theme.xaml"),
        "<ResourceDictionary/>",
    )
    .unwrap();
    let batch = watcher
        .changes
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    assert!(batch.diagnostics.is_empty());
    assert_eq!(batch.files.len(), 2);
    let discovered = batch
        .files
        .iter()
        .find(|file| file.uri.contains("New%20Theme"))
        .unwrap();
    assert_eq!(
        discovered.uri,
        "ferres://example/Views/Includes/New%20Theme.xaml"
    );
    assert_eq!(
        discovered.contents.as_deref().unwrap(),
        "<ResourceDictionary/>"
    );
    let mut session = manifest.session().unwrap();
    session.submit_files(batch);
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    let moved = directory.with_extension("moved");
    std::fs::rename(&directory, &moved).unwrap();
    let failure = watcher
        .changes
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    assert!(!failure.diagnostics.is_empty());
    assert!(failure.files.iter().all(|file| file.contents.is_err()));
    std::fs::rename(&moved, &directory).unwrap();
    let recovered = watcher
        .changes
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    assert!(recovered.diagnostics.is_empty());
    assert!(recovered.files.iter().all(|file| file.contents.is_ok()));
    drop(watcher);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn directory_scan_failure_blocks_pending_edits_until_recovery() {
    let mut session = session();
    let (_target, value) = target(&mut session, false);
    session.submit(
        "ferres://test/View.xaml".into(),
        "<Border Width=\"2\"/>".into(),
    );
    session.submit_files(FileBatch {
        revision: None,
        files: Vec::new(),
        diagnostics: vec!["directory unavailable".into()],
    });
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(&*value.borrow(), "before");
    session.submit_files(FileBatch {
        revision: None,
        files: Vec::new(),
        diagnostics: Vec::new(),
    });
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(&*value.borrow(), "<Border Width=\"2\"/>");
}

#[test]
fn missing_file_blocks_entire_revision_and_recovery_keeps_neighbor_edits() {
    let mut session = session();
    session
        .register_document("ferres://test/Other.xaml".into(), "<Other/>".into())
        .unwrap();
    let (_target, value) = target(&mut session, false);
    let view = "ferres://test/View.xaml";
    let other = "ferres://test/Other.xaml";
    let batch = |other_contents| FileBatch {
        revision: None,
        diagnostics: Vec::new(),
        files: vec![
            FileChange {
                uri: view.into(),
                contents: Ok("<View Width=\"17\"/>".into()),
            },
            FileChange {
                uri: other.into(),
                contents: other_contents,
            },
        ],
    };
    let revision = session.submit_files(batch(Err("temporarily absent".into())));
    let failure = session.apply_pending().unwrap();
    assert_eq!(failure.revision, revision);
    assert_eq!(failure.status, ReloadStatus::Failed);
    assert_eq!(&*value.borrow(), "before");
    assert!(session.apply_pending().is_none());
    session.submit_files(batch(Ok("<Other/>".into())));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(&*value.borrow(), "<View Width=\"17\"/>");
    assert_eq!(session.sources()[view], "<View Width=\"17\"/>");
}

#[test]
fn watcher_delivers_complete_snapshots_including_missing_and_recovered_files() {
    use std::time::Duration;
    let directory = std::env::temp_dir().join(format!("ferroui-batch-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let a = directory.join("A.xaml");
    let b = directory.join("B.xaml");
    std::fs::write(&a, "new A").unwrap();
    let watcher = FileWatcher::with_baseline(
        BTreeMap::from([("a".into(), a), ("b".into(), b.clone())]),
        Duration::from_millis(10),
        BTreeMap::from([("a".into(), "old A".into()), ("b".into(), "old B".into())]),
    );
    let missing = watcher
        .changes
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    assert_eq!(missing.files.len(), 2);
    assert_eq!(missing.files[0].contents.as_deref().unwrap(), "new A");
    assert!(missing.files[1].contents.is_err());
    std::fs::write(&b, "new B").unwrap();
    let recovery = watcher
        .changes
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    assert_eq!(recovery.files.len(), 2);
    assert_eq!(recovery.files[0].contents.as_deref().unwrap(), "new A");
    assert_eq!(recovery.files[1].contents.as_deref().unwrap(), "new B");
    drop(watcher);
    std::fs::remove_dir_all(directory).unwrap();
}
struct Change {
    value: Rc<RefCell<String>>,
    before: String,
    after: String,
    fail: bool,
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        *self.value.borrow_mut() = self.after.clone();
        if self.fail {
            Err(ReloadError::Failed("injected commit failure".into()))
        } else {
            Ok(())
        }
    }
    fn rollback(&mut self) {
        *self.value.borrow_mut() = self.before.clone();
    }
}
impl ReloadTarget for Target {
    fn documents(&self) -> Vec<String> {
        vec![
            "ferres://test/View.xaml".into(),
            "ferres://test/Other.xaml".into(),
        ]
    }
    fn prepare(&self, sources: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
        self.prepares.set(self.prepares.get() + 1);
        Ok(Box::new(Change {
            value: self.value.clone(),
            before: self.value.borrow().clone(),
            after: sources["ferres://test/View.xaml"].clone(),
            fail: self.fail.get(),
        }))
    }
}
fn session() -> HotReloadSession {
    let mut session = HotReloadSession::new();
    session
        .register_document("ferres://test/View.xaml".into(), "<Border/>".into())
        .unwrap();
    session
}
fn target(
    session: &mut HotReloadSession,
    fail: bool,
) -> (Rc<dyn ReloadTarget>, Rc<RefCell<String>>) {
    let value = Rc::new(RefCell::new("before".into()));
    let target: Rc<dyn ReloadTarget> = Rc::new(Target {
        value: value.clone(),
        fail: Rc::new(Cell::new(fail)),
        prepares: Rc::new(Cell::new(0)),
    });
    session.register_target(&target);
    (target, value)
}
fn submit(session: &mut HotReloadSession, text: &str) {
    session.submit("ferres://test/View.xaml".into(), text.into());
}
#[test]
fn rolls_back_partial_commit_and_keeps_source() {
    let mut session = session();
    let (_a, first) = target(&mut session, false);
    let (_b, second) = target(&mut session, true);
    submit(&mut session, "<Border Width=\"2\"/>");
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(&*first.borrow(), "before");
    assert_eq!(&*second.borrow(), "before");
    assert_eq!(session.sources()["ferres://test/View.xaml"], "<Border/>");
    assert_eq!(session.applied_revision(), 0);
}
#[test]
fn invalid_markup_does_not_replace_last_good() {
    let mut session = session();
    let (_target, value) = target(&mut session, false);
    submit(&mut session, "<Border>");
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(&*value.borrow(), "before");
}
#[test]
fn coalesces_to_latest_text() {
    let mut session = session();
    let (_target, value) = target(&mut session, false);
    submit(&mut session, "<Border Width=\"1\"/>");
    submit(&mut session, "<Border Width=\"2\"/>");
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied);
    assert_eq!(outcome.revision, 2);
    assert!(value.borrow().contains("2"));
    assert!(session.apply_pending().is_none());
}
#[test]
fn changed_named_contract_requires_restart() {
    let mut session = session();
    submit(&mut session, "<Border Name=\"new\"/>");
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
}
#[test]
fn targets_are_weak() {
    let mut session = session();
    let (target, value) = target(&mut session, false);
    drop(target);
    submit(&mut session, "<Border Width=\"2\"/>");
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(&*value.borrow(), "before");
}
#[test]
fn scoped_sources_restore_after_panic() {
    let sources = Rc::new(BTreeMap::from([(
        "ferres://test/View.xaml".into(),
        "<Border/>".into(),
    )]));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_sources(sources, || panic!("test"))
    }));
    assert!(result.is_err());
    assert!(ferroui_markup_xaml::source_overlay::source("ferres://test/View.xaml").is_none());
}

#[test]
fn runtime_content_replacement_retains_host_and_last_valid_control() {
    use ferroui_base::{threading::Dispatcher, Ref};
    use ferroui_controls::{Border, ContentControl};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let host = ContentControl::new();
    let model = Rc::new(42_i32) as ferroui_base::BoxedValue;
    host.set_data_context(Some(model.clone()));
    let mut session = session();
    let target: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        "ferres://test/View.xaml".into(),
        vec![],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&target);
    submit(
        &mut session,
        "<Border xmlns=\"https://github.com/ferroui\" Width=\"42\"/>",
    );
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    let content = host.content().unwrap();
    let border = Ref::<Border>::from_xaml_object(&content).unwrap();
    assert_eq!(border.width(), 42.0);
    assert!(Rc::ptr_eq(&host.data_context().unwrap(), &model));
    submit(
        &mut session,
        "<Border xmlns=\"https://github.com/ferroui\" Width=\"invalid\"/>",
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert!(Rc::ptr_eq(&host.content().unwrap(), &content));
}

#[test]
fn runtime_text_cursor_survives_repeated_subtree_swaps_and_rollback() {
    use ferroui_base::{threading::Dispatcher, Ref};
    use ferroui_controls::{ContentControl, TextBox};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let source = r#"<TextBox xmlns="https://github.com/ferroui" Name="edit" Width="100"/>"#;
    let uri = "ferres://cursor/View.xaml";
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), source.into()).unwrap();
    let host = ContentControl::new();
    let target: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        uri.into(),
        vec![],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&target);
    session.submit(uri.into(), source.into());
    assert_eq!(session.apply_pending().unwrap().status, ReloadStatus::Applied);
    Dispatcher::ui_thread().run_jobs(None);
    for revision in 1..=6 {
        let old = Ref::<TextBox>::from_xaml_object(&host.content().unwrap()).unwrap();
        let (start, end) = if revision % 2 == 0 { (5, 2) } else { (2, 5) };
        old.set_text(Some("draft text"));
        old.set_caret_index(end);
        old.set_selection_start(start);
        old.set_selection_end(end);
        let retired = old.downgrade();
        let update = source.replace("100", &(100 + revision).to_string());
        session.submit(uri.into(), update.clone());
        let failed = session
            .apply_pending_with(|| {
                Err(ReloadError::Failed("later target rejects the revision".into()))
            })
            .unwrap();
        assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
        Dispatcher::ui_thread().run_jobs(None);
        let current = Ref::<TextBox>::from_xaml_object(&host.content().unwrap()).unwrap();
        assert!(current.ptr_eq(&old));
        assert_eq!(
            (current.selection_start(), current.selection_end(), current.caret_index()),
            (start, end, end),
        );
        drop(current);
        session.submit(uri.into(), update);
        let applied = session.apply_pending().unwrap();
        assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
        let new = Ref::<TextBox>::from_xaml_object(&host.content().unwrap()).unwrap();
        assert!(!new.ptr_eq(&old));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(new.text().as_deref(), Some("draft text"));
        assert_eq!(
            (new.selection_start(), new.selection_end(), new.caret_index()),
            (start, end, end),
        );
        drop(old);
        assert!(retired.upgrade().is_none(), "state transfer retained a retired text box");
    }
}

#[test]
fn runtime_selection_restores_the_item_after_markup_reorders_it() {
    use ferroui_base::{threading::Dispatcher, Ref};
    use ferroui_controls::{ContentControl, ListBox};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let before = r#"<ListBox xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Name="choices"><x:String>a</x:String><x:String>b</x:String></ListBox>"#;
    let after = before.replace(
        "<x:String>a</x:String><x:String>b</x:String>",
        "<x:String>b</x:String><x:String>a</x:String>",
    );
    let uri = "ferres://selection/View.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), before.into())
        .unwrap();
    let host = ContentControl::new();
    let target: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        uri.into(),
        vec![],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&target);
    session.submit(uri.into(), before.into());
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    Dispatcher::ui_thread().run_jobs(None);
    let old = Ref::<ListBox>::from_xaml_object(&host.content().unwrap()).unwrap();
    old.set_selected_index(1);
    let selected = old.selected_item();
    session.submit(uri.into(), after);
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    Dispatcher::ui_thread().run_jobs(None);
    let new = Ref::<ListBox>::from_xaml_object(&host.content().unwrap()).unwrap();
    assert_eq!(new.selected_index(), 0);
    assert_eq!(new.selected_item(), selected);
}

#[test]
fn failed_batch_retains_other_document_drafts_until_repaired() {
    let mut session = session();
    let (_target, _value) = target(&mut session, false);
    session
        .register_document("ferres://test/Other.xaml".into(), "<Border/>".into())
        .unwrap();
    submit(&mut session, "<Border Width=\"12\"/>");
    session.submit("ferres://test/Other.xaml".into(), "<".into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert!(
        session.apply_pending().is_none(),
        "do not retry the same failed revision forever"
    );
    session.submit("ferres://test/Other.xaml".into(), "<Border/>".into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert!(session.sources()["ferres://test/View.xaml"].contains("12"));
}

#[test]
fn included_resource_edit_rebuilds_static_resource_consumers() {
    use ferroui_base::{threading::Dispatcher, Ref};
    use ferroui_controls::{Border, ContentControl};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let view = r#"<Border xmlns="https://github.com/ferroui"><Border.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="Values.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Border.Resources><Border Width="{StaticResource width}"/></Border>"#;
    let values = |number| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">{number}</x:Double></ResourceDictionary>"#
        )
    };
    let mut session = HotReloadSession::new();
    session
        .register_document("ferres://test/View.xaml".into(), view.into())
        .unwrap();
    session
        .register_document("ferres://test/Values.xaml".into(), values(12))
        .unwrap();
    let host = ContentControl::new();
    let target: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        "ferres://test/View.xaml".into(),
        vec!["ferres://test/Values.xaml".into()],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&target);
    session.submit("ferres://test/Values.xaml".into(), values(24));
    let result = session.apply_pending().unwrap();
    assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
    assert_eq!(
        Ref::<Border>::from_xaml_object(&host.content().unwrap())
            .unwrap()
            .child()
            .unwrap()
            .width(),
        24.0
    );
}

#[test]
fn bound_class_uses_accepted_text_and_unhooks_on_drop() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::Border;
    use ferroui_markup_xaml::{FerroXamlLoader, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let mut session = HotReloadSession::new();
    session
        .register_document(
            "ferres://test/Border.xaml".into(),
            r#"<Border xmlns="https://github.com/ferroui" Width="31"/>"#.into(),
        )
        .unwrap();
    session
        .bind_class(
            Border::TYPE,
            "ferres://test/Border.xaml".into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap();
    let border = Border::new();
    FerroXamlLoader::load_object(&(Rc::new(border.clone()) as ferroui_base::BoxedValue)).unwrap();
    assert_eq!(border.width(), 31.0);
    FerroXamlLoader::track_instance(Border::TYPE, &border);
    FerroXamlLoader::track_instance(Border::TYPE, &border);
    assert_eq!(FerroXamlLoader::live_instances(Border::TYPE).len(), 1);
    drop(border);
    assert!(FerroXamlLoader::live_instances(Border::TYPE).is_empty());
    drop(session);
    assert!(FerroXamlLoader::populate_override(Border::TYPE).is_none());
}

#[test]
fn watcher_observes_atomic_save_and_stops() {
    use std::{
        fs,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    let directory = std::env::temp_dir().join(format!(
        "ferroui-watcher-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("View.xaml");
    fs::write(&path, "<Border/>").unwrap();
    let watcher = FileWatcher::with_baseline(
        BTreeMap::from([("ferres://test/View.xaml".into(), path.clone())]),
        Duration::from_millis(10),
        BTreeMap::from([("ferres://test/View.xaml".into(), "<Border/>".into())]),
    );
    let temporary = directory.join("save.tmp");
    fs::write(&temporary, "<Border Width=\"2\"/>").unwrap();
    #[cfg(windows)]
    fs::remove_file(&path).unwrap();
    fs::rename(&temporary, &path).unwrap();
    let change = watcher
        .changes
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    assert_eq!(
        change.files.into_iter().next().unwrap().contents.unwrap(),
        "<Border Width=\"2\"/>"
    );
    drop(watcher);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn transitive_resource_revision_refreshes_all_instances_and_releases_old_trees() {
    use ferroui_base::{threading::Dispatcher, Ref};
    use ferroui_controls::{Border, ContentControl};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let view = r#"<Border xmlns="https://github.com/ferroui"><Border.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="Theme.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Border.Resources><Border Width="{StaticResource width}"/></Border>"#;
    let theme = r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceDictionary.MergedDictionaries><ResourceInclude Source="Values.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"#;
    let values = |number: &str| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">{number}</x:Double></ResourceDictionary>"#
        )
    };
    let mut session = HotReloadSession::new();
    for (name, text) in [
        ("View.xaml", view.to_owned()),
        ("Theme.xaml", theme.to_owned()),
        ("Values.xaml", values("1")),
    ] {
        session
            .register_document(format!("ferres://transitive/{name}"), text)
            .unwrap();
    }
    let hosts = [ContentControl::new(), ContentControl::new()];
    let targets: Vec<Rc<dyn ReloadTarget>> = hosts
        .iter()
        .map(|host| {
            Rc::new(ContentReloadTarget::new(
                host,
                "ferres://transitive/View.xaml".into(),
                vec![],
                RuntimeXamlLoaderConfiguration::default(),
            )) as Rc<dyn ReloadTarget>
        })
        .collect();
    for target in &targets {
        session.register_target(target);
    }
    let mut previous = Vec::new();
    for number in 1..=20 {
        session.submit(
            "ferres://transitive/Values.xaml".into(),
            values(&number.to_string()),
        );
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        Dispatcher::ui_thread().run_jobs(None);
        for weak in &previous {
            let weak: &ferroui_base::WeakRef<Border> = weak;
            assert!(
                weak.upgrade().is_none(),
                "detached root retained after reload"
            );
        }
        previous.clear();
        for host in &hosts {
            let root = Ref::<Border>::from_xaml_object(&host.content().unwrap()).unwrap();
            assert_eq!(root.child().unwrap().width(), number as f64);
            previous.push(root.downgrade());
        }
    }
    let last: Vec<_> = hosts.iter().map(|host| host.content().unwrap()).collect();
    session.submit("ferres://transitive/Values.xaml".into(), values("invalid"));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    for (host, old) in hosts.iter().zip(&last) {
        assert!(Rc::ptr_eq(&host.content().unwrap(), old));
    }
    assert!(session.sources()["ferres://transitive/Values.xaml"].contains(">20<"));
}

#[test]
fn lazy_loads_observe_only_accepted_sources_and_session_drop_unpublishes_them() {
    use ferroui_markup_xaml::source_overlay::source;
    let mut session = session();
    let (_target, _) = target(&mut session, false);
    assert_eq!(
        source("ferres://test/View.xaml").as_deref(),
        Some("<Border/>")
    );
    submit(&mut session, "<Border Width=\"3\"/>");
    assert_eq!(
        source("ferres://test/View.xaml").as_deref(),
        Some("<Border/>")
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(
        source("ferres://test/View.xaml").as_deref(),
        Some("<Border Width=\"3\"/>")
    );
    submit(&mut session, "<");
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(
        source("ferres://test/View.xaml").as_deref(),
        Some("<Border Width=\"3\"/>")
    );
    let mut competing = HotReloadSession::new();
    assert!(competing
        .register_document("ferres://test/View.xaml".into(), "<Border/>".into())
        .is_err());
    drop(session);
    assert!(source("ferres://test/View.xaml").is_none());
    competing
        .register_document("ferres://test/View.xaml".into(), "<Border/>".into())
        .unwrap();
}

#[test]
fn manifest_dependency_lookup_normalizes_uri_authority_and_handles_cycles() {
    let manifest = crate::ReloadManifest::parse(&serde_json::json!({
        "version": 1, "documents": [
            { "uri": "ferres://Fixture/View.xaml", "source_path": "View.xaml", "source": "<Border/>", "includes": ["Theme.xaml"] },
            { "uri": "ferres://Fixture/Theme.xaml", "source_path": "Theme.xaml", "source": "<ResourceDictionary/>", "includes": ["View.xaml"] }
        ]
    }).to_string()).unwrap();
    assert_eq!(
        manifest.transitive_dependencies("ferres://FIXTURE/View.xaml"),
        vec!["ferres://fixture/Theme.xaml"]
    );
}

#[test]
fn component_reload_preserves_root_model_and_installs_a_fresh_namescope() {
    use ferroui_base::{controls::NameScope, threading::Dispatcher, Ref};
    use ferroui_controls::{Border, ContentPage};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let text = |width: &str| {
        format!(
            r#"<ContentPage xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Fixture.Page"><Border x:Name="Part" Width="{width}"/></ContentPage>"#
        )
    };
    let root = ContentPage::new();
    root.set_width(321.0); // Application-owned root assignment stays intact.
    let model: ferroui_base::BoxedValue = Rc::new(42_i32);
    root.set_data_context(Some(model.clone()));
    let initial = Border::new();
    initial.set_width(1.0);
    root.set_content(Some(Rc::new(initial.clone())));
    let mut session = HotReloadSession::new();
    session
        .register_document("ferres://component/View.xaml".into(), text("1"))
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(crate::ComponentReloadTarget::content_page(
        &root,
        "ferres://component/View.xaml".into(),
        text("1"),
        RuntimeXamlLoaderConfiguration::default(),
    ));
    session.register_target(&target);
    let explicit = text("2")
        .replace("<Border", "<ContentPage.Content><Border")
        .replace("/></ContentPage>", "/></ContentPage.Content></ContentPage>");
    session.submit("ferres://component/View.xaml".into(), explicit);
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    assert_eq!(root.width(), 321.0);
    assert!(Rc::ptr_eq(&root.data_context().unwrap(), &model));
    let content = root.content().unwrap();
    let child = Ref::<Border>::from_xaml_object(&content).unwrap();
    assert_eq!(child.width(), 2.0);
    let scope = NameScope::get_name_scope(&root).unwrap();
    assert!(scope.find("Part").unwrap().ptr_eq(&child));
    session.submit("ferres://component/View.xaml".into(), text("invalid"));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert!(Rc::ptr_eq(&root.content().unwrap(), &content));
    assert!(NameScope::get_name_scope(&root).unwrap() == scope);
    let root_change = text("3").replace("<ContentPage ", "<ContentPage Width=\"500\" ");
    session.submit("ferres://component/View.xaml".into(), root_change);
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.width(), 321.0);
    for unsupported in [
        text("3")
            .replace("<Border", "<Border.Content><Border")
            .replace("/></ContentPage>", "/></Border.Content></ContentPage>"),
        text("3").replace(
            "<Border",
            "<ContentPage.Resources><ResourceDictionary/></ContentPage.Resources><Border",
        ),
    ] {
        session.submit("ferres://component/View.xaml".into(), unsupported);
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::RestartRequired
        );
        assert!(Rc::ptr_eq(&root.content().unwrap(), &content));
    }
}

#[test]
fn component_refresh_replaces_cached_controls_and_disposes_subscriptions_transactionally() {
    use ferroui_base::{
        controls::NameScopeRef, reactive::IDisposable, threading::Dispatcher, Ref, StyledElement,
    };
    use ferroui_controls::{Border, ContentPage};
    use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
    use std::cell::Cell;
    struct Refresh {
        cached: Rc<RefCell<Option<Ref<Border>>>>,
        subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
        events: Rc<Cell<usize>>,
        fail: Rc<Cell<bool>>,
    }
    struct RefreshChange {
        cached: Rc<RefCell<Option<Ref<Border>>>>,
        subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
        before: Option<Ref<Border>>,
        after: Ref<Border>,
        old_subscription: Option<Rc<dyn IDisposable>>,
        new_subscription: Option<Rc<dyn IDisposable>>,
        events: Rc<Cell<usize>>,
        fail: bool,
    }
    impl crate::ComponentRefresh for Refresh {
        fn prepare(
            &self,
            _: &Ref<StyledElement>,
            scope: Option<&NameScopeRef>,
        ) -> Result<Box<dyn PreparedReload>, ReloadError> {
            let after = scope
                .and_then(|scope| scope.find("Part"))
                .and_then(|part| part.cast::<Border>())
                .ok_or_else(|| ReloadError::Failed("required Part is missing".into()))?;
            Ok(Box::new(RefreshChange {
                cached: self.cached.clone(),
                subscription: self.subscription.clone(),
                before: self.cached.borrow().clone(),
                after,
                old_subscription: self.subscription.borrow().clone(),
                new_subscription: None,
                events: self.events.clone(),
                fail: self.fail.get(),
            }))
        }
    }
    impl PreparedReload for RefreshChange {
        fn commit(&mut self) -> Result<(), ReloadError> {
            let events = self.events.clone();
            let subscription = self
                .after
                .property_changed(move |_| events.set(events.get() + 1));
            self.new_subscription = Some(subscription.clone());
            *self.subscription.borrow_mut() = Some(subscription);
            *self.cached.borrow_mut() = Some(self.after.clone());
            if self.fail {
                Err(ReloadError::Failed("application refresh failed".into()))
            } else {
                Ok(())
            }
        }
        fn rollback(&mut self) {
            if let Some(subscription) = self.new_subscription.take() {
                subscription.dispose();
            }
            *self.cached.borrow_mut() = self.before.clone();
            *self.subscription.borrow_mut() = self.old_subscription.clone();
        }
        fn finish(&mut self) {
            if let Some(subscription) = self.old_subscription.take() {
                subscription.dispose();
            }
        }
    }
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let source = |number| {
        format!(
            r#"<ContentPage xmlns="https://github.com/ferroui"><Border Name="Part" Width="{number}"/></ContentPage>"#
        )
    };
    let root = ContentPage::new();
    let refresh = Rc::new(Refresh {
        cached: Rc::new(RefCell::new(None)),
        subscription: Rc::new(RefCell::new(None)),
        events: Rc::new(Cell::new(0)),
        fail: Rc::new(Cell::new(false)),
    });
    let mut session = HotReloadSession::new();
    session
        .register_document("ferres://refresh/View.xaml".into(), source(0))
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        crate::ComponentReloadTarget::content_page(
            &root,
            "ferres://refresh/View.xaml".into(),
            source(0),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_refresh(refresh.clone()),
    );
    session.register_target(&target);
    for revision in 1..=10 {
        let old = refresh.cached.borrow().clone();
        session.submit("ferres://refresh/View.xaml".into(), source(revision));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Applied
        );
        Dispatcher::ui_thread().run_jobs(None);
        let new = refresh.cached.borrow().clone().unwrap();
        let before = refresh.events.get();
        if let Some(old) = old {
            old.set_width(999.0);
        }
        assert_eq!(
            refresh.events.get(),
            before,
            "old child subscription survived reload"
        );
        new.set_width(1000.0);
        assert_eq!(
            refresh.events.get(),
            before + 1,
            "duplicate or missing new subscription"
        );
    }
    let old_content = root.content().unwrap();
    let old = refresh.cached.borrow().clone().unwrap();
    refresh.fail.set(true);
    session.submit("ferres://refresh/View.xaml".into(), source(11));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert!(Rc::ptr_eq(&root.content().unwrap(), &old_content));
    assert!(refresh.cached.borrow().as_ref().unwrap().ptr_eq(&old));
    let before = refresh.events.get();
    old.set_width(1001.0);
    assert_eq!(
        refresh.events.get(),
        before + 1,
        "rollback lost the original subscription"
    );
    struct LaterFailure;
    struct FailingChange;
    impl ReloadTarget for LaterFailure {
        fn documents(&self) -> Vec<String> {
            vec!["ferres://refresh/View.xaml".into()]
        }
        fn prepare(&self, _: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
            Ok(Box::new(FailingChange))
        }
    }
    impl PreparedReload for FailingChange {
        fn commit(&mut self) -> Result<(), ReloadError> {
            Err(ReloadError::Failed("later target failed".into()))
        }
        fn rollback(&mut self) {}
    }
    let later: Rc<dyn ReloadTarget> = Rc::new(LaterFailure);
    session.register_target(&later);
    refresh.fail.set(false);
    session.submit("ferres://refresh/View.xaml".into(), source(12));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert!(Rc::ptr_eq(&root.content().unwrap(), &old_content));
    assert!(refresh.cached.borrow().as_ref().unwrap().ptr_eq(&old));
    let before = refresh.events.get();
    old.set_width(1002.0);
    assert_eq!(
        refresh.events.get(),
        before + 1,
        "later failure lost the original subscription"
    );
    assert_eq!(session.sources()["ferres://refresh/View.xaml"], source(10));
    refresh.subscription.borrow_mut().take().unwrap().dispose();
}

#[test]
fn style_owner_reloads_existing_controls_and_restores_after_partial_commit() {
    use ferroui_base::{
        styling::{style_ptr_eq, IStyle},
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let source = |width| {
        format!(
            r#"<Styles xmlns="https://github.com/ferroui"><Style Selector="Border.active"><Setter Property="Width" Value="{width}"/></Style></Styles>"#
        )
    };
    let load = |text: &str| {
        Rc::<dyn IStyle>::from_xaml_object(
            &ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                RuntimeXamlLoaderDocument::with_base_uri(None, text),
                None,
            )
            .unwrap(),
        )
        .unwrap()
    };
    let root = Border::new();
    let child = Border::new();
    root.set_child(child.clone());
    child.classes().add("active");
    let application = load(
        r#"<Style xmlns="https://github.com/ferroui" Selector="Border"><Setter Property="Height" Value="33"/></Style>"#,
    );
    root.styles().add(load(&source(10)));
    root.styles().add(application.clone());
    root.apply_styling();
    child.apply_styling();
    assert_eq!(child.width(), 10.0);
    assert_eq!(child.height(), 33.0);
    let uri = "ferres://styleowner/Styles.xaml";
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), source(10)).unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        StyleReloadTarget::contribution(
            &root.styles(),
            0,
            uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&target);
    for width in 11..21 {
        let old = Rc::downgrade(&root.styles().get(0));
        session.submit(uri.into(), source(width));
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        root.apply_styling();
        child.apply_styling();
        assert_eq!(child.width(), width as f64);
        child.classes().remove("active");
        assert!(child.width().is_nan());
        child.classes().add("active");
        assert_eq!(child.width(), width as f64);
        assert_eq!(child.height(), 33.0);
        assert!(style_ptr_eq(&root.styles().get(1), &application));
        assert!(old.upgrade().is_none(), "detached style retained");
    }
    let old = root.styles().get(0);
    session.submit(uri.into(), source(100).replace("\"100\"", "\"invalid\""));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert!(style_ptr_eq(&root.styles().get(0), &old));
    assert_eq!(child.width(), 20.0);
    let panic_once = Rc::new(Cell::new(true));
    let subscription = root.styles().collection_changed(move |_| {
        if panic_once.replace(false) {
            panic!("observer rejected removal");
        }
    });
    session.submit(uri.into(), source(100));
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed, "{outcome:?}");
    subscription.dispose();
    root.apply_styling();
    child.apply_styling();
    assert_eq!(root.styles().count(), 2);
    assert!(style_ptr_eq(&root.styles().get(0), &old));
    assert!(style_ptr_eq(&root.styles().get(1), &application));
    assert_eq!(child.width(), 20.0);
    assert_eq!(child.height(), 33.0);
    assert_eq!(session.sources()[uri], source(20));
    session.submit(
        uri.into(),
        r#"<Styles xmlns="https://github.com/ferroui"/>"#.into(),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    root.apply_styling();
    child.apply_styling();
    assert!(child.width().is_nan());
    assert_eq!(child.height(), 33.0);
}

#[test]
fn style_reload_invalidates_templates_and_releases_detached_parts() {
    use ferroui_base::{styling::IStyle, threading::Dispatcher};
    use ferroui_controls::{Border, Button};
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let source = |width| {
        format!(
            r#"<Styles xmlns="https://github.com/ferroui"><Style Selector="Button"><Setter Property="Template"><ControlTemplate><Border Width="{width}"/></ControlTemplate></Setter></Style></Styles>"#
        )
    };
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_base_uri(None, &source(1)),
        None,
    )
    .unwrap();
    let root = Button::new();
    root.styles()
        .add(Rc::<dyn IStyle>::from_xaml_object(&loaded).unwrap());
    drop(loaded);
    root.apply_styling();
    root.apply_template();
    let uri = "ferres://templates/Styles.xaml";
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), source(1)).unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        StyleReloadTarget::contribution(
            &root.styles(),
            0,
            uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&target);
    for width in 2..12 {
        let old = root.visual_children().get(0).downgrade();
        session.submit(uri.into(), source(width));
        let outcome = session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        root.apply_styling();
        root.apply_template();
        let part = root.visual_children().get(0).cast::<Border>().unwrap();
        assert_eq!(part.width(), width as f64);
        assert!(old.upgrade().is_none(), "previous template part retained");
        assert_eq!(root.visual_children_count(), 1);
    }
    let part = root.visual_children().get(0);
    session.submit(uri.into(), source(100));
    let outcome = session
        .apply_pending_with(|| Err(ReloadError::Failed("patch rejected".into())))
        .unwrap();
    assert_eq!(outcome.status, ReloadStatus::Failed);
    root.apply_styling();
    root.apply_template();
    assert!(root.visual_children().get(0).ptr_eq(&part));
    assert_eq!(session.sources()[uri], source(11));
}

#[test]
fn style_resource_replacement_notifies_consumers_and_rolls_back_ownership() {
    use ferroui_base::{styling::IStyle, threading::Dispatcher, Ref};
    use ferroui_controls::Border;
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let load = |text: &str| {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(None, text),
            None,
        )
        .unwrap()
    };
    let root = Ref::<Border>::from_xaml_object(&load(
        r#"<Border xmlns="https://github.com/ferroui" Width="{DynamicResource metric}"/>"#,
    ))
    .unwrap();
    let source = |width| {
        format!(
            r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Styles.Resources><x:Double x:Key="metric">{width}</x:Double></Styles.Resources></Styles>"#
        )
    };
    root.styles()
        .add(Rc::<dyn IStyle>::from_xaml_object(&load(&source(7))).unwrap());
    assert_eq!(root.width(), 7.0);
    let uri = "ferres://styleresources/Styles.xaml";
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), source(7)).unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        StyleReloadTarget::contribution(
            &root.styles(),
            0,
            uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&target);
    let old = root.styles().get(0);
    session.submit(uri.into(), source(8));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert!(old.as_resource_provider().unwrap().owner().is_none());
    assert!(root
        .styles()
        .get(0)
        .as_resource_provider()
        .unwrap()
        .owner()
        .is_some());
    assert_eq!(root.width(), 8.0);
    session.submit(uri.into(), source(9));
    assert_eq!(
        session
            .apply_pending_with(|| Err(ReloadError::Failed("later step failed".into())))
            .unwrap()
            .status,
        ReloadStatus::Failed
    );
    assert_eq!(root.width(), 8.0);
    assert!(root
        .styles()
        .get(0)
        .as_resource_provider()
        .unwrap()
        .owner()
        .is_some());
    session.submit(
        uri.into(),
        r#"<Styles xmlns="https://github.com/ferroui"/>"#.into(),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert!(root.width().is_nan());
}

#[test]
fn component_owned_root_values_remove_restore_and_protect_application_overrides() {
    use ferroui_base::{
        styling::{Selectors, Setter, Style},
        threading::Dispatcher,
        Ref,
    };
    use ferroui_controls::ContentControl;
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let source = |width: &str, child: u32| {
        format!(
            r#"<ContentControl xmlns="https://github.com/ferroui" {width}><Border Height="{child}"/></ContentControl>"#
        )
    };
    let initial = source("Width=\"20\"", 1);
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_base_uri(None, &initial),
        None,
    )
    .unwrap();
    let root = Ref::<ContentControl>::from_xaml_object(&loaded).unwrap();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<ContentControl>(),
        [Setter::new(
            ferroui_base::layout::Layoutable::width_property(),
            10.0,
        )],
    ));
    root.apply_styling();
    root.set_height(88.0);
    let vm: ferroui_base::BoxedValue = Rc::new("same model");
    root.set_data_context(Some(vm.clone()));
    let uri = "ferres://rootvalues/View.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), initial.clone())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root,
            uri.into(),
            initial,
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_root_property(ferroui_base::layout::Layoutable::width_property()),
    );
    session.register_target(&target);
    session.submit(uri.into(), source("Width=\"30\"", 2));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(root.width(), 30.0);
    let old_content = root.content();
    session.submit(uri.into(), source("", 3));
    assert_eq!(
        session
            .apply_pending_with(|| Err(ReloadError::Failed("later target failed".into())))
            .unwrap()
            .status,
        ReloadStatus::Failed
    );
    assert_eq!(root.width(), 30.0);
    assert_eq!(root.content(), old_content);
    session.submit(uri.into(), source("", 4));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(
        root.width(),
        10.0,
        "removed local value must expose its style"
    );
    assert_eq!(root.height(), 88.0);
    assert!(Rc::ptr_eq(&root.data_context().unwrap(), &vm));
    session.submit(uri.into(), source("Width=\"40\"", 5));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(root.width(), 40.0);
    root.set_width(40.0); // Application ownership changes even when the value does not.
    let old_content = root.content();
    session.submit(uri.into(), source("", 6));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.width(), 40.0);
    assert_eq!(root.content(), old_content);
    root.set_width(77.0);
    let accepted = session.sources();
    let old_content = root.content();
    session.submit(uri.into(), source("", 6));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.width(), 77.0);
    assert_eq!(root.content(), old_content);
    assert_eq!(session.sources(), accepted);
}

#[test]
fn panel_owned_root_properties_rollback_and_reject_binding_edits() {
    use ferroui_base::{threading::Dispatcher, Ref};
    use ferroui_controls::{Panel, StackPanel};
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let source = |spacing: &str| {
        format!(
            r#"<StackPanel xmlns="https://github.com/ferroui" {spacing}><Border/></StackPanel>"#
        )
    };
    let initial = source("Spacing=\"5\"");
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_base_uri(None, &initial),
        None,
    )
    .unwrap();
    let root = Ref::<StackPanel>::from_xaml_object(&loaded).unwrap();
    let panel: Ref<Panel> = root.clone().upcast();
    let uri = "ferres://rootpanel/View.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), initial.clone())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        PanelReloadTarget::new(
            &panel,
            0..1,
            uri.into(),
            initial,
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap()
        .with_owned_root_property(StackPanel::spacing_property()),
    );
    session.register_target(&target);
    session.submit(uri.into(), source("Spacing=\"15\""));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(root.spacing(), 15.0);
    let child = root.children().get(0);
    session.submit(uri.into(), source("Spacing=\"20\""));
    assert_eq!(
        session
            .apply_pending_with(|| Err(ReloadError::Failed("late failure".into())))
            .unwrap()
            .status,
        ReloadStatus::Failed
    );
    assert_eq!(root.spacing(), 15.0);
    assert!(root.children().get(0).ptr_eq(&child));
    session.submit(uri.into(), source("Spacing=\"{Binding Spacing}\""));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.spacing(), 15.0);
    assert!(root.children().get(0).ptr_eq(&child));
    session.submit(uri.into(), source(""));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(root.spacing(), 0.0);
}

#[test]
fn partial_owned_root_assignment_failure_restores_every_value_and_graph() {
    use ferroui_base::{layout::Layoutable, threading::Dispatcher, Ref};
    use ferroui_controls::ContentControl;
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let initial = r#"<ContentControl xmlns="https://github.com/ferroui" Width="5" Height="10"><Border/></ContentControl>"#;
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_base_uri(None, initial),
        None,
    )
    .unwrap();
    let root = Ref::<ContentControl>::from_xaml_object(&loaded).unwrap();
    let old_content = root.content();
    let uri = "ferres://rootfailure/View.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), initial.into())
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root,
            uri.into(),
            initial.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_root_property(Layoutable::width_property())
        .with_owned_root_property(Layoutable::height_property()),
    );
    session.register_target(&target);
    let once = Rc::new(Cell::new(true));
    let subscription = root.property_changed(move |event| {
        if event.property().id() == Layoutable::height_property().id() && once.replace(false) {
            panic!("application rejected height assignment");
        }
    });
    session.submit(
        uri.into(),
        initial
            .replace("Width=\"5\"", "Width=\"6\"")
            .replace("Height=\"10\"", "Height=\"11\""),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(root.width(), 5.0);
    assert_eq!(root.height(), 10.0);
    assert_eq!(root.content(), old_content);
    assert_eq!(session.sources()[uri], initial);
    subscription.dispose();
}

#[test]
fn opening_a_target_retries_a_draft_without_another_file_save() {
    let mut session = session();
    session.submit(
        "ferres://test/View.xaml".into(),
        "<Border Width=\"7\"/>".into(),
    );
    let unavailable = session.apply_pending().unwrap();
    assert_eq!(unavailable.status, ReloadStatus::RestartRequired);
    assert_eq!(session.sources()["ferres://test/View.xaml"], "<Border/>");
    assert!(
        session.apply_pending().is_none(),
        "do not repeatedly retry an unchanged draft"
    );
    let (_target, value) = target(&mut session, false);
    let accepted = session.apply_pending().unwrap();
    assert_eq!(accepted.status, ReloadStatus::Applied);
    assert!(accepted.revision > unavailable.revision);
    assert_eq!(&*value.borrow(), "<Border Width=\"7\"/>");
    assert_eq!(
        session.sources()["ferres://test/View.xaml"],
        "<Border Width=\"7\"/>"
    );
}

#[test]
fn owned_root_property_rechecks_equal_value_writes_before_commit() {
    use ferroui_base::{layout::Layoutable, threading::Dispatcher, FerroObject, Ref};
    use ferroui_controls::ContentControl;
    let _scope = Dispatcher::unit_test_scope();
    let root = ContentControl::new();
    let candidate = ContentControl::new();
    root.set_width(20.0);
    candidate.set_width(30.0);
    let root_object: Ref<FerroObject> = root.clone().upcast();
    let candidate_object: Ref<FerroObject> = candidate.upcast();
    let property = crate::root_properties::own(&root_object, Layoutable::width_property());
    let mut change = property
        .prepare(
            &root_object,
            &candidate_object,
            "<ContentControl Width=\"20\"/>",
            "<ContentControl Width=\"30\"/>",
            &ferroui_markup_xaml_loader::binding_capture::BindingCaptureScope::new(
                "ferres://test/View.xaml".into(), [],
            ),
        )
        .unwrap()
        .unwrap();
    root.set_width(20.0);
    assert!(matches!(
        change.commit(),
        Err(ReloadError::RestartRequired(_))
    ));
    change.rollback();
    assert_eq!(root.width(), 20.0);
}

#[test]
fn application_resource_revision_prepares_static_consumers_against_new_values() {
    use ferroui_base::{controls::IResourceProvider, Ref};
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application, Border, ContentControl,
    };
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let values = |value| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">{value}</x:Double></ResourceDictionary>"#
        )
    };
    let resource_uri = "ferres://global/Values.xaml";
    let view_uri = "ferres://global/View.xaml";
    let view = r#"<Border xmlns="https://github.com/ferroui" Width="{StaticResource width}"/>"#;
    let mut session = HotReloadSession::new();
    session
        .register_document(resource_uri.into(), values(7))
        .unwrap();
    session
        .register_document(view_uri.into(), view.into())
        .unwrap();
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let base = ferroui_base::utilities::Uri::try_create(
        view_uri,
        ferroui_base::utilities::UriKind::Absolute,
    )
    .unwrap();
    let include = ferroui_markup_xaml::styling::ResourceInclude::new(Some(base));
    include.set_source(ferroui_base::utilities::Uri::try_create(
        "Values.xaml",
        ferroui_base::utilities::UriKind::RelativeOrAbsolute,
    ));
    assert_eq!(include.development_source().as_deref(), Some(resource_uri));
    let resources = Application::current().unwrap().resources();
    resources.add_merged_dictionary(include.as_resource_provider());
    let host = ContentControl::new();
    let consumer: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        view_uri.into(),
        vec![resource_uri.into()],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    let mut discovered = ResourceReloadTarget::discover_merged(
        &resources,
        RuntimeXamlLoaderConfiguration::default(),
    )
    .unwrap();
    assert_eq!(discovered.len(), 1);
    let owner: Rc<dyn ReloadTarget> = Rc::new(discovered.remove(0));
    assert_eq!(owner.documents(), vec![resource_uri.to_owned()]);
    // Registration order must not determine which resource revision is observed.
    session.register_target(&consumer);
    session.register_target(&owner);
    session.submit(view_uri.into(), view.into());
    let initial = session.apply_pending().unwrap();
    assert_eq!(initial.status, ReloadStatus::Applied, "{initial:?}");
    session.submit(resource_uri.into(), values(8));
    let applied = session.apply_pending().unwrap();
    assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
    let child = Ref::<Border>::from_xaml_object(&host.content().unwrap()).unwrap();
    child.initialize_if_needed();
    assert_eq!(
        child.width(),
        8.0,
        "static consumer used the old application resource"
    );
    let before = host.content();
    let notifications = Rc::new(std::cell::Cell::new(0));
    let observed = notifications.clone();
    let _subscription = Application::current()
        .unwrap()
        .resources_changed(move |_| observed.set(observed.get() + 1));
    session.submit(
        resource_uri.into(),
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"/>"#.into(),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(host.content(), before);
    assert_eq!(
        notifications.get(),
        0,
        "failed preparation published resource notifications"
    );
    let duplicate: Rc<dyn ReloadTarget> = Rc::new(
        ResourceReloadTarget::merged(
            &resources,
            0,
            resource_uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&duplicate);
    session.submit(resource_uri.into(), values(9));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(host.content(), before);
    assert_eq!(
        notifications.get(),
        0,
        "ambiguous ownership mutated live resources"
    );
}

#[test]
fn resource_previews_share_instances_and_resolve_chained_dependencies() {
    use ferroui_base::{
        controls::{IResourceProvider, ResourceDictionary, ResourceKey},
        Ref,
    };
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application, Border, ContentControl,
    };
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let values = |value| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Double x:Key="width">{value}</x:Double></ResourceDictionary>"#
        )
    };
    let shared = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Border x:Key="shared" Width="{StaticResource width}"/></ResourceDictionary>"#;
    let load = |source: &str| {
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(None, source),
            None,
        )
        .unwrap();
        Ref::<ResourceDictionary>::from_xaml_object(&value).unwrap()
    };
    let resources = Application::current().unwrap().resources();
    resources.add_merged_dictionary(Rc::<dyn IResourceProvider>::from(load(&values(7))));
    resources.add_merged_dictionary(Rc::<dyn IResourceProvider>::from(load(shared)));
    let value_uri = "ferres://chain/Values.xaml";
    let shared_uri = "ferres://chain/Shared.xaml";
    let view_uri = "ferres://chain/View.xaml";
    let view = r#"<Border xmlns="https://github.com/ferroui" Tag="{StaticResource shared}"/>"#;
    let mut session = HotReloadSession::new();
    session
        .register_document(value_uri.into(), values(7))
        .unwrap();
    session
        .register_document(shared_uri.into(), shared.into())
        .unwrap();
    session
        .register_document(view_uri.into(), view.into())
        .unwrap();
    let host = ContentControl::new();
    let consumer: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        view_uri.into(),
        vec![shared_uri.into(), value_uri.into()],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    let shared_owner: Rc<dyn ReloadTarget> = Rc::new(
        ResourceReloadTarget::merged(
            &resources,
            1,
            shared_uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap()
        .with_dependencies([value_uri.into()]),
    );
    let values_owner: Rc<dyn ReloadTarget> = Rc::new(
        ResourceReloadTarget::merged(
            &resources,
            0,
            value_uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&shared_owner);
    session.register_target(&consumer);
    session.register_target(&values_owner);
    session.submit(view_uri.into(), view.into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    let original = host.content();
    session.submit(value_uri.into(), values(8));
    assert_eq!(
        session
            .apply_pending_with(|| Err(ReloadError::Failed("later failure".into())))
            .unwrap()
            .status,
        ReloadStatus::Failed
    );
    assert_eq!(host.content(), original);
    let old_shared = resources
        .try_get_resource(&ResourceKey::from("shared"), None)
        .unwrap()
        .unwrap();
    assert_eq!(
        Ref::<Border>::from_xaml_object(&old_shared)
            .unwrap()
            .width(),
        7.0
    );
    session.submit(value_uri.into(), values(8));
    let applied = session.apply_pending().unwrap();
    assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
    let child = Ref::<Border>::from_xaml_object(&host.content().unwrap()).unwrap();
    let consumed = Ref::<Border>::from_xaml_object(&child.tag().unwrap()).unwrap();
    let published = resources
        .try_get_resource(&ResourceKey::from("shared"), None)
        .unwrap()
        .unwrap();
    let published = Ref::<Border>::from_xaml_object(&published).unwrap();
    assert_eq!(consumed.width(), 8.0);
    assert!(
        consumed.ptr_eq(&published),
        "static consumers must share the committed resource instance"
    );
}

#[test]
fn application_style_revision_prepares_static_consumers_against_new_values() {
    use ferroui_base::{controls::IResourceProvider, Ref};
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application, Border, ContentControl,
    };
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration};
    let _application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let values = |value| {
        format!(
            r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Styles.Resources><x:Double x:Key="width">{value}</x:Double></Styles.Resources></Styles>"#
        )
    };
    let resource_uri = "ferres://globalstyles/Values.xaml";
    let view_uri = "ferres://globalstyles/View.xaml";
    let view = r#"<Border xmlns="https://github.com/ferroui" Width="{StaticResource width}"/>"#;
    let mut session = HotReloadSession::new();
    session
        .register_document(resource_uri.into(), values(7))
        .unwrap();
    session
        .register_document(view_uri.into(), view.into())
        .unwrap();
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let base = ferroui_base::utilities::Uri::try_create(
        view_uri,
        ferroui_base::utilities::UriKind::Absolute,
    )
    .unwrap();
    let include = ferroui_markup_xaml::styling::StyleInclude::new(Some(base));
    include.set_source(ferroui_base::utilities::Uri::try_create(
        "Values.xaml",
        ferroui_base::utilities::UriKind::RelativeOrAbsolute,
    ));
    assert_eq!(include.development_source().as_deref(), Some(resource_uri));
    let styles = Application::current().unwrap().styles();
    let index = styles.count();
    styles.add(include.as_style());
    let host = ContentControl::new();
    let consumer: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
        &host,
        view_uri.into(),
        vec![resource_uri.into()],
        RuntimeXamlLoaderConfiguration::default(),
    ));
    let mut discovered =
        StyleReloadTarget::discover(&styles, RuntimeXamlLoaderConfiguration::default()).unwrap();
    assert_eq!(discovered.len(), 1);
    let owner: Rc<dyn ReloadTarget> = Rc::new(discovered.remove(0));
    assert_eq!(owner.documents(), vec![resource_uri.to_owned()]);
    // Registration order must not determine which resource revision is observed.
    session.register_target(&consumer);
    session.register_target(&owner);
    session.submit(view_uri.into(), view.into());
    let initial = session.apply_pending().unwrap();
    assert_eq!(initial.status, ReloadStatus::Applied, "{initial:?}");
    session.submit(resource_uri.into(), values(8));
    let applied = session.apply_pending().unwrap();
    assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
    let child = Ref::<Border>::from_xaml_object(&host.content().unwrap()).unwrap();
    child.initialize_if_needed();
    assert_eq!(
        child.width(),
        8.0,
        "static consumer used the old application resource"
    );
    let before = host.content();
    let notifications = Rc::new(std::cell::Cell::new(0));
    let observed = notifications.clone();
    let _subscription = Application::current()
        .unwrap()
        .resources_changed(move |_| observed.set(observed.get() + 1));
    session.submit(
        resource_uri.into(),
        r#"<Styles xmlns="https://github.com/ferroui"/>"#.into(),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert_eq!(host.content(), before);
    assert_eq!(
        notifications.get(),
        0,
        "failed preparation published resource notifications"
    );
    let duplicate: Rc<dyn ReloadTarget> = Rc::new(
        StyleReloadTarget::contribution(
            &styles,
            index,
            resource_uri.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap(),
    );
    session.register_target(&duplicate);
    session.submit(resource_uri.into(), values(9));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(host.content(), before);
    assert_eq!(
        notifications.get(),
        0,
        "ambiguous ownership mutated live resources"
    );
}

#[test]
fn linked_resource_provenance_does_not_retain_document_roots() {
    use ferroui_base::{
        controls::{IResourceProvider, ResourceDictionary},
        threading::Dispatcher,
        Ref,
    };
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderDocument};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = ferroui_base::utilities::Uri::try_create(
        "ferres://provenance/Values.xaml",
        ferroui_base::utilities::UriKind::Absolute,
    )
    .unwrap();
    let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_base_uri(
            Some(uri.clone()),
            r#"<ResourceDictionary xmlns="https://github.com/ferroui"/>"#,
        ),
        None,
    )
    .unwrap();
    let dictionary = Ref::<ResourceDictionary>::from_xaml_object(&value).unwrap();
    let weak = dictionary.downgrade();
    let provider: Rc<dyn IResourceProvider> = dictionary.into();
    assert_eq!(
        provider.development_source().as_deref(),
        Some(uri.absolute_uri())
    );
    drop(value);
    drop(provider);
    assert!(
        weak.upgrade().is_none(),
        "development provenance retained a detached resource root"
    );
}

#[test]
fn preserved_root_static_theme_tracks_resources_and_rolls_back() {
    use ferroui_base::{
        controls::{IResourceProvider, ResourceDictionary, ResourceKey},
        threading::Dispatcher,
        Ref, StyledElement,
    };
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application, ContentControl, Control, Window,
    };
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    let _application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let values = |opacity| {
        format!(
            r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><ControlTheme x:Key="RootTheme" TargetType="ContentControl"><Setter Property="Opacity" Value="{opacity}"/><Setter Property="Template"><ControlTemplate><Border Width="{opacity}"/></ControlTemplate></Setter></ControlTheme></ResourceDictionary>"#
        )
    };
    let load = |source: &str| {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(None, source),
            None,
        )
        .unwrap()
    };
    let resources = Application::current().unwrap().resources();
    resources.add_merged_dictionary(Rc::<dyn IResourceProvider>::from(
        Ref::<ResourceDictionary>::from_xaml_object(&load(&values(0.2))).unwrap(),
    ));
    let view = r#"<ContentControl xmlns="https://github.com/ferroui" Theme="{StaticResource RootTheme}"><Border/></ContentControl>"#;
    let root = Ref::<ContentControl>::from_xaml_object(&load(view)).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&root.clone().upcast::<Control>())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(root.opacity(), 0.2);
    let uri = "ferres://root-theme/View.xaml";
    let resource_uri = "ferres://root-theme/Theme.xaml";
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), view.into()).unwrap();
    session
        .register_document(resource_uri.into(), values(0.2))
        .unwrap();
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root,
            uri.into(),
            view.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_dependencies([resource_uri.into()])
        .with_owned_root_static_resource_property(StyledElement::theme_property()),
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
    session.register_target(&target);
    session.register_target(&owner);
    let old_theme = root.theme().unwrap();
    let retired_theme = old_theme.downgrade();
    let old_content = root.content();
    let old_part = root.visual_children().get(0);
    let retired_part = old_part.downgrade();
    session.submit(resource_uri.into(), values(0.4));
    assert_eq!(
        session
            .apply_pending_with(|| Err(ReloadError::Failed("later failure".into())))
            .unwrap()
            .status,
        ReloadStatus::Failed
    );
    Dispatcher::ui_thread().run_jobs(None);
    assert!(root.theme().unwrap().ptr_eq(&old_theme));
    assert!(
        root.visual_children().get(0).ptr_eq(&old_part),
        "rollback replaced the previous template graph"
    );
    assert_eq!(root.content(), old_content);
    assert_eq!(root.opacity(), 0.2);
    session.submit(resource_uri.into(), values(0.6));
    let outcome = session.apply_pending().unwrap();
    assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    Dispatcher::ui_thread().run_jobs(None);
    let published = resources
        .try_get_resource(&ResourceKey::from("RootTheme"), None)
        .unwrap()
        .unwrap();
    let published =
        Ref::<ferroui_base::styling::ControlTheme>::from_xaml_object(&published).unwrap();
    assert!(
        root.theme().unwrap().ptr_eq(&published),
        "preserved root still holds the previous static theme"
    );
    assert_eq!(root.opacity(), 0.6);
    let new_part = root
        .visual_children()
        .get(0)
        .cast::<ferroui_controls::Border>()
        .unwrap();
    assert_eq!(new_part.width(), 0.6);
    drop(old_part);
    assert!(
        retired_part.upgrade().is_none(),
        "retired template part retained after commit"
    );
    drop(old_theme);
    assert!(
        retired_theme.upgrade().is_none(),
        "retired root theme retained after commit"
    );
    let accepted_content = root.content();
    session.submit(
        resource_uri.into(),
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"/>"#.into(),
    );
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Failed
    );
    assert!(root.theme().unwrap().ptr_eq(&published));
    assert_eq!(root.content(), accepted_content);
    session.submit(resource_uri.into(), values(0.6));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    let without_theme = view.replace(" Theme=\"{StaticResource RootTheme}\"", "");
    session.submit(uri.into(), without_theme);
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    Dispatcher::ui_thread().run_jobs(None);
    assert!(root.theme().is_none());
    assert_eq!(root.opacity(), 1.0);
    session.submit(uri.into(), view.into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(root.opacity(), 0.6);
    // Equal-value application writes transfer ownership just like different values.
    root.set_theme(root.theme());
    let accepted_content = root.content();
    session.submit(resource_uri.into(), values(0.8));
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(root.content(), accepted_content);
    assert_eq!(root.opacity(), 0.6);
    window.close();
}

#[test]
fn paused_two_way_binding_retains_subscription_and_never_writes_stale_values() {
    use ferroui_base::{
        data::{BindingMode, ReflectionBinding},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    let target = Border::new();
    let binding = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    let expression = target.bind_binding(Layoutable::width_property(), &*binding);
    assert!(expression.can_pause_for_reload());
    assert_eq!(target.width(), 10.0);
    for value in 20..30 {
        assert!(expression.pause_for_reload());
        assert!(
            !expression.pause_for_reload(),
            "nested pause must not steal ownership"
        );
        let before = source.width();
        target.set_current_value(Layoutable::width_property(), 999.0);
        expression.update_source();
        assert_eq!(source.width(), before, "paused target wrote into the model");
        source.set_width(value as f64);
        assert_eq!(
            target.width(),
            999.0,
            "paused source changed the visible target"
        );
        expression.resume_after_reload();
        assert_eq!(
            target.width(),
            value as f64,
            "resume lost the latest source value"
        );
        target.set_current_value(Layoutable::width_property(), value as f64 + 0.5);
        assert_eq!(
            source.width(),
            value as f64 + 0.5,
            "original two-way subscription did not resume"
        );
    }
    expression.dispose();
    source.set_width(1000.0);
    assert_ne!(target.width(), 1000.0);
}

#[test]
fn paused_binding_preserves_invalid_input_until_a_real_source_change() {
    use ferroui_base::{
        data::{BindingMode, ReflectionBinding},
        threading::Dispatcher,
    };
    use ferroui_controls::{Border, DataValidationErrors, TextBox};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    let target = TextBox::new();
    let binding = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    let expression = target.bind_binding(TextBox::text_property(), &*binding);
    target.set_current_value(TextBox::text_property(), Some("invalid".into()));
    assert!(DataValidationErrors::get_has_errors(&target));
    assert_eq!(source.width(), 10.0);

    assert!(expression.pause_for_reload());
    expression.resume_after_reload();
    assert_eq!(target.text().as_deref(), Some("invalid"));
    assert!(DataValidationErrors::get_has_errors(&target));
    assert_eq!(source.width(), 10.0);

    assert!(expression.pause_for_reload());
    source.set_width(20.0);
    assert_eq!(target.text().as_deref(), Some("invalid"));
    assert!(DataValidationErrors::get_has_errors(&target));
    expression.resume_after_reload();
    assert_eq!(target.text().as_deref(), Some("20"));
    assert!(!DataValidationErrors::get_has_errors(&target));
    assert_eq!(source.width(), 20.0);
}

#[test]
fn paused_binding_drains_reentrant_source_changes_without_target_writes() {
    use ferroui_base::{
        data::{BindingMode, ReflectionBinding},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    let target = Border::new();
    let binding = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    let expression = target.bind_binding(Layoutable::width_property(), &*binding);
    let source_for_callback = source.clone();
    let target_for_callback = target.downgrade();
    let _subscription = target.property_changed(move |e| {
        let target_for_callback = target_for_callback.upgrade().unwrap();
        if e.property().id() == Layoutable::width_property().id()
            && target_for_callback.width() == 20.0
        {
            source_for_callback.set_width(30.0);
            target_for_callback.set_current_value(Layoutable::width_property(), 999.0);
        }
    });
    assert!(expression.pause_for_reload());
    source.set_width(20.0);
    expression.resume_after_reload();
    assert_eq!(target.width(), 30.0);
    assert_eq!(source.width(), 30.0);
    target.set_current_value(Layoutable::width_property(), 40.0);
    assert_eq!(source.width(), 40.0);
}

#[test]
fn local_binding_reload_rolls_back_and_retires_subscriptions() {
    use ferroui_base::{
        data::{BindingMode, LocalBindingReload, ReflectionBinding},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let original_source = Border::new();
    original_source.set_width(10.0);
    let next_source = Border::new();
    next_source.set_width(20.0);
    let target = Border::new();
    let original = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(original_source.clone())))
        .with_mode(BindingMode::TwoWay);
    let next = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(next_source.clone())))
        .with_mode(BindingMode::TwoWay);
    let expression = target.bind_binding(Layoutable::width_property(), &*original);
    let old_weak = Rc::downgrade(&expression);
    drop(expression);
    let root = target.clone().upcast();
    for value in 30..40 {
        let mut change =
            LocalBindingReload::prepare(&root, Layoutable::width_property(), Some(&*next), None)
                .unwrap();
        assert_eq!(target.width(), original_source.width());
        change.commit().unwrap();
        assert_eq!(target.width(), 20.0);
        assert_ne!(
            original_source.width(),
            20.0,
            "replacement wrote into old model"
        );
        original_source.set_width(value as f64);
        assert_eq!(
            target.width(),
            20.0,
            "detached original published while paused"
        );
        change.rollback().unwrap();
        assert_eq!(target.width(), value as f64);
        assert_eq!(
            next_source.width(),
            20.0,
            "rollback wrote into replacement model"
        );
    }
    {
        let mut change =
            LocalBindingReload::prepare(&root, Layoutable::width_property(), Some(&*next), None)
                .unwrap();
        change.commit().unwrap();
        // An unfinished transaction must restore the old binding on drop.
    }
    assert_eq!(target.width(), 39.0);
    let mut change =
        LocalBindingReload::prepare(&root, Layoutable::width_property(), Some(&*next), None)
            .unwrap();
    change.commit().unwrap();
    change.finish().unwrap();
    drop(change);
    assert!(
        old_weak.upgrade().is_none(),
        "finished reload retained old expression"
    );
    original_source.set_width(500.0);
    assert_eq!(
        target.width(),
        20.0,
        "retired subscription still updates target"
    );
    target.set_current_value(Layoutable::width_property(), 60.0);
    assert_eq!(next_source.width(), 60.0);
    assert_eq!(original_source.width(), 500.0);
}

#[test]
fn local_binding_reload_removal_restores_invalid_input_and_validation() {
    use ferroui_base::{
        data::{BindingMode, LocalBindingReload, ReflectionBinding},
        threading::Dispatcher,
    };
    use ferroui_controls::{Border, DataValidationErrors, TextBox};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    let target = TextBox::new();
    let binding = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    target.bind_binding(TextBox::text_property(), &*binding);
    target.set_current_value(TextBox::text_property(), Some("invalid".into()));
    let root = target.clone().upcast();
    let mut change =
        LocalBindingReload::prepare(&root, TextBox::text_property(), None, None).unwrap();
    change.commit().unwrap();
    assert!(target.text().is_none());
    assert!(!DataValidationErrors::get_has_errors(&target));
    change.rollback().unwrap();
    assert_eq!(target.text().as_deref(), Some("invalid"));
    assert!(DataValidationErrors::get_has_errors(&target));
    assert_eq!(source.width(), 10.0);
    source.set_width(20.0);
    assert_eq!(target.text().as_deref(), Some("20"));
    assert!(!DataValidationErrors::get_has_errors(&target));

    let mut change =
        LocalBindingReload::prepare(&root, TextBox::text_property(), None, None).unwrap();
    change.commit().unwrap();
    change.finish().unwrap();
    source.set_width(30.0);
    assert!(target.text().is_none());
    assert!(!target.has_local_value_binding(TextBox::text_property()));
}

#[test]
fn local_binding_reload_keeps_replacement_validation_on_finish() {
    use ferroui_base::{
        data::{BindingMode, LocalBindingReload, ReflectionBinding},
        threading::Dispatcher,
    };
    use ferroui_controls::{Border, DataValidationErrors, TextBox};
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    source.set_height(20.0);
    let target = TextBox::new();
    let original = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    let next = ReflectionBinding::new("Height")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    target.bind_binding(TextBox::text_property(), &*original);
    let mut change = LocalBindingReload::prepare(
        &target.clone().upcast(),
        TextBox::text_property(),
        Some(&*next),
        None,
    )
    .unwrap();
    change.commit().unwrap();
    target.set_current_value(TextBox::text_property(), Some("invalid".into()));
    assert!(DataValidationErrors::get_has_errors(&target));
    change.finish().unwrap();
    assert!(
        DataValidationErrors::get_has_errors(&target),
        "retired binding cleared new error"
    );
    assert_eq!(source.width(), 10.0);
    assert_eq!(source.height(), 20.0);
}

#[test]
fn local_binding_reload_rejects_unsupported_and_changed_owners() {
    use ferroui_base::{
        data::{BindingMode, LocalBindingReload, ReflectionBinding, UpdateSourceTrigger},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    let target = Border::new();
    let binding = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    target.bind_binding(Layoutable::width_property(), &*binding);
    let root = target.clone().upcast();
    let unsupported = ReflectionBinding::new("Width")
        .with_source(Some(Rc::new(source.clone())))
        .with_mode(BindingMode::TwoWay);
    unsupported.set_update_source_trigger(UpdateSourceTrigger::Explicit);
    assert!(LocalBindingReload::prepare(
        &root,
        Layoutable::width_property(),
        Some(&*unsupported),
        None,
    )
    .is_err());
    let mut change =
        LocalBindingReload::prepare(&root, Layoutable::width_property(), None, None).unwrap();
    target.set_current_value(Layoutable::width_property(), 10.0);
    assert!(
        change.commit().is_err(),
        "equal-value application write must invalidate preparation"
    );
    assert_eq!(target.width(), 10.0);
    source.set_width(20.0);
    assert_eq!(target.width(), 20.0);

    let mut change =
        LocalBindingReload::prepare(&root, Layoutable::width_property(), None, None).unwrap();
    change.commit().unwrap();
    target.set_width(50.0);
    assert!(change.rollback().is_err());
    assert_eq!(
        target.width(),
        50.0,
        "rollback overwrote application ownership"
    );
    source.set_width(30.0);
    assert_eq!(target.width(), 50.0);
}

#[test]
fn local_binding_reload_can_add_after_removal_and_restore_literal_or_style() {
    use ferroui_base::{
        data::{BindingPriority, LocalBindingReload, ReflectionBinding},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    let target = Border::new();
    let _style =
        target.set_value_with_priority(Layoutable::width_property(), 5.0, BindingPriority::Style);
    let root = target.clone().upcast();
    let binding = ReflectionBinding::new("Width").with_source(Some(Rc::new(source.clone())));
    for local in [None, Some(7.0)] {
        if let Some(value) = local {
            target.set_width(value);
        }
        let mut change =
            LocalBindingReload::prepare(&root, Layoutable::width_property(), Some(&*binding), None)
                .unwrap();
        change.commit().unwrap();
        assert_eq!(target.width(), 10.0);
        change.rollback().unwrap();
        assert_eq!(target.width(), local.unwrap_or(5.0));
        assert!(!target.has_local_value_binding(Layoutable::width_property()));
    }
    let mut install =
        LocalBindingReload::prepare(&root, Layoutable::width_property(), Some(&*binding), None)
            .unwrap();
    install.commit().unwrap();
    install.finish().unwrap();
    let mut remove =
        LocalBindingReload::prepare(&root, Layoutable::width_property(), None, None).unwrap();
    remove.commit().unwrap();
    remove.finish().unwrap();
    assert_eq!(target.width(), 5.0, "removal must reveal underlying style");
    let mut reinstall =
        LocalBindingReload::prepare(&root, Layoutable::width_property(), Some(&*binding), None)
            .unwrap();
    reinstall.commit().unwrap();
    reinstall.finish().unwrap();
    source.set_width(20.0);
    assert_eq!(target.width(), 20.0);
}

#[test]
fn local_binding_reload_unwinds_a_failed_initial_publication() {
    use ferroui_base::{
        data::{BindingOperations, LocalBindingReload, ReflectionBinding},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    source.set_height(20.0);
    let target = Border::new();
    let original = ReflectionBinding::new("Width").with_source(Some(Rc::new(source.clone())));
    let next = ReflectionBinding::new("Height").with_source(Some(Rc::new(source.clone())));
    let expression = target.bind_binding(Layoutable::width_property(), &*original);
    let once = Rc::new(Cell::new(true));
    let target_weak = target.downgrade();
    let _subscription = target.property_changed(move |e| {
        if e.property().id() == Layoutable::width_property().id()
            && target_weak.upgrade().unwrap().width() == 20.0
            && once.replace(false)
        {
            panic!("observer failed");
        }
    });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut change = LocalBindingReload::prepare(
            &target.clone().upcast(),
            Layoutable::width_property(),
            Some(&*next),
            None,
        )
        .unwrap();
        change.commit().unwrap();
    }))
    .is_err());
    assert_eq!(target.width(), 10.0);
    assert!(Rc::ptr_eq(
        &expression,
        &BindingOperations::get_binding_expression_base(&target, Layoutable::width_property())
            .unwrap(),
    ));
    source.set_width(30.0);
    assert_eq!(target.width(), 30.0);
}

#[test]
fn local_binding_reload_does_not_claim_reentrant_application_writes() {
    use ferroui_base::{
        data::{LocalBindingReload, ReflectionBinding},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    source.set_height(20.0);
    let target = Border::new();
    let original = ReflectionBinding::new("Width").with_source(Some(Rc::new(source.clone())));
    let next = ReflectionBinding::new("Height").with_source(Some(Rc::new(source.clone())));
    target.bind_binding(Layoutable::width_property(), &*original);
    let target_weak = target.downgrade();
    let _subscription = target.property_changed(move |e| {
        if e.property().id() == Layoutable::width_property().id() {
            let target = target_weak.upgrade().unwrap();
            if target.width() == 20.0 {
                // Even the same value changes ownership.
                target.set_width(20.0);
            }
        }
    });
    let mut change = LocalBindingReload::prepare(
        &target.clone().upcast(),
        Layoutable::width_property(),
        Some(&*next),
        None,
    )
    .unwrap();
    assert!(change.commit().is_err());
    assert!(change.rollback().is_err());
    assert_eq!(target.width(), 20.0);
    source.set_width(30.0);
    assert_eq!(target.width(), 20.0);
}

#[test]
fn local_binding_reload_snapshots_source_updates_at_commit() {
    use ferroui_base::{
        data::{LocalBindingReload, ReflectionBinding},
        layout::Layoutable,
        threading::Dispatcher,
    };
    use ferroui_controls::Border;
    let _scope = Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    let source = Border::new();
    source.set_width(10.0);
    source.set_height(20.0);
    let target = Border::new();
    let original = ReflectionBinding::new("Width").with_source(Some(Rc::new(source.clone())));
    let next = ReflectionBinding::new("Height").with_source(Some(Rc::new(source.clone())));
    target.bind_binding(Layoutable::width_property(), &*original);
    let mut change = LocalBindingReload::prepare(
        &target.clone().upcast(),
        Layoutable::width_property(),
        Some(&*next),
        None,
    )
    .unwrap();
    source.set_width(30.0);
    change.commit().unwrap();
    assert_eq!(target.width(), 20.0);
    change.rollback().unwrap();
    assert_eq!(
        target.width(),
        30.0,
        "rollback restored a stale preparation snapshot"
    );
}
