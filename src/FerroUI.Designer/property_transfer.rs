//! Authored appearance transfer. A clipboard owns source values, never live objects or
//! evaluated style/binding results. Paste plans are immutable, revision-checked source edits.
use crate::{DesignerSession, PreviewStatus, PropertyEditorContext};
use ferroui_base::{FerroProperty, TypeInfo, Visual, WeakRef};
use ferroui_designer_core::{Ast, Document, Edit, MarkupValue, NodeId, NodeKind};
use ferroui_designer_tools::{
    component_fragment, component_resource_expression, component_resource_resolution,
};
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::Arc;

const FRAMEWORK: &str = "https://github.com/ferroui";

/// Only authored appearance values participate. Identity, content, data, events,
/// behavior, layout placement and control size retain their destination semantics.
const APPEARANCE: &[(&str, &str)] = &[
    ("Background", "Brushes and borders"),
    ("Foreground", "Brushes and borders"),
    ("BorderBrush", "Brushes and borders"),
    ("BorderThickness", "Brushes and borders"),
    ("CornerRadius", "Brushes and borders"),
    ("BoxShadow", "Brushes and borders"),
    ("Opacity", "Appearance"),
    ("FontFamily", "Typography"),
    ("FontSize", "Typography"),
    ("FontWeight", "Typography"),
    ("FontStyle", "Typography"),
    ("FontStretch", "Typography"),
    ("FontFeatures", "Typography"),
    ("LetterSpacing", "Typography"),
    ("LineHeight", "Typography"),
    ("TextAlignment", "Typography"),
    ("TextWrapping", "Typography"),
    ("TextTrimming", "Typography"),
    ("TextDecorations", "Typography"),
    ("Fill", "Shape appearance"),
    ("Stroke", "Shape appearance"),
    ("StrokeThickness", "Shape appearance"),
    ("StrokeDashArray", "Shape appearance"),
    ("StrokeDashOffset", "Shape appearance"),
    ("StrokeLineCap", "Shape appearance"),
    ("StrokeJoin", "Shape appearance"),
    ("StrokeMiterLimit", "Shape appearance"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopiedPropertyKind {
    Literal,
    Object,
    Resource,
}

/// A displayable immutable clipboard entry. Its private value retains the exact source
/// literal or namespace-closed object graph, rather than the current rendered value.
#[derive(Clone)]
pub struct CopiedProperty {
    pub name: String,
    pub category: String,
    pub kind: CopiedPropertyKind,
    pub summary: String,
    property: &'static FerroProperty,
    value: CopiedValue,
}

#[derive(Clone)]
enum CopiedValue {
    Literal(String),
    Object(String),
    Resource {
        key: String,
        dynamic: bool,
        node: NodeId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyTransferIssue {
    pub target: Option<NodeId>,
    pub target_label: Option<String>,
    pub property: String,
    pub reason: String,
}

/// Workspace-local clipboard. Literal and self-contained object values can be pasted
/// into another document. Resources additionally require the exact captured source AST
/// and identical unambiguous lookup at every destination; they never silently rebind.
#[derive(Clone)]
pub struct PropertyClipboard {
    uri: String,
    ast: Arc<Ast>,
    label: String,
    entries: Vec<CopiedProperty>,
    skipped: Vec<PropertyTransferIssue>,
}

impl PropertyClipboard {
    pub fn source_uri(&self) -> &str {
        &self.uri
    }
    pub fn source_label(&self) -> &str {
        &self.label
    }
    pub fn entries(&self) -> &[CopiedProperty] {
        &self.entries
    }
    pub fn skipped(&self) -> &[PropertyTransferIssue] {
        &self.skipped
    }
    pub fn property_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyPasteChange {
    pub target: NodeId,
    pub target_label: String,
    pub property: String,
    /// Supported but already identical values remain visible without adding history.
    pub changed: bool,
}

/// An accepted set of compatible destination writes. Unsupported destinations are
/// reported explicitly; malformed values or protected layers reject planning as a whole.
#[derive(Clone)]
pub struct PropertyPastePreview {
    uri: String,
    ast: Arc<Ast>,
    revision: u64,
    generation: u64,
    selection: Vec<NodeId>,
    native_targets: Vec<(NodeId, WeakRef<Visual>)>,
    scope_epoch: u64,
    layer_epoch: u64,
    edits: Vec<Edit>,
    changes: Vec<PropertyPasteChange>,
    skipped: Vec<PropertyTransferIssue>,
}

impl PropertyPastePreview {
    /// Exact sequential source edits represented by this reviewed plan.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }
    pub fn changes(&self) -> &[PropertyPasteChange] {
        &self.changes
    }
    pub fn skipped(&self) -> &[PropertyTransferIssue] {
        &self.skipped
    }
    pub fn change_count(&self) -> usize {
        self.changes.iter().filter(|change| change.changed).count()
    }
    /// Ephemeral automation comparison token; it grants no authority and discloses
    /// no native addresses. Reconstructing a same-source preview changes the token.
    #[cfg(feature = "mcp")]
    pub(crate) fn native_identity_token(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut state = std::collections::hash_map::DefaultHasher::new();
        self.generation.hash(&mut state);
        self.scope_epoch.hash(&mut state);
        self.layer_epoch.hash(&mut state);
        self.native_targets.len().hash(&mut state);
        let inspector = ferroui_inspect::Inspector::shared();
        let mut inspector = inspector.borrow_mut();
        for (node, visual) in &self.native_targets {
            node.hash(&mut state);
            // Inspector IDs are monotonic, unlike allocation addresses that could
            // be reused between two stateless automation requests.
            visual
                .upgrade()
                .map(|visual| inspector.node_id(&visual))
                .hash(&mut state);
        }
        state.finish()
    }
    pub fn is_current(&self, session: &DesignerSession) -> bool {
        self.uri == session.document_uri()
            && self.revision == session.document().revision()
            && self.generation == session.generation()
            && Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
            && self.selection == session.selection()
            && self.scope_epoch == session.definition_scope_epoch()
            && self.layer_epoch == session.layer_view_epoch()
            && current_preview(session).is_ok()
            && self.native_targets.iter().all(|(node, original)| {
                session
                    .preview
                    .visual(*node)
                    .is_some_and(|current| original.points_to(&current))
            })
    }
}

impl DesignerSession {
    /// Capture the selected element's authored appearance. Reading is available to
    /// collaboration viewers; bindings and context-dependent object graphs are reported.
    pub fn copy_properties(&self) -> Result<PropertyClipboard, String> {
        let [node] = self.selection() else {
            return Err("Select one authored control to copy its appearance properties".into());
        };
        self.copy_properties_from(*node)
    }

    /// Capture an explicit authored source without changing selection. Native preview
    /// ownership limits this to the current document/definition scope. Hidden or locked
    /// source layers may be read; destination protections are enforced independently.
    pub fn copy_properties_from(&self, node: NodeId) -> Result<PropertyClipboard, String> {
        current_preview(self)?;
        let document = self.document();
        if !document
            .ast()
            .node(node)
            .is_some_and(|node| node.kind == NodeKind::Element)
        {
            return Err("Select an authored control to copy its appearance properties".into());
        }
        if let Some(scope) = self.definition_scope() {
            let mut current = Some(node);
            while current.is_some() && current != Some(scope.node) {
                current = current
                    .and_then(|id| document.ast().node(id))
                    .and_then(|node| node.parent);
            }
            if current.is_none() {
                return Err("The appearance source is outside the active definition scope".into());
            }
        }
        let visual = self
            .preview
            .visual(node)
            .ok_or("The selection has no authored preview")?;
        let mut clipboard = PropertyClipboard {
            uri: self.document_uri().into(),
            ast: document.ast_snapshot(),
            label: label(document, node),
            entries: Vec::new(),
            skipped: Vec::new(),
        };
        for &(name, category) in APPEARANCE {
            let Some(property) = visual.get_type().find_property(name) else {
                continue;
            };
            if !native_property(property) {
                continue;
            }
            let forms = authored_forms(document, node, visual.get_type(), property)?;
            if forms.is_empty() {
                continue;
            }
            let value = capture_value(self, node, property, &forms);
            match value {
                Ok(value) => {
                    let (kind, summary) = match &value {
                        CopiedValue::Literal(value) => (CopiedPropertyKind::Literal, value.clone()),
                        CopiedValue::Object(_) => {
                            (CopiedPropertyKind::Object, "Authored object value".into())
                        }
                        CopiedValue::Resource { key, dynamic, .. } => (
                            CopiedPropertyKind::Resource,
                            format!(
                                "{} resource: {key}",
                                if *dynamic { "Dynamic" } else { "Static" }
                            ),
                        ),
                    };
                    clipboard.entries.push(CopiedProperty {
                        name: name.into(),
                        category: category.into(),
                        kind,
                        summary,
                        property,
                        value,
                    });
                }
                Err(reason) => clipboard.skipped.push(PropertyTransferIssue {
                    target: Some(node),
                    target_label: Some(label(document, node)),
                    property: name.into(),
                    reason,
                }),
            }
        }
        Ok(clipboard)
    }

    /// Plan explicitly chosen clipboard properties for the complete current selection.
    /// Each destination uses its real native metadata and its authored lexical aliases.
    /// This method performs no source writes and remains available to viewers.
    pub fn prepare_property_paste(
        &self,
        clipboard: &PropertyClipboard,
        property_names: &[String],
    ) -> Result<PropertyPastePreview, String> {
        current_preview(self)?;
        if self.selection().is_empty() {
            return Err("Select at least one authored destination".into());
        }
        let names = property_names
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if names.len() != property_names.len() {
            return Err("Each appearance property may be selected only once".into());
        }
        for name in &names {
            if !clipboard.entries.iter().any(|entry| entry.name == *name) {
                return Err(format!(
                    "{name} is not present in the captured property clipboard"
                ));
            }
        }
        let mut preview = PropertyPastePreview {
            uri: self.document_uri().into(),
            ast: self.document().ast_snapshot(),
            revision: self.document().revision(),
            generation: self.generation(),
            selection: self.selection().to_vec(),
            native_targets: Vec::new(),
            scope_epoch: self.definition_scope_epoch(),
            layer_epoch: self.layer_view_epoch(),
            edits: Vec::new(),
            changes: Vec::new(),
            skipped: Vec::new(),
        };
        // Advance an isolated candidate after each value. Object replacements can change
        // later sibling IDs, and every operation must be planned against the same sequence
        // that the eventual single document transaction will execute.
        let mut candidate = self.document().clone();
        for &target in self.selection() {
            self.ensure_layer_manipulable(target)?;
            let visual = self
                .preview
                .visual(target)
                .ok_or("A destination has no authored preview")?;
            preview.native_targets.push((target, visual.downgrade()));
            for entry in clipboard
                .entries
                .iter()
                .filter(|entry| names.contains(entry.name.as_str()))
            {
                let property = visual
                    .get_type()
                    .find_property(&entry.name)
                    .filter(|property| {
                        native_property(property)
                            && property.property_type() == entry.property.property_type()
                    });
                let Some(property) = property else {
                    preview.skipped.push(PropertyTransferIssue {
                        target: Some(target),
                        target_label: Some(label(self.document(), target)),
                        property: entry.name.clone(),
                        reason: format!(
                            "{} does not expose a compatible writable native property",
                            label(self.document(), target)
                        ),
                    });
                    continue;
                };
                let value = match destination_value(self, clipboard, entry, target) {
                    Ok(value) => value,
                    Err(reason) => {
                        preview.skipped.push(PropertyTransferIssue {
                            target: Some(target),
                            target_label: Some(label(self.document(), target)),
                            property: entry.name.clone(),
                            reason,
                        });
                        continue;
                    }
                };
                if let (CopiedPropertyKind::Literal, CopiedValue::Literal(value)) =
                    (entry.kind, &value)
                {
                    validate_literal(target, property, value)?;
                }
                let forms = authored_forms(&candidate, target, visual.get_type(), property)?;
                let edits = replacement_edits(&candidate, target, &entry.name, &forms, &value)?;
                let before = candidate.source().to_owned();
                if !edits.is_empty() {
                    candidate
                        .apply("Validate property paste", edits.clone())
                        .map_err(|error| error.to_string())?;
                }
                let changed = candidate.source() != before;
                if changed {
                    preview.edits.extend(edits);
                }
                preview.changes.push(PropertyPasteChange {
                    target,
                    target_label: label(self.document(), target),
                    property: entry.name.clone(),
                    changed,
                });
            }
        }
        Ok(preview)
    }

    /// Commit the reviewed plan as one ordinary source transaction. Appearance pasting
    /// does not create animation keys, even while timeline recording is enabled.
    pub fn commit_property_paste(&mut self, preview: &PropertyPastePreview) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !preview.is_current(self) {
            return Err(
                "The document, selection or preview scope changed; reopen Paste properties".into(),
            );
        }
        for &target in &preview.selection {
            self.ensure_layer_manipulable(target)?;
        }
        if !preview.edits.is_empty() {
            self.apply("Paste appearance properties", preview.edits.clone())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn current_preview(session: &DesignerSession) -> Result<(), String> {
    if session.document().is_valid()
        && matches!(session.preview_status(), PreviewStatus::Ready { revision } if *revision == session.document().revision())
    {
        Ok(())
    } else {
        Err("Appearance transfer requires the current XAML to render successfully".into())
    }
}

fn native_property(property: &FerroProperty) -> bool {
    let module = property.owner_type().module_path();
    // Attached definitions such as TextElement.FontSize become ordinary properties
    // through add_owner on TextBlock/TemplatedControl. The caller resolves only the
    // destination type's registered members, rather than all globally attached ones.
    !property.is_read_only()
        && (module.starts_with("ferroui_base::") || module.starts_with("ferroui_controls::"))
}

fn label(document: &Document, node: NodeId) -> String {
    document
        .ast()
        .node(node)
        .map(|node| {
            node.xaml_name()
                .map(|name| format!("{} · {name}", node.local_name()))
                .unwrap_or_else(|| node.local_name().into())
        })
        .unwrap_or_else(|| format!("Element {node}"))
}

#[derive(Default)]
struct AuthoredForms {
    attribute: Option<String>,
    object: Option<NodeId>,
}
impl AuthoredForms {
    fn is_empty(&self) -> bool {
        self.attribute.is_none() && self.object.is_none()
    }
}

/// Resolve owner-qualified syntax against runtime type identity. A foreign property
/// with the same final member name is never removed or mistaken for the native value.
fn authored_forms(
    document: &Document,
    node: NodeId,
    type_: &'static TypeInfo,
    property: &FerroProperty,
) -> Result<AuthoredForms, String> {
    let element = document
        .ast()
        .node(node)
        .ok_or("The property destination no longer exists")?;
    // A Window is previewed by a Border projection. Its real authored owner still
    // participates in qualified property identity; the projection is not source authority.
    let declared_type = crate::native_types::registered_xml_type(
        element.namespace_uri.as_deref().unwrap_or(FRAMEWORK),
        element.local_name(),
    );
    let mut forms = AuthoredForms::default();
    for attribute in &element.attributes {
        if !attribute.is_namespace()
            && authored_property(document, node, type_, declared_type, &attribute.name)
                .is_some_and(|candidate| candidate.id() == property.id())
            && forms.attribute.replace(attribute.name.clone()).is_some()
        {
            return Err(format!(
                "{} has duplicate authored property aliases",
                property.name()
            ));
        }
    }
    for &child in &element.children {
        let Some(child) = document.ast().node(child) else {
            continue;
        };
        if child.kind == NodeKind::PropertyElement
            && authored_property(document, child.id, type_, declared_type, &child.name)
                .is_some_and(|candidate| candidate.id() == property.id())
            && forms.object.replace(child.id).is_some()
        {
            return Err(format!(
                "{} has multiple authored object values",
                property.name()
            ));
        }
    }
    if forms.attribute.is_some() && forms.object.is_some() {
        return Err(format!(
            "{} has both an attribute and an object value",
            property.name()
        ));
    }
    Ok(forms)
}

fn authored_property(
    document: &Document,
    at: NodeId,
    type_: &'static TypeInfo,
    declared_type: Option<&'static TypeInfo>,
    name: &str,
) -> Option<&'static FerroProperty> {
    let (prefix, local) = name.split_once(':').unwrap_or(("", name));
    if let Some((owner, member)) = local.rsplit_once('.') {
        let namespaces = document.ast().namespaces(at);
        let uri = namespaces
            .get(prefix)
            .map(String::as_str)
            .unwrap_or(FRAMEWORK);
        if !prefix.is_empty() && !namespaces.contains_key(prefix) {
            return None;
        }
        let owner = crate::native_types::registered_xml_type(uri, owner)?;
        let property = owner.find_property(member)?;
        let compatible_owner = owner.is_assignable_from(type_)
            || declared_type.is_some_and(|declared| owner.is_assignable_from(declared))
            || (property.is_attached()
                && type_
                    .find_property(member)
                    .is_some_and(|registered| registered.id() == property.id()));
        if !compatible_owner {
            return None;
        }
        Some(property)
    } else if prefix.is_empty() {
        type_.find_property(local)
    } else {
        None
    }
}

fn capture_value(
    session: &DesignerSession,
    node: NodeId,
    property: &'static FerroProperty,
    forms: &AuthoredForms,
) -> Result<CopiedValue, String> {
    let document = session.document();
    if let Some(name) = &forms.attribute {
        let value = document
            .ast()
            .node(node)
            .and_then(|node| node.attribute(name))
            .ok_or("The authored value is unavailable")?;
        return match MarkupValue::parse(value).map_err(|error| error.to_string())? {
            MarkupValue::Literal(_) => {
                if property.name() == "FontFamily" && value.contains('#') {
                    return Err("Font asset references retain their document base URI; use a shared font resource".into());
                }
                validate_literal(node, property, value)?;
                Ok(CopiedValue::Literal(value.into()))
            }
            MarkupValue::Extension(extension) => {
                let kind = crate::property_inspector_view::framework_extension(
                    document,
                    node,
                    &extension.name,
                );
                if !matches!(kind, Some("StaticResource" | "DynamicResource")) {
                    return Err("Bindings and application expressions depend on their original context and are not copied".into());
                }
                let [argument] = extension.arguments.as_slice() else {
                    return Err(
                        "Only an explicit string-key resource reference can be copied".into(),
                    );
                };
                if argument
                    .name
                    .as_deref()
                    .is_some_and(|name| name != "ResourceKey")
                {
                    return Err(
                        "Only an explicit string-key resource reference can be copied".into(),
                    );
                }
                let MarkupValue::Literal(key) = &argument.value else {
                    return Err("Computed resource keys depend on their original context".into());
                };
                let resource = component_resource_resolution(document, node, key, None)
                    .map_err(|error| error.to_string())?.resolved()
                    .ok_or("The resource is external, missing or theme-dependent; use the resource picker at the destination")?;
                Ok(CopiedValue::Resource {
                    key: key.clone(),
                    dynamic: kind == Some("DynamicResource"),
                    node: resource,
                })
            }
        };
    }
    let property_node = document
        .ast()
        .node(forms.object.ok_or("No authored value")?)
        .ok_or("The authored object is unavailable")?;
    // Export the actual property container directly: its owner alias may be declared
    // only on that wrapper, outside the selected element's lexical namespace map.
    let source = object_content(document, property_node.id)?;
    self_contained_object(document, property_node.id)?;
    Ok(CopiedValue::Object(source))
}

fn object_content(document: &Document, property: NodeId) -> Result<String, String> {
    let property = document
        .ast()
        .node(property)
        .ok_or("The object property disappeared")?;
    let mut source = String::new();
    for &child in &property.children {
        let child = document
            .ast()
            .node(child)
            .ok_or("The object value disappeared")?;
        if child.is_element() {
            source.push_str(
                &component_fragment(document, child.id).map_err(|error| error.to_string())?,
            );
        } else {
            source.push_str(&document.valid_source()[child.span.range()]);
        }
    }
    if !property.children.iter().any(|id| {
        document
            .ast()
            .node(*id)
            .is_some_and(|node| node.is_element())
    }) {
        return Err("The property has no transferable object graph".into());
    }
    Ok(source)
}

fn self_contained_object(document: &Document, property: NodeId) -> Result<(), String> {
    // Native value graphs with no identity, executable expressions, URI bases or
    // custom serializers can be copied without changing name/resource lookup scopes.
    const VALUE_TYPES: &[&str] = &[
        "SolidColorBrush",
        "LinearGradientBrush",
        "RadialGradientBrush",
        "ConicGradientBrush",
        "GradientStop",
        "GradientStops",
        "TransformGroup",
        "TranslateTransform",
        "RotateTransform",
        "ScaleTransform",
        "SkewTransform",
        "MatrixTransform",
        "TextDecoration",
        "TextDecorationCollection",
        "Pen",
    ];
    for node in document.ast().subtree(property) {
        if node.kind == NodeKind::PropertyElement {
            if node
                .attributes
                .iter()
                .any(|attribute| !attribute.is_namespace())
            {
                return Err(
                    "Object property wrappers with identity or context directives are not copied"
                        .into(),
                );
            }
            if node.id != property {
                let parent = node
                    .parent
                    .and_then(|id| document.ast().node(id))
                    .ok_or("The object property owner is unavailable")?;
                let parent_type = crate::native_types::registered_xml_type(
                    parent.namespace_uri.as_deref().unwrap_or(FRAMEWORK),
                    parent.local_name(),
                )
                .ok_or("The object property has a custom owner")?;
                let (owner, member) = node
                    .local_name()
                    .rsplit_once('.')
                    .ok_or("Invalid object property")?;
                let owner_type = crate::native_types::registered_xml_type(
                    node.namespace_uri.as_deref().unwrap_or(FRAMEWORK),
                    owner,
                )
                .ok_or("Application object properties retain their original context")?;
                if !owner_type.module_path().starts_with("ferroui_base::")
                    || !owner_type.is_assignable_from(parent_type)
                    || !has_value_property(owner_type, member)
                {
                    return Err(
                        "Foreign, attached or unknown object properties are not copied".into(),
                    );
                }
            }
            continue;
        }
        if !node.is_element() {
            continue;
        }
        let type_ = crate::native_types::registered_xml_type(
            node.namespace_uri.as_deref().unwrap_or(FRAMEWORK),
            node.local_name(),
        )
        .ok_or("Custom object values require their original application context")?;
        if !type_.module_path().starts_with("ferroui_base::")
            || !VALUE_TYPES.contains(&type_.name())
        {
            return Err("This object graph contains context-dependent values; use its object editor or a shared resource".into());
        }
        for attribute in &node.attributes {
            if attribute.is_namespace() {
                continue;
            }
            if attribute.name.contains(':')
                || attribute.name.contains('.')
                || matches!(attribute.name.as_str(), "Name" | "Source" | "BaseUri")
                || !has_value_property(type_, &attribute.name)
            {
                return Err("Object identities, attached/application metadata and URI references are not copied".into());
            }
            if matches!(
                attribute.markup().map_err(|error| error.to_string())?,
                MarkupValue::Extension(_)
            ) {
                return Err("Bindings and resource references inside object values retain their original context and are not copied".into());
            }
        }
    }
    Ok(())
}

fn has_value_property(mut type_: &'static TypeInfo, name: &str) -> bool {
    if type_
        .find_property(name)
        .is_some_and(|property| !property.is_attached())
    {
        return true;
    }
    loop {
        if type_
            .markup()
            .is_some_and(|markup| markup.find_property(name).is_some())
        {
            return true;
        }
        let Some(base) = type_.base_type() else {
            return false;
        };
        type_ = base;
    }
}

fn validate_literal(
    node: NodeId,
    property: &'static FerroProperty,
    value: &str,
) -> Result<(), String> {
    let context = PropertyEditorContext {
        property: property.name().into(),
        targets: vec![node],
        value: None,
        mixed: false,
        runtime_type: Some(property.property_type_name().into()),
        apply: Rc::new(|_| {}),
    };
    crate::resolve_property_editor_metadata(&context, Some(property))
        .prepare_literal(&context, value)
        .map(|_| ())
}

fn destination_value(
    session: &DesignerSession,
    clipboard: &PropertyClipboard,
    entry: &CopiedProperty,
    target: NodeId,
) -> Result<CopiedValue, String> {
    if let CopiedValue::Resource { key, dynamic, node } = &entry.value {
        if clipboard.uri != session.document_uri()
            || !Arc::ptr_eq(&clipboard.ast, &session.document().ast_snapshot())
        {
            return Err("Resource references require the unchanged source document; copy again or use the destination resource picker".into());
        }
        if component_resource_resolution(session.document(), target, key, None)
            .map_err(|error| error.to_string())?
            .resolved()
            != Some(*node)
        {
            return Err(
                "The resource resolves differently or is unavailable at this destination".into(),
            );
        }
        return component_resource_expression(session.document(), target, key, *dynamic)
            .map(CopiedValue::Literal)
            .map_err(|error| error.to_string());
    }
    Ok(entry.value.clone())
}

fn replacement_edits(
    document: &Document,
    target: NodeId,
    name: &str,
    forms: &AuthoredForms,
    value: &CopiedValue,
) -> Result<Vec<Edit>, String> {
    let element = document
        .ast()
        .node(target)
        .ok_or("The destination disappeared")?;
    let mut edits = Vec::new();
    match value {
        CopiedValue::Literal(value) => {
            if let Some(object) = forms.object {
                edits.push(Edit::RemoveNode { node: object });
            }
            let name = forms.attribute.as_deref().unwrap_or(name);
            if element.attribute(name) != Some(value.as_str()) {
                edits.push(Edit::SetAttribute {
                    node: target,
                    name: name.into(),
                    value: value.clone(),
                });
            }
        }
        CopiedValue::Object(source) => {
            if let Some(object) = forms.object {
                let object = document
                    .ast()
                    .node(object)
                    .ok_or("The old object value disappeared")?;
                let content_end = object
                    .closing_span
                    .map_or(object.opening_span.end, |span| span.start);
                if &document.valid_source()[object.opening_span.end..content_end] == source {
                    return Ok(edits);
                }
                // The wrapper can declare its own owner alias. Retain all original
                // namespace declarations and spelling while replacing only its content.
                let opening = &document.valid_source()[object.opening_span.range()];
                let replacement = if object.self_closing {
                    format!(
                        "{}>{source}</{}>",
                        opening
                            .strip_suffix("/>")
                            .ok_or("Invalid property wrapper")?,
                        object.name
                    )
                } else {
                    let closing = object
                        .closing_span
                        .ok_or("The object property is unclosed")?;
                    format!(
                        "{opening}{source}{}",
                        &document.valid_source()[closing.range()]
                    )
                };
                edits.push(Edit::Text {
                    span: object.span,
                    replacement,
                });
            } else {
                if let Some(attribute) = &forms.attribute {
                    edits.push(Edit::RemoveAttribute {
                        node: target,
                        name: attribute.clone(),
                    });
                }
                edits.push(Edit::InsertChild {
                    parent: target,
                    index: 0,
                    source: format!(
                        "<{}.{name}>{source}</{}.{name}>",
                        element.name, element.name
                    ),
                });
            }
        }
        CopiedValue::Resource { .. } => {
            return Err("The resource reference was not resolved for its destination".into())
        }
    }
    Ok(edits)
}

#[cfg(test)]
#[path = "property_transfer_tests.rs"]
mod tests;
