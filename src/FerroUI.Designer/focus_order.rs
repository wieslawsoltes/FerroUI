//! Keyboard-navigation authoring over an accepted native preview.
//!
//! Tab indices are local to the runtime's navigation groups. Generated template
//! controls remain visible in the catalog, but cannot become authored targets.
use crate::{DesignerSession, PreviewStatus};
use ferroui_base::controls::NameScopeExtensions;
use ferroui_base::input::{
    InputElement, InputElementImplExt, KeyboardNavigation, KeyboardNavigationMode,
};
use ferroui_base::{Ref, Visual};
use ferroui_controls::Label;
use ferroui_designer_core::{Ast, Attribute, Document, Edit, Node, NodeId, NodeKind};
use ferroui_inspect::Inspector;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const FRAMEWORK: &str = "https://github.com/ferroui";

/// One runtime tab stop or group entry. An absent source identity is intentional:
/// generated controls can influence navigation without being editable XAML nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusOrderItem {
    pub runtime_id: u64,
    pub node: Option<NodeId>,
    pub label: String,
    pub tab_index: i32,
    /// Tie-break order supplied by the native container, before TabIndex sorting.
    pub tree_order: usize,
    pub focusable: bool,
    pub is_tab_stop: bool,
    pub visible: bool,
    pub enabled: bool,
    pub group: Option<u64>,
    /// A non-focusable group participates only if entering it can reach a stop.
    pub group_has_tab_stop: bool,
}
impl FocusOrderItem {
    pub fn is_active(&self) -> bool {
        self.visible
            && self.enabled
            && (self.group_has_tab_stop || self.focusable && self.is_tab_stop)
    }
}

/// Each non-Continue container establishes a separate native TabIndex group.
/// Continue descendants share the nearest group's index space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusOrderScope {
    pub id: u64,
    pub node: Option<NodeId>,
    pub parent: Option<u64>,
    pub label: String,
    pub mode: KeyboardNavigationMode,
    pub blocked_by_ancestor: bool,
    pub items: Vec<FocusOrderItem>,
}
impl FocusOrderScope {
    pub fn can_reorder(&self) -> bool {
        self.items.len() > 1 && self.items.iter().all(|item| item.node.is_some())
    }
    pub fn mode_description(&self) -> &'static str {
        match self.mode {
            KeyboardNavigationMode::Continue => "Continue: indices share the enclosing group",
            KeyboardNavigationMode::Local => "Local: this group has its own index space",
            KeyboardNavigationMode::Cycle => "Cycle: Tab wraps within this group",
            KeyboardNavigationMode::Contained => "Contained: Tab stops at this group's edge",
            KeyboardNavigationMode::Once => "Once: entry selects one child; the next Tab leaves",
            KeyboardNavigationMode::None => "None: children are excluded from Tab navigation",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusLabelTarget {
    pub node: NodeId,
    pub label: String,
}

/// A real Label.Target association and the authored inputs reachable in its namescope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusLabelAssociation {
    pub node: NodeId,
    pub label: String,
    pub current_target: Option<NodeId>,
    pub candidates: Vec<FocusLabelTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusOrderCatalog {
    pub scopes: Vec<FocusOrderScope>,
    pub labels: Vec<FocusLabelAssociation>,
}

/// A retained dialog cannot edit another document, another definition scope, or a
/// preview whose navigation semantics have changed without a source edit.
#[derive(Clone, Debug)]
pub struct FocusOrderSnapshot {
    pub catalog: FocusOrderCatalog,
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    scope_epoch: u64,
}
impl FocusOrderSnapshot {
    pub fn is_current(&self, session: &DesignerSession) -> bool {
        session.document().is_valid()
            && session.document_uri() == self.uri
            && session.document().revision() == self.revision
            && session.definition_scope_epoch() == self.scope_epoch
            && Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
            && session
                .focus_order_catalog()
                .is_ok_and(|current| current == self.catalog)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusOrderEdit {
    /// The complete permutation of authored entries in one native group.
    SetOrder {
        scope: u64,
        order: Vec<NodeId>,
    },
    ResetOrder {
        scope: u64,
    },
    SetTabStop {
        node: NodeId,
        value: bool,
    },
    SetFocusable {
        node: NodeId,
        value: bool,
    },
    SetNavigation {
        node: NodeId,
        mode: KeyboardNavigationMode,
    },
    /// None removes the authored Target, revealing its inherited/default value.
    SetLabelTarget {
        label: NodeId,
        target: Option<NodeId>,
    },
}

impl DesignerSession {
    pub fn focus_order_catalog(&self) -> Result<FocusOrderCatalog, String> {
        if !matches!(self.preview_status(), PreviewStatus::Ready { revision } if *revision == self.document().revision())
        {
            return Err("Focus order requires an accepted preview of the current source".into());
        }
        let root = self
            .preview
            .root()
            .ok_or("The preview has no visual root")?
            .upcast::<InputElement>();
        let inspector = Inspector::shared();
        let root_id = inspector.borrow_mut().node_id(&root);
        let root_mode = KeyboardNavigation::get_tab_navigation(&root);
        let root_source = self.preview.source_id(&root);
        let mut scopes = vec![FocusOrderScope {
            id: root_id,
            node: root_source,
            parent: None,
            label: element_label(&root, root_source, self.document()),
            mode: root_mode,
            blocked_by_ancestor: false,
            items: Vec::new(),
        }];
        let mut scope_positions = BTreeMap::from([(root_id, 0)]);
        let mut inputs = Vec::new();
        let mut pending = vec![(root, root_id, true, false, true)];
        let mut seen = BTreeSet::new();
        while let Some((input, parent_scope, ancestors_visible, blocked, is_root)) = pending.pop() {
            let runtime_id = inspector.borrow_mut().node_id(&input);
            if !seen.insert(runtime_id) {
                continue;
            }
            let source = self.preview.source_id(&input);
            let mode = KeyboardNavigation::get_tab_navigation(&input);
            let is_group = !is_root && mode != KeyboardNavigationMode::Continue;
            let visible = ancestors_visible && input.is_visible();
            let source_has_navigation = source
                .and_then(|id| self.document().ast().node(id))
                .is_some_and(|node| {
                    ["TabIndex", "IsTabStop", "Focusable"]
                        .iter()
                        .any(|property| {
                            navigation_attributes(self.document(), node, property)
                                .is_ok_and(|attrs| !attrs.is_empty())
                        })
                });
            if !is_root && (input.focusable() || is_group || source_has_navigation) {
                let item = FocusOrderItem {
                    runtime_id,
                    node: source,
                    label: element_label(&input, source, self.document()),
                    tab_index: input.tab_index(),
                    tree_order: scopes[*scope_positions
                        .get(&parent_scope)
                        .expect("parent navigation scope")]
                    .items
                    .len(),
                    focusable: input.focusable(),
                    is_tab_stop: input.is_tab_stop(),
                    visible,
                    enabled: input.is_effectively_enabled(),
                    group: is_group.then_some(runtime_id),
                    group_has_tab_stop: false,
                };
                scopes[*scope_positions
                    .get(&parent_scope)
                    .expect("parent navigation scope")]
                .items
                .push(item);
            }
            let scope = if is_group {
                scope_positions.insert(runtime_id, scopes.len());
                scopes.push(FocusOrderScope {
                    id: runtime_id,
                    node: source,
                    parent: Some(parent_scope),
                    label: element_label(&input, source, self.document()),
                    mode,
                    blocked_by_ancestor: blocked,
                    items: Vec::new(),
                });
                runtime_id
            } else {
                parent_scope
            };
            let child_blocked = blocked || mode == KeyboardNavigationMode::None;
            let children = input.children_in_tab_focus_order().unwrap_or_else(|| {
                input
                    .visual_children_snapshot()
                    .map_or_else(Vec::new, |children| {
                        children
                            .iter()
                            .filter_map(|child| child.cast::<InputElement>())
                            .collect()
                    })
            });
            if let Some(source) = source {
                inputs.push((source, input.clone()));
            }
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, scope, visible, child_blocked, false)),
            );
        }
        // Stable sorting preserves the runtime child order for equal TabIndex values,
        // including custom InputElement child-order overrides.
        for scope in &mut scopes {
            scope.items.sort_by_key(|item| item.tab_index);
        }
        let mut group_entry = BTreeMap::new();
        for scope in scopes.iter_mut().rev() {
            for item in &mut scope.items {
                item.group_has_tab_stop = item
                    .group
                    .is_some_and(|id| group_entry.get(&id) == Some(&true));
            }
            group_entry.insert(
                scope.id,
                scope.mode != KeyboardNavigationMode::None
                    && scope.items.iter().any(FocusOrderItem::is_active),
            );
        }
        let mut labels = Vec::new();
        for (node, input) in &inputs {
            let Some(label) = input.cast::<Label>() else {
                continue;
            };
            let label_scope = NameScopeExtensions::find_name_scope(&label);
            let candidates = inputs
                .iter()
                .filter_map(|(target, candidate)| {
                    if target == node || !candidate.focusable() {
                        return None;
                    }
                    let candidate_scope = NameScopeExtensions::find_name_scope(candidate);
                    let same_scope = match (&label_scope, &candidate_scope) {
                        (Some(a), Some(b)) => std::rc::Rc::ptr_eq(a, b),
                        _ => false,
                    };
                    same_scope.then(|| FocusLabelTarget {
                        node: *target,
                        label: element_label(candidate, Some(*target), self.document()),
                    })
                })
                .collect();
            labels.push(FocusLabelAssociation {
                node: *node,
                label: element_label(input, Some(*node), self.document()),
                current_target: label
                    .target()
                    .and_then(|target| self.preview.source_id(&target)),
                candidates,
            });
        }
        Ok(FocusOrderCatalog { scopes, labels })
    }

    pub fn focus_order_snapshot(&self) -> Result<FocusOrderSnapshot, String> {
        Ok(FocusOrderSnapshot {
            catalog: self.focus_order_catalog()?,
            uri: self.document_uri().to_owned(),
            revision: self.document().revision(),
            ast: self.document().ast_snapshot(),
            scope_epoch: self.definition_scope_epoch(),
        })
    }

    /// Validate the entire staged dialog before making its single source/undo commit.
    /// Navigation changes are not animation keyframes and do not move runtime focus.
    pub fn commit_focus_order(
        &mut self,
        snapshot: &FocusOrderSnapshot,
        commands: &[FocusOrderEdit],
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !snapshot.is_current(self) {
            return Err(
                "The document, definition scope or native focus model changed; reopen Focus order"
                    .into(),
            );
        }
        let mut candidate = self.document().clone();
        let mut all_edits = Vec::new();
        for command in commands {
            let edits = plan_focus_edit(&candidate, &snapshot.catalog, command)?;
            if !edits.is_empty() {
                candidate
                    .apply("Validate focus order", edits.clone())
                    .map_err(|error| error.to_string())?;
                all_edits.extend(edits);
            }
        }
        if !all_edits.is_empty() {
            self.apply_at(snapshot.revision, "Edit keyboard navigation", all_edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn element_label(element: &InputElement, source: Option<NodeId>, document: &Document) -> String {
    let type_name = element.get_type().name();
    let authored = source.and_then(|node| document.ast().node(node));
    let name = authored
        .and_then(authored_name)
        .map(str::to_owned)
        .or_else(|| element.name());
    let label = name.filter(|name| !name.is_empty()).map_or_else(
        || type_name.to_owned(),
        |name| format!("{name} · {type_name}"),
    );
    if source.is_none() {
        format!("{label} (generated)")
    } else {
        label
    }
}

fn plan_focus_edit(
    document: &Document,
    catalog: &FocusOrderCatalog,
    command: &FocusOrderEdit,
) -> Result<Vec<Edit>, String> {
    match command {
        FocusOrderEdit::SetOrder { scope, order } => {
            let scope = catalog
                .scopes
                .iter()
                .find(|entry| entry.id == *scope)
                .ok_or("Unknown navigation group")?;
            let expected = scope.items.iter().map(|item| item.node.ok_or("A generated template entry owns part of this group's order; edit its definition first")).collect::<Result<BTreeSet<_>, _>>()?;
            let actual = order.iter().copied().collect::<BTreeSet<_>>();
            if actual != expected || actual.len() != order.len() {
                return Err("A tab order must contain each authored entry of one navigation group exactly once".into());
            }
            let mut edits = Vec::new();
            for (index, node) in order.iter().enumerate() {
                let index = i32::try_from(index).map_err(|_| "Too many focus entries")?;
                edits.extend(plan_navigation_property(
                    document,
                    *node,
                    "TabIndex",
                    Some(&index.to_string()),
                    false,
                )?);
            }
            Ok(edits)
        }
        FocusOrderEdit::ResetOrder { scope } => {
            let scope = catalog
                .scopes
                .iter()
                .find(|entry| entry.id == *scope)
                .ok_or("Unknown navigation group")?;
            let mut edits = Vec::new();
            for node in scope.items.iter().filter_map(|item| item.node) {
                edits.extend(plan_navigation_property(
                    document, node, "TabIndex", None, true,
                )?);
            }
            Ok(edits)
        }
        FocusOrderEdit::SetTabStop { node, value }
        | FocusOrderEdit::SetFocusable { node, value } => {
            if !catalog.scopes.iter().any(|scope| {
                scope.node == Some(*node) || scope.items.iter().any(|item| item.node == Some(*node))
            }) {
                return Err("The input is not an authored entry in this preview".into());
            }
            let property = if matches!(command, FocusOrderEdit::SetTabStop { .. }) {
                "IsTabStop"
            } else {
                "Focusable"
            };
            plan_navigation_property(
                document,
                *node,
                property,
                Some(if *value { "True" } else { "False" }),
                false,
            )
        }
        FocusOrderEdit::SetNavigation { node, mode } => {
            if !catalog.scopes.iter().any(|scope| scope.node == Some(*node)) {
                return Err("The navigation group has no authored container".into());
            }
            plan_navigation_property(
                document,
                *node,
                "TabNavigation",
                Some(navigation_mode_name(*mode)),
                false,
            )
        }
        FocusOrderEdit::SetLabelTarget { label, target } => {
            let association = catalog
                .labels
                .iter()
                .find(|entry| entry.node == *label)
                .ok_or("The target is not an authored Label in this preview")?;
            let Some(target) = target else {
                return plan_navigation_property(document, *label, "Target", None, true);
            };
            if !association
                .candidates
                .iter()
                .any(|candidate| candidate.node == *target)
            {
                return Err(
                    "A Label target must be an authored focusable input in the same XAML namescope"
                        .into(),
                );
            }
            let target_node = document
                .ast()
                .node(*target)
                .ok_or("The label target was removed")?;
            let mut edits = Vec::new();
            let name = match authored_name(target_node) {
                Some(name) if valid_name(name) => name.to_owned(),
                Some(_) => {
                    return Err(
                        "The label target has a name that cannot be referenced safely".into(),
                    );
                }
                None => {
                    let names = document
                        .ast()
                        .elements()
                        .filter_map(authored_name)
                        .collect::<BTreeSet<_>>();
                    let mut name = "FocusTarget".to_owned();
                    let mut suffix = 1;
                    while names.contains(name.as_str()) {
                        name = format!("FocusTarget{suffix}");
                        suffix += 1;
                    }
                    edits.push(Edit::SetAttribute {
                        node: *target,
                        name: "Name".into(),
                        value: name.clone(),
                    });
                    name
                }
            };
            let extension_name = framework_name(document, *label, "Binding", &mut edits);
            // Names are validated XAML identifiers, so this cannot inject an
            // argument or change the markup-extension grammar.
            let binding = format!("{{{extension_name} ElementName={name}}}");
            edits.extend(plan_navigation_property(
                document,
                *label,
                "Target",
                Some(&binding),
                true,
            )?);
            Ok(edits)
        }
    }
}

pub fn navigation_mode_name(mode: KeyboardNavigationMode) -> &'static str {
    match mode {
        KeyboardNavigationMode::Continue => "Continue",
        KeyboardNavigationMode::Cycle => "Cycle",
        KeyboardNavigationMode::Contained => "Contained",
        KeyboardNavigationMode::Once => "Once",
        KeyboardNavigationMode::None => "None",
        KeyboardNavigationMode::Local => "Local",
    }
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

fn authored_name(node: &Node) -> Option<&str> {
    node.xaml_name().or_else(|| {
        node.attributes
            .iter()
            .find(|attribute| {
                attribute.name.rsplit(':').next() == Some("Name")
                    && known_namespace(attribute.namespace_uri.as_deref(), None)
            })
            .map(|attribute| attribute.value.as_str())
    })
}

fn known_namespace(namespace: Option<&str>, owner: Option<&str>) -> bool {
    match namespace {
        Some(FRAMEWORK) => true,
        Some("using:FerroUI.Input" | "clr-namespace:FerroUI.Input;assembly=FerroUI.Base") => {
            owner.is_none_or(|owner| matches!(owner, "InputElement" | "KeyboardNavigation"))
        }
        Some(
            "using:FerroUI.Controls" | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls",
        ) => owner.is_none_or(|owner| owner == "Label"),
        _ => false,
    }
}

fn matches_navigation_member(
    document: &Document,
    target: &Node,
    lexical: &str,
    namespace: Option<&str>,
    property: &str,
    property_element: bool,
) -> bool {
    let local = lexical.rsplit(':').next().unwrap_or(lexical);
    let (owner, member) = local
        .rsplit_once('.')
        .map_or((None, local), |(owner, member)| (Some(owner), member));
    if member != property {
        return false;
    }
    if property_element
        && owner == Some(target.local_name())
        && namespace == target.namespace_uri.as_deref()
    {
        return true;
    }
    let owner_matches = match property {
        "TabIndex" | "IsTabStop" => {
            owner.is_none_or(|owner| matches!(owner, "InputElement" | "KeyboardNavigation"))
        }
        "Focusable" => owner.is_none_or(|owner| owner == "InputElement"),
        "TabNavigation" => owner == Some("KeyboardNavigation"),
        "Target" => owner.is_none_or(|owner| owner == "Label"),
        _ => false,
    };
    if !owner_matches {
        return false;
    }
    if !lexical.contains(':') && owner.is_none() && !property_element {
        return true;
    }
    let namespaces = document.ast().namespaces(target.id);
    let namespace = if !lexical.contains(':') && !property_element {
        namespaces.get("").map(String::as_str)
    } else {
        namespace
    };
    known_namespace(namespace, owner)
}

fn navigation_attributes<'a>(
    document: &Document,
    node: &'a Node,
    property: &str,
) -> Result<Vec<&'a Attribute>, String> {
    let attributes = node
        .attributes
        .iter()
        .filter(|attribute| {
            !attribute.is_namespace()
                && matches_navigation_member(
                    document,
                    node,
                    &attribute.name,
                    attribute.namespace_uri.as_deref(),
                    property,
                    false,
                )
        })
        .collect::<Vec<_>>();
    if attributes.len() > 1 {
        return Err(format!(
            "{property} is authored more than once through equivalent aliases"
        ));
    }
    Ok(attributes)
}

fn plan_navigation_property(
    document: &Document,
    id: NodeId,
    property: &str,
    value: Option<&str>,
    replace_expression: bool,
) -> Result<Vec<Edit>, String> {
    let node = document
        .ast()
        .node(id)
        .filter(|node| node.kind == NodeKind::Element)
        .ok_or("Keyboard navigation needs an authored control")?;
    let attributes = navigation_attributes(document, node, property)?;
    let attribute = attributes.first().copied();
    let elements = node
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|child| {
            child.kind == NodeKind::PropertyElement
                && matches_navigation_member(
                    document,
                    node,
                    &child.name,
                    child.namespace_uri.as_deref(),
                    property,
                    true,
                )
        })
        .collect::<Vec<_>>();
    if !replace_expression
        && (!elements.is_empty()
            || attribute.is_some_and(|attribute| attribute.value.trim_start().starts_with('{')))
    {
        return Err(format!(
            "{property} is bound or authored as an object; edit/reset that value explicitly before ordering"
        ));
    }
    if attribute.is_some_and(|attribute| Some(attribute.value.as_str()) == value)
        && elements.is_empty()
    {
        return Ok(Vec::new());
    }
    let mut edits = elements
        .into_iter()
        .map(|element| Edit::RemoveNode { node: element.id })
        .collect::<Vec<_>>();
    if let Some(value) = value {
        let name = match attribute {
            Some(attribute) => attribute.name.clone(),
            None if property == "TabNavigation" => {
                framework_name(document, id, "KeyboardNavigation.TabNavigation", &mut edits)
            }
            None => property.to_owned(),
        };
        edits.push(Edit::SetAttribute {
            node: id,
            name,
            value: value.to_owned(),
        });
    } else if let Some(attribute) = attribute {
        edits.push(Edit::RemoveAttribute {
            node: id,
            name: attribute.name.clone(),
        });
    }
    Ok(edits)
}

fn framework_name(
    document: &Document,
    node: NodeId,
    member: &str,
    edits: &mut Vec<Edit>,
) -> String {
    let namespaces = document.ast().namespaces(node);
    if namespaces.get("").is_some_and(|uri| uri == FRAMEWORK) {
        return member.to_owned();
    }
    if let Some((prefix, _)) = namespaces
        .iter()
        .find(|(prefix, uri)| !prefix.is_empty() && uri.as_str() == FRAMEWORK)
    {
        return format!("{prefix}:{member}");
    }
    let mut prefix = "focus".to_owned();
    let mut suffix = 1;
    while namespaces.contains_key(&prefix) {
        prefix = format!("focus{suffix}");
        suffix += 1;
    }
    edits.push(Edit::SetAttribute {
        node,
        name: format!("xmlns:{prefix}"),
        value: FRAMEWORK.into(),
    });
    format!("{prefix}:{member}")
}

#[path = "focus_order_overlay.rs"]
mod overlay;
pub(crate) use overlay::render_focus_order;

#[cfg(test)]
#[path = "focus_order_tests.rs"]
mod tests;
