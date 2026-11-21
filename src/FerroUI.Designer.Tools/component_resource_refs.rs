//! Lexical resource lookup and structured markup references shared by catalog and edits.
use super::*;
use super::resource_graph::ResourceScope;
use ferroui_designer_core::design_fixtures::is_fixtures;
use ferroui_designer_core::{MarkupArgument, MarkupExtension};

#[path = "component_resource_copy_keys.rs"]
pub(super) mod copy_keys;

/// The runtime's active theme and its inheritance chain, nearest inherited variant
/// first. `None` (or `Default`) looks only in Default dictionaries before merges.
/// Keys are decoded string theme keys; custom expression keys remain uncertain.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentResourceContext {
    pub theme: Option<String>,
    pub inherited_themes: Vec<String>,
}

/// All declarations which might supply a resource in this context. With no theme
/// context, this is a conservative union across theme variants and their fallbacks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentResourceResolution {
    pub candidates: Vec<NodeId>,
    /// Some applicable theme/lookup path has no authored declaration for the key.
    pub may_be_missing: bool,
    /// An external provider, unsupported key, or invalid dictionary is unresolved.
    pub external: bool,
}

/// The authored results of a lookup across local and referenced dictionaries.
pub struct ScopedResourceResolution<'a> {
    pub candidates: Vec<ComponentResourceLocation<'a>>,
    pub may_be_missing: bool,
    pub external: bool,
}
impl ComponentResourceResolution {
    /// A single declaration only when every possible lookup path resolves to it.
    pub fn resolved(&self) -> Option<NodeId> {
        (!self.may_be_missing && !self.external && self.candidates.len() == 1)
            .then(|| self.candidates[0])
    }
}

#[derive(Default)]
pub(super) struct Lookup<'a> {
    candidates: BTreeSet<ComponentResourceLocation<'a>>,
    may_be_missing: bool,
    external: bool,
    /// The authored scope which determines every variant, when it can be proven.
    provider: Option<ComponentResourceLocation<'a>>,
}
impl<'a> Lookup<'a> {
    pub(super) fn unique(&self) -> Option<ComponentResourceLocation<'a>> {
        (!self.may_be_missing && !self.external && self.candidates.len() == 1)
            .then(|| *self.candidates.first().expect("one candidate"))
    }

    pub(super) fn provider(&self) -> Option<ComponentResourceLocation<'a>> {
        self.provider.filter(|_| !self.external && !self.may_be_missing)
    }

    fn missing() -> Self {
        Self {
            may_be_missing: true,
            ..Self::default()
        }
    }
    fn external() -> Self {
        Self {
            external: true,
            ..Self::missing()
        }
    }
    /// Join mutually possible theme/provider outcomes.
    fn alternative(&mut self, other: Self) {
        self.provider = if self.candidates.is_empty() && !self.may_be_missing && !self.external {
            other.provider()
        } else {
            None
        };
        self.candidates.extend(other.candidates);
        self.may_be_missing |= other.may_be_missing;
        self.external |= other.external;
    }
    /// Later scopes are reachable only along lookup paths that can still miss.
    fn fallback(&mut self, other: Self) {
        if self.may_be_missing {
            self.provider = if self.candidates.is_empty() && !self.external { other.provider() } else { None };
            self.candidates.extend(other.candidates);
            self.may_be_missing = other.may_be_missing;
            self.external |= other.external;
        }
    }
}

/// Resolve a resource without guessing an active theme. Local entries take precedence;
/// then theme dictionaries and their Default fallback; then reverse-order merges and
/// ancestors. Ambiguous theme variants and unindexed external providers return None.
pub fn resolve_component_resource(
    document: &Document,
    target: NodeId,
    key: &str,
) -> Option<NodeId> {
    component_resource_resolution(document, target, key, None)
        .ok()?
        .resolved()
}

/// Resolve using the actual runtime theme and complete inherited-variant chain.
pub fn resolve_component_resource_in_context(
    document: &Document,
    target: NodeId,
    key: &str,
    context: &ComponentResourceContext,
) -> Option<NodeId> {
    component_resource_resolution(document, target, key, Some(context))
        .ok()?
        .resolved()
}

/// Inspect resource lookup, including possible references which cannot safely be
/// represented by a single NodeId. Authoring uses this conservative information to
/// protect resources referenced only in some themes or behind external providers.
pub fn component_resource_resolution(
    document: &Document,
    target: NodeId,
    key: &str,
    context: Option<&ComponentResourceContext>,
) -> Result<ComponentResourceResolution> {
    let result = resolve(ResourceScope::root(document, "", None), target, key, context)?;
    Ok(ComponentResourceResolution {
        candidates: result.candidates.into_iter().map(|candidate| candidate.node).collect(),
        may_be_missing: result.may_be_missing,
        external: result.external,
    })
}

/// Resolve a declaration from the accepted include graph. An opaque provider,
/// unknown theme path or duplicate key can prevent proving a single target.
pub fn component_resource_location<'a>(
    document: &'a Document,
    address: &'a str,
    target: NodeId,
    key: &str,
    context: Option<&ComponentResourceContext>,
    sources: &'a dyn ComponentResourceSources,
) -> Result<Option<ComponentResourceLocation<'a>>> {
    let result = resolve(ResourceScope::root(document, address, Some(sources)), target, key, context)?;
    Ok(result.unique())
}

/// Resolve a scope through the same include graph and theme rules as the native inspector.
pub fn scoped_resource_resolution<'a>(
    document: &'a Document,
    address: &'a str,
    target: NodeId,
    key: &str,
    context: Option<&ComponentResourceContext>,
    sources: &'a dyn ComponentResourceSources,
) -> Result<ScopedResourceResolution<'a>> {
    let result = resolve(ResourceScope::root(document, address, Some(sources)), target, key, context)?;
    Ok(ScopedResourceResolution {
        candidates: result.candidates.into_iter().collect(),
        may_be_missing: result.may_be_missing,
        external: result.external,
    })
}

type Visited<'a> = BTreeSet<(&'a str, NodeId)>;

pub(super) fn resolve<'a>(
    scope: ResourceScope<'a>,
    target: NodeId,
    key: &str,
    context: Option<&ComponentResourceContext>,
) -> Result<Lookup<'a>> {
    let document = scope.document;
    valid_document(document)?;
    element(document, target)?;
    if let Some(context) = context {
        let mut seen = BTreeSet::new();
        for theme in context.theme.iter().chain(&context.inherited_themes) {
            require(
                !theme.is_empty() && seen.insert(theme),
                "Theme inheritance must contain distinct nonempty keys",
            )?;
        }
        require(
            context.inherited_themes.is_empty()
                || context
                    .theme
                    .as_deref()
                    .is_some_and(|theme| theme != "Default"),
            "Inherited themes require a concrete active theme",
        )?;
        require(
            !context
                .inherited_themes
                .iter()
                .any(|theme| theme == "Default"),
            "Default is a fallback, not an inherited theme",
        )?;
    }
    let mut result = Lookup::missing();
    let mut ancestor = Some(target);
    while result.may_be_missing {
        let Some(id) = ancestor else { break };
        let node = document
            .ast()
            .node(id)
            .ok_or_else(|| ToolError("Missing resource ancestor".into()))?;
        result.fallback(lookup_scope(
            scope,
            node,
            key,
            context,
            &mut BTreeSet::new(),
        ));
        ancestor = node.parent;
    }
    Ok(result)
}

fn lookup_scope<'a>(
    scope: ResourceScope<'a>,
    owner: &Node,
    key: &str,
    context: Option<&ComponentResourceContext>,
    seen: &mut Visited<'a>,
) -> Lookup<'a> {
    let id = (scope.address, owner.id);
    if !seen.insert(id) {
        return Lookup::external();
    }
    let mut result = lookup_owner(scope, owner, key, context, seen);
    if result.provider.is_none() && !result.external && !result.may_be_missing && !result.candidates.is_empty() {
        result.provider = Some(scope.location(owner.id));
    }
    seen.remove(&id);
    result
}

fn lookup_owner<'a>(
    scope: ResourceScope<'a>,
    owner: &Node,
    key: &str,
    context: Option<&ComponentResourceContext>,
    seen: &mut Visited<'a>,
) -> Lookup<'a> {
    let document = scope.document;
    if framework(owner) && owner.local_name() == "ResourceDictionary" {
        if external_dictionary(document, owner) {
            return Lookup::external();
        }
        return lookup_children(scope, &owner.children, key, context, seen);
    }
    let Ok(resources) = property_elements(document, owner, "Resources") else {
        return Lookup::external();
    };
    let children = resources
        .into_iter()
        .flat_map(|node| node.children.iter().copied())
        .collect::<Vec<_>>();
    lookup_children(scope, &children, key, context, seen)
}

fn dictionary_property(document: &Document, node: &Node, member: &str) -> bool {
    let Some(owner) = node.parent.and_then(|id| document.ast().node(id)) else {
        return false;
    };
    framework(owner)
        && owner.local_name() == "ResourceDictionary"
        && node.kind == NodeKind::PropertyElement
        && PropertyIdentity::query(document, owner.id, member)
            .is_ok_and(|identity| identity.matches_element(owner, node))
}

fn lookup_children<'a>(
    scope: ResourceScope<'a>,
    children: &[NodeId],
    key: &str,
    context: Option<&ComponentResourceContext>,
    seen: &mut Visited<'a>,
) -> Lookup<'a> {
    let document = scope.document;
    let nodes = children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .collect::<Vec<_>>();
    let local = nodes
        .iter()
        .filter(|node| literal_key(node).as_deref() == Some(key))
        .map(|node| scope.location(node.id))
        .collect::<BTreeSet<_>>();
    if !local.is_empty() {
        return Lookup {
            candidates: local,
            ..Lookup::default()
        };
    }
    let mut result = Lookup::missing();
    // Explicit dictionary wrappers are the property's actual resource collection.
    for node in nodes.iter().rev().filter(|node| {
        framework(node)
            && node.local_name() == "ResourceDictionary"
            && key_attribute(node).is_none()
    }) {
        if !result.may_be_missing {
            return result;
        }
        result.fallback(lookup_scope(scope, node, key, context, seen));
    }
    let themes = nodes
        .iter()
        .filter(|node| dictionary_property(document, node, "ThemeDictionaries"))
        .collect::<Vec<_>>();
    if result.may_be_missing && !themes.is_empty() {
        let mut alternatives = Lookup {
            external: themes.len() > 1,
            ..Lookup::default()
        };
        for themes in themes {
            alternatives.alternative(lookup_themes(
                scope,
                themes,
                key,
                context,
                &mut seen.clone(),
            ));
        }
        result.fallback(alternatives);
    }
    let merged = nodes
        .iter()
        .filter(|node| dictionary_property(document, node, "MergedDictionaries"))
        .collect::<Vec<_>>();
    if result.may_be_missing && !merged.is_empty() {
        let mut alternatives = Lookup {
            external: merged.len() > 1,
            ..Lookup::default()
        };
        for merged in merged {
            let mut group = Lookup::missing();
            let mut group_seen = seen.clone();
            for child in merged
                .children
                .iter()
                .rev()
                .filter_map(|id| document.ast().node(*id))
                .filter(|node| node.is_element())
            {
                if !group.may_be_missing {
                    break;
                }
                let value = lookup_dictionary(scope, child, key, context, &mut group_seen);
                group.fallback(value);
            }
            alternatives.alternative(group);
        }
        result.fallback(alternatives);
    }
    result
}

fn lookup_themes<'a>(
    scope: ResourceScope<'a>,
    themes: &Node,
    key: &str,
    context: Option<&ComponentResourceContext>,
    seen: &mut Visited<'a>,
) -> Lookup<'a> {
    let document = scope.document;
    let variants = themes
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();
    let mut result = Lookup::missing();
    if let Some(context) = context {
        for theme in context
            .theme
            .iter()
            .chain(&context.inherited_themes)
            .filter(|name| name.as_str() != "Default")
        {
            if !result.may_be_missing {
                return result;
            }
            result.fallback(lookup_theme_name(
                scope,
                &variants,
                theme,
                key,
                Some(context),
                seen,
            ));
        }
    } else {
        // No runtime theme is known. Every non-default dictionary might be selected
        // directly or through inheritance; a variant absent from this scope may fall
        // through to Default. Unioning outcomes also covers nested theme dictionaries.
        for variant in &variants {
            let name = theme_key(document, variant);
            if name.as_deref() == Some("Default") {
                continue;
            }
            let mut value =
                lookup_dictionary(scope, variant, key, context, &mut seen.clone());
            if name.is_none() && (!value.candidates.is_empty() || value.external) {
                value.external = true;
            }
            result.alternative(value);
        }
    }
    if result.may_be_missing {
        result.fallback(lookup_theme_name(
            scope, &variants, "Default", key, context, seen,
        ));
    }
    result
}

fn lookup_theme_name<'a>(
    scope: ResourceScope<'a>,
    variants: &[&Node],
    theme: &str,
    key: &str,
    context: Option<&ComponentResourceContext>,
    seen: &mut Visited<'a>,
) -> Lookup<'a> {
    let mut result: Option<Lookup> = None;
    let mut matches = 0;
    for variant in variants {
        let name = theme_key(scope.document, variant);
        if name.as_deref().is_some_and(|name| name != theme) {
            continue;
        }
        let mut value = lookup_dictionary(scope, variant, key, context, &mut seen.clone());
        if name.is_none() {
            // This runtime expression may denote the requested variant or another one.
            value.may_be_missing = true;
            value.external |= !value.candidates.is_empty();
        } else {
            matches += 1;
        }
        if let Some(result) = &mut result {
            result.alternative(value);
        } else {
            result = Some(value);
        }
    }
    let mut result = result.unwrap_or_else(Lookup::missing);
    result.external |= matches > 1;
    result
}

fn lookup_dictionary<'a>(
    scope: ResourceScope<'a>,
    variant: &Node,
    key: &str,
    context: Option<&ComponentResourceContext>,
    seen: &mut Visited<'a>,
) -> Lookup<'a> {
    if framework(variant) && variant.local_name() == "ResourceDictionary" {
        return lookup_scope(scope, variant, key, context, seen);
    }
    if let Some(included) = scope.include(variant) {
        let ast = included.document.ast();
        if let Some(root) = ast.node(ast.root()).filter(|node| {
            framework(node) && node.local_name() == "ResourceDictionary"
        }) {
            return lookup_scope(included, root, key, context, seen);
        }
    }
    Lookup::external()
}

/// Decode an authored theme layer using the same rules as palette and resource lookup.
pub fn theme_key(document: &Document, node: &Node) -> Option<String> {
    if let Some(key) = literal_key(node) {
        // The runtime's built-in ThemeVariant converter trims surrounding whitespace.
        return Some(if matches!(key.trim(), "Default" | "Light" | "Dark") {
            key.trim().into()
        } else {
            key
        });
    }
    let MarkupValue::Extension(extension) = key_attribute(node)?.markup().ok()? else {
        return None;
    };
    let namespaces = document.ast().namespaces(node.id);
    let (prefix, name) = extension
        .name
        .rsplit_once(':')
        .unwrap_or(("", &extension.name));
    if namespaces.get(prefix).map(String::as_str) != Some(XAML)
        || name != "Static"
        || extension.arguments.len() != 1
    {
        return None;
    }
    let argument = &extension.arguments[0];
    if !matches!(argument.name.as_deref(), None | Some("Member")) {
        return None;
    }
    let MarkupValue::Literal(member) = &argument.value else {
        return None;
    };
    let (owner, value) = member.rsplit_once('.')?;
    let (prefix, owner) = owner.rsplit_once(':').unwrap_or(("", owner));
    if owner != "ThemeVariant"
        || !namespaces
            .get(prefix)
            .is_some_and(|namespace| framework_type_namespace(owner, namespace))
    {
        return None;
    }
    matches!(value, "Default" | "Light" | "Dark").then(|| value.into())
}

pub fn component_resource_references(
    document: &Document,
    resource: NodeId,
) -> Result<Vec<ComponentReference>> {
    valid_document(document)?;
    element(document, resource)?;
    Ok(all_references(document)?
        .into_iter()
        .filter(|(_, _, _, target)| *target == resource)
        .map(|(node, property, dynamic, _)| ComponentReference {
            node,
            property,
            dynamic,
        })
        .collect())
}

pub(crate) fn all_references(document: &Document) -> Result<Vec<(NodeId, String, bool, NodeId)>> {
    let mut result = Vec::new();
    for node in document.ast().elements() {
        for (property, dynamic, key) in resource_uses(document, node)? {
            for target in component_resource_resolution(document, node.id, &key, None)?.candidates {
                result.push((node.id, property.clone(), dynamic, target));
            }
        }
    }
    Ok(result)
}

/// Keep unresolved uses as well as resolved ones: moving/copying a definition can
/// capture a previously unknown key in its new lexical resource scope.
pub(super) fn resource_uses(document: &Document, node: &Node) -> Result<Vec<(String, bool, String)>> {
    let mut result = Vec::new();
    if framework(node)
        && matches!(
            node.local_name(),
            "StaticResource"
                | "DynamicResource"
                | "StaticResourceExtension"
                | "DynamicResourceExtension"
        )
    {
        if let Some((property, key)) = object_resource_key(document, node)? {
            result.push((
                property,
                matches!(
                    node.local_name(),
                    "DynamicResource" | "DynamicResourceExtension"
                ),
                key,
            ));
        }
    }
    for attribute in &node.attributes {
        if attribute.is_namespace() || is_fixtures(attribute) || !attribute.value.starts_with('{') {
            continue;
        }
        let value = attribute.markup().map_err(ToolError)?;
        visit_resources(document, node.id, &value, &mut |key, dynamic| {
            result.push((attribute.name.clone(), dynamic, key.into()));
        });
    }
    Ok(result)
}

pub(super) fn object_resource_key(document: &Document, node: &Node) -> Result<Option<(String, String)>> {
    let Some(attribute) = component_property_attribute(document, node.id, "ResourceKey")? else {
        return Ok(None);
    };
    Ok(match attribute.markup().map_err(ToolError)? {
        MarkupValue::Literal(key) => Some((attribute.name.clone(), key)),
        _ => None,
    })
}

/// A copied definition retains the meaning of every locally resolvable reference.
/// References to resources inside the copied subtree follow their corresponding copy;
/// references outside it must still resolve to the original declaration. External
/// dictionaries are intentionally left unresolved, as in the catalog's lookup contract.
pub(crate) fn validate_copied_references(
    document: &Document,
    original: NodeId,
    candidate: &Document,
    copied: NodeId,
) -> Result<()> {
    let before = document
        .ast()
        .subtree(original)
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();
    let after = candidate
        .ast()
        .subtree(copied)
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();
    require(
        before.len() == after.len(),
        "The copied definition changed its object structure",
    )?;
    let copies = before
        .iter()
        .zip(&after)
        // The root receives a new resource key. A reference to the old root key
        // continues to refer to the original definition, unlike nested resources.
        .filter(|(a, b)| literal_key(a) == literal_key(b))
        .map(|(a, b)| (a.id, b.id))
        .collect::<BTreeMap<_, _>>();
    for (before, after) in before.into_iter().zip(after) {
        let original_uses = resource_uses(document, before)?;
        let copied_uses = resource_uses(candidate, after)?;
        require(
            original_uses == copied_uses,
            "The copied definition changed its resource expressions",
        )?;
        for (_, _, key) in original_uses {
            let mut expected = component_resource_resolution(document, before.id, &key, None)?;
            expected.candidates = expected
                .candidates
                .into_iter()
                .map(|resource| copies.get(&resource).copied().unwrap_or(resource))
                .collect();
            expected.candidates.sort_unstable();
            let actual = component_resource_resolution(candidate, after.id, &key, None)?;
            require(
                actual == expected,
                format!(
                    "Copying this definition would change the resource '{key}'; choose a scope that preserves its dependencies"
                ),
            )?;
        }
    }
    Ok(())
}

pub(crate) fn extension_kind(document: &Document, node: NodeId, name: &str) -> Option<bool> {
    let (prefix, local) = name.rsplit_once(':').unwrap_or(("", name));
    if !document
        .ast()
        .namespaces(node)
        .get(prefix)
        .is_some_and(|namespace| framework_type_namespace(local, namespace))
    {
        return None;
    }
    match local {
        "StaticResource" | "StaticResourceExtension" => Some(false),
        "DynamicResource" | "DynamicResourceExtension" => Some(true),
        _ => None,
    }
}

pub(crate) fn resource_argument(extension: &MarkupExtension) -> Option<usize> {
    extension
        .arguments
        .iter()
        .position(|argument| argument.name.as_deref() == Some("ResourceKey"))
        .or_else(|| {
            extension
                .arguments
                .iter()
                .position(|argument| argument.name.is_none())
        })
}

fn visit_resources(
    document: &Document,
    node: NodeId,
    value: &MarkupValue,
    visit: &mut impl FnMut(&str, bool),
) {
    let MarkupValue::Extension(extension) = value else {
        return;
    };
    if let Some(dynamic) = extension_kind(document, node, &extension.name) {
        if let Some(index) = resource_argument(extension) {
            if let MarkupValue::Literal(key) = &extension.arguments[index].value {
                visit(key, dynamic);
            }
        }
    }
    for argument in &extension.arguments {
        visit_resources(document, node, &argument.value, visit);
    }
}

pub(crate) fn rename_resource(
    document: &Document,
    resource: NodeId,
    key: &str,
) -> Result<Vec<Edit>> {
    let node = element(document, resource)?;
    let old = literal_key(node)
        .ok_or_else(|| ToolError("Only explicit string-key resources can be renamed".into()))?;
    super::authoring::validate_key(key)?;
    if old == key {
        return Ok(Vec::new());
    }
    let owner = resource_owner(document, resource)
        .ok_or_else(|| ToolError("The resource is not in an authored dictionary".into()))?;
    super::authoring::ensure_key_available(document, owner, key, Some(resource))?;
    let mut edits = Vec::new();
    for element in document.ast().elements() {
        for (_, _, used_key) in resource_uses(document, element)? {
            if used_key == old {
                let resolution = component_resource_resolution(document, element.id, &old, None)?;
                require(
                    !resolution.candidates.contains(&resource)
                        || resolution.resolved() == Some(resource),
                    "This reference selects different resources across theme variants or external dictionaries; rename the resource family together in source",
                )?;
            }
        }
        if framework(element)
            && matches!(
                element.local_name(),
                "StaticResource"
                    | "DynamicResource"
                    | "StaticResourceExtension"
                    | "DynamicResourceExtension"
            )
        {
            if let Some((property, object_key)) = object_resource_key(document, element)? {
                if object_key == old
                    && resolve_component_resource(document, element.id, &old) == Some(resource)
                {
                    edits.push(Edit::SetAttribute {
                        node: element.id,
                        name: property,
                        value: escaped_literal(key),
                    });
                }
            }
        }
        for attribute in &element.attributes {
            if attribute.is_namespace() || is_fixtures(attribute) || !attribute.value.starts_with('{') {
                continue;
            }
            let mut value = attribute.markup().map_err(ToolError)?;
            if rewrite_resource(document, element.id, &mut value, resource, key) {
                edits.push(Edit::SetAttribute {
                    node: element.id,
                    name: attribute.name.clone(),
                    value: serialize(&value),
                });
            }
        }
    }
    edits.push(Edit::SetAttribute {
        node: resource,
        name: key_attribute(node).expect("literal key").name.clone(),
        value: escaped_literal(key),
    });
    let before = all_references(document)?;
    let mut candidate = document.clone();
    candidate
        .apply("Validate resource rename", edits.clone())
        .map_err(error)?;
    // Existing resolved references must retain the same declaration identity. A rename
    // cannot capture another key through an ancestor, merged dictionary or local shadow.
    let after = all_references(&candidate)?;
    require(
        before == after,
        "Renaming this resource would change another reference's scope; choose a different key",
    )?;
    Ok(edits)
}

fn rewrite_resource(
    document: &Document,
    node: NodeId,
    value: &mut MarkupValue,
    target: NodeId,
    key: &str,
) -> bool {
    let MarkupValue::Extension(extension) = value else {
        return false;
    };
    let mut changed = false;
    if extension_kind(document, node, &extension.name).is_some() {
        if let Some(index) = resource_argument(extension) {
            if let MarkupValue::Literal(old) = &extension.arguments[index].value {
                if resolve_component_resource(document, node, old) == Some(target) {
                    extension.arguments[index].value = MarkupValue::Literal(key.into());
                    changed = true;
                }
            }
        }
    }
    for argument in &mut extension.arguments {
        changed |= rewrite_resource(document, node, &mut argument.value, target, key);
    }
    changed
}

pub(crate) fn escaped_literal(value: &str) -> String {
    if value.starts_with('{') {
        format!("{{}}{value}")
    } else {
        value.into()
    }
}
pub(crate) fn argument(value: &str) -> String {
    format!(
        "'{}'",
        escaped_literal(value)
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
    )
}
fn serialize(value: &MarkupValue) -> String {
    match value {
        MarkupValue::Literal(value) => argument(value),
        MarkupValue::Extension(extension) => {
            let arguments = extension
                .arguments
                .iter()
                .map(|MarkupArgument { name, value }| {
                    let value = serialize(value);
                    name.as_ref()
                        .map_or(value.clone(), |name| format!("{name}={value}"))
                })
                .collect::<Vec<_>>()
                .join(", ");
            if arguments.is_empty() {
                format!("{{{}}}", extension.name)
            } else {
                format!("{{{} {arguments}}}", extension.name)
            }
        }
    }
}

pub(crate) fn reference_expression(
    document: &Document,
    node: NodeId,
    key: &str,
    dynamic: bool,
) -> Result<String> {
    let namespaces = document.ast().namespaces(node);
    let prefix = namespaces
        .iter()
        .find(|(_, uri)| framework_type_namespace("StaticResource", uri))
        .map(|(prefix, _)| prefix)
        .ok_or_else(|| {
            ToolError("A FerroUI namespace is required to apply a resource reference".into())
        })?;
    let name = if dynamic {
        "DynamicResource"
    } else {
        "StaticResource"
    };
    let qualified = if prefix.is_empty() {
        name.into()
    } else {
        format!("{prefix}:{name}")
    };
    Ok(format!("{{{qualified} {}}}", argument(key)))
}
