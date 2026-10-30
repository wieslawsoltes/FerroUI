use crate::{
    DesignerWorkspace, PreviewEnvironment, PreviewStageConfiguration, PreviewStageMatrixEditor,
    PreviewStageStatus,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Point, Ref, Size};
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::{Border, Button, CheckBox, Control, FlyoutPresenter, TextBlock, Window};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_core::Document;
use std::cell::Cell;
use std::rc::Rc;

const SOURCE: &str = include_str!("../../docs/designer/samples/PreviewStages.xaml");
const URI: &str = "memory://matrix-sample.xaml";

fn services() -> TestServices {
    let mut services = TestServices::styled_window();
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1400.0, 1000.0);
        native.max_auto_size_hint.set(Size::new(2000.0, 1600.0));
        native.setup_create_popup(|_| None);
        native.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        native
    }));
    services
}

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing matrix control {name}"))
}

fn check(editor: &PreviewStageMatrixEditor, name: &str, checked: bool) {
    named::<CheckBox>(&editor.control(), name).set_is_checked(Some(checked));
}

struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
}
impl Fixture {
    fn new() -> Self {
        let workspace = DesignerWorkspace::with_uri(SOURCE, URI.into()).unwrap();
        workspace.set_timeline_visible(false);
        let window = Window::new();
        window.set_width(1400.0);
        window.set_height(1000.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        window.update_layout();
        workspace.set_preview_stages_visible(true).unwrap();
        window.update_layout();
        Self { workspace, window }
    }
    fn editor(&self) -> Rc<PreviewStageMatrixEditor> {
        let stage = self.workspace.preview_stages()[0].id;
        let anchor = named::<Control>(
            &self.workspace.preview_stage_view().unwrap().control(),
            "StageCompareMatrix",
        );
        self.workspace
            .show_preview_stage_matrix_editor(stage, &anchor)
            .unwrap();
        self.window.update_layout();
        self.workspace.preview_stage_matrix_editor().unwrap()
    }
    fn layout(&self) {
        self.window.update_layout();
        self.workspace.refresh(true);
        self.window.update_layout();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn matrix_editor_counts_detached_choices_and_rejects_capacity_before_host_apply() {
    let _app = UnitTestApplication::start(services());
    let document = Document::new(SOURCE).unwrap();
    let base = PreviewStageConfiguration::new("Screen", URI, PreviewEnvironment::default());
    let editor = PreviewStageMatrixEditor::new(base, document, 3);
    let applied = Rc::new(Cell::new(0));
    let calls = applied.clone();
    editor.on_apply(move |_| {
        calls.set(calls.get() + 1);
        Ok(())
    });
    for name in ["MatrixSize0", "MatrixSize1", "MatrixTheme0", "MatrixTheme1"] {
        check(&editor, name, true);
    }
    assert_eq!(editor.draft().count().unwrap(), 4);
    assert!(named::<TextBlock>(&editor.control(), "MatrixSummary")
        .text()
        .unwrap()
        .contains("4 combinations"));
    assert!(!named::<Button>(&editor.control(), "MatrixApply").is_enabled());
    assert!(editor.apply().is_err());
    assert_eq!(applied.get(), 0);
    check(&editor, "MatrixSize1", false);
    assert_eq!(editor.draft().count().unwrap(), 2);
    assert!(named::<Button>(&editor.control(), "MatrixApply").is_enabled());
    assert!(named::<TextBlock>(&editor.control(), "MatrixPlan")
        .text()
        .unwrap()
        .contains("Light"));
    editor.cancel();
    assert!(editor.apply().is_err());
    assert_eq!(applied.get(), 0);
}

#[test]
fn native_matrix_editor_applies_two_sizes_themes_and_real_states_without_authoring() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    f.workspace.session().borrow_mut().set_read_only(true);
    let document = f
        .workspace
        .session()
        .borrow()
        .document()
        .snapshot_without_history();
    let owner = document.ast().find_name("PreviewScene").unwrap();
    f.workspace.session().borrow_mut().select([owner]);
    let selection = f.workspace.session().borrow().selection().to_vec();
    let before = f.workspace.preview_stage_manifest();
    let editor = f.editor();
    for name in ["MatrixSize0", "MatrixSize1", "MatrixTheme0", "MatrixTheme1"] {
        check(&editor, name, true);
    }
    let groups = VisualStateGroup::read(&document);
    let group_index = groups
        .iter()
        .position(|group| group.owner == owner && group.name == "Screen")
        .unwrap();
    for name in ["Loading", "Content"] {
        let index = groups[group_index]
            .states
            .iter()
            .position(|state| state.name == name)
            .unwrap();
        check(&editor, &format!("MatrixState{group_index}_{index}"), true);
    }
    assert_eq!(editor.draft().count().unwrap(), 8);
    assert_eq!(f.workspace.preview_stage_manifest(), before);
    assert!(named::<TextBlock>(&editor.control(), "MatrixSummary")
        .text()
        .unwrap()
        .contains("8 new stages"));
    editor.apply().unwrap();
    f.layout();
    let stages = f.workspace.preview_stages();
    assert_eq!(stages.len(), 9);
    for stage in stages.iter().skip(1) {
        assert!(
            matches!(stage.status, PreviewStageStatus::Ready { .. }),
            "{}: {:?}",
            stage.configuration.name,
            stage.status
        );
        let selected = &stage.configuration.states[0].state;
        let host = f.workspace.preview_stage_control(stage.id).unwrap();
        for (state, panel) in [("Loading", "LoadingPanel"), ("Content", "ContentPanel")] {
            let panel = host
                .get_visual_descendants()
                .filter_map(|visual| visual.cast::<Border>())
                .find(|control| control.name().as_deref() == Some(panel))
                .unwrap();
            assert_eq!(panel.is_visible(), selected == state);
        }
    }
    assert_eq!(f.workspace.session().borrow().selection(), selection);
    assert_eq!(f.workspace.session().borrow().document().source(), SOURCE);
    assert_eq!(f.workspace.session().borrow().document().revision(), 0);
    assert!(!f.workspace.session().borrow().can_undo());
    assert!(
        editor.apply().is_err(),
        "A closed editor cannot add its matrix twice"
    );
}

#[test]
fn matrix_editor_cancel_and_stale_source_or_stage_reject_without_losing_drafts() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    let before = f.workspace.preview_stage_manifest();
    let editor = f.editor();
    check(&editor, "MatrixTheme1", true);
    named::<Button>(&editor.control(), "MatrixCancel")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(f.workspace.preview_stage_manifest(), before);
    assert!(editor.apply().is_err());

    let editor = f.editor();
    check(&editor, "MatrixTheme1", true);
    let id = f.workspace.preview_stages()[0].id;
    let mut config = f.workspace.preview_stage(id).unwrap().configuration;
    config.name = "Changed after opening editor".into();
    f.workspace.update_preview_stage(id, config).unwrap();
    let accepted = f.workspace.preview_stage_manifest();
    assert!(editor.apply().is_err());
    assert_eq!(f.workspace.preview_stage_manifest(), accepted);

    let editor = f.editor();
    check(&editor, "MatrixTheme0", true);
    let draft = SOURCE.replace("PreviewScene", "EditedScene");
    f.workspace.source_editor().set_text(Some(&draft));
    assert!(editor.apply().is_err());
    assert_eq!(f.workspace.preview_stage_manifest(), accepted);
    assert_eq!(
        f.workspace.source_editor().text().as_deref(),
        Some(draft.as_str())
    );
    assert_eq!(f.workspace.session().borrow().document().source(), draft);
}

#[test]
fn native_matrix_footer_stays_visible_below_scrolling_choices() {
    let _app = UnitTestApplication::start(services());
    let f = Fixture::new();
    f.window.set_width(980.0);
    f.layout();
    let editor = f.editor();
    let presenter = editor
        .control()
        .get_visual_ancestors()
        .find_map(|visual| visual.cast::<FlyoutPresenter>())
        .expect("Native matrix popup presenter");
    let apply = named::<Button>(&editor.control(), "MatrixApply");
    let scroll = named::<Control>(&editor.control(), "MatrixScroll");
    let footer = named::<Control>(&editor.control(), "MatrixFooter");
    let apply_position = apply.translate_point(Point::default(), &presenter).unwrap();
    assert!(
        apply_position.x >= 0.0
            && apply_position.x + apply.bounds().width <= presenter.bounds().width + 0.5
    );
    assert!(
        apply_position.y >= 0.0
            && apply_position.y + apply.bounds().height <= presenter.bounds().height + 0.5
    );
    let footer_position = footer
        .translate_point(Point::default(), &editor.control())
        .unwrap();
    let scroll_position = scroll
        .translate_point(Point::default(), &editor.control())
        .unwrap();
    assert!(footer_position.y >= scroll_position.y + scroll.bounds().height);
    editor.cancel();
    assert_eq!(f.workspace.preview_stages().len(), 1);
}
