//! Bind portable dictionary source patches to an indexed workspace and document version.
use super::{
    DeclarationIndex, LanguageService, ResourceLookup, State, occurrences, resource_edges,
};
use crate::{DocumentEdits, LanguageError, LineIndex, Position, Range, Result, TextEdit, require};
use ferroui_designer_core::{Document, MarkupValue, SourceSpan, TextPatch};
use ferroui_designer_project::{
    PreviewSourceBuffer, dictionary_destination, file_path, local_preview_sources,
    suggested_dictionary_name,
};
use ferroui_designer_tools::{
    DictionarySite, dictionary_extraction_site, plan_dictionary_extraction,
};
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct DictionaryExtractionRequest {
    pub version: i64,
    pub position: Position,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryExtractionTarget {
    pub range: Range,
    pub entries: usize,
    pub suggested_file: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedDictionary {
    pub uri: String,
    pub file_name: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DictionaryExtractionPlan {
    pub review: String,
    pub target: DictionaryExtractionTarget,
    pub edit: DocumentEdits,
    pub file: ExtractedDictionary,
}

impl LanguageService {
    pub fn dictionary_extraction_target(
        &self,
        uri: &str,
        request: DictionaryExtractionRequest,
    ) -> Result<DictionaryExtractionTarget> {
        let (state, identity, site) = self.extraction_site(uri, request)?;
        target(state, &identity, &site)
    }

    pub fn plan_dictionary_extraction(
        &self,
        uri: &str,
        request: DictionaryExtractionRequest,
        file_name: &str,
    ) -> Result<DictionaryExtractionPlan> {
        let (state, identity, site) = self.extraction_site(uri, request)?;
        let destination = dictionary_destination(&identity, file_name)?;
        require(
            !self
                .documents
                .get(&destination.uri)
                .is_some_and(|state| state.opened),
            "A draft for the dictionary destination is already open",
        )?;
        let planned = plan_dictionary_extraction(&state.document, &site, &destination.reference)
            .map_err(tool_error)?;
        self.check_moved_context(&identity, &state.document, site.node, planned.patch.span)?;
        let original = TextPatch::apply_all(state.document.source(), vec![planned.patch.clone()])?;
        let mut staged = self.clone();
        staged.set_state(
            identity.clone(),
            Some(candidate(Document::new(original)?, state.version)),
        );
        staged.set_state(
            destination.uri.clone(),
            Some(candidate(planned.dictionary, 0)),
        );
        for (node, key) in &planned.static_resources {
            require(
                staged.has_local_resource(&destination.uri, *node, key),
                format!(
                    "Static resource '{key}' needs a definition in the moved dictionary or one of its includes"
                ),
            )?;
        }
        staged.check_extraction_includes(&identity, &destination.uri)?;
        let target = target(state, &identity, &site)?;
        let range = state.index.range(planned.patch.span)?;
        let extracted = staged.documents[&destination.uri].document.source();
        let mut id = DefaultHasher::new();
        (
            state.version,
            &identity,
            &destination.uri,
            extracted,
            planned.patch.span,
            &planned.patch.replacement,
        )
            .hash(&mut id);
        Ok(DictionaryExtractionPlan {
            review: format!("{:016x}", id.finish()),
            target,
            edit: DocumentEdits {
                uri: identity,
                version: state.version,
                edits: vec![TextEdit {
                    range,
                    new_text: planned.patch.replacement,
                }],
            },
            file: ExtractedDictionary {
                uri: destination.uri.clone(),
                file_name: destination.file_name,
                source: extracted.into(),
            },
        })
    }

    fn extraction_site(
        &self,
        uri: &str,
        request: DictionaryExtractionRequest,
    ) -> Result<(&State, String, DictionarySite)> {
        let identity = self.document_uri(uri)?.into_owned();
        require(
            self.workspace_root(&identity).is_some(),
            "Open the view in a workspace to extract its resources",
        )?;
        let authored = file_path(uri)?;
        let physical = file_path(&identity)?;
        require(
            authored
                .parent()
                .and_then(|path| path.canonicalize().ok())
                .as_deref()
                == physical.parent(),
            "Open the real source file so sibling resource paths resolve in the same directory",
        )?;
        let state = self.state(&identity)?;
        require(
            state.version == request.version,
            "The XAML changed. Prepare the extraction again",
        )?;
        require(
            state.document.is_valid(),
            "Repair the XAML before extracting resources",
        )?;
        let offset = state.index.offset(request.position)?;
        let anchor = state
            .document
            .ast()
            .element_at(offset)
            .ok_or_else(selection_error)?;
        let site = dictionary_extraction_site(&state.document, anchor)
            .map_err(tool_error)?
            .ok_or_else(selection_error)?;
        Ok((state, identity, site))
    }

    fn has_local_resource(&self, uri: &str, at: u64, key: &str) -> bool {
        let state = &self.documents[uri];
        let mut current = Some(at);
        let mut outcome = ResourceLookup::missing();
        while let Some(id) = current {
            outcome.fallback(self.resources_at(
                uri,
                id,
                key,
                &Default::default(),
                &DeclarationIndex::new(),
            ));
            if !outcome.may_be_missing {
                return !outcome.matches.is_empty();
            }
            current = state.document.ast().node(id).and_then(|node| node.parent);
        }
        false
    }

    fn check_extraction_includes(&self, uri: &str, extracted: &str) -> Result<()> {
        let buffers = self
            .documents
            .iter()
            .filter(|(_, state)| state.opened)
            .map(|(uri, state)| PreviewSourceBuffer {
                uri: uri.clone(),
                source: Some(state.document.source().into()),
            })
            .collect::<Vec<_>>();
        let document = &self.documents[extracted].document;
        let sources = local_preview_sources(
            self.workspace_root(uri).map(|root| root.as_path()),
            extracted,
            document,
            &buffers,
        );
        if let Some(error) = sources.unavailable.values().next() {
            return Err(LanguageError(error.clone()));
        }
        Ok(())
    }

    fn check_moved_context(
        &self,
        uri: &str,
        document: &Document,
        root: u64,
        moved: SourceSpan,
    ) -> Result<()> {
        let ast = document.ast();
        let mut ancestor = ast.node(root);
        while let Some(node) = ancestor {
            require(
                !node.attributes.iter().any(xml_base),
                "The dictionary uses an XML base URI from its current scope",
            )?;
            ancestor = node.parent.and_then(|id| ast.node(id));
        }
        for node in ast.subtree(root).filter(|node| node.is_element()) {
            for attribute in node
                .attributes
                .iter()
                .filter(|attribute| within(moved, attribute.span))
            {
                require(
                    !xml_base(attribute),
                    "The dictionary defines a custom XML base URI",
                )?;
                let directive = attribute.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                    && matches!(
                        attribute.name.rsplit(':').next(),
                        Some("Class" | "ClassModifier" | "FieldModifier" | "Subclass")
                    );
                let event = self
                    .schema_for(uri)
                    .attribute_property(document, node, attribute)
                    .is_some_and(|property| property.event);
                require(
                    !directive && !event,
                    "The dictionary contains a class directive or authored event handler",
                )?;
                if self
                    .schema_for(uri)
                    .attribute_property(document, node, attribute)
                    .is_some()
                    && matches!(attribute.markup(), Ok(MarkupValue::Extension(extension)) if extension.name.ends_with(":Bind"))
                {
                    return Err(LanguageError(
                        "A compiled x:Bind expression depends on the current view".into(),
                    ));
                }
            }
        }
        for name in crate::name_occurrences(document)
            .into_iter()
            .filter(|name| within(moved, name.span))
        {
            let selector = ast
                .node(name.node)
                .and_then(|node| node.attribute_info("Selector"))
                .is_some_and(|attribute| within(attribute.value_span, name.span));
            require(
                selector
                    || ast
                        .node(name.scope)
                        .is_some_and(|scope| within(moved, scope.span)),
                format!("The name '{}' belongs to the view's namescope", name.name),
            )?;
        }
        Ok(())
    }
}

fn within(outer: SourceSpan, inner: SourceSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn xml_base(attribute: &ferroui_designer_core::Attribute) -> bool {
    attribute.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
        && attribute.name.rsplit(':').next() == Some("base")
}

fn target(state: &State, uri: &str, site: &DictionarySite) -> Result<DictionaryExtractionTarget> {
    Ok(DictionaryExtractionTarget {
        range: state.index.range(site.span)?,
        entries: site.entries,
        suggested_file: suggested_dictionary_name(uri)?,
    })
}

fn candidate(document: Document, version: i64) -> State {
    State {
        index: LineIndex::new(document.source()),
        symbols: occurrences(&document),
        resource_edges: resource_edges(&document),
        document,
        version,
        opened: true,
    }
}

fn tool_error(error: ferroui_designer_tools::ToolError) -> LanguageError {
    LanguageError(error.to_string())
}

fn selection_error() -> LanguageError {
    LanguageError("Choose an inline Resources property or one of its entries".into())
}
