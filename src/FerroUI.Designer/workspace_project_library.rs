use super::DesignerWorkspace;
use ferroui_designer_core::SourceSpan;
use ferroui_designer_language::{ProjectLibraryEntry, ProjectLibraryResolution};
use std::rc::Rc;

impl DesignerWorkspace {
    pub(crate) fn open_project_library_definition(
        self: &Rc<Self>,
        entry: ProjectLibraryEntry,
        preview: bool,
    ) -> Result<String, String> {
        if self.pending_source.get() {
            return Err("Apply the pending XAML before opening a definition".into());
        }
        self.synchronize_language_service()?;
        let uri = self
            .try_language_service()?
            .project_library_uri(&entry)
            .map_err(|error| error.to_string())?;
        self.visit_source_document(&uri, |workspace| {
            workspace.synchronize_language_service()?;
            let span = {
                let service = workspace.try_language_service()?;
                let version = service
                    .version(&uri)
                    .ok_or("The Library document is unavailable")?;
                let request = ProjectLibraryResolution {
                    entry,
                    uri: uri.clone(),
                    version,
                };
                service
                    .resolve_project_library(&request)
                    .map_err(|error| error.to_string())?;
                let index = service.index(&uri).map_err(|error| error.to_string())?;
                let range = request.entry.definition.range;
                let start = index
                    .offset(range.start)
                    .map_err(|error| error.to_string())?;
                let end = index.offset(range.end).map_err(|error| error.to_string())?;
                if index.source() != workspace.session.borrow().document().source() {
                    return Err("The Library source changed before navigation".into());
                }
                SourceSpan::new(start, end)
            };
            if preview {
                let node = workspace
                    .session
                    .borrow()
                    .document()
                    .ast()
                    .nodes_in_source_order()
                    .find(|node| node.name_span == span)
                    .map(|node| node.id)
                    .ok_or("The declaration is unavailable in the current design")?;
                workspace.edit_component_on_canvas(node)?;
            } else {
                workspace.select_source_span(span);
            }
            Ok(uri.clone())
        })
    }
}
