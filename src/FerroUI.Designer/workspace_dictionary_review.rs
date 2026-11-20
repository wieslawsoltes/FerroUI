//! Studio's read-only view of the shared extraction plan.
use super::{DesignerWorkspace, ExtractionSite};
use crate::icons;
use crate::workspace::inspector_actions::{SOURCE, add_button, text};
use ferroui_base::{Ref, layout::Orientation, media::FontFamily};
use ferroui_controls::{
    Button, Control, Flyout, PlacementMode, StackPanel, TabControl, TabItem, TextBox,
    automation::AutomationProperties,
};
use ferroui_designer_language::DictionaryExtractionPlan;
use std::rc::Rc;

impl DesignerWorkspace {
    pub(crate) fn connect_dictionary_extraction(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("ExtractResourceDictionary");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Top);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let workspace = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(popup)) = (workspace.upgrade(), popup.upgrade()) {
                    render_picker(&workspace, &popup);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn render_picker(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>) {
    if workspace.pending_source.get() {
        workspace.apply_source();
    }
    let panel = body(workspace);
    flyout.set_content(Some(Control::boxed(panel.clone())));
    let target = workspace.extraction_site(None).and_then(|site| {
        workspace
            .extraction_target(&site)
            .map(|target| (site, target))
    });
    let (site, target) = match target {
        Ok(target) => target,
        Err(error) => {
            panel.children().add(text(&error).upcast::<Control>());
            return;
        }
    };
    let site = Rc::new(site);
    panel
        .children()
        .add(text("Resource dictionary file").upcast::<Control>());
    let row = toolbar();
    let name = TextBox::new();
    name.set_min_width(250.0);
    name.set_text(Some(&target.suggested_file));
    AutomationProperties::set_name(&name, Some("New dictionary file name"));
    row.children().add(name.clone().upcast::<Control>());
    let status = text("");
    let file = name.downgrade();
    let message = status.downgrade();
    let popup = flyout.downgrade();
    let owner = Rc::downgrade(workspace);
    add_button(&row, SOURCE, "Review extraction", move || {
        let (Some(workspace), Some(popup), Some(input)) =
            (owner.upgrade(), popup.upgrade(), file.upgrade())
        else {
            return;
        };
        let name = input.text().unwrap_or_default();
        match workspace.plan_extraction(&site, name.trim()) {
            Ok(plan) => render_review(&workspace, &popup, site.clone(), plan),
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error));
                }
            }
        }
    });
    panel.children().add(row.upcast::<Control>());
    panel.children().add(status.upcast::<Control>());
}

fn render_review(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    site: Rc<ExtractionSite>,
    plan: DictionaryExtractionPlan,
) {
    let [edit] = plan.edit.edits.as_slice() else {
        return;
    };
    let session = workspace.session.borrow();
    let source = session.document().source();
    let index = ferroui_designer_language::LineIndex::new(source);
    let Ok(span) = index.span(edit.range) else {
        return;
    };
    let panel = body(workspace);
    let tabs = TabControl::new();
    for (label, source) in [
        ("Original source", &source[span.range()]),
        ("Source replacement", &edit.new_text),
        (&plan.file.file_name, &plan.file.source),
    ] {
        let tab = TabItem::new();
        tab.set_header(Some(Rc::new(label.to_owned())));
        let editor = TextBox::new();
        editor.set_text(Some(source));
        editor.set_is_read_only(true);
        editor.set_accepts_return(true);
        editor.set_height(244.0);
        editor.set_font_family(FontFamily::from("monospace"));
        editor.set_font_size(11.0);
        AutomationProperties::set_name(&editor, Some(label));
        tab.set_content(Some(Control::boxed(editor)));
        tabs.items().add(Some(Control::boxed(tab)));
    }
    tabs.set_selected_index(1);
    drop(session);
    panel.children().add(tabs.upcast::<Control>());
    let actions = toolbar();
    let status = text("");
    let message = status.downgrade();
    let owner = Rc::downgrade(workspace);
    let popup = flyout.downgrade();
    add_button(&actions, icons::CHECK, "Apply extraction", move || {
        let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) else {
            return;
        };
        match workspace.apply_extraction(&site, &plan.file.file_name, &plan.review) {
            Ok(_) => popup.hide(),
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error));
                }
            }
        }
    });
    let popup = flyout.downgrade();
    add_button(&actions, icons::CLOSE, "Cancel extraction", move || {
        if let Some(popup) = popup.upgrade() {
            popup.hide();
        }
    });
    panel.children().add(actions.upcast::<Control>());
    panel.children().add(status.upcast::<Control>());
    flyout.set_content(Some(Control::boxed(panel)));
}

fn body(workspace: &DesignerWorkspace) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.set_width((workspace.root.bounds().width - 40.0).clamp(280.0, 700.0));
    panel
}

fn toolbar() -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(6.0);
    row
}
