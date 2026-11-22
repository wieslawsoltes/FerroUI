use crate::{
    DesignerSession, PreviewEnvironment, PreviewHostOptions, PreviewStageBoard,
    PreviewStageConfiguration, PreviewStageSource, PreviewStageStatus, PreviewStatus,
};
use ferroui_base::{
    platform::{AssetLoader, IPlatformRenderInterface},
    utilities::UriRef,
    FerroLocator, LocatorExtensions,
};
use ferroui_controls::{
    testing::{TestServices, UnitTestApplication},
    ContentControl,
};
use ferroui_designer_core::Document;
use ferroui_designer_project::{file_uri, local_preview_sources_with_resources, MarkupCatalog};
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

const ADDRESS: &str = "ferres://studio.asset.preview.tests/Assets/Approval%20badge.png";
const BADGE: &str = "Assets/Approval badge.png";
// A 6 × 3 RGB raster with a solid fill.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x06\
\x00\x00\x00\x03\x08\x02\x00\x00\x00\x3f\x63\xe9\xac\x00\x00\x00\x11IDA\
\x54\x78\x9c\x63\x30\x0e\x9a\x8a\x86\x18\x88\x12\x02\x00\x2e\x8d\x13\xd5\x00\x3f\
\x23\xdf\x00\x00\x00\x00IEND\xae\x42\x60\x82";

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn packaged_asset_preview_is_scoped_and_missing_inputs_keep_the_last_stage() {
    let _locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    let services = TestServices {
        render_interface: FerroLocator::current().get_service::<dyn IPlatformRenderInterface>(),
        ..TestServices::styled_window()
    };
    let _app = UnitTestApplication::start(services);
    let root =
        Fixture(std::env::temp_dir().join(format!("ferroui-native-asset-{}", std::process::id())));
    let out = root.0.join("out");
    let path = root.0.join(BADGE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::create_dir_all(&out).unwrap();
    let manifest = root.0.join("Cargo.toml");
    fs::write(
        &manifest,
        "[package]\nname='asset-preview'\nversion='0.0.0'\n[lib]\npath='lib.rs'\n",
    )
    .unwrap();
    fs::write(root.0.join("lib.rs"), "").unwrap();
    fs::write(&path, PNG).unwrap();
    fs::write(out.join("preview.xamlmeta"), r#"{"format":2,"name":"Studio.Asset.Preview.Tests","crate_name":"asset_preview","types":[]}"#).unwrap();
    fs::write(out.join("packaged-assets.json"), serde_json::json!({
        "version": 1, "manifest": manifest, "assembly":"Studio.Asset.Preview.Tests", "crate_name":"asset_preview",
        "assets":[{"key": format!("/{BADGE}"), "source": BADGE}],
    }).to_string()).unwrap();

    let catalog = Arc::new(MarkupCatalog::load(&[out]).unwrap());
    let resources = Arc::new(catalog.compiled_resources(&root.0).unwrap());
    let source = format!(
        "<Image xmlns='https://github.com/ferroui' Source='{ADDRESS}' Width='120' Height='60' />"
    );
    let uri = file_uri(root.0.join("View.xaml")).unwrap();
    let asset = UriRef::parse(ADDRESS).unwrap();
    assert!(!AssetLoader::exists(&asset, None));
    let empty = SourceSnapshot::new(Default::default(), Default::default());
    let scoped = crate::PreviewAssets::new(resources.packaged_assets()).apply(empty.clone());
    scoped.run(|| {
        assert!(AssetLoader::exists(&asset, None));
        let folder = UriRef::parse("ferres://studio.asset.preview.tests/Assets").unwrap();
        assert_eq!(AssetLoader::get_assets(&folder, None)[0].as_str(), ADDRESS);
        empty.run(|| assert!(!AssetLoader::exists(&asset, None)));
        assert!(AssetLoader::exists(&asset, None));
    });
    let mut session = DesignerSession::with_preview_fixture(
        &source,
        ContentControl::new(),
        uri.clone(),
        PreviewHostOptions::default(),
        None,
    )
    .unwrap();
    session.set_workspace_preview_root(root.0.clone());
    session.set_workspace_preview_resources(Some(resources.clone()), true);
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    assert!(session.open_view_preview().is_ok());
    #[cfg(feature = "mcp")]
    author_published_raster(resources.clone(), &root.0);

    let workspace =
        crate::DesignerWorkspace::new("<StackPanel xmlns='https://github.com/ferroui' />").unwrap();
    workspace
        .set_solution(ferroui_designer_project::Solution::discover(&root.0).unwrap())
        .unwrap();
    workspace
        .language_service()
        .set_project_catalog(root.0.clone(), catalog)
        .unwrap();
    let authoring_uri = file_uri(root.0.join("Workspace.xaml")).unwrap();
    workspace
        .open_document(source.clone(), authoring_uri)
        .unwrap();
    let toolbox = workspace.asset_toolbox().unwrap();
    assert!(toolbox.set_category(Some("Packaged images")));
    assert_eq!(
        toolbox.visible_asset_ids(),
        ["ferres://Studio.Asset.Preview.Tests/Assets/Approval%20badge.png"]
    );
    assert!(workspace.session().borrow().can_manipulate());

    let document = Document::new(&source).unwrap();
    let sync = |board: &mut PreviewStageBoard| {
        let local = local_preview_sources_with_resources(
            Some(&root.0),
            &uri,
            &document,
            &[],
            Some(&resources),
        );
        board
            .sync_sources_with_local_includes(
                &uri,
                vec![PreviewStageSource {
                    uri: uri.clone(),
                    document: document.clone(),
                    pending: false,
                }],
                PreviewHostOptions::default(),
                1,
                &BTreeMap::from([(uri.as_str(), &local)]),
            )
            .unwrap();
    };
    let mut board = PreviewStageBoard::new();
    sync(&mut board);
    let stage = board
        .add(PreviewStageConfiguration::new(
            "Approval badge",
            &uri,
            PreviewEnvironment::default(),
        ))
        .unwrap();
    let accepted = board.stage(stage).unwrap();
    assert!(
        matches!(accepted.status, PreviewStageStatus::Ready { .. }),
        "{:?}",
        accepted.status
    );
    let dependency = file_uri(path.canonicalize().unwrap()).unwrap();
    assert!(board.asset_dependencies().contains(&dependency));
    assert!(!AssetLoader::exists(&asset, None));

    fs::remove_file(&path).unwrap();
    sync(&mut board);
    let missing = board.stage(stage).unwrap();
    assert!(
        matches!(missing.status, PreviewStageStatus::Failed { .. }),
        "{:?}",
        missing.status
    );
    assert_eq!(missing.rendered_revision, accepted.rendered_revision);
    assert!(board.asset_dependencies().contains(&dependency));
    fs::write(&path, PNG).unwrap();
    sync(&mut board);
    let repaired = board.stage(stage).unwrap();
    assert!(
        matches!(repaired.status, PreviewStageStatus::Ready { .. }),
        "{:?}",
        repaired.status
    );
    assert!(repaired.epoch > missing.epoch);
}

#[cfg(feature = "mcp")]
fn author_published_raster(
    resources: Arc<ferroui_designer_project::CompiledResources>,
    root: &std::path::Path,
) {
    use ferroui_mcp::{Permissions, UiToolProvider};
    use serde_json::json;
    use std::{cell::RefCell, rc::Rc};

    let source = "<StackPanel xmlns='https://github.com/ferroui'><Image Name='Badge' Width='120' /></StackPanel>";
    let mut session = DesignerSession::with_preview_fixture(
        source,
        ContentControl::new(),
        file_uri(root.join("Authoring.xaml")).unwrap(),
        PreviewHostOptions::default(),
        None,
    )
    .unwrap();
    session.set_workspace_preview_root(root.into());
    session.set_workspace_preview_resources(Some(resources), true);
    let parent = session.document().ast().root();
    let node = session.document().ast().find_name("Badge").unwrap();
    let session = Rc::new(RefCell::new(session));
    let provider = crate::mcp::DesignerMcp::new(&session);
    let read = Permissions::default();
    let write = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let assets = provider.call("designer_toolbox", &json!({}), read).unwrap();
    let published = assets["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| {
            asset["category"] == "Packaged images" && asset["title"] == "Approval badge.png"
        })
        .unwrap();
    let address = published["id"].as_str().unwrap();
    assert_eq!(
        address,
        "ferres://Studio.Asset.Preview.Tests/Assets/Approval%20badge.png"
    );
    let assign = json!({"expected_revision":0,"node":node,"project_asset":address});
    assert!(provider
        .call("designer_set_image_source", &assign, read)
        .is_err());
    let mut forged = assign.clone();
    forged["property"] = json!("Width");
    assert!(provider
        .call("designer_set_image_source", &forged, write)
        .is_err());
    let result = provider
        .call("designer_set_image_source", &assign, write)
        .unwrap();
    assert_eq!(result["revision"], 1);
    assert!(session.borrow().document().source().contains(address));
    assert!(provider
        .call("designer_set_image_source", &assign, write)
        .is_err());
    provider
        .call("designer_undo", &json!({"expected_revision":1}), write)
        .unwrap();
    assert_eq!(session.borrow().document().source(), source);

    let insert = json!({"expected_revision":2,"asset":address,"anchor":parent,"position":"inside"});
    provider.call("designer_place", &insert, write).unwrap();
    assert!(session.borrow().document().source().contains(address));
    assert!(session.borrow().can_manipulate());
    provider
        .call("designer_undo", &json!({"expected_revision":3}), write)
        .unwrap();
    assert_eq!(session.borrow().document().source(), source);
}
