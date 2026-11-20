//! Collect the declarations of one authored theme dictionary without joining unrelated palettes.
use super::super::super::{
    DeclarationIndex, LanguageService, Occurrence, ResourceEdgeKind, SymbolKind, resource_scope,
};
use super::super::same_declaration;
use crate::{Location, Result, require};
use ferroui_designer_tools::component_theme_key;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

type Declaration = (String, Occurrence);
type Dictionary = (String, u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeResourceFamily {
    pub owner: Location,
    pub members: Vec<ThemeResourceMember>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeResourceMember {
    pub layers: Vec<String>,
    pub declaration: Location,
}

pub(super) struct ThemeFamily {
    owner: Dictionary,
    layers: Vec<(String, Dictionary)>,
    members: Vec<(Declaration, Vec<String>)>,
}

impl ThemeFamily {
    pub(super) fn declarations(&self) -> Vec<Declaration> {
        self.members
            .iter()
            .map(|(declaration, _)| declaration.clone())
            .collect()
    }

    pub(super) fn describe(&self, service: &LanguageService) -> Result<ThemeResourceFamily> {
        let (file, owner) = &self.owner;
        let state = service.state(file)?;
        let node = state
            .document
            .ast()
            .node(*owner)
            .expect("indexed theme owner");
        Ok(ThemeResourceFamily {
            owner: service.location(file, node.name_span)?,
            members: self
                .members
                .iter()
                .map(|((uri, occurrence), layers)| {
                    Ok(ThemeResourceMember {
                        layers: layers.clone(),
                        declaration: service.location(uri, occurrence.span)?,
                    })
                })
                .collect::<Result<_>>()?,
        })
    }

    pub(super) fn check_name(&self, service: &LanguageService, name: &str) -> Result<()> {
        for (uri, scope) in
            std::iter::once(&self.owner).chain(self.layers.iter().map(|(_, scope)| scope))
        {
            let existing = service.resources_at(
                uri,
                *scope,
                name,
                &BTreeSet::new(),
                &DeclarationIndex::new(),
            );
            require(
                existing.matches.is_empty(),
                "This key already resolves in the theme family",
            )?;
        }
        Ok(())
    }

    fn add(members: &mut Vec<(Declaration, Vec<String>)>, entry: Declaration, layer: &str) {
        if let Some((_, layers)) = members
            .iter_mut()
            .find(|(candidate, _)| same_declaration(candidate, &entry))
        {
            layers.push(layer.to_owned());
        } else {
            members.push((entry, vec![layer.to_owned()]));
        }
    }
}

impl LanguageService {
    pub(super) fn resource_theme_family(
        &self,
        targets: &[Declaration],
    ) -> Result<Option<ThemeFamily>> {
        let Some((source, key)) = targets.first() else {
            return Ok(None);
        };
        let declarations = self
            .declarations_for(&key.name, &DeclarationIndex::new())
            .iter()
            .filter(|(_, entry)| entry.kind == SymbolKind::Resource)
            .cloned()
            .collect::<Vec<_>>();
        if declarations.len() < 2 {
            return Ok(None);
        }
        let workspace = self.workspace_root(source);
        let mut result = None;
        for (file, state) in &self.documents {
            if self.workspace_root(file) != workspace || !state.document.is_valid() {
                continue;
            }
            for (owner, edges) in &state.resource_edges {
                let edges = edges
                    .iter()
                    .filter(|edge| edge.kind != ResourceEdgeKind::Collection);
                let mut labels = BTreeSet::new();
                let Some(layers) = edges
                    .map(|edge| {
                        let provider = state.document.ast().node(edge.node)?;
                        let label = component_theme_key(&state.document, provider)?;
                        if !matches!(label.as_str(), "Default" | "Light" | "Dark")
                            || !labels.insert(label.clone())
                        {
                            return None;
                        }
                        let scope = self.resource_edge_target(file, edge)?;
                        let imported = self.state(&scope.0).ok()?;
                        (self.workspace_root(&scope.0) == workspace && imported.document.is_valid())
                            .then_some((label, scope))
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if layers.is_empty() {
                    continue;
                }
                let mut family = ThemeFamily {
                    owner: (file.clone(), *owner),
                    layers,
                    members: Vec::new(),
                };
                for entry in declarations.iter().filter(|(uri, entry)| {
                    uri == file && resource_scope(&state.document, entry.node) == *owner
                }) {
                    ThemeFamily::add(&mut family.members, entry.clone(), "Shared");
                }
                let mut ambiguous = family.members.len() > 1;
                for (layer, (uri, scope)) in &family.layers {
                    let entries = self.resources_at(
                        uri,
                        *scope,
                        &key.name,
                        &BTreeSet::new(),
                        &DeclarationIndex::new(),
                    );
                    match entries.matches.as_slice() {
                        [] => {}
                        [entry] if !entries.may_be_missing && !entries.themed => {
                            ThemeFamily::add(&mut family.members, entry.clone(), layer)
                        }
                        _ => ambiguous = true,
                    }
                }
                if ambiguous
                    || family.members.len() < 2
                    || !targets.iter().all(|target| {
                        family
                            .members
                            .iter()
                            .any(|(entry, _)| same_declaration(entry, target))
                    })
                {
                    continue;
                }
                require(
                    result.is_none(),
                    "This key belongs to multiple theme families. Open an unambiguous palette or consumer",
                )?;
                result = Some(family);
            }
        }
        Ok(result)
    }
}
