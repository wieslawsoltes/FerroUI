//! Resolve values separately from the declaration identities used by refactoring.
use super::references::{Lookup, extension_kind, resolve, resource_argument, theme_key};
use super::resource_graph::ResourceScope;
use super::*;
use crate::component_palette::{PaletteColorKind, palette_color};

#[cfg(test)]
#[path = "component_resource_value_tests.rs"]
mod tests;

const MAX_REFERENCES: usize = 64;

pub struct ResourceValue<'a> {
    scope: ResourceScope<'a>,
    node: NodeId,
    anchor: Option<(ResourceScope<'a>, NodeId)>,
    mounts: Vec<(ResourceScope<'a>, NodeId)>,
}

pub struct ResourceValueStep<'a> {
    pub key: String,
    pub value: ResourceValue<'a>,
    pub reference: Option<&'static str>,
}

pub struct ResourceValueTrace<'a> {
    pub steps: Vec<ResourceValueStep<'a>>,
    pub issue: Option<String>,
}

struct Reference {
    key: String,
    dynamic: bool,
}

impl<'a> ResourceValue<'a> {
    pub fn is_static_alias(document: &Document, node: &Node) -> bool {
        extension_kind(document, node.id, &node.name) == Some(false)
            && resource_owner(document, node.id).is_some()
    }

    pub fn lookup(
        document: &'a Document,
        address: &'a str,
        sources: Option<&'a dyn ComponentResourceSources>,
        at: NodeId,
        key: &str,
        context: Option<&ComponentResourceContext>,
    ) -> Result<Option<Self>> {
        let mut scope = ResourceScope::root(document, address, sources);
        scope.track_value = true;
        Ok(Self::from_lookup(
            scope,
            resolve(scope, at, key, context)?,
            None,
            &[],
        ))
    }

    pub fn location(&self) -> ComponentResourceLocation<'a> {
        self.scope.location(self.node)
    }

    pub fn document(&self) -> &'a Document {
        self.scope.document
    }

    /// Collapse object-form StaticResource aliases when the terminal type is known.
    pub fn alias_target(
        self,
        key: &str,
        context: Option<&ComponentResourceContext>,
    ) -> Option<Self> {
        let trace = self.trace(key, context, false);
        if trace.issue.is_some() {
            return None;
        }
        trace.steps.into_iter().last().map(|step| step.value)
    }

    pub fn color_trace(
        self,
        key: &str,
        context: &ComponentResourceContext,
    ) -> ResourceValueTrace<'a> {
        self.trace(key, Some(context), true)
    }

    fn from_lookup(
        scope: ResourceScope<'a>,
        lookup: Lookup<'a>,
        anchor: Option<(ResourceScope<'a>, NodeId)>,
        outer_mounts: &[(ResourceScope<'a>, NodeId)],
    ) -> Option<Self> {
        let location = lookup.unique()?;
        let mut origin = lookup.origin?;
        origin.mounts.extend_from_slice(outer_mounts);
        Some(Self {
            scope: origin.scope,
            node: location.node,
            anchor: origin
                .found_at
                .and_then(|at| resource_host(scope, at))
                .or(anchor),
            mounts: origin.mounts,
        })
    }

    fn trace(
        self,
        key: &str,
        context: Option<&ComponentResourceContext>,
        color: bool,
    ) -> ResourceValueTrace<'a> {
        let mut seen = BTreeSet::from([(self.scope.address, self.node)]);
        let mut steps = vec![ResourceValueStep {
            key: key.into(),
            value: self,
            reference: None,
        }];
        let mut needs_color = false;
        let result = (|| {
            loop {
                let value = &steps.last().expect("root declaration").value;
                let node = element(value.document(), value.node)?;
                let reference = if let Some(reference) = object_reference(value.document(), node)? {
                    require(
                        !reference.dynamic,
                        "A keyed DynamicResource is a binding with no fixed resource type",
                    )?;
                    reference
                } else if color && !needs_color {
                    let Some(reference) = brush_color_reference(value.document(), node)? else {
                        return Ok(());
                    };
                    needs_color = true;
                    reference
                } else {
                    require(
                        !needs_color
                            || palette_color(value.document(), value.node)?
                                .is_some_and(|entry| entry.kind == PaletteColorKind::Color),
                        "The brush's Color reference does not resolve to an authored Color",
                    )?;
                    return Ok(());
                };
                require(
                    steps.len() < MAX_REFERENCES,
                    "The resource reference chain is too long",
                )?;
                let next = value.follow(&reference, context)?.ok_or_else(|| {
                    ToolError(format!(
                        "Resource '{}' has no unambiguous authored value",
                        reference.key
                    ))
                })?;
                require(
                    seen.insert((next.scope.address, next.node)),
                    "The resource references form a cycle",
                )?;
                steps.push(ResourceValueStep {
                    key: reference.key,
                    value: next,
                    reference: Some(if reference.dynamic {
                        "DynamicResource"
                    } else {
                        "StaticResource"
                    }),
                });
            }
        })();
        ResourceValueTrace {
            steps,
            issue: result.err().map(|error: ToolError| error.0),
        }
    }

    fn follow(
        &self,
        reference: &Reference,
        context: Option<&ComponentResourceContext>,
    ) -> Result<Option<Self>> {
        if reference.dynamic {
            let (scope, at) = self.anchor.ok_or_else(|| {
                ToolError("The dynamic resource host is unavailable in this scope".into())
            })?;
            // A brush declared under an element uses that element's actual theme.
            // A brush from a mounted provider retains the provider's declared theme.
            let declared = (scope.address != self.scope.address
                && self.scope.theme_at(self.node).is_some())
            .then(|| self.declared_context(context))
            .transpose()?;
            let lookup = resolve(scope, at, &reference.key, declared.as_ref().or(context))?;
            return Ok(Self::from_lookup(scope, lookup, self.anchor, &[]));
        }
        // Static resources run during loading using the declaration's ambient stack.
        let theme = self.declared_context(context)?;
        let parents = std::iter::once((self.scope, self.node)).chain(self.mounts.iter().copied());
        for (index, (scope, at)) in parents.enumerate() {
            let lookup = resolve(scope, at, &reference.key, Some(&theme))?;
            if lookup.external || !lookup.candidates.is_empty() {
                return Ok(Self::from_lookup(
                    scope,
                    lookup,
                    self.anchor,
                    &self.mounts[index..],
                ));
            }
        }
        Ok(None)
    }

    fn declared_context(
        &self,
        active: Option<&ComponentResourceContext>,
    ) -> Result<ComponentResourceContext> {
        let Some(layer) = self.scope.theme_at(self.node) else {
            return Ok(ComponentResourceContext::default());
        };
        let unresolved = || ToolError("The declaration's theme inheritance is unavailable".into());
        let name = theme_key(layer.document, layer.node).ok_or_else(unresolved)?;
        let inherited_themes = if matches!(name.as_str(), "Default" | "Light" | "Dark") {
            Vec::new()
        } else {
            let active = active.ok_or_else(unresolved)?;
            if active.theme.as_deref() == Some(name.as_str()) {
                active.inherited_themes.clone()
            } else {
                let index = active
                    .inherited_themes
                    .iter()
                    .position(|theme| *theme == name)
                    .ok_or_else(unresolved)?;
                active.inherited_themes[index + 1..].to_vec()
            }
        };
        Ok(ComponentResourceContext {
            theme: Some(name),
            inherited_themes,
        })
    }
}

fn resource_host(scope: ResourceScope<'_>, at: NodeId) -> Option<(ResourceScope<'_>, NodeId)> {
    let mut current = Some(at);
    while let Some(node) = current.and_then(|id| scope.document.ast().node(id)) {
        if node.kind == NodeKind::Element
            && !matches!(
                node.local_name(),
                "ResourceDictionary" | "Style" | "ControlTheme" | "Styles" | "ResourceInclude"
            )
        {
            return Some((scope, node.id));
        }
        current = node.parent;
    }
    None
}

fn object_reference(document: &Document, node: &Node) -> Result<Option<Reference>> {
    let Some(dynamic) = extension_kind(document, node.id, &node.name) else {
        return Ok(None);
    };
    let attribute = component_property_attribute(document, node.id, "ResourceKey")?;
    let members = property_elements(document, node, "ResourceKey")?;
    let key = match (attribute, members.as_slice()) {
        (Some(attribute), []) => attribute.markup().map_err(ToolError)?,
        (None, [member]) => MarkupValue::parse(property_text(document, member).unwrap_or_default())
            .map_err(ToolError)?,
        _ => return Err(unverified_key()),
    };
    let MarkupValue::Literal(key) = key else {
        return Err(unverified_key());
    };
    require(!key.is_empty(), unverified_key().0)?;
    Ok(Some(Reference { key, dynamic }))
}

fn brush_color_reference(document: &Document, node: &Node) -> Result<Option<Reference>> {
    if !palette_color(document, node.id)?
        .is_some_and(|entry| entry.kind == PaletteColorKind::SolidColorBrush)
    {
        return Ok(None);
    }
    let attribute = component_property_attribute(document, node.id, "Color")?;
    let members = property_elements(document, node, "Color")?;
    let value = match (attribute, members.as_slice()) {
        (Some(attribute), []) => attribute.markup().map_err(ToolError)?,
        (None, [member]) => {
            let children = document.ast().element_children(member.id);
            if let [child] = children.as_slice() {
                return object_reference(document, element(document, *child)?);
            }
            return Ok(None);
        }
        _ => return Ok(None),
    };
    let MarkupValue::Extension(extension) = value else {
        return Ok(None);
    };
    let Some(dynamic) = extension_kind(document, node.id, &extension.name) else {
        return Ok(None);
    };
    let index = resource_argument(&extension).ok_or_else(unverified_key)?;
    let MarkupValue::Literal(key) = &extension.arguments[index].value else {
        return Err(unverified_key());
    };
    require(!key.is_empty(), unverified_key().0)?;
    Ok(Some(Reference {
        key: key.clone(),
        dynamic,
    }))
}

fn property_text<'a>(document: &'a Document, property: &Node) -> Option<&'a str> {
    let mut content = property
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|child| child.kind != NodeKind::Comment);
    let first = content.next()?;
    if first.kind != NodeKind::Text || content.next().is_some() {
        return None;
    }
    first.text.as_deref().map(str::trim)
}

fn unverified_key() -> ToolError {
    ToolError("The resource reference needs a literal string key".into())
}
