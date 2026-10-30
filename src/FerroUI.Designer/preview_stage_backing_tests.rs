use super::*;
use ferroui_base::media::Color;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{TextBox, Window};

const URI: &str = "ferres://FerroUI.Designer.Tests/OverlayBacking.xaml";

fn sync(
    board: &mut PreviewStageBoard,
    document: &Document,
    options: PreviewHostOptions,
    options_epoch: u64,
) {
    board
        .sync_sources(
            URI,
            vec![PreviewStageSource {
                uri: URI.into(),
                document: document.clone(),
                pending: false,
            }],
            options,
            options_epoch,
        )
        .unwrap();
}

fn setup() -> (PreviewStageBoard, PreviewStageId, Document, Ref<Window>) {
    let document = Document::new(
        r#"<TextBox xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Tests.OverlayBacking" Text="Original"/>"#,
    )
    .unwrap();
    let mut board = PreviewStageBoard::new();
    sync(&mut board, &document, PreviewHostOptions::default(), 1);
    let mut configuration =
        PreviewStageConfiguration::new("Overlay", URI, PreviewEnvironment::default());
    configuration.environment.theme = PreviewTheme::Inherit;
    configuration.mode = PreviewStageMode::Live;
    let id = board.add(configuration).unwrap();
    let window = Window::new();
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    window.set_width(900.0);
    window.set_height(600.0);
    window.set_content(Some(Control::boxed(board.control(id).unwrap())));
    window.show();
    window.update_layout();
    (board, id, document, window)
}

fn rendered(board: &PreviewStageBoard, id: PreviewStageId) -> &RenderedStage {
    board.stages[board.index(id).unwrap()]
        .rendered
        .as_ref()
        .unwrap()
}

fn color(backing: &Border) -> Option<Color> {
    backing
        .background()
        .map(|brush| brush.as_solid_color_brush().unwrap().color())
}

#[test]
fn overlay_backing_tracks_inherited_theme_without_rebuilding_or_changing_authority() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (mut board, id, document, window) = setup();
    let input = rendered(&board, id)
        .preview
        .root()
        .unwrap()
        .cast::<TextBox>()
        .unwrap();
    let backing = rendered(&board, id).backing.clone();
    input.set_text(Some("Retained native input"));
    let epoch = board.epoch();
    let stage_epoch = board.stage(id).unwrap().epoch;
    let manifest = serde_json::to_string(&board.manifest()).unwrap();
    assert_eq!(color(&backing), Some(Color::parse("#FFFFFFFF").unwrap()));

    board.set_prototype_overlay_backing(id, true).unwrap();
    assert!(backing.background().is_none());
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    window.update_layout();
    assert_eq!(input.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert!(backing.background().is_none());

    board.set_prototype_overlay_backing(id, false).unwrap();
    assert_eq!(color(&backing), Some(Color::parse("#FF202020").unwrap()));
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    window.update_layout();
    assert_eq!(color(&backing), Some(Color::parse("#FFFFFFFF").unwrap()));
    assert_eq!(rendered(&board, id).backing, backing);
    assert_eq!(
        rendered(&board, id).preview.root().unwrap(),
        input.clone().upcast::<Control>()
    );
    assert_eq!(input.text().as_deref(), Some("Retained native input"));
    assert_eq!(board.epoch(), epoch);
    assert_eq!(board.stage(id).unwrap().epoch, stage_epoch);
    assert_eq!(serde_json::to_string(&board.manifest()).unwrap(), manifest);
    assert_eq!(document.revision(), 0);
    assert!(board.set_prototype_overlay_backing(u64::MAX, true).is_err());
    assert_eq!(board.epoch(), epoch);
    window.close();
}

#[test]
fn overlay_backing_survives_failed_candidates_and_refresh_but_is_not_duplicated() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (mut board, id, document, window) = setup();
    let accepted = rendered(&board, id).preview.root().unwrap();
    let backing = rendered(&board, id).backing.clone();
    board.set_prototype_overlay_backing(id, true).unwrap();
    let mut failing = PreviewHostOptions::default();
    failing.register_root(
        "Tests.OverlayBacking",
        Rc::new(|_| Err("Expected native construction failure".into())),
    );
    sync(&mut board, &document, failing, 2);
    assert!(matches!(
        board.stage(id).unwrap().status,
        PreviewStageStatus::Failed { .. }
    ));
    assert_eq!(rendered(&board, id).preview.root().unwrap(), accepted);
    assert_eq!(rendered(&board, id).backing, backing);
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    window.update_layout();
    assert!(backing.background().is_none());
    board.set_prototype_overlay_backing(id, false).unwrap();
    assert_eq!(color(&backing), Some(Color::parse("#FF202020").unwrap()));
    board.set_prototype_overlay_backing(id, true).unwrap();

    sync(&mut board, &document, PreviewHostOptions::default(), 3);
    assert!(matches!(
        board.stage(id).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    assert_ne!(rendered(&board, id).preview.root().unwrap(), accepted);
    assert!(rendered(&board, id).backing.background().is_none());
    let duplicate = board.duplicate(id).unwrap();
    assert!(rendered(&board, duplicate).backing.background().is_some());
    board.set_prototype_overlay_backing(id, false).unwrap();
    assert_eq!(
        color(&rendered(&board, id).backing),
        Some(Color::parse("#FF202020").unwrap())
    );
    window.close();
}
