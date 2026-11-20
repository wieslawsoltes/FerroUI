//! Studio uses the same checked location and owner as the VS Code Styles dock.
use super::{palette, text, tool_button, Context, DesignerWorkspace, SOURCE};
use crate::style_sources::{StyleEditorLocation, StyleNavigation, StyleSource};
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;
use ferroui_controls::{Control, StackPanel};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::component_palette::PaletteLayer;
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq)]
pub(in crate::workspace) struct StyleSourceVisit {
    pub(super) uri: String,
    consumer: String,
    editor: StyleEditorLocation,
}

impl StyleSourceVisit {
    pub(super) fn palette(&self) -> Option<(NodeId, PaletteLayer)> {
        match self.editor {
            StyleEditorLocation::Palette { owner, layer, .. } => Some((owner, layer)),
            StyleEditorLocation::Styles { .. } => None,
        }
    }
}

impl DesignerWorkspace {
    pub(crate) fn open_included_style_source(
        self: &Rc<Self>,
        navigation: StyleNavigation,
    ) -> Result<StyleEditorLocation, String> {
        if self.pending_source.get() || self.session.borrow().document_uri() != navigation.consumer
        {
            return Err("The consuming design changed. Reopen its included sources.".into());
        }
        let uri = &navigation.source.location.uri;
        self.visit_source_document(uri, |workspace| {
            let (editor, span, has_definition) = {
                let session = workspace.session.borrow();
                let (editor, span) = navigation
                    .source
                    .editor_in(session.document(), session.document_uri())?;
                let has_definition = session
                    .component_catalog()?
                    .entries
                    .iter()
                    .any(|entry| entry.node == editor.owner());
                (editor, span, has_definition)
            };
            workspace.components.borrow_mut().style_source_visit = Some(StyleSourceVisit {
                uri: workspace.session.borrow().document_uri().into(),
                consumer: navigation.consumer,
                editor,
            });
            if has_definition {
                workspace.inspect_component(editor.owner())?;
            } else {
                workspace.set_navigation_pane(crate::NavigationPane::Library);
                workspace.create_component();
            }
            workspace.select_source_span(span);
            Ok(editor)
        })
    }

    pub(super) fn return_from_style_source(
        self: &Rc<Self>,
        visit: &StyleSourceVisit,
    ) -> Result<(), String> {
        if self.components.borrow().style_source_visit.as_ref() != Some(visit)
            || self.session.borrow().document_uri() != visit.uri
        {
            return Err("The included source tab changed.".into());
        }
        self.show_open_document(&visit.consumer)?;
        self.components.borrow_mut().style_source_visit = None;
        Ok(())
    }
}

impl Context {
    pub(super) fn open_source(&self, source: StyleSource) -> Result<(), String> {
        let workspace = self.workspace()?;
        let navigation = workspace.session.borrow().open_style_source(source)?;
        workspace.open_included_style_source(navigation)?;
        Ok(())
    }
}

pub(super) fn picker(context: Context) -> Result<Ref<Control>, String> {
    let workspace = context.workspace()?;
    let session = workspace.session.borrow();
    let sources = session.style_source_choices()?;
    if sources.is_empty() {
        return Ok(text("No included style or palette declarations in this preview.").upcast());
    }
    let consumer = ferroui_designer_project::file_path(session.document_uri()).ok();
    let labels = sources
        .iter()
        .map(|choice| {
            let uri = &choice.source.location.uri;
            let path = ferroui_designer_project::file_path(uri).ok();
            let name = path
                .as_deref()
                .map(|path| {
                    consumer
                        .as_deref()
                        .and_then(|consumer| consumer.parent())
                        .and_then(|parent| path.strip_prefix(parent).ok())
                        .unwrap_or(path)
                        .display()
                        .to_string()
                })
                .unwrap_or_else(|| uri.clone());
            format!("{} · {name}", choice.label)
        })
        .collect();
    let model = palette::choice(
        "IncludedStyleSource",
        "Included style or palette",
        labels,
        0,
    );
    let open = tool_button(SOURCE, "Open included declaration");
    open.set_name(Some("OpenIncludedStyleSource".into()));
    let selection = model.downgrade();
    open.click(move |_, _| {
        let Some(choice) = selection
            .upgrade()
            .and_then(|model| sources.get(model.selected_index() as usize))
        else {
            return;
        };
        context.report(context.open_source(choice.source.clone()));
    });
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(5.0);
    row.children().add(model.upcast::<Control>());
    row.children().add(open.upcast::<Control>());
    Ok(row.upcast())
}
