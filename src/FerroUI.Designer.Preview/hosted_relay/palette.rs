//! Palette actions over the exact authored graph accepted by the preview application.
use super::{sources::AuthoredSources, Relay};
use crate::hosted_animation::{checked_source_patch, source_patch};
use crate::hosted_frame::FrameIdentity;
use ferroui_designer::{PaletteEdit, ScopedPalette};
use ferroui_designer_core::{Document, Node, NodeId};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_tools::component_palette::palette_layers;
use ferroui_designer_tools::theme_variants::HighContrastProvider;
use ferroui_designer_tools::{
    component_property_elements, component_resource_references, ComponentCatalog, ComponentKind,
    ComponentResourceContext,
};
use ferroui_remote_protocol::designer::DesignPaletteContextMessage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Deserialize)]
pub(crate) struct Request {
    #[serde(flatten)]
    frame: FrameIdentity,
    #[serde(flatten)]
    action: Action,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Action {
    Catalog,
    Source {
        address: String,
    },
    Read {
        target: Target,
    },
    References {
        target: Target,
        node: NodeId,
    },
    Plan {
        target: Target,
        edit: PaletteEdit,
        source: String,
    },
    Locate {
        address: String,
        range: Range,
        source: String,
    },
}

#[derive(Deserialize)]
struct Target {
    address: String,
    owner: NodeId,
}

#[derive(Serialize)]
struct Scope {
    node: NodeId,
    label: String,
}

pub(super) struct HostedPalette {
    document: Document,
    uri: String,
    sources: AuthoredSources,
    contrast: Option<ComponentResourceContext>,
    providers: Vec<HighContrastProvider>,
}

impl Relay {
    pub(crate) fn palette(&self, request: Request) -> Result<Value, String> {
        let state = self.events.0.lock().unwrap();
        if state.disconnected
            || self.sent != Some((request.frame.revision, request.frame.sequence))
            || state.requested != Some(request.frame.revision)
            || !matches!(state.outcome, Some(Ok(())))
        {
            return Err(
                "Read the current app-hosted preview frame before editing its palette".into(),
            );
        }
        drop(state);
        self.palette
            .as_ref()
            .ok_or("Use an XAML App Preview with palette support")?
            .handle(request.action)
    }
}

impl HostedPalette {
    pub(super) fn new(
        document: Document,
        uri: String,
        sources: AuthoredSources,
        context: DesignPaletteContextMessage,
    ) -> Self {
        let contrast = context.contrast.and_then(|keys| {
            let mut keys = keys.into_iter();
            Some(ComponentResourceContext {
                theme: Some(keys.next()?),
                inherited_themes: keys.collect(),
            })
        });
        let providers = HighContrastProvider::ALL
            .into_iter()
            .filter(|provider| {
                context
                    .assemblies
                    .iter()
                    .any(|name| name == provider.assembly())
            })
            .collect();
        Self {
            document,
            uri,
            sources,
            contrast,
            providers,
        }
    }

    fn handle(&self, action: Action) -> Result<Value, String> {
        match action {
            Action::Catalog => self.catalog(),
            Action::Source { address } => Ok(json!({"uri": self.source_at(&address)?.0})),
            Action::Read { target } => {
                let (uri, document) = self.target(&target)?;
                let palette = self.scope(document, &target.address).report(target.owner)?;
                Ok(json!({"uri":uri, "palette":palette}))
            }
            Action::References { target, node } => {
                let (uri, document) = self.target(&target)?;
                if !layer_has_color(document, target.owner, node, false)? {
                    return Err("Choose a declared color in the reported palette".into());
                }
                let index = LineIndex::new(document.source());
                let ast = document.ast();
                let references = component_resource_references(document, node)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|reference| {
                        let node = ast
                            .node(reference.node)
                            .ok_or("The resource reference changed")?;
                        let range = index
                            .range(node.name_span)
                            .map_err(|error| error.to_string())?;
                        Ok::<_, String>(json!({
                            "label":node.xaml_name().unwrap_or(node.local_name()), "range":range,
                            "property":reference.property, "dynamic":reference.dynamic,
                        }))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(json!({"uri":uri,"references":references}))
            }
            Action::Plan {
                target,
                edit,
                source,
            } => {
                let (uri, document) = self.target(&target)?;
                let bom = self.check_source(document, &source)?;
                require_palette_edit(document, target.owner, &edit)?;
                let edits = self.scope(document, &target.address).plan(&edit)?;
                let mut draft = document.clone();
                draft
                    .apply("Edit app preview palette", edits)
                    .map_err(|error| error.to_string())?;
                let updated = format!("{bom}{}", without_bom(draft.source()));
                let patch = checked_source_patch(&source, &updated)?.map(source_patch);
                Ok(json!({"uri":uri, "patches":patch.into_iter().collect::<Vec<_>>()}))
            }
            Action::Locate {
                address,
                range,
                source,
            } => {
                let (uri, document) = self.source_at(&address)?;
                let bom = self.check_source(document, &source)?;
                let captured = document.source();
                let captured_bom = captured.len() - without_bom(captured).len();
                let span = LineIndex::new(captured)
                    .span(range)
                    .map_err(|error| error.to_string())?;
                let start = span
                    .start
                    .checked_sub(captured_bom)
                    .ok_or("The palette source range starts before the XAML content")?;
                let selected = ferroui_designer_core::SourceSpan::new(
                    start + bom.len(),
                    span.end - captured_bom + bom.len(),
                );
                let range = LineIndex::new(&source)
                    .range(selected)
                    .map_err(|error| error.to_string())?;
                Ok(json!({"uri":uri, "range":range}))
            }
        }
    }

    fn catalog(&self) -> Result<Value, String> {
        let mut documents = Vec::new();
        for (address, uri, document) in std::iter::once((
            self.sources.base_uri.as_str(),
            self.uri.as_str(),
            &self.document,
        ))
        .chain(
            self.sources
                .includes
                .authored_sources()
                .map(|(address, included)| (address, included.uri.as_str(), &included.document)),
        ) {
            documents.push(json!({"address":address, "uri":uri, "scopes":scopes(document)?}));
        }
        Ok(json!({"documents":documents}))
    }

    fn source_at(&self, address: &str) -> Result<(&str, &Document), String> {
        if address == self.sources.base_uri {
            return Ok((&self.uri, &self.document));
        }
        self.sources
            .includes
            .authored_sources()
            .find(|(candidate, _)| *candidate == address)
            .map(|(_, include)| (include.uri.as_str(), &include.document))
            .ok_or_else(|| "Choose a source from the accepted app preview".into())
    }

    fn target(&self, target: &Target) -> Result<(&str, &Document), String> {
        let (uri, document) = self.source_at(&target.address)?;
        if !scopes(document)?
            .iter()
            .any(|scope| scope.node == target.owner)
        {
            return Err("Choose an authored palette scope from the app preview".into());
        }
        Ok((uri, document))
    }

    fn scope<'a>(&'a self, document: &'a Document, address: &'a str) -> ScopedPalette<'a> {
        ScopedPalette::new(
            document,
            address,
            Some(&self.sources.includes),
            self.contrast.clone(),
            self.providers.clone(),
        )
    }

    fn check_source<'a>(&self, document: &Document, source: &'a str) -> Result<&'a str, String> {
        let text = without_bom(source);
        if text != without_bom(document.source()) {
            return Err(
                "The XAML buffer changed. Wait for the application preview to update.".into(),
            );
        }
        Ok(&source[..source.len() - text.len()])
    }
}

fn without_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

fn scopes(document: &Document) -> Result<Vec<Scope>, String> {
    let ast = document.ast();
    let declarations: BTreeSet<_> = ComponentCatalog::declarations(document)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.kind,
                ComponentKind::ResourceDictionary
                    | ComponentKind::Style
                    | ComponentKind::ControlTheme
            )
        })
        .map(|entry| entry.node)
        .collect();
    let is_scope = |node: &Node| {
        node.id == ast.root()
            || declarations.contains(&node.id)
            || component_property_elements(document, node.id, "Resources")
                .is_ok_and(|elements| !elements.is_empty())
    };
    Ok(ast
        .elements()
        .filter(|node| is_scope(node))
        .map(|node| Scope {
            node: node.id,
            label: node.xaml_name().unwrap_or(node.local_name()).into(),
        })
        .collect())
}

fn require_palette_edit(
    document: &Document,
    owner: NodeId,
    edit: &PaletteEdit,
) -> Result<(), String> {
    let belongs = match edit {
        PaletteEdit::Add { owner: target, .. } => *target == owner,
        PaletteEdit::Set { node, .. } => layer_has_color(document, owner, *node, true)?,
    };
    belongs
        .then_some(())
        .ok_or_else(|| "Choose a color in the reported palette scope".into())
}

fn layer_has_color(
    document: &Document,
    owner: NodeId,
    node: NodeId,
    editable: bool,
) -> Result<bool, String> {
    let layers = palette_layers(document, owner).map_err(|error| error.to_string())?;
    Ok(layers
        .iter()
        .flat_map(|layer| &layer.colors)
        .any(|color| color.node == node && (!editable || color.editable)))
}
