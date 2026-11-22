//! Stage-local inspection with source navigation guarded by the inspected model.
use super::DesignerWorkspace;
use crate::{PreviewStageAccessibleNode, PreviewStageInfo, WorkspaceViewMode};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::TextWrapping;
use ferroui_base::Ref;
use ferroui_controls::{
    Button, Control, Flyout, ScrollViewer, StackPanel, TabControl, TabItem, TextBlock,
};
use ferroui_designer_core::{NodeId, SourceSpan};
use std::rc::Rc;

type Reveal = Rc<dyn Fn(Option<NodeId>, SourceSpan)>;

impl DesignerWorkspace {
    pub(super) fn show_preview_stage_checks(
        self: &Rc<Self>,
        stage: &PreviewStageInfo,
        anchor: &Control,
    ) -> Result<(), String> {
        let findings = self.preview_stage_checks(stage.id)?;
        let accessibility = self.preview_stage_accessibility(stage.id)?;
        if let Some(old) = self.preview_stages.flyout.borrow().as_ref() {
            old.hide();
        }

        let model = self.preview_stage_model();
        let uri = stage.configuration.source_uri.clone();
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        let close = flyout.downgrade();
        let weak = Rc::downgrade(self);
        let reveal: Reveal = Rc::new(move |node, span| {
            if let Some(flyout) = close.upgrade() {
                flyout.hide();
            }
            let Some(workspace) = weak.upgrade() else {
                return;
            };
            if workspace.preview_stage_model() != model {
                workspace
                    .status
                    .set_text(Some("The preview changed. Reopen Checks."));
                return;
            }
            let result = workspace.activate_document(&uri).map(|_| {
                if let Some(node) = node {
                    workspace.session.borrow_mut().select([node]);
                    workspace.runtime_selection.set(None);
                }
                if workspace.view_mode() == WorkspaceViewMode::Design {
                    workspace.set_view_mode(WorkspaceViewMode::Split);
                }
                workspace.refresh(true);
                workspace.select_source_span(span);
            });
            workspace.report(result);
        });

        let tabs = TabControl::new();
        tabs.set_width(436.0);
        let issues = tab(&tabs, "Findings");
        if findings.is_empty() {
            issues.children().add(text("No findings").upcast::<Control>());
        }
        for finding in findings {
            let control = finding.name.as_deref().unwrap_or(&finding.control);
            add_row(
                &issues,
                &format!("{control} · {}", finding.message),
                finding.source.map(|span| (Some(finding.node), span)),
                &reveal,
            );
        }
        let order = tab(&tabs, "Tab order");
        if accessibility.tab_order.is_empty() {
            order.children().add(text("No Tab stops").upcast::<Control>());
        }
        for (index, row) in accessibility.tab_order.iter().enumerate() {
            add_accessible_row(&order, row, &format!("{}. ", index + 1), &reveal);
        }
        let tree = tab(&tabs, "Accessibility tree");
        for row in &accessibility.tree {
            add_accessible_row(&tree, row, &" ".repeat(row.depth.min(12)), &reveal);
        }
        if accessibility.truncated {
            tree.children()
                .add(text("Preview exceeded the inspection limit").upcast::<Control>());
        }
        tabs.set_selected_index(0);
        flyout.set_content(Some(Control::boxed(tabs.upcast::<Control>())));
        *self.preview_stages.flyout.borrow_mut() = Some(flyout.clone());
        flyout.show_at(anchor);
        Ok(())
    }
}

fn tab(tabs: &TabControl, label: &str) -> Ref<StackPanel> {
    let tab = TabItem::new();
    tab.set_header(Some(Rc::new(label.to_owned())));
    let scroll = ScrollViewer::new();
    scroll.set_max_height(320.0);
    let rows = StackPanel::new();
    rows.set_spacing(4.0);
    scroll.set_content(Some(Control::boxed(rows.clone().upcast::<Control>())));
    tab.set_content(Some(Control::boxed(scroll.upcast::<Control>())));
    tabs.items()
        .add(Some(Control::boxed(tab.upcast::<Control>())));
    rows
}

fn add_accessible_row(
    panel: &StackPanel,
    row: &PreviewStageAccessibleNode,
    prefix: &str,
    reveal: &Reveal,
) {
    let label = if row.name.is_empty() {
        &row.control
    } else {
        &row.name
    };
    let enabled = if row.enabled { "" } else { " · Disabled" };
    let visible = if row.offscreen { " · Offscreen" } else { "" };
    add_row(
        panel,
        &format!("{prefix}{label} · {}{enabled}{visible}", row.role),
        row.source.map(|span| (row.node, span)),
        reveal,
    );
}

fn add_row(
    panel: &StackPanel,
    label: &str,
    source: Option<(Option<NodeId>, SourceSpan)>,
    reveal: &Reveal,
) {
    let title = text(label);
    let Some((node, span)) = source else {
        panel.children().add(title.upcast::<Control>());
        return;
    };
    let action = Button::new();
    action.classes().add("nav");
    action.set_horizontal_alignment(HorizontalAlignment::Stretch);
    action.set_horizontal_content_alignment(HorizontalAlignment::Left);
    action.set_content(Some(Control::boxed(title.upcast::<Control>())));
    let reveal = reveal.clone();
    action.click(move |_, _| reveal(node, span));
    panel.children().add(action.upcast::<Control>());
}

fn text(label: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(label));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
