use super::{DocumentEdits, LanguageService, Location, WorkspaceEdit};
use crate::{
    LanguageError, LineIndex, Position, Result, TextEdit, event_handler_name, handler_edit, require,
};
use ferroui_build_scan::scanner::rust_identifier;
use ferroui_designer_core::{Document, Edit, SourceSpan, TextPatch};
use ferroui_designer_project::file_path;
use serde::{Deserialize, Serialize};

#[path = "handler_contract.rs"]
mod contract;
use contract::event_at;
pub(crate) const MAX_RUST_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandlerRequest {
    pub version: i64,
    pub position: Position,
    /// Designer callers identify the event separately and point at the control's name.
    pub event: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandlerTarget {
    pub rust_uri: String,
    pub existing_name: Option<String>,
    pub suggested_name: String,
    pub event: String,
    pub declaring_type: String,
    pub parameter_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct HandlerDraft {
    pub uri: String,
    pub version: i64,
    pub source: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct HandlerPlan {
    pub handler: String,
    pub edit: WorkspaceEdit,
    pub declaration: Location,
}

impl LanguageService {
    pub fn handler_target(&self, uri: &str, request: &HandlerRequest) -> Result<HandlerTarget> {
        let contract = self.handler_contract(uri, request)?;
        let source = contract.source()?;
        let text = read_rust(&source.uri)?;
        let registration = source.registration(&text)?;
        let stem = format!(
            "On{}{}",
            title(
                contract
                    .node
                    .xaml_name()
                    .unwrap_or(contract.node.local_name())
            ),
            title(&contract.signature.member)
        );
        let in_use = |name: &str| {
            contract.catalog.has_markup_method(contract.class, name)
                || registration.names.iter().any(|entry| entry.name == name)
        };
        let suggested_name = contract.existing.map(str::to_owned).unwrap_or_else(|| {
            if !in_use(&stem) {
                return stem;
            }
            (2..)
                .map(|suffix| format!("{stem}{suffix}"))
                .find(|name| !in_use(name))
                .expect("finite method list")
        });
        Ok(HandlerTarget {
            rust_uri: source.uri,
            existing_name: contract.existing.map(str::to_owned),
            suggested_name,
            event: contract.event,
            declaring_type: contract.signature.owner,
            parameter_types: contract.signature.parameters,
        })
    }

    pub fn plan_handler(
        &self,
        uri: &str,
        request: &HandlerRequest,
        draft: &HandlerDraft,
        name: &str,
    ) -> Result<HandlerPlan> {
        let contract = self.handler_contract(uri, request)?;
        require(
            name.len() <= 160 && event_handler_name(name) && rust_identifier(name),
            "Enter a valid Rust handler identifier",
        )?;
        require(
            contract.existing.is_none_or(|current| current == name),
            "The event already names a different handler",
        )?;
        require(
            !contract.catalog.has_markup_method(contract.class, name),
            "This handler is already registered in the project schema",
        )?;
        let source = contract.source()?;
        require(
            source.uri == ferroui_designer_project::canonical_file_uri(&draft.uri)?,
            "The code-behind file changed. Prepare the edit again",
        )?;
        require(
            draft.source.len() <= MAX_RUST_BYTES,
            "Code-behind exceeds 2 MiB",
        )?;
        let registration = source.registration(&draft.source)?;
        require(
            !registration.names.iter().any(|entry| entry.name == name),
            "This handler is already registered in the Rust buffer",
        )?;
        let rust = handler_edit::insert_registration(
            &draft.source,
            &registration,
            &contract.class.name,
            name,
            &contract.signature.parameters,
        )?;
        let authored_rust = TextPatch::apply_all(&draft.source, rust.clone())?;
        let registration = source.registration(&authored_rust)?;
        let produced = registration
            .names
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| LanguageError("The Rust handler registration did not reparse".into()))?;

        let state = self.state(uri)?;
        let xaml = if contract.existing.is_some() {
            Vec::new()
        } else {
            state
                .document
                .plan_attribute_patches(vec![Edit::SetAttribute {
                    node: contract.node.id,
                    name: contract.event,
                    value: name.into(),
                }])?
        };
        Document::new(&TextPatch::apply_all(
            state.document.source(),
            xaml.clone(),
        )?)?;
        Ok(HandlerPlan {
            handler: name.into(),
            declaration: Location {
                uri: draft.uri.clone(),
                range: LineIndex::new(&authored_rust).range(SourceSpan::new(
                    produced.name_range.start,
                    produced.name_range.end,
                ))?,
            },
            edit: WorkspaceEdit {
                document_changes: vec![
                    DocumentEdits {
                        uri: self.document_uri(uri)?.into_owned(),
                        version: request.version,
                        edits: wire_edits(&state.index, xaml)?,
                    },
                    DocumentEdits {
                        uri: draft.uri.clone(),
                        version: draft.version,
                        edits: wire_edits(&LineIndex::new(&draft.source), rust)?,
                    },
                ],
            },
        })
    }

    pub fn handler_definition(
        &self,
        uri: &str,
        request: &HandlerRequest,
        draft: Option<&HandlerDraft>,
    ) -> Result<Option<Location>> {
        let contract = self.handler_contract(uri, request)?;
        let Some(name) = contract.existing else {
            return Ok(None);
        };
        let site = contract.source()?;
        let saved;
        let (uri, text) = if let Some(draft) = draft {
            require(
                site.uri == ferroui_designer_project::canonical_file_uri(&draft.uri)?
                    && draft.source.len() <= MAX_RUST_BYTES,
                "The Rust buffer no longer matches the code-behind target",
            )?;
            (draft.uri.as_str(), draft.source.as_str())
        } else {
            saved = read_rust(&site.uri)?;
            (site.uri.as_str(), saved.as_str())
        };
        let registration = site.registration(text)?;
        let mut matches = registration.names.iter().filter(|entry| entry.name == name);
        let Some(entry) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Ok(None);
        }
        Ok(Some(Location {
            uri: uri.into(),
            range: LineIndex::new(text).range(SourceSpan::new(
                entry.name_range.start,
                entry.name_range.end,
            ))?,
        }))
    }

    pub(crate) fn offer_handler_fix(&self, uri: &str, request: &HandlerRequest) -> bool {
        self.is_handler_attribute(uri, request.position)
            && self.handler_contract(uri, request).is_ok_and(|contract| {
                (contract
                    .existing
                    .is_none_or(|name| !contract.catalog.has_markup_method(contract.class, name)))
                    && contract.source().ok().and_then(|site| {
                        let text = read_rust(&site.uri).ok()?;
                        let registration = site.registration(&text).ok()?;
                        Some(contract.existing.is_none_or(|name| {
                            !registration.names.iter().any(|entry| entry.name == name)
                        }))
                    }) == Some(true)
            })
    }

    pub(crate) fn is_handler_attribute(&self, uri: &str, position: Position) -> bool {
        self.state(uri)
            .ok()
            .and_then(|state| {
                let (node, event, _) =
                    event_at(&state.document, state.index.offset(position).ok()?, None).ok()?;
                self.schema_for(uri).attribute_property(
                    &state.document,
                    node,
                    node.attribute_info(&event)?,
                )
            })
            .is_some_and(|info| info.event)
    }
}

pub(crate) fn read_rust(uri: &str) -> Result<String> {
    use std::io::Read;
    let path = file_path(uri)?;
    let mut source = String::new();
    std::fs::File::open(&path)
        .and_then(|file| {
            file.take((MAX_RUST_BYTES + 1) as u64)
                .read_to_string(&mut source)
        })
        .map_err(|error| LanguageError(format!("Cannot read code-behind: {error}")))?;
    require(source.len() <= MAX_RUST_BYTES, "Code-behind exceeds 2 MiB")?;
    Ok(source)
}

fn title(name: &str) -> String {
    name.split(|ch: char| !ch.is_alphanumeric())
        .filter_map(|word| {
            let mut chars = word.chars();
            Some(
                chars
                    .next()?
                    .to_uppercase()
                    .chain(chars)
                    .collect::<String>(),
            )
        })
        .collect()
}

fn wire_edits(index: &LineIndex, patches: Vec<TextPatch>) -> Result<Vec<TextEdit>> {
    patches
        .into_iter()
        .map(|patch| {
            Ok(TextEdit {
                range: index.range(patch.span)?,
                new_text: patch.replacement,
            })
        })
        .collect()
}
