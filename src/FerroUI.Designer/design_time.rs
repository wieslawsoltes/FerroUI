//! Design-time XAML is authored independently of the application's values. The preview
//! projection resolves namespace identities, preserves source ownership, and is never saved.
use crate::DesignerSession;
use ferroui_designer_core::{Ast, Document, Edit, Node, NodeId, NodeKind, SourceSpan};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[path = "design_time_editor.rs"]
mod editor;
pub use editor::{DesignTimeEditor, DesignTimeEditorModel};

pub const DESIGN_NAMESPACE: &str = "http://schemas.microsoft.com/expression/blend/2008";
const COMPATIBILITY_NAMESPACE: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const FRAMEWORK: &str = "https://github.com/ferroui";

/// `ferro_class_info!` metadata is owned by TypeInfo and need not carry a reciprocal
/// type_info callback. Resolve that ownership instead of treating such classes as opaque.
pub(crate) fn native_type_for_node(node: &Node) -> Option<&'static ferroui_base::TypeInfo> {
    let metadata = crate::editors::markup_type_for_node(node)?;
    metadata.type_info.map(|type_| type_()).or_else(|| {
        ferroui_base::TypeInfo::registered_types()
            .into_iter()
            .find(|type_| {
                type_
                    .markup()
                    .is_some_and(|candidate| std::ptr::eq(candidate, metadata))
            })
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignTimeProperty {
    pub name: String,
    pub value: String,
    pub object: bool,
}

/// Every command changes only a design-namespace value. Application attributes,
/// bindings and property elements retain their exact source spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesignTimeEdit {
    SetProperty {
        node: NodeId,
        property: String,
        value: Option<String>,
    },
    SetObject {
        node: NodeId,
        property: String,
        source: Option<String>,
    },
    SampleItems {
        node: NodeId,
        count: Option<usize>,
    },
}

/// A popup owns an immutable acceptance token. Leaving and returning to the same
/// selection or definition cannot make an obsolete draft current again.
#[derive(Clone)]
pub struct DesignTimeSnapshot {
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    selection: Vec<NodeId>,
    scope_epoch: u64,
    generation: u64,
}
impl DesignTimeSnapshot {
    pub fn capture(session: &DesignerSession) -> Self {
        Self {
            uri: session.document_uri().into(),
            revision: session.document().revision(),
            ast: session.document().ast_snapshot(),
            selection: session.selection().to_vec(),
            scope_epoch: session.definition_scope_epoch(),
            generation: session.generation(),
        }
    }
    pub fn is_current(&self, session: &DesignerSession) -> bool {
        session.document().is_valid()
            && self.uri == session.document_uri()
            && self.revision == session.document().revision()
            && Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
            && self.selection == session.selection()
            && self.scope_epoch == session.definition_scope_epoch()
            && self.generation == session.generation()
    }
    pub fn apply(
        &self,
        session: &mut DesignerSession,
        edits: &[DesignTimeEdit],
    ) -> Result<(), String> {
        if !self.is_current(session) {
            return Err(
                "The document, selection or preview scope changed. Reopen Design Data.".into(),
            );
        }
        session.apply_design_time_edits(edits)
    }
}

impl DesignerSession {
    pub fn design_time_enabled(&self) -> bool {
        self.preview.design_time_enabled()
    }

    /// Presentation state; source and undo history remain untouched. Viewers can inspect
    /// both the sample-data preview and the authored application values.
    pub fn set_design_time_enabled(&mut self, enabled: bool) -> Result<(), String> {
        if !self.document.is_valid() {
            return Err("Repair the XAML before switching design data.".into());
        }
        self.preview
            .set_design_time_enabled(enabled, &self.document)?;
        self.definition_scope_epoch = self.definition_scope_epoch.wrapping_add(1);
        self.generation = self.generation.wrapping_add(1);
        self.refresh_layer_visuals();
        Ok(())
    }

    pub fn design_time_properties(&self, node: NodeId) -> Result<Vec<DesignTimeProperty>, String> {
        design_time_properties(&self.document, node)
    }

    pub fn apply_design_time_edits(&mut self, edits: &[DesignTimeEdit]) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let edits = plan_design_time_edits(&self.document, edits)?;
        if !edits.is_empty() {
            self.apply("Edit design-time data", edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// The author-provided design size, independent from explicit environment presets.
    /// Invalid/nonliteral hints return None. Explicit d:Width/Height remain control values.
    pub fn design_preview_size(&self) -> (Option<f64>, Option<f64>) {
        if !self.design_time_enabled() {
            return (None, None);
        }
        let properties =
            design_time_properties(&self.document, self.document.ast().root()).unwrap_or_default();
        let dimension = |names: &[&str]| {
            properties
                .iter()
                .find(|property| names.contains(&property.name.as_str()) && !property.object)
                .and_then(|property| property.value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value > 0.0)
        };
        let legacy = |name| {
            self.document
                .ast()
                .node(self.document.ast().root())
                .and_then(|root| root.attribute(name))
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value > 0.0)
        };
        (
            dimension(&["DesignWidth", "Width"]).or_else(|| legacy("Design.Width")),
            dimension(&["DesignHeight", "Height"]).or_else(|| legacy("Design.Height")),
        )
    }
}

pub fn design_time_properties(
    document: &Document,
    node: NodeId,
) -> Result<Vec<DesignTimeProperty>, String> {
    let owner = document
        .ast()
        .node(node)
        .filter(|node| node.kind == NodeKind::Element)
        .ok_or("Select an authored control or template part.")?;
    let mut result = Vec::new();
    for attribute in &owner.attributes {
        if attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE) {
            result.push(DesignTimeProperty {
                name: local(&attribute.name).into(),
                value: attribute.value.clone(),
                object: false,
            });
        }
    }
    for child in document.ast().element_children(node) {
        let child = document.ast().node(child).unwrap();
        if child.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE) {
            if let Some((_, property)) = child.local_name().split_once('.') {
                let start = child.opening_span.end;
                let end = child.closing_span.map_or(start, |span| span.start);
                result.push(DesignTimeProperty {
                    name: property.into(),
                    value: document.valid_source()[start..end].to_owned(),
                    object: true,
                });
            }
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Source-only planning is independently reusable by host editors and automation.
/// A private candidate evaluates the complete batch before the caller commits once.
pub fn plan_design_time_edits(
    document: &Document,
    commands: &[DesignTimeEdit],
) -> Result<Vec<Edit>, String> {
    if !document.is_valid() {
        return Err("Repair the XAML before editing design data.".into());
    }
    let mut candidate = document.clone();
    let mut result = Vec::new();
    for command in commands {
        let (node, property, value, object) = match command {
            DesignTimeEdit::SetProperty {
                node,
                property,
                value,
            } => (*node, property.clone(), value.clone(), false),
            DesignTimeEdit::SetObject {
                node,
                property,
                source,
            } => (*node, property.clone(), source.clone(), true),
            DesignTimeEdit::SampleItems { node, count } => {
                if count.is_some_and(|count| count > crate::design_time_samples::MAX_SAMPLE_ITEMS) {
                    return Err(format!(
                        "Sample item count must be between 0 and {}.",
                        crate::design_time_samples::MAX_SAMPLE_ITEMS
                    ));
                }
                let (prefix, _) = design_prefix(&candidate, *node)?;
                (
                    *node,
                    "ItemsSource".into(),
                    count.map(|count| format!("{{{prefix}:SampleData ItemCount={count}}}")),
                    false,
                )
            }
        };
        validate_property(&property)?;
        if value.as_ref().is_some_and(|value| value.len() > 262_144) {
            return Err("A design-time value cannot exceed 256 KiB.".into());
        }
        let owner = candidate
            .ast()
            .node(node)
            .filter(|node| node.kind == NodeKind::Element)
            .ok_or("Select an authored control or template part.")?;
        let existing = design_time_properties(&candidate, node)?;
        let matching = existing
            .iter()
            .filter(|entry| entry.name == property)
            .collect::<Vec<_>>();
        if value.as_ref().is_some_and(|value| {
            matching.len() == 1 && matching[0].object == object && matching[0].value == *value
        }) || value.is_none() && matching.is_empty()
        {
            continue;
        }
        let mut edits = Vec::new();
        for attribute in &owner.attributes {
            if attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
                && local(&attribute.name) == property
            {
                edits.push(Edit::RemoveAttribute {
                    node,
                    name: attribute.name.clone(),
                });
            }
        }
        for child in candidate.ast().element_children(node) {
            let child = candidate.ast().node(child).unwrap();
            if child.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
                && child
                    .local_name()
                    .split_once('.')
                    .is_some_and(|(_, name)| name == property)
            {
                edits.push(Edit::RemoveNode { node: child.id });
            }
        }
        if let Some(value) = value {
            let owner_name = owner.local_name().to_owned();
            let (prefix, namespace_edits) = design_prefix(&candidate, node)?;
            edits.extend(namespace_edits);
            if object {
                // InsertChild validates the fragment in the target's real lexical namespace
                // scope. Do not instantiate application objects to validate a source draft.
                let index = candidate.ast().element_children(node).len();
                edits.push(Edit::InsertChild { parent: node, index: index.saturating_sub(edits.iter().filter(|edit| matches!(edit, Edit::RemoveNode { .. })).count()), source: format!("<{prefix}:{owner_name}.{property}>{value}</{prefix}:{owner_name}.{property}>") });
            } else {
                if !property.contains('.')
                    && matches!(
                        property.as_str(),
                        "Width" | "Height" | "DesignWidth" | "DesignHeight"
                    )
                    && !value.starts_with('{')
                    && value != "Auto"
                {
                    let dimension = value.parse::<f64>().map_err(|_| "Design dimensions must be a nonnegative number, Auto or an expression.")?;
                    if !dimension.is_finite() || dimension < 0.0 {
                        return Err("Design dimensions must be finite and nonnegative.".into());
                    }
                }
                edits.push(Edit::SetAttribute {
                    node,
                    name: format!("{prefix}:{property}"),
                    value,
                });
            }
        }
        if !edits.is_empty() {
            candidate
                .apply("Plan design data", edits.clone())
                .map_err(|error| error.to_string())?;
            result.extend(edits);
        }
    }
    Ok(result)
}

fn validate_property(property: &str) -> Result<(), String> {
    if property.is_empty()
        || property.len() > 256
        || !property.split('.').all(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
                && characters.all(|c| c.is_alphanumeric() || c == '_')
        })
        || property.split('.').count() > 2
        || matches!(property, "Name" | "Class" | "Key" | "Ignorable")
    {
        return Err("Use a property name, such as Text, DataContext or Grid.Row; identities and directives are not design overrides.".into());
    }
    Ok(())
}

fn design_prefix(document: &Document, node: NodeId) -> Result<(String, Vec<Edit>), String> {
    let owner = document
        .ast()
        .node(node)
        .ok_or("The design-data owner was removed.")?;
    let namespaces = document.ast().namespaces(node);
    let choose = |namespace: &str, preferred: &str| -> (String, Option<Edit>) {
        if let Some((prefix, _)) = namespaces
            .iter()
            .find(|(prefix, uri)| !prefix.is_empty() && uri.as_str() == namespace)
        {
            return (prefix.clone(), None);
        }
        let mut prefix = preferred.to_owned();
        let mut suffix = 1usize;
        while namespaces.contains_key(&prefix) {
            prefix = format!("{preferred}{suffix}");
            suffix += 1;
        }
        (
            prefix.clone(),
            Some(Edit::SetAttribute {
                node,
                name: format!("xmlns:{prefix}"),
                value: namespace.into(),
            }),
        )
    };
    let (design, declaration) = choose(DESIGN_NAMESPACE, "d");
    let (compatibility, compatibility_declaration) = choose(COMPATIBILITY_NAMESPACE, "mc");
    let mut edits = Vec::new();
    if let Some(edit) = declaration {
        edits.push(edit);
    }
    if let Some(edit) = compatibility_declaration {
        edits.push(edit);
    }
    let current = owner.attributes.iter().find(|attribute| {
        attribute.namespace_uri.as_deref() == Some(COMPATIBILITY_NAMESPACE)
            && local(&attribute.name) == "Ignorable"
    });
    let mut tokens = current.map_or_else(Vec::new, |attribute| {
        attribute
            .value
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    if !tokens.iter().any(|token| token == &design) {
        tokens.push(design.clone());
        edits.push(Edit::SetAttribute {
            node,
            name: current.map_or_else(
                || format!("{compatibility}:Ignorable"),
                |attribute| attribute.name.clone(),
            ),
            value: tokens.join(" "),
        });
    }
    Ok((design, edits))
}

pub(crate) struct DesignTimeProjection {
    pub document: Document,
    /// Generated source identity -> the input document identity. Definition scopes
    /// compose this map with their own input-to-authored map in RuntimePreview.
    pub identities: BTreeMap<NodeId, NodeId>,
    pub samples: Vec<crate::design_time_samples::SampleRequest>,
    pub notices: Vec<String>,
}

#[derive(Clone)]
struct Patch {
    span: SourceSpan,
    replacement: String,
}

/// `base_source` may already contain the width-preserving Window/root projection.
/// Its Unicode line/column map is composed before variable-width design substitutions.
pub(crate) fn project(
    document: &Document,
    base_source: &str,
    enabled: bool,
) -> Result<Option<DesignTimeProjection>, String> {
    let has_design = document.ast().elements().any(|node| {
        node.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
            || attached_design_element(node)
            || node.attributes.iter().any(|attribute| {
                attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
                    || (attribute.namespace_uri.is_none() && attribute.name.starts_with("Design."))
            })
    });
    if !has_design {
        return Ok(None);
    }
    let base = Document::new(base_source).map_err(|error| error.to_string())?;
    if !enabled
        && base
            .ast()
            .node(base.ast().root())
            .is_some_and(|node| node.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE))
    {
        return Ok(Some(DesignTimeProjection {
            document: Document::new(format!("<Border xmlns=\"{FRAMEWORK}\"/>"))
                .map_err(|error| error.to_string())?,
            identities: BTreeMap::new(),
            samples: Vec::new(),
            notices: vec![
                "The document root exists only at design time; the application preview is empty."
                    .into(),
            ],
        }));
    }
    let input_locations = element_locations(document);
    let base_to_input = element_locations(&base)
        .into_iter()
        .filter_map(|(position, base_node)| {
            input_locations
                .get(&position)
                .map(|input_node| (base_node, *input_node))
        })
        .collect::<BTreeMap<_, _>>();
    let mut patches = Vec::new();
    let mut samples = Vec::new();
    let mut notices = Vec::new();
    for node in base.ast().elements() {
        if enabled {
            let mut overrides = BTreeSet::new();
            let declared = node
                .attributes
                .iter()
                .filter_map(|attribute| {
                    if node.id == base.ast().root() && root_size_hint(attribute) {
                        return None;
                    }
                    if attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE) {
                        Some(local(&attribute.name))
                    } else if attribute.namespace_uri.is_none() {
                        attribute.name.strip_prefix("Design.")
                    } else {
                        None
                    }
                })
                .chain(
                    base.ast()
                        .element_children(node.id)
                        .into_iter()
                        .filter_map(|id| {
                            let child = base.ast().node(id)?;
                            if node.id == base.ast().root()
                                && attached_design_element(child)
                                && matches!(child.local_name(), "Design.Width" | "Design.Height")
                            {
                                return None;
                            }
                            (child.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
                                || attached_design_element(child))
                            .then(|| {
                                child
                                    .local_name()
                                    .split_once('.')
                                    .map(|(_, property)| property)
                            })
                            .flatten()
                        }),
                );
            for declared in declared {
                let property = match declared {
                    "DesignWidth" => "Width",
                    "DesignHeight" => "Height",
                    "PreviewWith" | "DesignStyle" => continue,
                    property => property,
                };
                if !overrides.insert(property) {
                    return Err(format!(
                        "{} has more than one design-time override for {property}.",
                        node.name
                    ));
                }
            }
        }
        if node.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE) || attached_design_element(node)
        {
            if !enabled {
                mask(&mut patches, node.span, base_source);
                continue;
            }
            let (namespace, name) =
                if let Some((owner, property)) = node.local_name().split_once('.') {
                    let parent = node
                        .parent
                        .and_then(|id| base.ast().node(id))
                        .ok_or("A design-time property requires an owner.")?;
                    if parent.id == base.ast().root()
                        && attached_design_element(node)
                        && matches!(property, "Width" | "Height")
                    {
                        mask(&mut patches, node.span, base_source);
                        continue;
                    }
                    suppress_runtime_value(&base, parent, property, Some(node.id), &mut patches);
                    if matches!(property, "PreviewWith" | "DesignStyle") {
                        continue;
                    }
                    let attached = attached_design_element(node);
                    (
                        if attached || owner == parent.local_name() {
                            parent.namespace_uri.as_deref().unwrap_or(FRAMEWORK)
                        } else {
                            FRAMEWORK
                        },
                        if attached {
                            format!("{}.{property}", parent.local_name())
                        } else {
                            node.local_name().to_owned()
                        },
                    )
                } else {
                    (FRAMEWORK, node.local_name().to_owned())
                };
            let qualified = runtime_name(&base, node, namespace, &name, &mut patches);
            rename(&mut patches, node, &qualified);
        }
        for attribute in &node.attributes {
            let design = attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE);
            let attached =
                attribute.namespace_uri.is_none() && attribute.name.starts_with("Design.");
            if !design && !attached {
                continue;
            }
            if node.id == base.ast().root() && root_size_hint(attribute) {
                // Root design sizes are host hints. Explicit d:Width/d:Height are
                // overrides; choosing an environment preset must remain responsive.
                mask(&mut patches, attribute.span, base_source);
                continue;
            }
            let original = if attached {
                attribute.name.trim_start_matches("Design.")
            } else {
                local(&attribute.name)
            };
            if !enabled {
                mask(&mut patches, attribute.span, base_source);
                continue;
            }
            let property = match original {
                "DesignWidth" => "Width",
                "DesignHeight" => "Height",
                "PreviewWith" | "DesignStyle" => continue,
                property => property,
            };
            validate_property(property)?;
            if let Some(count) =
                crate::design_time_samples::sample_count(&base, node.id, &attribute.value)?
            {
                if !matches!(property, "ItemsSource" | "DataContext") {
                    return Err(
                        "SampleData is supported on design-time ItemsSource and DataContext."
                            .into(),
                    );
                }
                suppress_runtime_value(&base, node, property, None, &mut patches);
                mask(&mut patches, attribute.span, base_source);
                samples.push(crate::design_time_samples::request(
                    &base,
                    node.id,
                    property,
                    count,
                    &mut notices,
                ));
            } else {
                suppress_runtime_value(&base, node, property, None, &mut patches);
                patches.push(Patch {
                    span: attribute.name_span,
                    replacement: property.into(),
                });
            }
        }
    }
    for sample in &samples {
        for (span, value) in &sample.binding_replacements {
            patches.push(Patch {
                span: *span,
                replacement: value.clone(),
            });
        }
    }
    // Parent suppression owns every nested patch. This is essential when a d:Content
    // replacement hides an application subtree that itself contains design directives.
    patches.sort_by_key(|patch| (patch.span.start, std::cmp::Reverse(patch.span.end)));
    let mut disjoint: Vec<Patch> = Vec::new();
    for patch in patches {
        if !disjoint.last().is_some_and(|outer| {
            patch.span.start < outer.span.end
                || (patch.span == outer.span && patch.span.start != patch.span.end)
        }) {
            disjoint.push(patch);
        }
    }
    let mut source = base_source.to_owned();
    for patch in disjoint.iter().rev() {
        source.replace_range(patch.span.start..patch.span.end, &patch.replacement);
    }
    let projected =
        Document::new(source).map_err(|error| format!("Design-time projection: {error}"))?;
    let by_offset = projected
        .ast()
        .elements()
        .map(|node| (node.span.start, node.id))
        .collect::<BTreeMap<_, _>>();
    let mut identities = BTreeMap::new();
    let mut base_to_projected = BTreeMap::new();
    for node in base.ast().elements() {
        let delta: isize = disjoint
            .iter()
            .filter(|patch| patch.span.end <= node.span.start)
            .map(|patch| {
                patch.replacement.len() as isize - (patch.span.end - patch.span.start) as isize
            })
            .sum();
        let Some(offset) = node.span.start.checked_add_signed(delta) else {
            continue;
        };
        if let Some(projected) = by_offset.get(&offset) {
            if let Some(input) = base_to_input.get(&node.id) {
                identities.insert(*projected, *input);
            }
            base_to_projected.insert(node.id, *projected);
        }
    }
    samples.retain_mut(|sample| {
        if let Some(projected) = base_to_projected.get(&sample.node) {
            sample.node = *projected;
            true
        } else {
            false
        }
    });
    if enabled {
        notices.push(
            "Design-time values are visible; application values remain unchanged in source.".into(),
        );
    }
    Ok(Some(DesignTimeProjection {
        document: projected,
        identities,
        samples,
        notices,
    }))
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}
fn root_size_hint(attribute: &ferroui_designer_core::Attribute) -> bool {
    attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
        && matches!(local(&attribute.name), "DesignWidth" | "DesignHeight")
        || attribute.namespace_uri.is_none()
            && matches!(attribute.name.as_str(), "Design.Width" | "Design.Height")
}
fn attached_design_element(node: &Node) -> bool {
    node.local_name().starts_with("Design.")
        && matches!(
            node.namespace_uri.as_deref(),
            Some(
                FRAMEWORK
                    | "using:FerroUI.Controls"
                    | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
            )
        )
}
fn mask(patches: &mut Vec<Patch>, span: SourceSpan, source: &str) {
    patches.push(Patch {
        span,
        replacement: source[span.start..span.end]
            .chars()
            .map(|character| {
                if matches!(character, '\n' | '\r' | '\t') {
                    character
                } else {
                    ' '
                }
            })
            .collect(),
    });
}
fn rename(patches: &mut Vec<Patch>, node: &Node, name: &str) {
    patches.push(Patch {
        span: node.name_span,
        replacement: name.into(),
    });
    if let Some(closing) = node.closing_span {
        patches.push(Patch {
            span: SourceSpan::new(closing.start + 2, closing.start + 2 + node.name.len()),
            replacement: name.into(),
        });
    }
}
fn runtime_name(
    document: &Document,
    node: &Node,
    namespace: &str,
    name: &str,
    patches: &mut Vec<Patch>,
) -> String {
    let namespaces = document.ast().namespaces(node.id);
    if let Some((prefix, _)) = namespaces.iter().find(|(_, uri)| uri.as_str() == namespace) {
        return if prefix.is_empty() {
            name.into()
        } else {
            format!("{prefix}:{name}")
        };
    }
    let mut prefix = "preview".to_owned();
    let mut suffix = 1;
    while namespaces.contains_key(&prefix) {
        prefix = format!("preview{suffix}");
        suffix += 1;
    }
    patches.push(Patch {
        span: SourceSpan::new(node.name_span.end, node.name_span.end),
        replacement: format!(" xmlns:{prefix}=\"{}\"", escape(namespace)),
    });
    format!("{prefix}:{name}")
}
fn suppress_runtime_value(
    document: &Document,
    owner: &Node,
    property: &str,
    keep: Option<NodeId>,
    patches: &mut Vec<Patch>,
) {
    for attribute in &owner.attributes {
        let qualified = local(&attribute.name);
        let name_matches = qualified == property
            || qualified
                .rsplit_once('.')
                .is_some_and(|(_, name)| name == property);
        if name_matches
            && !attribute.name.starts_with("Design.")
            && (attribute.namespace_uri.is_none()
                || attribute.namespace_uri == owner.namespace_uri
                || attribute.namespace_uri.as_deref() == Some(FRAMEWORK))
        {
            mask(patches, attribute.span, document.valid_source());
        }
    }
    let content = native_type_for_node(owner)
        .and_then(|type_| type_.content_property())
        .or_else(|| {
            crate::editors::markup_type_for_node(owner).and_then(|type_| {
                let mut current = Some(type_);
                while let Some(type_) = current {
                    if type_.content_property.is_some() {
                        return type_.content_property;
                    }
                    current = type_.base_type();
                }
                None
            })
        });
    for id in &owner.children {
        if Some(*id) == keep {
            continue;
        }
        let child = document.ast().node(*id).unwrap();
        let normal_property = child.kind == NodeKind::PropertyElement
            && child.namespace_uri.as_deref() != Some(DESIGN_NAMESPACE)
            && (child.namespace_uri == owner.namespace_uri
                || child.namespace_uri.as_deref() == Some(FRAMEWORK))
            && child
                .local_name()
                .rsplit_once('.')
                .is_some_and(|(_, name)| name == property);
        let implicit_content = content == Some(property)
            && (child.kind == NodeKind::Element
                || matches!(child.kind, NodeKind::Text | NodeKind::CData)
                    && child
                        .text
                        .as_ref()
                        .is_some_and(|text| !text.trim().is_empty()));
        if normal_property || implicit_content {
            mask(patches, child.span, document.valid_source());
        }
    }
}
fn element_locations(document: &Document) -> BTreeMap<(usize, usize), NodeId> {
    let mut nodes = document.ast().elements().collect::<Vec<_>>();
    nodes.sort_by_key(|node| node.span.start);
    let (mut offset, mut line, mut column) = (0usize, 1usize, 1usize);
    let mut result = BTreeMap::new();
    for node in nodes {
        for character in document.valid_source()[offset..node.span.start].chars() {
            if character == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        offset = node.span.start;
        result.insert((line, column), node.id);
    }
    result
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

#[cfg(test)]
#[path = "design_time_tests.rs"]
mod tests;
