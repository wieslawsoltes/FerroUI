use super::{DesignerWorkspace, RUST_MOVE_REVIEW};
use crate::{
    icons,
    workspace::inspector_actions::{add_button, text, SOURCE},
};
use ferroui_base::{layout::Orientation, media::FontFamily, Ref};
use ferroui_controls::{
    automation::AutomationProperties, Button, ComboBox, Control, Flyout, PlacementMode, StackPanel,
    TabControl, TabItem, TextBox,
};
use ferroui_designer_language::FileMovePlan;
use ferroui_designer_project::file_path;
use std::rc::Rc;

impl DesignerWorkspace {
    pub(crate) fn connect_file_move(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("MoveXamlFile");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Top);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let owner = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) {
                    choose_destination(&workspace, &popup);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn choose_destination(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>) {
    if workspace.pending_source.get() {
        workspace.apply_source();
    }
    let body = panel(workspace);
    flyout.set_content(Some(Control::boxed(body.clone())));
    let from = workspace.current_xaml_file();
    let from = match from {
        Ok(uri) => uri,
        Err(message) => {
            body.children().add(text(&message).upcast::<Control>());
            return;
        }
    };
    body.children()
        .add(text("Destination in the project").upcast::<Control>());
    let row = toolbar();
    let input = TextBox::new();
    input.set_min_width(280.0);
    input.set_text(Some(&relative(workspace, &from)));
    AutomationProperties::set_name(&input, Some("Destination XAML path"));
    row.children().add(input.clone().upcast::<Control>());
    let status = text("");
    let message = status.downgrade();
    let owner = Rc::downgrade(workspace);
    let popup = flyout.downgrade();
    let path = input.downgrade();
    add_button(&row, SOURCE, "Review file move", move || {
        let (Some(workspace), Some(popup), Some(input)) =
            (owner.upgrade(), popup.upgrade(), path.upgrade())
        else {
            return;
        };
        let destination = input.text().unwrap_or_default().trim().to_owned();
        match workspace.plan_project_file_move(&destination) {
            Ok(plan) => show_review(&workspace, &popup, destination, Rc::new(plan)),
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error));
                }
            }
        }
    });
    body.children().add(row.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
}

fn show_review(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    destination: String,
    plan: Rc<FileMovePlan>,
) {
    let body = panel(workspace);
    let title = format!(
        "{}  →  {}",
        relative(workspace, &plan.from),
        relative(workspace, &plan.to)
    );
    body.children().add(text(&title).upcast::<Control>());
    let file = ComboBox::new();
    AutomationProperties::set_name(&file, Some("File to review"));
    for document in &plan.documents {
        file.items()
            .add(Some(Rc::new(relative(workspace, &document.edit.uri))));
    }
    let tabs = TabControl::new();
    let before = source_tab(&tabs, "Original source");
    let after = source_tab(&tabs, "Proposed source");
    let choose = plan
        .documents
        .iter()
        .position(|doc| doc.edit.uri == plan.from)
        .unwrap_or_default();
    file.set_selected_index(choose as i32);
    let show = {
        let plan = plan.clone();
        let (file, before, after) = (file.downgrade(), before.downgrade(), after.downgrade());
        move || {
            let (Some(file), Some(before), Some(after)) =
                (file.upgrade(), before.upgrade(), after.upgrade())
            else {
                return;
            };
            if let Some(document) = plan.documents.get(file.selected_index().max(0) as usize) {
                before.set_text(Some(&document.before));
                after.set_text(Some(&document.after));
            }
        }
    };
    show();
    file.selection_changed(move |_, _| show());
    body.children().add(file.upcast::<Control>());
    body.children().add(tabs.upcast::<Control>());
    let actions = toolbar();
    let needs_rust = !plan.generated_rust.is_empty();
    let status = text(if needs_rust { RUST_MOVE_REVIEW } else { "" });
    status.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    if !needs_rust {
        let popup = flyout.downgrade();
        let owner = Rc::downgrade(workspace);
        let message = status.downgrade();
        add_button(&actions, icons::CHECK, "Apply file move", move || {
            let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) else {
                return;
            };
            match workspace.apply_project_file_move(&destination, &plan.review) {
                Ok(_) => popup.hide(),
                Err(error) => {
                    if let Some(message) = message.upgrade() {
                        message.set_text(Some(&error));
                    }
                }
            }
        });
    }
    let popup = flyout.downgrade();
    add_button(&actions, icons::CLOSE, "Cancel file move", move || {
        if let Some(popup) = popup.upgrade() {
            popup.hide();
        }
    });
    body.children().add(actions.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
    flyout.set_content(Some(Control::boxed(body)));
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

fn panel(workspace: &DesignerWorkspace) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.set_width((workspace.root.bounds().width - 40.0).clamp(300.0, 760.0));
    panel
}

fn toolbar() -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(6.0);
    row
}

fn relative(workspace: &DesignerWorkspace, uri: &str) -> String {
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
