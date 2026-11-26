use super::*;
use crate::{PreviewStatus, mcp::DesignerMcp};
use ferroui_base::{Ref, interactivity::RoutedEventArgs};
use ferroui_controls::{
    Button, Control, Flyout, Window,
    automation::AutomationProperties,
    testing::{TestServices, UnitTestApplication},
};
use ferroui_designer_core::SourceSpan;
use ferroui_designer_project::{Solution, file_uri};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const XAML: &str = concat!(
    "<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\r\n",
    "  <Grid.Resources>\r\n    <!-- Preserve palette 🎨 -->\r\n    <Color x:Key='Ink'>#245783</Color>\r\n",
    "    <SolidColorBrush x:Key='Card' Color='{StaticResource Ink}'/>\r\n  </Grid.Resources>\r\n",
    "  <Border Background='{StaticResource Card}' Padding='24'><TextBlock Text='Customer account'/></Border>\r\n</Grid>\r\n"
);

#[test]
fn studio_mcp_previews_extraction_and_undoes_the_created_file_in_one_step() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (files, workspace) = Files::open();
    let provider = DesignerMcp::with_workspace(&workspace);
    let write = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let session = workspace.session();
    let node = session.borrow().document().ast().root();
    let revision = || json!(session.borrow().document().revision());
    let file_name = "Palette & cards.xaml";
    let target = files.root.join(file_name);
    let read = json!({"expected_revision":revision(),"node":node,"file_name":file_name});
    let preview = provider
        .call(
            "designer_dictionary_extraction",
            &read,
            Permissions::default(),
        )
        .unwrap();
    assert!(
        preview["file"]["source"]
            .as_str()
            .unwrap()
            .contains("Preserve palette 🎨")
    );
    assert!(!target.exists());
    let mut apply = read;
    apply["review"] = json!("bad review");
    assert!(
        provider
            .call("designer_extract_dictionary", &apply, write)
            .unwrap_err()
            .contains("extraction changed")
    );
    assert!(!target.exists());
    assert_eq!(session.borrow().authored_source(), XAML);
    apply["review"] = preview["review"].clone();
    assert!(
        provider
            .call(
                "designer_extract_dictionary",
                &apply,
                Permissions::default()
            )
            .is_err()
    );
    let applied = provider
        .call("designer_extract_dictionary", &apply, write)
        .unwrap();
    assert_eq!(applied, preview);
    let modified = session.borrow().authored_source().to_owned();
    assert!(modified.contains("ResourceInclude Source=\"Palette%20%26%20cards.xaml\""));
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        applied["file"]["source"].as_str().unwrap()
    );
    assert_eq!(
        fs::read_to_string(files.root.join("View.xaml")).unwrap(),
        XAML
    );
    assert!(
        matches!(
            session.borrow().preview_status(),
            PreviewStatus::Ready { .. }
        ),
        "{:?}",
        session.borrow().preview_status()
    );
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), XAML);
    assert!(!target.exists());
    fs::write(&target, "A file created outside the editor").unwrap();
    assert!(
        provider
            .call(
                "designer_redo",
                &json!({"expected_revision":revision()}),
                write
            )
            .is_err()
    );
    assert_eq!(session.borrow().authored_source(), XAML);
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "A file created outside the editor"
    );
    fs::remove_file(&target).unwrap();
    provider
        .call(
            "designer_redo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), modified);
    assert!(matches!(
        session.borrow().preview_status(),
        PreviewStatus::Ready { .. }
    ));
    fs::write(&target, "Different external contents").unwrap();
    assert!(
        provider
            .call(
                "designer_undo",
                &json!({"expected_revision":revision()}),
                write
            )
            .is_err()
    );
    assert_eq!(session.borrow().authored_source(), modified);
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "Different external contents"
    );
}

#[test]
fn studio_source_toolbar_reviews_before_creating_the_dictionary() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (files, workspace) = Files::open();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    let caret = XAML.find("SolidColorBrush").unwrap();
    workspace.select_source_span(SourceSpan::new(caret, caret));
    window.update_layout();
    let anchor = workspace
        .control()
        .get_control::<Button>("ExtractResourceDictionary");
    let flyout = anchor
        .flyout()
        .and_then(|value| value.cast::<Flyout>())
        .unwrap();
    let destination = files.root.join("View.Resources.xaml");
    let activate = |name: &str| {
        let content = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
        let button = content
            .get_logical_descendants()
            .find(|control| AutomationProperties::get_name(control).as_deref() == Some(name))
            .and_then(|control| control.cast::<Button>())
            .unwrap();
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    };
    flyout.show_at(&anchor);
    activate("Review extraction");
    assert!(!destination.exists());
    assert_eq!(workspace.session().borrow().authored_source(), XAML);
    activate("Cancel extraction");
    assert!(!destination.exists());
    flyout.show_at(&anchor);
    activate("Review extraction");
    activate("Apply extraction");
    assert!(destination.is_file());
    assert!(
        workspace
            .session()
            .borrow()
            .authored_source()
            .contains("View.Resources.xaml")
    );
    workspace.source_history(false).unwrap();
    assert_eq!(workspace.session().borrow().authored_source(), XAML);
    assert!(!destination.exists());
    window.close();
}

struct Files {
    root: PathBuf,
}

impl Files {
    fn open() -> (Self, Rc<DesignerWorkspace>) {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ferroui-studio-dictionary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let path = root.join("View.xaml");
        fs::write(&path, XAML).unwrap();
        let workspace = DesignerWorkspace::with_uri(XAML, file_uri(&path).unwrap()).unwrap();
        workspace
            .set_solution(Solution::discover(&root).unwrap())
            .unwrap();
        (Self { root }, workspace)
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
