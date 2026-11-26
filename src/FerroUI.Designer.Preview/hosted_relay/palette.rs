//! Palette actions over the source graph accepted by the preview application.
use super::{
    authored::{AuthoredPreview, Request},
    Relay,
};
use ferroui_designer::{PaletteEdit, ScopedPalette};
use ferroui_designer_core::{Document, Node, NodeId};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_tools::component_palette::palette_layers;
use ferroui_designer_tools::{
    component_property_elements, component_resource_references, ComponentCatalog, ComponentKind,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub(crate) enum Action {
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
pub(crate) struct Target {
    address: String,
    owner: NodeId,
}

#[derive(Serialize)]
struct Scope {
    node: NodeId,
    label: String,
}

impl Relay {
    pub(crate) fn palette(&self, request: Request<Action>) -> Result<Value, String> {
        self.authored(request.frame)?.palette(request.action)
    }
}

impl AuthoredPreview {
    fn palette(&self, action: Action) -> Result<Value, String> {
        match action {
            Action::Catalog => {
                let documents = self
                    .documents()
                    .map(|(address, uri, document)| {
                        Ok(json!({ "address":address, "uri":uri, "scopes":scopes(document)? }))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(json!({ "documents":documents }))
            }
            Action::Source { address } => Ok(json!({"uri":self.source_at(&address)?.0})),
            Action::Read { target } => {
                let (uri, document) = self.palette_target(&target)?;
                let palette = self
                    .palette_scope(document, &target.address)
                    .report(target.owner)?;
                Ok(json!({"uri":uri,"palette":palette}))
            }
            Action::References { target, node } => {
                let (uri, document) = self.palette_target(&target)?;
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
            } => self.plan(&target.address, &source, |document| {
                self.palette_target(&target)?;
                require_palette_edit(document, target.owner, &edit)?;
                self.palette_scope(document, &target.address).plan(&edit)
            }),
            Action::Locate {
                address,
                range,
                source,
            } => self.locate(&address, range, &source),
        }
    }

    fn palette_target(&self, target: &Target) -> Result<(&str, &Document), String> {
        let (uri, document) = self.source_at(&target.address)?;
        if !scopes(document)?
            .iter()
            .any(|scope| scope.node == target.owner)
        {
            return Err("Choose an authored palette scope from the app preview".into());
        }
        Ok((uri, document))
    }

    fn palette_scope<'a>(&'a self, document: &'a Document, address: &'a str) -> ScopedPalette<'a> {
        ScopedPalette::new(
            document,
            address,
            Some(&self.sources.includes),
            self.contrast.clone(),
            self.providers.clone(),
        )
    }
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
