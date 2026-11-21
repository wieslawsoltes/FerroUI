use super::DesignerWorkspace;
use ferroui_base::{Ref, layout::Orientation, media::FontFamily};
use ferroui_controls::{
    ComboBox, Control, StackPanel, TabControl, TabItem, TextBox, automation::AutomationProperties,
};
use ferroui_designer_language::ReviewedDocument;
use ferroui_designer_project::file_path;
use std::rc::Rc;

pub(in super::super) fn source_diffs(
    workspace: &DesignerWorkspace,
    body: &Ref<StackPanel>,
    active: &str,
    documents: Rc<Vec<ReviewedDocument>>,
) {
    let file = ComboBox::new();
    AutomationProperties::set_name(&file, Some("File to review"));
    for document in documents.iter() {
        file.items()
            .add(Some(Rc::new(relative(workspace, &document.edit.uri))));
    }
    let tabs = TabControl::new();
    let before = source_tab(&tabs, "Original source");
    let after = source_tab(&tabs, "Proposed source");
    let choose = documents
        .iter()
        .position(|doc| doc.edit.uri == active)
        .unwrap_or_default();
    file.set_selected_index(choose as i32);
    let show = {
        let (file, before, after) = (file.downgrade(), before.downgrade(), after.downgrade());
        move || {
            let (Some(file), Some(before), Some(after)) =
                (file.upgrade(), before.upgrade(), after.upgrade())
            else {
                return;
            };
            if let Some(document) = documents.get(file.selected_index().max(0) as usize) {
                before.set_text(Some(&document.before));
                after.set_text(Some(&document.after));
            }
        }
    };
    show();
    file.selection_changed(move |_, _| show());
    body.children().add(file.upcast::<Control>());
    body.children().add(tabs.upcast::<Control>());
}

fn source_tab(tabs: &Ref<TabControl>, label: &str) -> Ref<TextBox> {
    let editor = TextBox::new();
    editor.set_is_read_only(true);
    editor.set_accepts_return(true);
    editor.set_height(244.0);
    editor.set_font_family(FontFamily::from("monospace"));
    editor.set_font_size(11.0);
    AutomationProperties::set_name(&editor, Some(label));
    let tab = TabItem::new();
    tab.set_header(Some(Rc::new(label.to_owned())));
    tab.set_content(Some(Control::boxed(editor.clone())));
    tabs.items().add(Some(Control::boxed(tab)));
    editor
}

pub(in super::super) fn panel(workspace: &DesignerWorkspace) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.set_width((workspace.root.bounds().width - 40.0).clamp(300.0, 760.0));
    panel
}

pub(in super::super) fn toolbar() -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(6.0);
    row
}

pub(in super::super) fn relative(workspace: &DesignerWorkspace, uri: &str) -> String {
    let file = file_path(uri);
    let solution = workspace.solution();
    file.as_ref()
        .ok()
        .and_then(|file| file.strip_prefix(&solution.as_ref()?.root).ok())
        .map_or_else(
            || uri.into(),
            |path| path.to_string_lossy().replace('\\', "/"),
        )
}
