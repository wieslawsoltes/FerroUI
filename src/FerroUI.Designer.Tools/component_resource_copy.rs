//! Copy authored resource dependencies whose original identity a new scope cannot retain.
use super::references::{copy_keys::rewrite_keys, resolve, resource_uses};
use super::resource_graph::ResourceScope;
use super::*;
use ferroui_designer_core::TextPatch;

/// The logical addresses and authored buffers used by the accepted preview.
pub struct ResourceCopySources<'a> {
    pub address: &'a str,
    pub graph: &'a dyn ComponentResourceSources,
    pub includes: &'a [IncludedResourceDocument<'a>],
}

impl<'a> ResourceCopySources<'a> {
    pub(super) fn scope<'d>(&'d self, document: &'d Document) -> ResourceScope<'d> {
        ResourceScope::root(document, self.address, Some(self.graph))
    }

    fn origin(&self, location: ComponentResourceLocation<'a>) -> Result<ResourceScope<'a>> {
        self.includes
            .iter()
            .find(|document| {
                document.address == location.address && Some(document.uri) == location.uri
            })
            .map(|document| ResourceScope::authored(document, self.graph))
            .ok_or_else(|| ToolError("A dependency has no accepted source document.".into()))
    }
}

type Relocate<'a> = dyn Fn(&str, &Document) -> Result<String> + 'a;

pub(super) struct ResourceCopyPlanner<'a> {
    sources: &'a ResourceCopySources<'a>,
    relocate: &'a Relocate<'a>,
    reserved: BTreeSet<String>,
    prefix: String,
    active: BTreeSet<ComponentResourceLocation<'a>>,
    keys: BTreeMap<ComponentResourceLocation<'a>, String>,
    definitions: Vec<(String, CopiedDefinition<'a>)>,
}

pub(super) struct CopiedDefinition<'a> {
    pub source: String,
    nodes: Vec<NodeId>,
    bindings: Vec<CopiedBinding<'a>>,
}

struct CopiedBinding<'a> {
    index: usize,
    key: String,
    target: CopiedTarget<'a>,
    by_provider: bool,
}

enum CopiedTarget<'a> {
    Internal(NodeId),
    Retained(ComponentResourceLocation<'a>),
    Imported,
}

impl<'a> ResourceCopyPlanner<'a> {
    pub(super) fn new(
        document: &Document,
        sources: &'a ResourceCopySources<'a>,
        copied_key: &str,
        relocate: &'a Relocate<'a>,
    ) -> Self {
        let mut reserved = BTreeSet::new();
        for document in
            std::iter::once(document).chain(sources.includes.iter().map(|include| include.document))
        {
            for node in document.ast().elements() {
                reserved.extend(literal_key(node));
                reserved.extend(
                    resource_uses(document, node)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(_, _, key)| key),
                );
            }
        }
        let prefix = copied_key
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .take(48)
            .collect::<String>();
        let prefix = if prefix.is_empty() {
            "TemplateCopy".into()
        } else {
            prefix
        };
        Self {
            sources,
            relocate,
            reserved,
            prefix,
            active: BTreeSet::new(),
            keys: BTreeMap::new(),
            definitions: Vec::new(),
        }
    }

    pub(super) fn fragment(&self, root: ComponentResourceLocation<'a>) -> Result<Document> {
        let origin = self.sources.origin(root)?;
        let fragment =
            Document::new(component_fragment(origin.document, root.node)?).map_err(error)?;
        Document::new((self.relocate)(origin.address, &fragment)?).map_err(error)
    }

    pub(super) fn rewrite(
        &mut self,
        root: ComponentResourceLocation<'a>,
        mut fragment: Document,
        destination: Option<(&Document, NodeId)>,
    ) -> Result<CopiedDefinition<'a>> {
        let origin = self.sources.origin(root)?;
        let nodes = subtree(origin.document, root.node);
        let targets = destination.map(|(document, root)| subtree(document, root));
        let fragment_nodes = subtree(&fragment, fragment.ast().root());
        require(
            nodes.len() == fragment_nodes.len()
                && targets.as_ref().is_none_or(|ids| ids.len() == nodes.len()),
            "The copied definition changed its object structure.",
        )?;
        let mut bindings = Vec::new();
        let mut edits = Vec::new();
        for (index, (&original, &copied)) in nodes.iter().zip(&fragment_nodes).enumerate() {
            let node = element(&fragment, copied)?;
            edits.extend(rewrite_keys(&fragment, node, |key| {
                let lookup = resolve(origin, original, key, None)?;
                let definition = lookup.unique();
                let by_provider = definition.is_none();
                let dependency = definition.or_else(|| lookup.provider()).ok_or_else(|| {
                    ToolError(format!(
                        "The resource '{key}' has no provable authored scope across themes."
                    ))
                })?;
                let inside = (by_provider || dependency != root)
                    && dependency.address == root.address
                    && nodes.contains(&dependency.node);
                let retained = if let (false, Some((document, _)), Some(ids)) =
                    (inside, destination, &targets)
                {
                    let lookup = resolve(self.sources.scope(document), ids[index], key, None)?;
                    let actual = if by_provider {
                        lookup.provider()
                    } else {
                        lookup.unique()
                    };
                    actual == Some(dependency)
                } else {
                    false
                };
                let (next, target) = if inside {
                    (None, CopiedTarget::Internal(dependency.node))
                } else if retained {
                    (None, CopiedTarget::Retained(dependency))
                } else if let Some(dependency) = definition {
                    (Some(self.import(dependency)?), CopiedTarget::Imported)
                } else {
                    return Err(ToolError(format!(
                        "The theme-dependent resource '{key}' would change scope in the copy."
                    )));
                };
                bindings.push(CopiedBinding {
                    index,
                    key: next.clone().unwrap_or_else(|| key.into()),
                    target,
                    by_provider,
                });
                Ok(next)
            })?);
        }
        apply_fragment_edits(&mut fragment, edits)?;
        Ok(CopiedDefinition {
            source: fragment.source().into(),
            nodes,
            bindings,
        })
    }

    fn import(&mut self, resource: ComponentResourceLocation<'a>) -> Result<String> {
        if let Some(key) = self.keys.get(&resource) {
            return Ok(key.clone());
        }
        require(
            self.active.len() < 128,
            "The resource dependency chain is too deep to copy.",
        )?;
        require(
            self.active.insert(resource),
            "A cyclic resource dependency cannot be copied independently.",
        )?;
        let origin = self.sources.origin(resource)?;
        let key = literal_key(element(origin.document, resource.node)?).ok_or_else(|| {
            ToolError("A copied resource must have an authored string key.".into())
        })?;
        let stem = key.chars().take(128).collect::<String>();
        let base = format!("{}_{stem}", self.prefix);
        let mut copy_key = base.clone();
        let mut suffix = 2;
        while !self.reserved.insert(copy_key.clone()) {
            copy_key = format!("{base}{suffix}");
            suffix += 1;
        }
        let fragment = self.fragment(resource)?;
        let definition = self.rewrite(resource, fragment, None)?;
        self.active.remove(&resource);
        self.keys.insert(resource, copy_key.clone());
        self.definitions.push((copy_key.clone(), definition));
        Ok(copy_key)
    }

    pub(super) fn install(
        &self,
        candidate: &mut Document,
        owner: Option<NodeId>,
    ) -> Result<BTreeMap<String, NodeId>> {
        let mut installed = BTreeMap::new();
        for (key, copy) in &self.definitions {
            let owner = owner.ok_or_else(|| {
                ToolError(
                    "The template has no writable resource scope for its private dependencies."
                        .into(),
                )
            })?;
            let mut edits = plan_component_action(
                candidate,
                &ComponentAction::CreateResource {
                    owner,
                    key: key.clone(),
                    source: copy.source.clone(),
                },
            )?;
            match_insertion_line_endings(candidate, &mut edits);
            candidate
                .apply("Stage a private template dependency", edits)
                .map_err(error)?;
            let node = resolve(self.sources.scope(candidate), owner, key, None)?
                .unique()
                .filter(|location| location.uri.is_none())
                .ok_or_else(|| ToolError("The private resource was shadowed.".into()))?;
            installed.insert(key.clone(), node.node);
        }
        Ok(installed)
    }

    pub(super) fn validate(
        &self,
        candidate: &Document,
        root: NodeId,
        template: &CopiedDefinition<'a>,
        installed: &BTreeMap<String, NodeId>,
    ) -> Result<()> {
        let scope = self.sources.scope(candidate);
        template.validate(scope, root, installed)?;
        for (key, copy) in &self.definitions {
            copy.validate(scope, installed[key], installed)?;
        }
        Ok(())
    }
}

impl CopiedDefinition<'_> {
    fn validate(
        &self,
        scope: ResourceScope<'_>,
        root: NodeId,
        installed: &BTreeMap<String, NodeId>,
    ) -> Result<()> {
        let nodes = subtree(scope.document, root);
        require(
            nodes.len() == self.nodes.len(),
            "The copied definition changed its object structure.",
        )?;
        let copies = self
            .nodes
            .iter()
            .copied()
            .zip(nodes.iter().copied())
            .collect::<BTreeMap<_, _>>();
        for binding in &self.bindings {
            let expected = match binding.target {
                CopiedTarget::Internal(node) => scope.location(copies[&node]),
                CopiedTarget::Retained(location) => location,
                CopiedTarget::Imported => scope.location(installed[&binding.key]),
            };
            let lookup = resolve(scope, nodes[binding.index], &binding.key, None)?;
            let actual = if binding.by_provider {
                lookup.provider()
            } else {
                lookup.unique()
            };
            require(
                actual == Some(expected),
                format!(
                    "The copy would change the meaning of the resource '{}'.",
                    binding.key
                ),
            )?;
        }
        Ok(())
    }
}

fn subtree(document: &Document, root: NodeId) -> Vec<NodeId> {
    document
        .ast()
        .subtree(root)
        .filter(|node| node.is_element())
        .map(|node| node.id)
        .collect()
}

pub(super) fn match_insertion_line_endings(document: &Document, edits: &mut [Edit]) {
    let source = document.source();
    for edit in edits {
        let Edit::InsertChild {
            parent,
            source: fragment,
            ..
        } = edit
        else {
            continue;
        };
        let Some(parent) = document.ast().node(*parent) else {
            continue;
        };
        let crlf = source[..parent.span.start].rsplit_once('\n').map_or_else(
            || source.contains("\r\n"),
            |(before, _)| before.ends_with('\r'),
        );
        let text = fragment.replace("\r\n", "\n");
        *fragment = if crlf {
            text.replace('\n', "\r\n")
        } else {
            text
        };
    }
}

fn apply_fragment_edits(document: &mut Document, edits: Vec<Edit>) -> Result<()> {
    let mut patches = Vec::new();
    let mut attributes = Vec::new();
    for edit in edits {
        match edit {
            Edit::Text { span, replacement } => patches.push(TextPatch { span, replacement }),
            edit => attributes.push(edit),
        }
    }
    document
        .apply_text_patches(
            document.revision(),
            "Rewrite private resource keys",
            patches,
        )
        .map_err(error)?;
    document
        .apply("Rewrite private resource attributes", attributes)
        .map_err(error)?;
    Ok(())
}
