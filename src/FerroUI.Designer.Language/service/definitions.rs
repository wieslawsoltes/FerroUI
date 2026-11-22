use super::handlers::{read_rust, MAX_RUST_BYTES};
use super::style_selectors;
use super::{HandlerRequest, LanguageService, LineIndex, Location, Range, Result, SourceSpan};
use ferroui_designer_project::{file_uri, MarkupSource};
use std::sync::Arc;

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

enum Definition {
    Locations(Vec<Location>),
    Authored(Arc<MarkupSource>),
}

impl Definition {
    fn member(location: Option<Location>, authored: Option<Arc<MarkupSource>>) -> Self {
        match (location, authored) {
            (None, Some(authored)) => Self::Authored(authored),
            (location, _) => Self::Locations(location.into_iter().collect()),
        }
    }
}

impl LanguageService {
    pub fn definitions(&self, uri: &str, offset: usize) -> Result<Vec<Location>> {
        self.definitions_with_source(uri, offset, |_| None)
    }

    /// The Rust buffer needed to resolve an authored definition against an editor draft.
    pub fn definition_source_uri(&self, uri: &str, offset: usize) -> Result<Option<String>> {
        let Definition::Authored(source) = self.definition_at(uri, offset)? else {
            return Ok(None);
        };
        source.file().map(file_uri).transpose().map_err(Into::into)
    }

    pub fn definitions_with_source(
        &self,
        uri: &str,
        offset: usize,
        draft: impl FnOnce(&str) -> Option<String>,
    ) -> Result<Vec<Location>> {
        let definition = match self.definition_at(uri, offset)? {
            Definition::Locations(locations) => return Ok(locations),
            Definition::Authored(definition) => definition,
        };
        let Some(file) = definition.file() else {
            return Ok(Vec::new());
        };
        let uri = file_uri(file)?;
        let Some(source) = draft(&uri).or_else(|| read_rust(&uri).ok()) else {
            return Ok(Vec::new());
        };
        if source.len() > MAX_RUST_BYTES {
            return Ok(Vec::new());
        }
        let Some(range) = definition.range_in(&source) else {
            return Ok(Vec::new());
        };
        Ok(vec![Location {
            uri,
            range: LineIndex::new(&source).range(SourceSpan::new(range.start, range.end))?,
        }])
    }

    fn definition_at(&self, uri: &str, offset: usize) -> Result<Definition> {
        let schema = self.schema_for(uri);
        let state = self.state(uri)?;
        let position = state.index.position(offset)?;
        if let Some(occurrence) = state
            .symbols
            .iter()
            .find(|item| item.span.contains(offset) || item.span.end == offset)
        {
            let locations = self
                .resolve(uri, occurrence)
                .into_iter()
                .map(|(uri, found)| self.location(&uri, found.span))
                .collect::<Result<Vec<_>>>()?;
            return Ok(Definition::Locations(locations));
        }
        if !state.document.is_valid() {
            return Ok(Definition::Locations(Vec::new()));
        }
        if let Some((property, _)) = crate::binding::at(schema, &state.document, offset) {
            return Ok(Definition::member(property.definition, property.authored));
        }
        if self.is_handler_attribute(uri, position) {
            let request = HandlerRequest {
                version: state.version,
                position,
                event: None,
            };
            if let Ok(Some(location)) = self.handler_definition(uri, &request, None) {
                return Ok(Definition::Locations(vec![location]));
            }
        }
        if let Some(node) = state
            .document
            .ast()
            .element_at(offset)
            .and_then(|id| state.document.ast().node(id))
        {
            let at_name = node.name_span.contains(offset)
                || node.closing_span.is_some_and(|span| {
                    SourceSpan::new(span.start + 2, span.start + 2 + node.name.len())
                        .contains(offset)
                });
            if at_name {
                if let Some(info) = schema.type_info(&node.name, node.namespace_uri.as_deref()) {
                    return Ok(Definition::member(
                        info.definition.clone(),
                        info.authored.clone(),
                    ));
                }
                if let Some(property) = node
                    .parent
                    .and_then(|id| state.document.ast().node(id))
                    .filter(|_| node.name.contains('.'))
                    .and_then(|owner| schema.element_property(owner, node))
                {
                    return Ok(Definition::member(property.definition, property.authored));
                }
            }
            if let Some(info) = crate::binding::type_at(schema, &state.document, node, offset) {
                return Ok(Definition::member(
                    info.definition.clone(),
                    info.authored.clone(),
                ));
            }
            if let Some(selector) = style_selectors::references(schema, &state.document, node) {
                if let Some(ty) = selector
                    .types
                    .iter()
                    .find(|site| site.span.contains(offset))
                    .and_then(|site| {
                        schema.exact_type_info(&site.spelling, Some(site.namespace.as_deref()?))
                    })
                {
                    return Ok(Definition::member(
                        ty.definition.clone(),
                        ty.authored.clone(),
                    ));
                }
                if let Some(member) = selector
                    .properties
                    .into_iter()
                    .find(|site| site.span.contains(offset))
                    .and_then(|site| site.member)
                {
                    return Ok(Definition::member(member.definition, member.authored));
                }
            }
            for attribute in &node.attributes {
                if attribute.name_span.contains(offset) {
                    if let Some(property) =
                        schema.attribute_property(&state.document, node, attribute)
                    {
                        return Ok(Definition::member(property.definition, property.authored));
                    }
                }
                if node.id == state.document.ast().root()
                    && attribute.value_span.contains(offset)
                    && attribute.namespace_uri.as_deref() == Some(XAML)
                    && attribute.name.rsplit(':').next() == Some("Class")
                {
                    if let Some(definition) = self.code_behind_definition(uri, &attribute.value)? {
                        return Ok(definition);
                    }
                }
                if attribute.name == "Source" && attribute.value_span.contains(offset) {
                    if let Ok(target) = self.resource_uri(uri, &attribute.value) {
                        let unresolved = self.project_schema_for(uri).is_some_and(|(_, project)| {
                            project.catalog.packaged_assets().knows_assembly(&target)
                        });
                        if !unresolved {
                            return Ok(Definition::Locations(vec![Location {
                                uri: target,
                                range: Range::default(),
                            }]));
                        }
                    }
                }
            }
        }
        if let Some((_, project)) = self.project_schema_for(uri) {
            let site = super::file_move_literals::paths(&state.document, schema)
                .into_iter().find(|site| site.contains_offset(offset));
            if let Some(site) = site {
                if let Ok(Some(source)) = project.catalog.packaged_assets().resolve(&site.value) {
                    return Ok(Definition::Locations(vec![Location {
                        uri: file_uri(source.file)?,
                        range: Range::default(),
                    }]));
                }
            }
        }
        Ok(Definition::Locations(Vec::new()))
    }

    fn code_behind_definition(&self, uri: &str, name: &str) -> Result<Option<Definition>> {
        let Some((root, project)) = self.project_schema_for(uri) else {
            return Ok(None);
        };
        let Some((package, targets)) = self.cargo_package_targets(root)? else {
            return Ok(None);
        };
        let crates = targets
            .iter()
            .map(|target| target.name.replace('-', "_"))
            .chain(std::iter::once(package.replace('-', "_")))
            .collect();
        let Some(owner) = project.catalog.code_behind_type(name.trim(), &crates) else {
            return Ok(None);
        };
        let info = project
            .effective
            .type_info(&owner.class.name, Some(&owner.namespace()));
        Ok(info.map(|info| Definition::member(info.definition.clone(), info.authored.clone())))
    }
}
