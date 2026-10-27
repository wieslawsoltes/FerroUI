use crate::layout_helpers::{
    LayoutHandleAction, LayoutHelperRequest, LayoutHelperScene, LayoutPlacement,
};
use crate::{Axis, Point, PropertyRegistry, Rect, Result, ToolError, error, require};
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockSide {
    Left,
    Top,
    Right,
    Bottom,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelativeRelation {
    LeftOf,
    RightOf,
    Above,
    Below,
    AlignLeftWith,
    AlignRightWith,
    AlignTopWith,
    AlignBottomWith,
    AlignHorizontalCenterWith,
    AlignVerticalCenterWith,
}
impl RelativeRelation {
    pub fn property(self) -> &'static str {
        match self {
            Self::LeftOf => "RelativePanel.LeftOf",
            Self::RightOf => "RelativePanel.RightOf",
            Self::Above => "RelativePanel.Above",
            Self::Below => "RelativePanel.Below",
            Self::AlignLeftWith => "RelativePanel.AlignLeftWith",
            Self::AlignRightWith => "RelativePanel.AlignRightWith",
            Self::AlignTopWith => "RelativePanel.AlignTopWith",
            Self::AlignBottomWith => "RelativePanel.AlignBottomWith",
            Self::AlignHorizontalCenterWith => "RelativePanel.AlignHorizontalCenterWith",
            Self::AlignVerticalCenterWith => "RelativePanel.AlignVerticalCenterWith",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum GridLength {
    Auto,
    Pixels(f64),
    Star(f64),
}
impl GridLength {
    pub fn parse(source: &str) -> Result<Self> {
        let s = source.trim();
        if s.eq_ignore_ascii_case("Auto") {
            return Ok(Self::Auto);
        }
        let result = if let Some(weight) = s.strip_suffix('*') {
            Self::Star(if weight.is_empty() {
                1.0
            } else {
                number(weight)?
            })
        } else {
            Self::Pixels(number(s)?)
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(self) -> Result<()> {
        match self {
            Self::Auto => Ok(()),
            Self::Pixels(value) | Self::Star(value) => require(
                value.is_finite() && value >= 0.0,
                "Grid lengths must be finite and nonnegative",
            ),
        }
    }
    pub fn to_xaml(self) -> String {
        match self {
            Self::Auto => "Auto".into(),
            Self::Pixels(value) => value.to_string(),
            Self::Star(value) => {
                if value == 1.0 {
                    "*".into()
                } else {
                    format!("{value}*")
                }
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridLines {
    pub rows: Vec<f64>,
    pub columns: Vec<f64>,
}
impl GridLines {
    pub fn validate(&self) -> Result<()> {
        for lines in [&self.rows, &self.columns] {
            require(
                lines.len() >= 2
                    && lines[0] == 0.0
                    && lines.iter().all(|v| v.is_finite() && *v >= 0.0)
                    && lines.windows(2).all(|w| w[0] <= w[1]),
                "Grid boundaries must start at zero and be finite/nondecreasing",
            )?;
        }
        Ok(())
    }
}
/// Effective layout values supplied by the live host. A slot is the rectangle passed to the
/// child's arrange method, before its margins, alignment and render transform are applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedLayout {
    pub margin: [f64; 4],
    pub slot: Option<Rect>,
    pub visible: bool,
}
impl Default for ResolvedLayout {
    fn default() -> Self {
        Self {
            margin: [0.0; 4],
            slot: None,
            visible: true,
        }
    }
}
impl ResolvedLayout {
    fn validate(self) -> Result<()> {
        require(
            self.margin.iter().all(|value| value.is_finite()),
            "Resolved margins must be finite",
        )?;
        if let Some(slot) = self.slot {
            slot.validate()?;
        }
        Ok(())
    }
}
/// Bounds and resolved slots use one common coordinate space. Grid boundaries use panel-local units.
pub struct LayoutContext<'a> {
    pub document: &'a Document,
    pub bounds: &'a BTreeMap<NodeId, Rect>,
    grid_lines: Option<&'a BTreeMap<NodeId, GridLines>>,
    resolved_layout: Option<&'a BTreeMap<NodeId, ResolvedLayout>>,
    resolved_properties: Option<&'a BTreeMap<NodeId, BTreeMap<String, String>>>,
}
impl<'a> LayoutContext<'a> {
    pub fn new(document: &'a Document, bounds: &'a BTreeMap<NodeId, Rect>) -> Self {
        Self {
            document,
            bounds,
            grid_lines: None,
            resolved_layout: None,
            resolved_properties: None,
        }
    }
    pub fn with_grid_lines(mut self, lines: &'a BTreeMap<NodeId, GridLines>) -> Self {
        self.grid_lines = Some(lines);
        self
    }
    pub fn with_resolved_layout(mut self, layout: &'a BTreeMap<NodeId, ResolvedLayout>) -> Self {
        self.resolved_layout = Some(layout);
        self
    }
    /// Effective runtime values used to interpret panel layout, including styles and bindings.
    /// These values never replace authored expressions when an operation writes a property.
    pub fn with_resolved_properties(
        mut self,
        values: &'a BTreeMap<NodeId, BTreeMap<String, String>>,
    ) -> Self {
        self.resolved_properties = Some(values);
        self
    }
    pub fn property(&self, node: NodeId, name: &str) -> Option<&str> {
        self.resolved_properties
            .and_then(|values| values.get(&node))
            .and_then(|values| values.get(name))
            .map(String::as_str)
            .or_else(|| {
                self.document
                    .ast()
                    .node(node)
                    .and_then(|node| {
                        crate::layout_attributes::layout_attribute(self.document, node, name)
                            .ok()
                            .flatten()
                    })
                    .map(|attribute| attribute.value.as_str())
            })
    }
    /// The authored children, preserving their source order and explicit content slot.
    pub fn children(&self, panel: NodeId) -> Result<Vec<NodeId>> {
        authored_children(self, panel)
    }
    /// The AST owner of the panel's content, including explicit Children/Content/Child elements.
    pub fn content_container(&self, panel: NodeId) -> Result<NodeId> {
        child_container(self, panel)
    }
    /// Resolved allocation before margin/alignment/render transforms, if supplied by the host.
    pub fn resolved_layout(&self, node: NodeId) -> Result<ResolvedLayout> {
        self.layout(node)
    }
    pub fn is_visible(&self, node: NodeId) -> Result<bool> {
        self.visible(node)
    }
    /// Actual Grid track boundaries. A portable fallback resolves unconstrained pixel/star tracks.
    pub fn grid_boundaries(&self, panel: NodeId) -> Result<GridLines> {
        if let Some(lines) = self.grid_lines.and_then(|all| all.get(&panel)) {
            lines.validate()?;
            return Ok(lines.clone());
        }
        require_plain_grid(self, panel)?;
        let bounds = self.bounds(panel)?;
        Ok(GridLines {
            rows: resolved_lines(
                &grid_definitions(self, panel, Axis::Vertical)?,
                bounds.height,
            )?,
            columns: resolved_lines(
                &grid_definitions(self, panel, Axis::Horizontal)?,
                bounds.width,
            )?,
        })
    }
    fn ensure_resolved(&self, node: NodeId, name: &str) -> Result<()> {
        if self
            .resolved_properties
            .and_then(|values| values.get(&node))
            .is_some_and(|values| values.contains_key(name))
        {
            Ok(())
        } else {
            ensure_literal(self, node, name)
        }
    }
    pub fn bounds(&self, node: NodeId) -> Result<Rect> {
        let result = *self
            .bounds
            .get(&node)
            .ok_or_else(|| ToolError(format!("Measured bounds are unavailable for node {node}")))?;
        result.validate()?;
        Ok(result)
    }
    pub(crate) fn element(&self, node: NodeId) -> Result<&Node> {
        let n = self
            .document
            .ast()
            .node(node)
            .ok_or_else(|| ToolError(format!("Unknown node {node}")))?;
        require(n.is_element(), "Layout tools require an element")?;
        Ok(n)
    }
    pub(crate) fn layout(&self, node: NodeId) -> Result<ResolvedLayout> {
        let result = if let Some(layout) = self.resolved_layout.and_then(|map| map.get(&node)) {
            *layout
        } else {
            ensure_literal(self, node, "Margin")?;
            ResolvedLayout {
                margin: thickness(self.element(node)?.attribute("Margin").unwrap_or("0"))?,
                ..ResolvedLayout::default()
            }
        };
        result.validate()?;
        Ok(result)
    }
    pub(crate) fn visible(&self, node: NodeId) -> Result<bool> {
        if let Some(layout) = self.resolved_layout.and_then(|map| map.get(&node)) {
            return Ok(layout.visible);
        }
        ensure_literal(self, node, "IsVisible")?;
        let value = self.element(node)?.attribute("IsVisible").unwrap_or("True");
        require(
            value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false"),
            "IsVisible must be a boolean or resolved by the live host",
        )?;
        Ok(value.eq_ignore_ascii_case("true"))
    }
    pub(crate) fn parent(&self, node: NodeId) -> Result<NodeId> {
        let mut parent = self.element(node)?.parent;
        while let Some(id) = parent {
            let n = self.element(id)?;
            if n.kind != NodeKind::PropertyElement {
                return Ok(id);
            }
            parent = n.parent;
        }
        Err(ToolError(
            "Root layout belongs to the artboard; select a child".into(),
        ))
    }
}
#[derive(Clone, Debug)]
pub enum LayoutAction {
    Move {
        node: NodeId,
        delta: Point,
    },
    Resize {
        node: NodeId,
        before: Rect,
        after: Rect,
    },
    Reorder {
        node: NodeId,
        index: usize,
    },
    Drop {
        parent: NodeId,
        source: String,
        position: Point,
    },
    GridCell {
        node: NodeId,
        row: usize,
        column: usize,
        row_span: usize,
        column_span: usize,
    },
    GridTrack {
        panel: NodeId,
        axis: Axis,
        index: usize,
        length: GridLength,
    },
    Dock {
        node: NodeId,
        side: DockSide,
    },
    Relative {
        node: NodeId,
        relation: RelativeRelation,
        target: Option<NodeId>,
    },
    PanelProperty {
        panel: NodeId,
        name: String,
        value: String,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutCapabilities {
    pub move_items: bool,
    pub resize_items: bool,
    pub reorder: bool,
    pub drop_items: bool,
    pub generated_items: bool,
}
/// Third-party panels register source planners, with the same transactional validation as built-ins.
pub trait LayoutAdapter {
    fn panel_type(&self) -> &str;
    fn capabilities(&self) -> LayoutCapabilities;
    fn panel_properties(&self) -> &[&str];
    fn plan(&self, context: &LayoutContext<'_>, action: &LayoutAction) -> Result<Vec<Edit>>;
    /// Optional design-time feature provider. The default draws only a generic container outline.
    fn design_helpers(
        &self,
        context: &LayoutContext<'_>,
        request: &LayoutHelperRequest,
    ) -> Result<LayoutHelperScene> {
        LayoutHelperScene::generic(context, request)
    }
    /// Optional semantic placement provider. Indices always reference the original AST snapshot.
    fn placement(
        &self,
        _context: &LayoutContext<'_>,
        _panel: NodeId,
        _position: Point,
        _dragged: &[NodeId],
    ) -> Result<Option<LayoutPlacement>> {
        Ok(None)
    }
    /// Override to implement Custom helper intents. Standard actions share existing source planners.
    fn plan_helper(
        &self,
        context: &LayoutContext<'_>,
        action: &LayoutHandleAction,
        value: f64,
    ) -> Result<Vec<Edit>> {
        crate::layout_helpers::plan_standard_helper(context, action, value, |action| {
            self.plan(context, action)
        })
    }
}
pub struct LayoutRegistry {
    adapters: BTreeMap<String, Box<dyn LayoutAdapter>>,
    revision: u64,
    builtin_names: BTreeSet<String>,
}
impl Default for LayoutRegistry {
    fn default() -> Self {
        Self::built_in()
    }
}
impl LayoutRegistry {
    pub fn new() -> Self {
        Self {
            adapters: BTreeMap::new(),
            revision: 0,
            builtin_names: BTreeSet::new(),
        }
    }
    pub fn built_in() -> Self {
        let mut registry = Self::new();
        for name in [
            "Panel",
            "Canvas",
            "Grid",
            "StackPanel",
            "ReversibleStackPanel",
            "WrapPanel",
            "DockPanel",
            "UniformGrid",
            "RelativePanel",
            "FlexPanel",
            "VirtualizingStackPanel",
            "VirtualizingCarouselPanel",
            "VirtualizingPanel",
            "DateTimePickerPanel",
            "TextSelectionHandleCanvas",
            "TableViewColumnHeadersPresenter",
            "TableViewCellsPresenter",
            "AdornerLayer",
            "OverlayLayer",
            "PopupOverlayLayer",
            "TextSelectorLayer",
            "Border",
            "Decorator",
            "ContentControl",
            "UserControl",
            "Window",
            "ScrollViewer",
            "Viewbox",
            "Button",
            "ToggleButton",
        ] {
            registry.register(Box::new(BuiltIn { name }));
            registry.builtin_names.insert(name.to_owned());
        }
        registry
    }
    pub fn register(&mut self, adapter: Box<dyn LayoutAdapter>) -> Option<Box<dyn LayoutAdapter>> {
        self.revision = self.revision.wrapping_add(1);
        self.builtin_names.remove(adapter.panel_type());
        self.adapters
            .insert(adapter.panel_type().to_owned(), adapter)
    }
    /// Changes whenever a provider is registered/replaced; hosts capture it in active gestures.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn design_helpers(
        &self,
        context: &LayoutContext<'_>,
        request: &LayoutHelperRequest,
    ) -> Result<LayoutHelperScene> {
        require(
            context.document.is_valid(),
            "Layout helpers require valid source",
        )?;
        let panel = context.element(request.panel)?;
        let adapter = self
            .adapter_for(context.document, panel.id)
            .ok_or_else(|| ToolError(format!("Register a layout adapter for {}", panel.name)))?;
        let mut scene = adapter.design_helpers(context, request)?;
        require(
            scene.panel == request.panel,
            "The feature provider returned a different panel",
        )?;
        if !adapter.capabilities().resize_items || adapter.capabilities().generated_items {
            for handle in &mut scene.handles {
                handle.disabled_reason =
                    Some("This panel owns generated or read-only layout".into());
            }
        }
        scene.validate()?;
        Ok(scene)
    }
    pub fn placement(
        &self,
        context: &LayoutContext<'_>,
        panel: NodeId,
        position: Point,
        dragged: &[NodeId],
    ) -> Result<Option<LayoutPlacement>> {
        position.validate()?;
        let element = context.element(panel)?;
        let adapter = self
            .adapter_for(context.document, element.id)
            .ok_or_else(|| ToolError(format!("Register a layout adapter for {}", element.name)))?;
        let capabilities = adapter.capabilities();
        if !capabilities.drop_items || capabilities.generated_items {
            return Ok(Some(LayoutPlacement::rejected(
                context,
                panel,
                "This panel owns generated runtime content",
            )?));
        }
        let placement = adapter.placement(context, panel, position, dragged)?;
        if let Some(placement) = &placement {
            placement.validate()?;
        }
        Ok(placement)
    }
    pub fn plan_helper(
        &self,
        context: &LayoutContext<'_>,
        panel: NodeId,
        action: &LayoutHandleAction,
        value: f64,
    ) -> Result<Vec<Edit>> {
        require(value.is_finite(), "Helper values must be finite")?;
        require(
            context.document.is_valid(),
            "Layout helper edits require valid source",
        )?;
        match action {
            LayoutHandleAction::PanelProperty { panel: owner, .. }
            | LayoutHandleAction::GridTrack { panel: owner, .. } => {
                require(*owner == panel, "The helper belongs to a different panel")?;
            }
            LayoutHandleAction::ThicknessSide { node, .. } => {
                require(
                    *node == panel || context.parent(*node)? == panel,
                    "The thickness helper belongs to a different layout",
                )?;
            }
            LayoutHandleAction::Custom { .. } => {}
        }
        let element = context.element(panel)?;
        let adapter = self
            .adapter_for(context.document, element.id)
            .ok_or_else(|| ToolError(format!("Register a layout adapter for {}", element.name)))?;
        require(
            adapter.capabilities().resize_items && !adapter.capabilities().generated_items,
            "This panel does not permit helper edits",
        )?;
        let edits = adapter.plan_helper(context, action, value)?;
        let mut candidate = context.document.clone();
        candidate
            .apply("Validate layout helper", edits.clone())
            .map_err(error)?;
        Ok(edits)
    }
    pub fn supports(&self, panel_type: &str) -> bool {
        self.adapter(panel_type).is_some()
    }
    pub fn panel_types(&self) -> impl Iterator<Item = &str> {
        self.adapters.keys().map(String::as_str)
    }
    /// Resolve canonical XAML type identity before legacy lexical names. A custom namespace
    /// with a built-in local name never silently inherits that framework panel's semantics.
    pub fn adapter_for(&self, document: &Document, node: NodeId) -> Option<&dyn LayoutAdapter> {
        let element = document.ast().node(node)?;
        if let Some(uri) = element.namespace_uri.as_deref() {
            if let Some(adapter) = self
                .adapters
                .get(&format!("{{{uri}}}{}", element.local_name()))
            {
                return Some(adapter.as_ref());
            }
        }
        let framework = matches!(
            element.namespace_uri.as_deref(),
            None | Some(
                "https://github.com/ferroui"
                    | "using:FerroUI.Controls"
                    | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
            )
        );
        if framework || !self.builtin_names.contains(&element.name) {
            if let Some(adapter) = self.adapters.get(&element.name) {
                return Some(adapter.as_ref());
            }
        }
        if framework || !self.builtin_names.contains(element.local_name()) {
            return self.adapters.get(element.local_name()).map(Box::as_ref);
        }
        None
    }
    /// Whether canonical resolution selected an unchanged framework provider.
    pub fn is_builtin_adapter_for(&self, document: &Document, node: NodeId) -> bool {
        self.adapter_for(document, node)
            .is_some_and(|adapter| self.builtin_names.contains(adapter.panel_type()))
    }
    pub fn adapter(&self, panel_type: &str) -> Option<&dyn LayoutAdapter> {
        self.adapters
            .get(panel_type)
            .or_else(|| self.adapters.get(local_name(panel_type)))
            .map(Box::as_ref)
    }
    /// Exact lookup for application-qualified adapters without confusing an application type
    /// with a built-in panel that happens to have the same local name.
    pub fn adapter_exact(&self, panel_type: &str) -> Option<&dyn LayoutAdapter> {
        self.adapters.get(panel_type).map(Box::as_ref)
    }
    pub fn plan(&self, context: &LayoutContext<'_>, action: LayoutAction) -> Result<Vec<Edit>> {
        self.plan_many(context, [action])
    }
    /// Plans independent actions against one immutable snapshot and validates their combined
    /// transaction once. Insertion indices and node identities refer to the original snapshot.
    pub fn plan_many(
        &self,
        context: &LayoutContext<'_>,
        actions: impl IntoIterator<Item = LayoutAction>,
    ) -> Result<Vec<Edit>> {
        self.plan_batch(actions.into_iter().map(|action| (context, action)))
    }
    /// A batch may use different measured coordinate spaces, but every context must reference
    /// the same Document. This supports multiselection across different transformed parents
    /// without cloning and reparsing the complete source once per selected element.
    pub fn plan_batch<'context, 'document: 'context>(
        &self,
        plans: impl IntoIterator<Item = (&'context LayoutContext<'document>, LayoutAction)>,
    ) -> Result<Vec<Edit>> {
        let mut document: Option<&Document> = None;
        let mut edits = Vec::new();
        let mut relative_constraints = Vec::new();
        let mut content_hosts = BTreeSet::new();
        let mut attributes = BTreeMap::new();
        for (context, action) in plans {
            if let Some(previous) = document {
                require(
                    std::ptr::eq(previous, context.document),
                    "Layout batch contexts must reference the same document snapshot",
                )?;
            } else {
                document = Some(context.document);
            }
            if let LayoutAction::Relative {
                node,
                relation,
                target,
            } = &action
            {
                relative_constraints.push((*node, *relation, *target));
            }
            if let LayoutAction::Drop { parent, .. } = &action {
                if single_content(local_name(&context.element(*parent)?.name)) {
                    content_hosts.insert(*parent);
                }
            }
            for edit in self.plan_snapshot(context, &action)? {
                let attribute = match &edit {
                    Edit::SetAttribute { node, name, value } => {
                        Some(((*node, name.clone()), Some(value.clone())))
                    }
                    Edit::RemoveAttribute { node, name } => Some(((*node, name.clone()), None)),
                    _ => None,
                };
                if let Some((key, value)) = attribute {
                    if let Some(previous) = attributes.insert(key, value.clone()) {
                        require(
                            previous == value,
                            "Batch actions write conflicting values to the same property; combine them in one adapter action",
                        )?;
                    }
                }
                edits.push(edit);
            }
        }
        if let Some(document) = document {
            let mut candidate = document.clone();
            candidate
                .apply("Validate layout plan", edits.clone())
                .map_err(error)?;
            let bounds = BTreeMap::new();
            let candidate_context = LayoutContext::new(&candidate, &bounds);
            for (node, relation, target) in relative_constraints {
                relative_plan(&candidate_context, node, relation, target)?;
            }
            for host in content_hosts {
                require(
                    authored_children(&candidate_context, host)?.len() <= 1,
                    "Batch would insert multiple children into a single-content host",
                )?;
            }
        }
        Ok(edits)
    }
    fn plan_snapshot(
        &self,
        context: &LayoutContext<'_>,
        action: &LayoutAction,
    ) -> Result<Vec<Edit>> {
        require(
            context.document.is_valid(),
            "Layout edits require valid source",
        )?;
        let parent = match action {
            LayoutAction::Drop { parent, .. } => *parent,
            LayoutAction::GridTrack { panel, .. } | LayoutAction::PanelProperty { panel, .. } => {
                *panel
            }
            LayoutAction::Move { node, .. }
            | LayoutAction::Resize { node, .. }
            | LayoutAction::Reorder { node, .. }
            | LayoutAction::GridCell { node, .. }
            | LayoutAction::Dock { node, .. }
            | LayoutAction::Relative { node, .. } => context.parent(*node)?,
        };
        let panel = context.element(parent)?;
        let adapter = self
            .adapter_for(context.document, panel.id)
            .ok_or_else(|| ToolError(format!("Register a layout adapter for {}", panel.name)))?;
        adapter.plan(context, action)
    }
    pub fn move_selection(
        &self,
        context: &LayoutContext<'_>,
        nodes: &[NodeId],
        delta: Point,
    ) -> Result<Vec<Edit>> {
        delta.validate()?;
        self.plan_many(
            context,
            crate::outermost(context.document, nodes)?
                .into_iter()
                .map(|node| LayoutAction::Move { node, delta }),
        )
    }
    pub fn align_selection(
        &self,
        context: &LayoutContext<'_>,
        nodes: &[NodeId],
        alignment: crate::Alignment,
    ) -> Result<Vec<Edit>> {
        let rects = crate::outermost(context.document, nodes)?
            .into_iter()
            .map(|node| Ok((node, context.bounds(node)?)))
            .collect::<Result<Vec<_>>>()?;
        self.apply_deltas(context, crate::align(&rects, alignment)?)
    }
    pub fn distribute_selection(
        &self,
        context: &LayoutContext<'_>,
        nodes: &[NodeId],
        axis: Axis,
    ) -> Result<Vec<Edit>> {
        let rects = crate::outermost(context.document, nodes)?
            .into_iter()
            .map(|node| Ok((node, context.bounds(node)?)))
            .collect::<Result<Vec<_>>>()?;
        self.apply_deltas(context, crate::distribute(&rects, axis)?)
    }
    fn apply_deltas(
        &self,
        context: &LayoutContext<'_>,
        deltas: BTreeMap<NodeId, Point>,
    ) -> Result<Vec<Edit>> {
        self.plan_many(
            context,
            deltas
                .into_iter()
                .map(|(node, delta)| LayoutAction::Move { node, delta }),
        )
    }
}
struct BuiltIn {
    name: &'static str,
}
impl BuiltIn {
    fn managed(&self) -> bool {
        matches!(
            self.name,
            "VirtualizingStackPanel"
                | "VirtualizingCarouselPanel"
                | "VirtualizingPanel"
                | "DateTimePickerPanel"
                | "TextSelectionHandleCanvas"
                | "TableViewColumnHeadersPresenter"
                | "TableViewCellsPresenter"
                | "AdornerLayer"
                | "OverlayLayer"
                | "PopupOverlayLayer"
                | "TextSelectorLayer"
        )
    }
}
impl LayoutAdapter for BuiltIn {
    fn panel_type(&self) -> &str {
        self.name
    }
    fn capabilities(&self) -> LayoutCapabilities {
        let editable = !self.managed();
        LayoutCapabilities {
            move_items: editable,
            resize_items: editable,
            reorder: editable && !single_content(self.name),
            drop_items: editable,
            generated_items: self.managed(),
        }
    }
    fn panel_properties(&self) -> &[&str] {
        match self.name {
            "Grid" => &[
                "RowDefinitions",
                "ColumnDefinitions",
                "RowSpacing",
                "ColumnSpacing",
                "ShowGridLines",
            ],
            "StackPanel" => &["Orientation", "Spacing"],
            "ReversibleStackPanel" => &["Orientation", "Spacing", "ReverseOrder"],
            "WrapPanel" => &[
                "Orientation",
                "ItemWidth",
                "ItemHeight",
                "ItemSpacing",
                "LineSpacing",
                "ItemsAlignment",
            ],
            "DockPanel" => &["LastChildFill", "HorizontalSpacing", "VerticalSpacing"],
            "UniformGrid" => &[
                "Rows",
                "Columns",
                "FirstColumn",
                "RowSpacing",
                "ColumnSpacing",
            ],
            "FlexPanel" => &[
                "Direction",
                "JustifyContent",
                "AlignItems",
                "AlignContent",
                "Wrap",
                "RowSpacing",
                "ColumnSpacing",
            ],
            "VirtualizingStackPanel" => &["Orientation", "CacheLength"],
            "DateTimePickerPanel" => &["ItemHeight", "PanelType", "ItemFormat", "ShouldLoop"],
            _ => &[],
        }
    }
    fn design_helpers(
        &self,
        context: &LayoutContext<'_>,
        request: &LayoutHelperRequest,
    ) -> Result<LayoutHelperScene> {
        crate::layout_helpers::built_in_helpers(context, request, self.name, self.managed())
    }
    fn placement(
        &self,
        context: &LayoutContext<'_>,
        panel: NodeId,
        position: Point,
        dragged: &[NodeId],
    ) -> Result<Option<LayoutPlacement>> {
        if self.managed() {
            return Ok(Some(LayoutPlacement::rejected(
                context,
                panel,
                "This panel owns generated runtime content",
            )?));
        }
        built_in_placement(context, panel, position, self.name, dragged)
            .or_else(|reason| LayoutPlacement::rejected(context, panel, reason.to_string()))
            .map(Some)
    }
    fn plan(&self, context: &LayoutContext<'_>, action: &LayoutAction) -> Result<Vec<Edit>> {
        if let LayoutAction::PanelProperty { panel, name, value } = action {
            require(
                self.panel_properties().contains(&name.as_str()),
                format!("{} is not a layout property of {}", name, self.name),
            )?;
            ensure_literal(context, *panel, name)?;
            PropertyRegistry::built_in().plan(
                context.document,
                &[*panel],
                name,
                Some(value),
                crate::ExpressionPolicy::PreserveExpressions,
            )?;
            return crate::plan_layout_attributes(
                context.document,
                *panel,
                &[(name.clone(), value.clone())],
            );
        }
        require(
            !self.managed(),
            "This panel owns generated runtime containers; edit its item source/template or panel properties",
        )?;
        match action {
            LayoutAction::Move { node, delta } => move_plan(context, *node, *delta, self.name),
            LayoutAction::Resize {
                node,
                before,
                after,
            } => {
                before.validate()?;
                after.validate()?;
                require(
                    after.width > 0.0 && after.height > 0.0,
                    "Resize dimensions must be positive",
                )?;
                if before == after {
                    return Ok(vec![]);
                }
                ensure_literal(context, *node, "Width")?;
                ensure_literal(context, *node, "Height")?;
                let delta = Point::new(after.x - before.x, after.y - before.y);
                let mut edits = if self.name == "Canvas" {
                    // Normalize right/bottom anchors even when the leading edge did not move.
                    // Otherwise changing Width/Height moves the opposite anchored edge.
                    canvas_position_plan(context, *node, delta)?
                } else {
                    ensure_literal(context, *node, "Margin")?;
                    let mut margin = context.layout(*node)?.margin;
                    margin[0] += delta.x;
                    margin[1] += delta.y;
                    margin[2] -= delta.x + after.width - before.width;
                    margin[3] -= delta.y + after.height - before.height;
                    // Keep the outer allocation and alignment slack constant. This preserves
                    // the dragged rectangle for centered/stretched controls and avoids
                    // changing sibling slots in size-to-content flow panels.
                    vec![margin_edit(*node, margin)?]
                };
                edits.extend([
                    set(*node, "Width", after.width),
                    set(*node, "Height", after.height),
                ]);
                Ok(edits)
            }
            LayoutAction::Reorder { node, index } => {
                reorder_plan(context, *node, *index, self.name)
            }
            LayoutAction::Drop {
                parent,
                source,
                position,
            } => drop_plan(context, *parent, source, *position, self.name),
            LayoutAction::GridCell {
                node,
                row,
                column,
                row_span,
                column_span,
            } => {
                require(
                    self.name == "Grid",
                    "Grid cell placement requires a Grid parent",
                )?;
                let parent = context.parent(*node)?;
                let (rows, columns) =
                    if let Some(lines) = context.grid_lines.and_then(|all| all.get(&parent)) {
                        lines.validate()?;
                        (lines.rows.len() - 1, lines.columns.len() - 1)
                    } else {
                        (
                            grid_definitions(context, parent, Axis::Vertical)?.len(),
                            grid_definitions(context, parent, Axis::Horizontal)?.len(),
                        )
                    };
                require(
                    *row_span > 0
                        && *column_span > 0
                        && row.checked_add(*row_span).is_some_and(|end| end <= rows)
                        && column
                            .checked_add(*column_span)
                            .is_some_and(|end| end <= columns),
                    "Grid cell/span extends beyond its definitions",
                )?;
                for property in ["Grid.Row", "Grid.Column", "Grid.RowSpan", "Grid.ColumnSpan"] {
                    ensure_literal(context, *node, property)?;
                }
                Ok(vec![
                    text_set(*node, "Grid.Row", row.to_string()),
                    text_set(*node, "Grid.Column", column.to_string()),
                    text_set(*node, "Grid.RowSpan", row_span.to_string()),
                    text_set(*node, "Grid.ColumnSpan", column_span.to_string()),
                ])
            }
            LayoutAction::GridTrack {
                panel,
                axis,
                index,
                length,
            } => {
                require(self.name == "Grid", "Grid track editing requires Grid")?;
                grid_track_plan(context, *panel, *axis, *index, *length)
            }
            LayoutAction::Dock { node, side } => {
                require(
                    self.name == "DockPanel",
                    "Dock placement requires DockPanel",
                )?;
                ensure_literal(context, *node, "DockPanel.Dock")?;
                Ok(vec![text_set(*node, "DockPanel.Dock", format!("{side:?}"))])
            }
            LayoutAction::Relative {
                node,
                relation,
                target,
            } => {
                require(
                    self.name == "RelativePanel",
                    "Relative constraints require RelativePanel",
                )?;
                relative_plan(context, *node, *relation, *target)
            }
            LayoutAction::PanelProperty { .. } => unreachable!(),
        }
    }
}
fn move_plan(
    context: &LayoutContext<'_>,
    node: NodeId,
    delta: Point,
    panel: &str,
) -> Result<Vec<Edit>> {
    delta.validate()?;
    if delta.x == 0.0 && delta.y == 0.0 {
        return Ok(vec![]);
    }
    if panel == "Canvas" {
        canvas_position_plan(context, node, delta)
    } else {
        ensure_literal(context, node, "Margin")?;
        let mut margin = context.layout(node)?.margin;
        margin[0] += delta.x;
        margin[1] += delta.y;
        margin[2] -= delta.x;
        margin[3] -= delta.y;
        Ok(vec![margin_edit(node, margin)?])
    }
}
fn margin_edit(node: NodeId, margin: [f64; 4]) -> Result<Edit> {
    require(margin.iter().all(|v| v.is_finite()), "Margin overflow")?;
    Ok(text_set(
        node,
        "Margin",
        format!("{},{},{},{}", margin[0], margin[1], margin[2], margin[3]),
    ))
}
fn canvas_position_plan(
    context: &LayoutContext<'_>,
    node: NodeId,
    delta: Point,
) -> Result<Vec<Edit>> {
    delta.validate()?;
    let element = context.element(node)?;
    for property in ["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"] {
        ensure_literal(context, node, property)?;
    }
    let measured = if element.attribute("Canvas.Left").is_none()
        || element.attribute("Canvas.Top").is_none()
    {
        let layout = context.layout(node)?;
        let p = context.bounds(context.parent(node)?)?;
        let position = if let Some(slot) = layout.slot {
            Point::new(slot.x - p.x, slot.y - p.y)
        } else {
            let bounds = context.bounds(node)?;
            Point::new(
                bounds.x - p.x - layout.margin[0],
                bounds.y - p.y - layout.margin[1],
            )
        };
        Some(position)
    } else {
        None
    };
    let x = element
        .attribute("Canvas.Left")
        .map(number)
        .transpose()?
        .unwrap_or_else(|| measured.unwrap().x);
    let y = element
        .attribute("Canvas.Top")
        .map(number)
        .transpose()?
        .unwrap_or_else(|| measured.unwrap().y);
    let position = Point::new(x + delta.x, y + delta.y);
    position.validate()?;
    let mut edits = vec![
        set(node, "Canvas.Left", position.x),
        set(node, "Canvas.Top", position.y),
    ];
    for name in ["Canvas.Right", "Canvas.Bottom"] {
        if element.attribute(name).is_some() {
            edits.push(Edit::RemoveAttribute {
                node,
                name: name.into(),
            });
        }
    }
    Ok(edits)
}
fn reorder_plan(
    context: &LayoutContext<'_>,
    node: NodeId,
    index: usize,
    panel_type: &str,
) -> Result<Vec<Edit>> {
    require(
        !single_content(panel_type),
        "A single-content host has no sibling order; select its inner panel",
    )?;
    let panel = context.parent(node)?;
    let container = child_container(context, panel)?;
    let mut children = authored_children(context, panel)?;
    children.retain(|id| *id != node);
    require(index <= children.len(), "Reorder index exceeds child count")?;
    if panel_type == "FlexPanel" {
        children.sort_by_key(|id| {
            context
                .property(*id, "Flex.Order")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
        });
        children.insert(index, node);
        let mut edits = vec![];
        for (index, node) in children.into_iter().enumerate() {
            ensure_literal(context, node, "Flex.Order")?;
            edits.push(text_set(node, "Flex.Order", index.to_string()));
        }
        return Ok(edits);
    }
    let raw = context
        .document
        .ast()
        .element_children(container)
        .into_iter()
        .filter(|id| *id != node)
        .collect::<Vec<_>>();
    let raw_index = if let Some(next) = children.get(index) {
        raw.iter().position(|id| id == next).unwrap_or(raw.len())
    } else {
        raw.len()
    };
    Ok(vec![Edit::MoveNode {
        node,
        parent: container,
        index: raw_index,
    }])
}
fn drop_plan(
    context: &LayoutContext<'_>,
    parent: NodeId,
    source: &str,
    position: Point,
    panel_type: &str,
) -> Result<Vec<Edit>> {
    let placement = built_in_placement(context, parent, position, panel_type, &[])?;
    if let Some(reason) = placement.rejection {
        return Err(ToolError(reason));
    }
    let attributes = placement
        .attributes
        .iter()
        .map(|(name, value)| (name.as_str(), value.clone()))
        .collect::<Vec<_>>();
    let source = position_fragment(context, parent, source, &attributes)?;
    Ok(vec![Edit::InsertChild {
        parent: placement.container,
        index: placement.index,
        source,
    }])
}

pub(crate) fn built_in_placement(
    context: &LayoutContext<'_>,
    parent: NodeId,
    position: Point,
    panel_type: &str,
    dragged: &[NodeId],
) -> Result<LayoutPlacement> {
    position.validate()?;
    let container = child_container(context, parent)?;
    let panel = context.element(parent)?;
    let mut children = authored_children(context, parent)?;
    children.retain(|id| !dragged.contains(id));
    // Source-only drop planners historically work without panel measurements for flow/content
    // hosts. Native helper scenes still require actual bounds through their generic outline.
    let bounds = context.bounds.get(&parent).copied().unwrap_or_default();
    bounds.validate()?;
    let mut preview = bounds;
    let mut description = format!("Place in {panel_type}");
    if single_content(panel_type) {
        let has_attribute = ["Content", "Child"]
            .iter()
            .any(|name| panel.attribute(name).is_some_and(|value| !value.is_empty()));
        let has_text = context
            .element(container)?
            .children
            .iter()
            .filter_map(|id| context.document.ast().node(*id))
            .any(|node| {
                matches!(node.kind, NodeKind::Text | NodeKind::CData)
                    && node
                        .text
                        .as_ref()
                        .is_some_and(|text| !text.trim().is_empty())
            });
        require(
            children.is_empty() && !has_attribute && !has_text,
            "This host already has content; select its inner panel or remove the existing content before dropping",
        )?;
    }
    if matches!(
        panel_type,
        "StackPanel" | "ReversibleStackPanel" | "WrapPanel"
    ) {
        context.ensure_resolved(parent, "Orientation")?;
        if panel_type == "ReversibleStackPanel" {
            context.ensure_resolved(parent, "ReverseOrder")?;
        }
    }
    if panel_type == "FlexPanel" {
        context.ensure_resolved(parent, "Direction")?;
        for child in &children {
            context.ensure_resolved(*child, "Flex.Order")?;
        }
    }
    if panel_type == "FlexPanel" {
        children.sort_by_key(|id| {
            context
                .property(*id, "Flex.Order")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
        });
    }
    let mut attrs = vec![];
    let mut index = children.len();
    if matches!(panel_type, "Canvas" | "Grid" | "UniformGrid" | "DockPanel") {
        let bounds = context.bounds(parent)?;
        let local = Point::new(position.x - bounds.x, position.y - bounds.y);
        match panel_type {
            "Canvas" => {
                attrs.push(("Canvas.Left", local.x.to_string()));
                attrs.push(("Canvas.Top", local.y.to_string()));
                preview = Rect::new(position.x, position.y, 32.0, 24.0);
                description = format!("Canvas · x {}  y {}", compact(local.x), compact(local.y));
            }
            "Grid" => {
                let lines = context.grid_boundaries(parent)?;
                let row = cell_at(&lines.rows, local.y);
                let column = cell_at(&lines.columns, local.x);
                let row_gap = number(context.property(parent, "RowSpacing").unwrap_or("0"))?;
                let column_gap = number(context.property(parent, "ColumnSpacing").unwrap_or("0"))?;
                // Boundaries are track starts plus the final content edge. Nonfinal
                // strides include one following gutter, which is outside the cell.
                let width = lines.columns[column + 1]
                    - lines.columns[column]
                    - if column + 2 < lines.columns.len() {
                        column_gap
                    } else {
                        0.0
                    };
                let height = lines.rows[row + 1]
                    - lines.rows[row]
                    - if row + 2 < lines.rows.len() {
                        row_gap
                    } else {
                        0.0
                    };
                preview = Rect::new(
                    bounds.x + lines.columns[column],
                    bounds.y + lines.rows[row],
                    width.max(0.0),
                    height.max(0.0),
                );
                description = format!("Grid · row {}  column {}", row + 1, column + 1);
                attrs.push(("Grid.Row", row.to_string()));
                attrs.push(("Grid.Column", column.to_string()));
            }
            "UniformGrid" => {
                require(
                    bounds.width > 0.0 && bounds.height > 0.0,
                    "UniformGrid must have positive measured size",
                )?;
                let mut visible = Vec::new();
                for child in children {
                    if context.visible(child)? {
                        visible.push(child);
                    }
                }
                children = visible;
                let mut first = integer(context.property(parent, "FirstColumn").unwrap_or("0"))?;
                let mut rows = integer(context.property(parent, "Rows").unwrap_or("0"))?;
                let mut columns = integer(context.property(parent, "Columns").unwrap_or("0"))?;
                // Runtime resets FirstColumn before calculating automatic dimensions.
                if first >= columns {
                    first = 0;
                }
                let count = children
                    .len()
                    .checked_add(1)
                    .and_then(|n| n.checked_add(first))
                    .ok_or_else(|| ToolError("UniformGrid item count overflow".into()))?;
                if rows == 0 && columns == 0 {
                    columns = (count as f64).sqrt().ceil() as usize;
                    rows = columns;
                } else if columns == 0 {
                    columns = count.div_ceil(rows);
                } else if rows == 0 {
                    rows = count.div_ceil(columns);
                }
                let column = uniform_cell_at(
                    local.x,
                    bounds.width,
                    columns,
                    number(context.property(parent, "ColumnSpacing").unwrap_or("0"))?,
                )?;
                let row = uniform_cell_at(
                    local.y,
                    bounds.height,
                    rows,
                    number(context.property(parent, "RowSpacing").unwrap_or("0"))?,
                )?;
                let gap_x = number(context.property(parent, "ColumnSpacing").unwrap_or("0"))?;
                let gap_y = number(context.property(parent, "RowSpacing").unwrap_or("0"))?;
                let cell_width = ((bounds.width - columns.saturating_sub(1) as f64 * gap_x)
                    / columns.max(1) as f64)
                    .max(0.0);
                let cell_height = ((bounds.height - rows.saturating_sub(1) as f64 * gap_y)
                    / rows.max(1) as f64)
                    .max(0.0);
                index = row
                    .min(rows.saturating_sub(1))
                    .saturating_mul(columns)
                    .saturating_add(column.min(columns.saturating_sub(1)))
                    .saturating_sub(first)
                    .min(children.len());
                // Empty cells beyond the current content append to the next available
                // slot; reserved leading cells insert at FirstColumn. Preview the exact
                // effective insertion slot rather than the unfillable hovered cell.
                let slot = index + first;
                let row = slot / columns.max(1);
                let column = slot % columns.max(1);
                preview = Rect::new(
                    bounds.x + column as f64 * (cell_width + gap_x),
                    bounds.y + row as f64 * (cell_height + gap_y),
                    cell_width,
                    cell_height,
                );
                description = format!("Uniform grid · row {}  column {}", row + 1, column + 1);
            }
            "DockPanel" => {
                let sides = [
                    (local.x.abs(), DockSide::Left),
                    (local.y.abs(), DockSide::Top),
                    ((bounds.width - local.x).abs(), DockSide::Right),
                    ((bounds.height - local.y).abs(), DockSide::Bottom),
                ];
                let side = sides.iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap().1;
                attrs.push(("DockPanel.Dock", format!("{side:?}")));
                preview = match side {
                    DockSide::Left => {
                        Rect::new(bounds.x, bounds.y, bounds.width * 0.25, bounds.height)
                    }
                    DockSide::Right => Rect::new(
                        bounds.x + bounds.width * 0.75,
                        bounds.y,
                        bounds.width * 0.25,
                        bounds.height,
                    ),
                    DockSide::Top => {
                        Rect::new(bounds.x, bounds.y, bounds.width, bounds.height * 0.25)
                    }
                    DockSide::Bottom => Rect::new(
                        bounds.x,
                        bounds.y + bounds.height * 0.75,
                        bounds.width,
                        bounds.height * 0.25,
                    ),
                };
                description = format!("Dock · {side:?}");
                if context
                    .property(parent, "LastChildFill")
                    .is_none_or(|v| !v.eq_ignore_ascii_case("False"))
                    && !children.is_empty()
                {
                    index = children.len() - 1;
                }
            }
            _ => {}
        }
    } else if matches!(
        panel_type,
        "StackPanel" | "ReversibleStackPanel" | "WrapPanel" | "FlexPanel"
    ) {
        let horizontal = if panel_type == "FlexPanel" {
            context
                .property(parent, "Direction")
                .unwrap_or("Row")
                .starts_with("Row")
        } else {
            context
                .property(parent, "Orientation")
                .unwrap_or(if panel_type == "WrapPanel" {
                    "Horizontal"
                } else {
                    "Vertical"
                })
                == "Horizontal"
        };
        let reversed = if panel_type == "FlexPanel" {
            context
                .property(parent, "Direction")
                .is_some_and(|v| v.ends_with("Reverse"))
        } else {
            panel_type == "ReversibleStackPanel"
                && context
                    .property(parent, "ReverseOrder")
                    .is_some_and(|v| v.eq_ignore_ascii_case("True"))
        };
        let mut distance = f64::INFINITY;
        for (i, child) in children.iter().enumerate() {
            if let Some(rect) = context.bounds.get(child) {
                rect.validate()?;
                let c = rect.center();
                let d = (c.x - position.x).hypot(c.y - position.y);
                if d < distance {
                    distance = d;
                    let after = if horizontal {
                        position.x > c.x
                    } else {
                        position.y > c.y
                    };
                    index = i + usize::from(after != reversed);
                    preview = if horizontal {
                        Rect::new(
                            if after { rect.right() } else { rect.x },
                            rect.y,
                            0.0,
                            rect.height,
                        )
                    } else {
                        Rect::new(
                            rect.x,
                            if after { rect.bottom() } else { rect.y },
                            rect.width,
                            0.0,
                        )
                    };
                    description = format!(
                        "Insert {} item {}",
                        if after { "after" } else { "before" },
                        i + 1
                    );
                }
            }
        }
        if panel_type == "FlexPanel" {
            let order = children
                .get(index)
                .or_else(|| children.last())
                .and_then(|child| context.property(*child, "Flex.Order"))
                .unwrap_or("0");
            let order = order
                .parse::<i64>()
                .map_err(|_| ToolError("Resolved Flex.Order must be an integer".into()))?;
            // Equal-order items retain source order. Inserting next to the chosen neighbor
            // needs no mutation of existing order bindings or style values.
            attrs.push(("Flex.Order", order.to_string()));
        }
    }
    let raw = context.document.ast().element_children(container);
    let raw_index = children
        .get(index)
        .and_then(|next| raw.iter().position(|id| id == next))
        .unwrap_or(raw.len());
    Ok(LayoutPlacement {
        container,
        index: raw_index,
        attributes: attrs
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
        bounds: preview,
        description,
        rejection: None,
    })
}

fn uniform_cell_at(position: f64, extent: f64, count: usize, spacing: f64) -> Result<usize> {
    require(spacing >= 0.0, "UniformGrid spacing must be nonnegative")?;
    let gaps = count.saturating_sub(1) as f64 * spacing;
    require(gaps.is_finite(), "UniformGrid spacing overflow")?;
    let unit = ((extent - gaps) / count.max(1) as f64).max(0.0) + spacing;
    Ok(if unit > 0.0 {
        (position / unit).floor().max(0.0) as usize
    } else {
        0
    })
}
fn position_fragment(
    context: &LayoutContext<'_>,
    parent: NodeId,
    source: &str,
    attributes: &[(&str, String)],
) -> Result<String> {
    let declarations = context
        .document
        .ast()
        .namespaces(parent)
        .into_iter()
        .map(|(prefix, uri)| {
            let name = if prefix.is_empty() {
                "xmlns".to_owned()
            } else {
                format!("xmlns:{prefix}")
            };
            format!(" {name}=\"{}\"", escape(&uri))
        })
        .collect::<String>();
    let mut fragment = Document::new(format!(
        "<DesignerFragment{declarations}>{source}</DesignerFragment>"
    ))
    .map_err(error)?;
    let wrapper = fragment.ast().root();
    let elements = fragment.ast().element_children(wrapper);
    require(elements.len() == 1, "Drop one control subtree at a time")?;
    let root = elements[0];
    let values = attributes
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect::<Vec<_>>();
    let edits = crate::plan_layout_attributes(&fragment, root, &values)?;
    fragment
        .apply("Position dropped control", edits)
        .map_err(error)?;
    let wrapper = fragment.ast().node(wrapper).unwrap();
    let end = wrapper
        .closing_span
        .ok_or_else(|| ToolError("Invalid fragment wrapper".into()))?
        .start;
    Ok(fragment.source()[wrapper.opening_span.end..end].to_owned())
}
fn grid_track_plan(
    context: &LayoutContext<'_>,
    panel: NodeId,
    axis: Axis,
    index: usize,
    length: GridLength,
) -> Result<Vec<Edit>> {
    length.validate()?;
    let (property, item, value) = match axis {
        Axis::Horizontal => ("ColumnDefinitions", "ColumnDefinition", "Width"),
        Axis::Vertical => ("RowDefinitions", "RowDefinition", "Height"),
    };
    let element = context.element(panel)?;
    ensure_literal_attribute(element, property)?;
    if let Some(collection) = element
        .children
        .iter()
        .filter_map(|id| context.document.ast().node(*id))
        .find(|node| node.name.ends_with(&format!(".{property}")))
    {
        let definitions = collection
            .children
            .iter()
            .filter_map(|id| context.document.ast().node(*id))
            .filter(|n| local_name(&n.name) == item)
            .collect::<Vec<_>>();
        let target = definitions
            .get(index)
            .ok_or_else(|| ToolError("Grid track index out of range".into()))?;
        ensure_literal(context, target.id, value)?;
        return Ok(vec![text_set(target.id, value, length.to_xaml())]);
    }
    let mut definitions = grid_definitions(context, panel, axis)?;
    let target = definitions
        .get_mut(index)
        .ok_or_else(|| ToolError("Grid track index out of range".into()))?;
    *target = length;
    Ok(vec![text_set(
        panel,
        property,
        definitions
            .iter()
            .map(|v| v.to_xaml())
            .collect::<Vec<_>>()
            .join(","),
    )])
}
pub(crate) fn grid_definitions(
    context: &LayoutContext<'_>,
    panel: NodeId,
    axis: Axis,
) -> Result<Vec<GridLength>> {
    let (property, item, value) = match axis {
        Axis::Horizontal => ("ColumnDefinitions", "ColumnDefinition", "Width"),
        Axis::Vertical => ("RowDefinitions", "RowDefinition", "Height"),
    };
    let element = context.element(panel)?;
    if let Some(source) = element.attribute(property) {
        return source.split(',').map(GridLength::parse).collect();
    }
    if let Some(collection) = element
        .children
        .iter()
        .filter_map(|id| context.document.ast().node(*id))
        .find(|node| node.name.ends_with(&format!(".{property}")))
    {
        let values = collection
            .children
            .iter()
            .filter_map(|id| context.document.ast().node(*id))
            .filter(|node| local_name(&node.name) == item)
            .map(|node| GridLength::parse(node.attribute(value).unwrap_or("*")))
            .collect::<Result<Vec<_>>>()?;
        if !values.is_empty() {
            return Ok(values);
        }
    }
    Ok(vec![GridLength::Star(1.0)])
}
fn require_plain_grid(context: &LayoutContext<'_>, panel: NodeId) -> Result<()> {
    let node = context.element(panel)?;
    for property in ["RowSpacing", "ColumnSpacing"] {
        if let Some(value) = node.attribute(property) {
            require(
                number(value)? == 0.0,
                "Grid spacing requires measured grid lines from the live layout",
            )?;
        }
    }
    for child in node
        .children
        .iter()
        .filter_map(|id| context.document.ast().node(*id))
    {
        if !child.name.ends_with(".RowDefinitions") && !child.name.ends_with(".ColumnDefinitions") {
            continue;
        }
        for definition in child
            .children
            .iter()
            .filter_map(|id| context.document.ast().node(*id))
        {
            require(
                !definition.attributes.iter().any(|attribute| {
                    matches!(
                        attribute.name.as_str(),
                        "MinWidth" | "MaxWidth" | "MinHeight" | "MaxHeight" | "SharedSizeGroup"
                    )
                }),
                "Constrained Grid tracks require measured grid lines from the live layout",
            )?;
        }
    }
    Ok(())
}
fn resolved_lines(lengths: &[GridLength], extent: f64) -> Result<Vec<f64>> {
    require(
        !lengths.iter().any(|v| matches!(v, GridLength::Auto)),
        "Auto Grid tracks need measured grid lines from the live layout",
    )?;
    let fixed = lengths
        .iter()
        .filter_map(|v| {
            if let GridLength::Pixels(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .sum::<f64>();
    let weights = lengths
        .iter()
        .filter_map(|v| {
            if let GridLength::Star(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .sum::<f64>();
    require(
        fixed.is_finite() && weights.is_finite(),
        "Grid lengths overflow",
    )?;
    let remaining = (extent - fixed).max(0.0);
    let mut lines = vec![0.0];
    for length in lengths {
        let size = match length {
            GridLength::Pixels(n) => *n,
            GridLength::Star(n) => {
                if weights > 0.0 {
                    remaining * (*n / weights)
                } else {
                    0.0
                }
            }
            GridLength::Auto => unreachable!(),
        };
        let end = lines.last().unwrap() + size;
        require(end.is_finite(), "Grid extent overflow")?;
        lines.push(end);
    }
    Ok(lines)
}
fn cell_at(lines: &[f64], position: f64) -> usize {
    lines
        .partition_point(|boundary| *boundary <= position)
        .saturating_sub(1)
        .min(lines.len().saturating_sub(2))
}
fn relative_plan(
    context: &LayoutContext<'_>,
    node: NodeId,
    relation: RelativeRelation,
    target: Option<NodeId>,
) -> Result<Vec<Edit>> {
    let property = relation.property();
    ensure_literal(context, node, property)?;
    let Some(target) = target else {
        return Ok(vec![Edit::RemoveAttribute {
            node,
            name: property.into(),
        }]);
    };
    require(
        node != target && context.parent(node)? == context.parent(target)?,
        "Relative constraints must reference another child of the same panel",
    )?;
    let element = context.element(target)?;
    let name = element
        .xaml_name()
        .ok_or_else(|| ToolError("Give the relative target an x:Name or Name first".into()))?;
    let siblings = authored_children(context, context.parent(node)?)?;
    let names = siblings
        .iter()
        .filter_map(|id| {
            context
                .document
                .ast()
                .node(*id)
                .and_then(|n| n.xaml_name())
                .map(|name| (name.to_owned(), *id))
        })
        .collect::<BTreeMap<_, _>>();
    let mut edges: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
    for id in siblings {
        let element = context.element(id)?;
        for attribute in &element.attributes {
            if !attribute.name.starts_with("RelativePanel.")
                || attribute.name.ends_with("WithPanel")
                || (id == node && attribute.name == property)
            {
                continue;
            }
            require(
                !attribute.value.trim_start().starts_with('{'),
                "Resolve dynamic relative constraints in the live host before editing the static constraint graph",
            )?;
            if let Some(other) = names.get(&attribute.value) {
                edges.entry(id).or_default().push(*other);
            }
        }
    }
    edges.entry(node).or_default().push(target);
    let mut visited = BTreeSet::new();
    let mut pending = vec![target];
    while let Some(next) = pending.pop() {
        require(next != node, "Relative constraint would introduce a cycle")?;
        if visited.insert(next) {
            pending.extend(edges.get(&next).into_iter().flatten().copied());
        }
    }
    Ok(vec![text_set(node, property, name.to_owned())])
}
pub(crate) fn child_container(context: &LayoutContext<'_>, panel: NodeId) -> Result<NodeId> {
    let element = context.element(panel)?;
    Ok(element
        .children
        .iter()
        .filter_map(|id| context.document.ast().node(*id))
        .find(|node| {
            node.kind == NodeKind::PropertyElement
                && if single_content(local_name(&element.name)) {
                    node.name.ends_with(".Content") || node.name.ends_with(".Child")
                } else {
                    node.name.ends_with(".Children")
                }
        })
        .map_or(panel, |n| n.id))
}
pub(crate) fn authored_children(context: &LayoutContext<'_>, panel: NodeId) -> Result<Vec<NodeId>> {
    Ok(context
        .document
        .ast()
        .element_children(child_container(context, panel)?)
        .into_iter()
        .filter(|id| {
            context
                .document
                .ast()
                .node(*id)
                .is_some_and(|node| node.kind != NodeKind::PropertyElement)
        })
        .collect())
}
pub(crate) fn ensure_literal_attribute(node: &Node, property: &str) -> Result<()> {
    require(
        !node
            .attribute(property)
            .is_some_and(|v| v.trim_start().starts_with('{') && !v.trim_start().starts_with("{}")),
        format!("{property} is an expression; edit it explicitly instead of overwriting it"),
    )
}
pub(crate) fn ensure_literal(
    context: &LayoutContext<'_>,
    node: NodeId,
    property: &str,
) -> Result<()> {
    crate::layout_attributes::ensure_layout_literal(
        context.document,
        context.element(node)?,
        property,
    )
}

pub(crate) fn thickness(source: &str) -> Result<[f64; 4]> {
    let values = source
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|p| !p.is_empty())
        .map(number)
        .collect::<Result<Vec<_>>>()?;
    match values.as_slice() {
        [all] => Ok([*all; 4]),
        [x, y] => Ok([*x, *y, *x, *y]),
        [l, t, r, b] => Ok([*l, *t, *r, *b]),
        _ => Err(ToolError(
            "Thickness requires one, two or four finite components".into(),
        )),
    }
}
pub(crate) fn number(source: &str) -> Result<f64> {
    let number = source
        .trim()
        .parse::<f64>()
        .map_err(|_| ToolError(format!("Expected a finite number: {source}")))?;
    require(number.is_finite(), "Numeric values must be finite")?;
    Ok(number)
}
pub(crate) fn integer(source: &str) -> Result<usize> {
    source
        .parse()
        .map_err(|_| ToolError("Expected a nonnegative integer".into()))
}
fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}
fn text_set(node: NodeId, name: &str, value: String) -> Edit {
    Edit::SetAttribute {
        node,
        name: name.into(),
        value,
    }
}
fn set(node: NodeId, name: &str, value: f64) -> Edit {
    text_set(node, name, value.to_string())
}
fn escape(source: &str) -> String {
    source
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

pub(crate) fn single_content(name: &str) -> bool {
    matches!(
        name,
        "Border"
            | "Decorator"
            | "ContentControl"
            | "UserControl"
            | "Window"
            | "ScrollViewer"
            | "Viewbox"
            | "Button"
            | "ToggleButton"
    )
}

fn compact(value: f64) -> String {
    crate::layout_helpers::compact(value)
}
