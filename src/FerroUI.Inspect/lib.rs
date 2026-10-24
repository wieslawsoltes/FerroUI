//! Transport-independent inspection of live FerroUI objects, shared by the MCP server
//! (`ferroui-mcp`, the JSON tools of [`execute`]) and the Chrome DevTools Protocol endpoint
//! (`ferroui-devtools`, the per-window [`target::Target`]). Both use one [`Inspector`] per UI
//! thread ([`Inspector::shared`]): one ID space, one set of roots and the same primitives for
//! properties, styles, selectors, layout boxes, screenshots and highlights. All methods run on the
//! UI thread. IDs are weak, stable for an object's lifetime, never reused, and start at 2 (0 and 1
//! are left to protocols, such as CDP's document node).
mod frames;
mod input;
mod logs;
mod schema;
pub mod target;
use base64::Engine;
use ferroui_base::controls::{IResourceHost, ResourceKey};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingOperations, BindingPriority};
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::media::{Colors, SolidColorBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{ControlTheme, Selector, Style};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, PixelSize, Rect, Ref,
    StyledElement, Thickness, TypeInfo, Vector, Visual, WeakRef,
};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::primitives::{AdornerLayer, TemplatedControl};
use ferroui_controls::{Application, Border, Control, TopLevel};
use ferroui_markup::markup::parsers::{SelectorGrammar, SelectorParser, SelectorSyntax};
pub use logs::{LogCapture, LogEntry, LogGuard};
pub use schema::{tools, validate};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Mutating operations are disabled unless explicitly enabled by the application.
#[derive(Clone, Copy, Debug, Default)]
pub struct Permissions {
    pub allow_property_writes: bool,
    pub allow_input: bool,
}

pub type InspectResult = Result<Value, String>;
pub type NodeId = u64;

/// The tree that children are read from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeKind {
    #[default]
    Visual,
    Logical,
    Automation,
}
impl TreeKind {
    fn parse(name: &str) -> Self {
        match name {
            "logical" => Self::Logical,
            "automation" => Self::Automation,
            _ => Self::Visual,
        }
    }
}

/// A registered or attached property of an object, its effective value and where it comes from.
#[derive(Clone)]
pub struct PropertyInfo {
    pub property: &'static FerroProperty,
    /// `Owner.Name` for an attached property.
    pub name: String,
    pub value: BoxedValue,
    /// `None` for a direct property, which has no priority.
    pub priority: Option<BindingPriority>,
    pub read_only: bool,
    pub has_binding: bool,
    pub overridden_current_value: bool,
}
impl PropertyInfo {
    pub fn priority_name(&self) -> String {
        self.priority
            .map_or_else(|| "Direct".into(), |priority| format!("{priority:?}"))
    }
    pub fn is_local(&self) -> bool {
        self.priority == Some(BindingPriority::LocalValue)
    }
}

/// A frame of the value store that applies to an object: a style, a control theme, a template or
/// the local values.
#[derive(Clone)]
pub struct StyleFrame {
    pub kind: String,
    /// The selector of a style, or `Theme <type>` of a control theme.
    pub selector: Option<String>,
    pub active: bool,
    pub priority: String,
    pub setters: Vec<(String, Option<BoxedValue>)>,
}

/// The layout boxes of a visual, in the coordinates of another one.
#[derive(Clone, Debug)]
pub struct BoxModel {
    pub content: Rect,
    pub padding: Rect,
    pub border: Rect,
    pub margin: Rect,
}

pub struct Png {
    pub data: Vec<u8>,
    pub width: f64,
    pub height: f64,
}

/// A highlight adorner of [`Inspector::highlight`]; dropping it removes the adorner.
pub struct Highlight(Rc<dyn IDisposable>);
impl Drop for Highlight {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

ferroui_base::ferro_thread_local! {
    static SHARED: RefCell<Option<Rc<RefCell<Inspector>>>> = const { RefCell::new(None) };
}

pub struct Inspector {
    next_id: NodeId,
    nodes: HashMap<NodeId, WeakRef<FerroObject>>,
    addresses: HashMap<usize, NodeId>,
    roots: Vec<WeakRef<TopLevel>>,
    frames: HashMap<NodeId, frames::FrameStats>,
    overlays: Vec<WeakRef<Visual>>,
}

impl Default for Inspector {
    fn default() -> Self {
        Self::new()
    }
}

impl Inspector {
    pub fn new() -> Self {
        Dispatcher::ui_thread().verify_access();
        Self {
            next_id: 2,
            nodes: HashMap::new(),
            addresses: HashMap::new(),
            roots: Vec::new(),
            frames: HashMap::new(),
            overlays: Vec::new(),
        }
    }

    /// The inspector of this UI thread, shared by every server that runs on it.
    pub fn shared() -> Rc<RefCell<Inspector>> {
        SHARED.with(|shared| {
            shared
                .borrow_mut()
                .get_or_insert_with(|| Rc::new(RefCell::new(Inspector::new())))
                .clone()
        })
    }

    /// Add a top level for a custom lifetime or tests. Desktop windows are discovered automatically.
    pub fn register_root(&mut self, root: &TopLevel) {
        if !self.roots.iter().any(|weak| weak.points_to(&root.to_ref())) {
            self.roots.push(root.to_ref().downgrade());
        }
    }

    pub fn node_id(&mut self, object: &FerroObject) -> NodeId {
        let address = object as *const FerroObject as usize;
        if let Some(id) = self.addresses.get(&address) {
            if self
                .nodes
                .get(id)
                .and_then(WeakRef::upgrade)
                .is_some_and(|old| old == object.to_ref())
            {
                return *id;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, object.to_ref().downgrade());
        self.addresses.insert(address, id);
        id
    }

    pub fn resolve(&self, id: NodeId) -> Result<Ref<FerroObject>, String> {
        self.nodes
            .get(&id)
            .and_then(WeakRef::upgrade)
            .ok_or_else(|| format!("Unknown or expired node_id: {id}"))
    }

    fn prune(&mut self) {
        self.nodes.retain(|_, weak| weak.upgrade().is_some());
        self.addresses.retain(|_, id| self.nodes.contains_key(id));
        self.overlays.retain(|weak| weak.upgrade().is_some());
    }

    /// The open top levels: the registered roots and the windows of the desktop lifetime.
    pub fn windows(&mut self) -> Vec<Ref<TopLevel>> {
        self.prune();
        self.roots.retain(|weak| weak.upgrade().is_some());
        let mut roots: Vec<_> = self
            .roots
            .iter()
            .filter_map(WeakRef::upgrade)
            .filter(|r| r.platform_impl().is_some())
            .collect();
        if let Some(lifetime) = Application::current().and_then(|a| a.application_lifetime()) {
            if let Some(desktop) = lifetime.as_classic_desktop_style_application_lifetime() {
                for window in desktop.windows() {
                    let root = window.upcast::<TopLevel>();
                    if !roots.contains(&root) {
                        roots.push(root);
                    }
                }
            }
        }
        for root in &roots {
            let id = self.node_id(root);
            self.frames
                .entry(id)
                .or_insert_with(|| frames::FrameStats::new(root));
        }
        let live: HashSet<_> = roots.iter().map(|r| self.node_id(r)).collect();
        self.frames.retain(|id, _| live.contains(id));
        roots
    }

    fn roots(&mut self, args: &Value) -> Result<Vec<Ref<FerroObject>>, String> {
        if let Some(id) = args["node_id"].as_u64() {
            Ok(vec![self.resolve(id)?])
        } else {
            Ok(self.windows().into_iter().map(Ref::upcast).collect())
        }
    }

    fn object(&self, args: &Value) -> Result<Ref<FerroObject>, String> {
        self.resolve(args["node_id"].as_u64().ok_or("node_id required")?)
    }

    /// The control that owns an automation peer, or the object itself.
    pub fn visual_object(object: &Ref<FerroObject>) -> Ref<FerroObject> {
        object
            .cast::<ControlAutomationPeer>()
            .and_then(|p| p.try_owner())
            .map(Ref::upcast)
            .unwrap_or_else(|| object.clone())
    }

    /// Whether the object is a highlight adorner of this inspector, which trees leave out.
    pub fn is_overlay(&self, object: &FerroObject) -> bool {
        self.overlays
            .iter()
            .filter_map(WeakRef::upgrade)
            .any(|overlay| overlay.upcast::<FerroObject>() == object.to_ref())
    }

    /// The children of the object in the tree, without highlight adorners.
    pub fn children(&self, object: &Ref<FerroObject>, tree: TreeKind) -> Vec<Ref<FerroObject>> {
        let children: Vec<Ref<FerroObject>> = match tree {
            TreeKind::Automation => Self::peer(object)
                .map(|p| p.get_children().iter().cloned().map(Ref::upcast).collect())
                .unwrap_or_default(),
            TreeKind::Logical => object
                .cast::<StyledElement>()
                .map(|o| {
                    o.logical_children()
                        .to_vec()
                        .into_iter()
                        .map(Ref::upcast)
                        .collect()
                })
                .unwrap_or_default(),
            TreeKind::Visual => object
                .cast::<Visual>()
                .map(|o| {
                    o.visual_children()
                        .to_vec()
                        .into_iter()
                        .map(Ref::upcast)
                        .collect()
                })
                .unwrap_or_default(),
        };
        if self.overlays.is_empty() {
            return children;
        }
        children
            .into_iter()
            .filter(|child| !self.is_overlay(child))
            .collect()
    }

    pub fn peer(object: &Ref<FerroObject>) -> Result<Ref<AutomationPeer>, String> {
        object
            .cast::<AutomationPeer>()
            .or_else(|| {
                object
                    .cast::<Control>()
                    .map(|c| ControlAutomationPeer::create_peer_for_element(&c))
            })
            .ok_or("Node has no automation peer".into())
    }

    fn registered(object: &FerroObject) -> Vec<&'static FerroProperty> {
        let registry = FerroPropertyRegistry::instance();
        let mut result = registry.get_registered_for(object).to_vec();
        result.extend(
            registry
                .get_registered_attached(object.get_type())
                .iter()
                .copied(),
        );
        result.sort_by_key(|p| property_name(p));
        result.dedup_by_key(|p| *p as *const FerroProperty);
        result
    }

    /// A registered or attached (`Owner.Name`) property of the object.
    pub fn property(object: &FerroObject, name: &str) -> Result<&'static FerroProperty, String> {
        Self::registered(object)
            .into_iter()
            .find(|p| property_name(p) == name)
            .ok_or_else(|| format!("Unknown property: {name}"))
    }

    /// The registered and attached properties of the object, sorted by name.
    pub fn properties(object: &FerroObject) -> Vec<PropertyInfo> {
        Self::registered(object)
            .into_iter()
            .map(|p| {
                let diagnostic = object.get_diagnostic(p);
                PropertyInfo {
                    property: p,
                    name: property_name(p),
                    value: diagnostic.value().clone(),
                    priority: (!p.is_direct()).then(|| diagnostic.priority()),
                    read_only: p.is_read_only(),
                    has_binding: BindingOperations::get_binding_expression_base(object, p)
                        .is_some(),
                    overridden_current_value: diagnostic.is_overridden_current_value(),
                }
            })
            .collect()
    }

    /// Sets a local value: `None` sets null, text is converted with the framework's converters.
    /// Read-only properties, values that do not convert and values that fail validation are
    /// refused before the object is changed.
    pub fn set_property(
        object: &FerroObject,
        name: &str,
        value: Option<BoxedValue>,
    ) -> Result<&'static FerroProperty, String> {
        let property = Self::property(object, name)?;
        if property.is_read_only() {
            return Err(format!("{name} is read-only"));
        }
        let converted = ValueTypes::try_convert(
            value.as_ref(),
            ValueType::new(property.property_type(), property.property_type_name()),
        )
        .flatten()
        .ok_or_else(|| format!("Cannot convert the value to the type of {name}"))?;
        if !property.is_valid_value(converted.as_any()) {
            return Err(format!("Validation of {name} rejected the value"));
        }
        object.set_value_untyped(property, converted.as_any(), BindingPriority::LocalValue);
        Ok(property)
    }

    pub fn clear_property(
        object: &FerroObject,
        name: &str,
    ) -> Result<&'static FerroProperty, String> {
        let property = Self::property(object, name)?;
        if property.is_read_only() {
            return Err(format!("{name} is read-only"));
        }
        object.clear_value_untyped(property);
        Ok(property)
    }

    /// The frames of the value store that apply to the object, active or not.
    pub fn styles(object: &FerroObject) -> Vec<StyleFrame> {
        object
            .get_value_store_diagnostic()
            .applied_frames()
            .iter()
            .map(|frame| {
                let source = frame.source().and_then(|s| ValueTypes::as_object(&*s));
                let selector = source
                    .as_ref()
                    .and_then(|s| s.cast::<Style>())
                    .and_then(|s| s.selector())
                    .map(|s| s.to_string_with_owner(None))
                    .or_else(|| {
                        source.and_then(|s| s.cast::<ControlTheme>()).map(|t| {
                            format!(
                                "Theme {}",
                                t.target_type().map(TypeInfo::name).unwrap_or_default()
                            )
                        })
                    });
                StyleFrame {
                    kind: format!("{:?}", frame.type_()),
                    selector,
                    active: frame.is_active(),
                    priority: format!("{:?}", frame.priority()),
                    setters: frame
                        .values()
                        .into_iter()
                        .map(|v| (property_name(v.property), v.value))
                        .collect(),
                }
            })
            .collect()
    }

    /// Parses a selector of the framework's syntax. Type names resolve among the known types and
    /// the types of the live nodes. Forms that the selector constructors or the matcher do not
    /// accept outside a style (nesting, a dangling combinator) are refused here.
    pub fn parse_selector(&self, text: &str) -> Result<Option<Selector>, String> {
        if text.trim().is_empty() || text.len() > 4096 {
            return Err("Selector must contain 1..4096 bytes".into());
        }
        let mut nesting = 0usize;
        for character in text.chars() {
            if character == '(' {
                nesting += 1;
            }
            if nesting > 32 {
                return Err("Selector nesting is too deep".into());
            }
            if character == ')' {
                nesting = nesting.saturating_sub(1);
            }
        }
        validate_selector(&SelectorGrammar::parse(text).map_err(|e| e.to_string())?)?;
        let mut live: HashMap<String, &'static TypeInfo> = HashMap::new();
        for object in self.nodes.values().filter_map(WeakRef::upgrade) {
            let mut type_ = Some(object.get_type());
            while let Some(t) = type_ {
                live.insert(t.name().into(), t);
                type_ = t.base_type();
            }
        }
        SelectorParser::new(move |namespace, name| {
            if namespace.is_empty() {
                TypeInfo::find_by_name(name).or_else(|| live.get(name).copied())
            } else {
                TypeInfo::find(namespace, name)
            }
        })
        .parse(text)
        .map_err(|e| e.to_string())
    }

    /// The content, padding, border and margin boxes of the visual, in the coordinates of
    /// `relative_to`.
    pub fn box_model(visual: &Ref<Visual>, relative_to: &Ref<Visual>) -> Result<BoxModel, String> {
        let transform = visual
            .transform_to_visual(relative_to)
            .ok_or("Node has no transform to the root")?;
        let border = Rect::from_size(visual.bounds().size());
        let margin = visual.get_value(Layoutable::margin_property());
        let padding = visual.get_value(TemplatedControl::padding_property());
        let border_thickness = visual
            .cast::<Border>()
            .map(|b| b.border_thickness())
            .or_else(|| {
                visual
                    .cast::<TemplatedControl>()
                    .map(|c| c.border_thickness())
            })
            .unwrap_or_default();
        let padding_rect = border.deflate_thickness(border_thickness);
        Ok(BoxModel {
            content: padding_rect
                .deflate_thickness(padding)
                .transform_to_aabb(transform),
            padding: padding_rect.transform_to_aabb(transform),
            border: border.transform_to_aabb(transform),
            margin: border
                .inflate_thickness(margin)
                .transform_to_aabb(transform),
        })
    }

    /// A PNG of the visual at the scale (0.1 to 4), at most 16 megapixels.
    pub fn screenshot(visual: &Ref<Visual>, scale: f64) -> Result<Png, String> {
        let size = visual.bounds().size();
        let width = (size.width * scale).ceil();
        let height = (size.height * scale).ceil();
        if !width.is_finite()
            || !height.is_finite()
            || width < 1.0
            || height < 1.0
            || width * height > 16_000_000.0
        {
            return Err("Screenshot size is empty or exceeds 16 megapixels".into());
        }
        let bitmap = RenderTargetBitmap::with_dpi(
            PixelSize::new(width as i32, height as i32),
            Vector::new(96.0 * scale, 96.0 * scale),
        );
        bitmap.render(visual);
        let mut data = Vec::new();
        let result = bitmap.save(&mut data, &PngBitmapEncoderOptions::DEFAULT.into());
        bitmap.dispose();
        result.map_err(|e| e.to_string())?;
        Ok(Png {
            data,
            width,
            height,
        })
    }

    /// Outlines the visual on its adorner layer until the returned guard is dropped. The adorner
    /// is left out of the trees of every client of this inspector.
    pub fn highlight(&mut self, visual: &Ref<Visual>) -> Result<Highlight, String> {
        let layer = AdornerLayer::get_adorner_layer(visual).ok_or("Node has no adorner layer")?;
        let border = Border::new();
        border.set_is_hit_test_visible(false);
        border.set_border_thickness(Thickness::uniform(2.0));
        border.set_border_brush(Some(
            SolidColorBrush::with_color(Colors::DODGER_BLUE).into(),
        ));
        AdornerLayer::set_adorned_element(&border, visual.clone());
        self.overlays
            .push(border.clone().upcast::<Visual>().downgrade());
        layer.children().add(border.clone());
        Ok(Highlight(Disposable::create(move || {
            layer.children().remove(&border);
        })))
    }

    fn describe(&mut self, object: &Ref<FerroObject>, properties: Option<&Value>) -> Value {
        let id = self.node_id(object);
        let visual_object = Self::visual_object(object);
        let mut result = json!({"node_id":id,"type":visual_object.get_type().name()});
        if let Some(styled) = visual_object.cast::<StyledElement>() {
            result["name"] = json!(styled.name());
        }
        if let Some(visual) = visual_object.cast::<Visual>() {
            result["bounds"] = rect(visual.bounds());
            result["visible"] = json!(visual.is_visible());
            if let Some(root) = visual.visual_root() {
                if let Some(transform) = visual.transform_to_visual(&root) {
                    result["bounds_in_root"] =
                        rect(Rect::from_size(visual.bounds().size()).transform_to_aabb(transform));
                    result["transform_to_root"] = json!(format!("{transform:?}"));
                }
            }
        }
        if let Some(layout) = visual_object.cast::<Layoutable>() {
            result["desired_size"] =
                json!({"width":layout.desired_size().width,"height":layout.desired_size().height});
        }
        if let Ok(peer) = Self::peer(object) {
            result["automation_id"] = json!(peer.get_automation_id());
            result["text"] = json!(peer.get_name());
            result["role"] = json!(format!("{:?}", peer.get_automation_control_type()));
            result["enabled"] = json!(peer.is_enabled());
        }
        if let Some(properties) = properties {
            result["properties"] = properties_json(&visual_object, Some(properties));
        }
        result
    }

    fn tree(
        &mut self,
        object: Ref<FerroObject>,
        tree: TreeKind,
        depth: u64,
        properties: Option<&Value>,
        visited: &mut HashSet<NodeId>,
    ) -> Value {
        let object = if tree == TreeKind::Automation {
            Self::peer(&object).map(Ref::upcast).unwrap_or(object)
        } else {
            object
        };
        let id = self.node_id(&object);
        if visited.len() >= 10000 || !visited.insert(id) {
            return json!({"node_id":id,"truncated":true});
        }
        let mut result = self.describe(&object, properties);
        let children = self.children(&object, tree);
        result["child_count"] = json!(children.len());
        if depth > 0 {
            let mut snapshots = Vec::new();
            for child in children {
                if visited.len() >= 10000 {
                    result["truncated"] = json!(true);
                    break;
                }
                snapshots.push(self.tree(child, tree, depth - 1, properties, visited));
            }
            result["children"] = Value::Array(snapshots);
        } else if !children.is_empty() {
            result["truncated"] = json!(true);
        }
        result
    }

    /// The matches of the filters of `find_elements`, and whether the search stopped at the node
    /// limit before it covered the tree.
    fn find(&mut self, args: &Value) -> Result<(Vec<Ref<FerroObject>>, bool), String> {
        let selector = args["selector"]
            .as_str()
            .map(|s| self.parse_selector(s))
            .transpose()?
            .flatten();
        let mut stack = self.roots(args)?;
        stack.reverse();
        let mut visited = HashSet::new();
        let mut found = Vec::new();
        let limit = args["limit"].as_u64().unwrap_or(1000) as usize;
        while let Some(object) = stack.pop() {
            if !visited.insert(self.node_id(&object)) {
                continue;
            }
            if visited.len() > 10000 {
                return Ok((found, true));
            }
            let mut children = self.children(&object, TreeKind::Visual);
            children.extend(self.children(&object, TreeKind::Logical));
            stack.extend(children.into_iter().rev());
            let target = Self::visual_object(&object);
            if args["type"]
                .as_str()
                .is_some_and(|s| target.get_type().name() != s)
            {
                continue;
            }
            if args["name"].as_str().is_some_and(|s| {
                target
                    .cast::<StyledElement>()
                    .and_then(|o| o.name())
                    .as_deref()
                    != Some(s)
            }) {
                continue;
            }
            if let Some(selector) = &selector {
                if !target
                    .cast::<StyledElement>()
                    .is_some_and(|o| selector.match_(&o, None, false).is_match())
                {
                    continue;
                }
            }
            if let Some(s) = args["automation_id"].as_str() {
                if Self::peer(&target)
                    .ok()
                    .and_then(|p| p.get_automation_id())
                    .as_deref()
                    != Some(s)
                {
                    continue;
                }
            }
            if let Some(s) = args["text"].as_str() {
                if !Self::peer(&target).is_ok_and(|p| p.get_name().contains(s)) {
                    continue;
                }
            }
            found.push(target);
            if found.len() >= limit {
                break;
            }
        }
        Ok((found, false))
    }

    /// Execute a single UI-thread operation of the MCP tools. `wait_for` is a nonblocking
    /// condition probe; the transport polls it until `matched` or its deadline, leaving the UI
    /// free to render. Input tools are run by [`execute`] instead, which does not hold the shared
    /// inspector while the application handles the input.
    pub fn execute(&mut self, name: &str, args: &Value, permissions: Permissions) -> InspectResult {
        Dispatcher::ui_thread().verify_access();
        validate(name, args)?;
        catch(|| {
            if is_input(name) {
                let object = self.prepare_input(args, permissions)?;
                input::execute(name, &object, args, permissions)?;
                return Ok(json!({"ok":true}));
            }
            self.execute_inner(name, args, permissions)
        })
    }

    fn prepare_input(
        &mut self,
        args: &Value,
        permissions: Permissions,
    ) -> Result<Ref<FerroObject>, String> {
        if !permissions.allow_input {
            return Err("Input is disabled".into());
        }
        self.prune();
        self.object(args)
    }

    fn execute_inner(
        &mut self,
        name: &str,
        args: &Value,
        permissions: Permissions,
    ) -> InspectResult {
        self.prune();
        match name {
            "list_windows" => {
                let roots = self.windows();
                Ok(Value::Array(
                    roots
                        .iter()
                        .map(|r| self.describe(&r.clone().upcast(), None))
                        .collect(),
                ))
            }
            "get_tree" => {
                let roots = self.roots(args)?;
                let mut visited = HashSet::new();
                let tree = TreeKind::parse(args["tree"].as_str().unwrap_or("visual"));
                Ok(Value::Array(
                    roots
                        .into_iter()
                        .map(|root| {
                            self.tree(
                                root,
                                tree,
                                args["depth"].as_u64().unwrap_or(8),
                                args.get("properties"),
                                &mut visited,
                            )
                        })
                        .collect(),
                ))
            }
            "find_elements" => {
                // Past the node limit the matches found so far are the result; none is an error.
                let (found, truncated) = self.find(args)?;
                if truncated && found.is_empty() {
                    return Err("Search exceeded 10000 nodes; scope it with node_id".into());
                }
                Ok(Value::Array(
                    found.iter().map(|o| self.describe(o, None)).collect(),
                ))
            }
            "get_properties" => Ok(properties_json(
                &Self::visual_object(&self.object(args)?),
                args.get("properties"),
            )),
            "set_property" => {
                if !permissions.allow_property_writes {
                    return Err("Property writes are disabled".into());
                }
                let object = Self::visual_object(&self.object(args)?);
                let name = args["property"].as_str().unwrap();
                let property = if args["clear"].as_bool().unwrap_or(false) {
                    Self::clear_property(&object, name)?
                } else {
                    let value = args
                        .get("value")
                        .ok_or("value required unless clear is true")?;
                    let source: Option<BoxedValue> = match value {
                        Value::Null => None,
                        Value::Bool(v) => Some(Rc::new(*v)),
                        Value::Number(v) => Some(Rc::new(v.as_f64().ok_or("Invalid number")?)),
                        Value::String(v) => Some(Rc::new(v.clone())),
                        _ => {
                            return Err(
                                "Use a boolean, number, string (framework syntax), or null".into()
                            )
                        }
                    };
                    Self::set_property(&object, name, source)?
                };
                Ok(properties_json(
                    &object,
                    Some(&json!([property_name(property)])),
                ))
            }
            "get_styles" => {
                let object = Self::visual_object(&self.object(args)?);
                Ok(Value::Array(
                    Self::styles(&object)
                        .into_iter()
                        .map(|frame| {
                            json!({"kind":frame.kind,"selector":frame.selector,"active":frame.active,"priority":frame.priority,"setters":frame.setters.into_iter().map(|(property,value)|json!({"property":property,"value":boxed(value)})).collect::<Vec<_>>()})
                        })
                        .collect(),
                ))
            }
            "get_resources" => {
                let object = Self::visual_object(&self.object(args)?)
                    .cast::<StyledElement>()
                    .ok_or("Node is not a styled element")?;
                let host: &dyn IResourceHost = &*object;
                let keys: Vec<ResourceKey> = if let Some(keys) = args["keys"].as_array() {
                    keys.iter()
                        .map(|k| ResourceKey::from(k.as_str().unwrap()))
                        .collect()
                } else {
                    // Listing the local keys leaves an element without resources as it is.
                    object
                        .resources_if_created()
                        .map(|resources| resources.keys())
                        .unwrap_or_default()
                };
                Ok(Value::Array(keys.into_iter().map(|key| { let value = host.try_find_resource(&key,object.actual_theme_variant().as_ref()); json!({"key":key.as_str().map(str::to_owned).unwrap_or_else(||format!("{key:?}")),"found":value.is_some(),"value":boxed(value.flatten())}) }).collect()))
            }
            "screenshot" => {
                let object = self
                    .roots(args)?
                    .into_iter()
                    .next()
                    .ok_or("No window available")?;
                let visual = Self::visual_object(&object)
                    .cast::<Visual>()
                    .ok_or("Node is not visual")?;
                let png = Self::screenshot(&visual, args["scale"].as_f64().unwrap_or(1.0))?;
                Ok(
                    json!({"mimeType":"image/png","data":base64::engine::general_purpose::STANDARD.encode(png.data),"width":png.width,"height":png.height}),
                )
            }
            "wait_for" => {
                let (found, truncated) = self.find(args)?;
                if truncated && found.is_empty() {
                    return Err("Search exceeded 10000 nodes; scope it with node_id".into());
                }
                let condition = args["condition"].as_str().unwrap_or("exists");
                if condition == "property_equals"
                    && (args["property"].as_str().is_none() || args.get("value").is_none())
                {
                    return Err("property and value required for property_equals".into());
                }
                let matched = match condition {
                    "absent" => found.is_empty(),
                    "visible" => found.iter().any(|o| {
                        o.cast::<Visual>().is_some_and(|v| v.is_visible())
                            && Self::peer(o).is_ok_and(|p| !p.is_offscreen())
                    }),
                    "enabled" => found
                        .iter()
                        .any(|o| Self::peer(o).is_ok_and(|p| p.is_enabled())),
                    "property_equals" => found.iter().any(|o| {
                        Self::property(o, args["property"].as_str().unwrap())
                            .is_ok_and(|p| boxed(Some(o.get_value_untyped(p))) == args["value"])
                    }),
                    _ => !found.is_empty(),
                };
                Ok(
                    json!({"matched":matched,"node_ids":found.iter().map(|o|self.node_id(o)).collect::<Vec<_>>()}),
                )
            }
            "get_logs" => Ok(LogCapture::snapshot(args["after"].as_u64().unwrap_or(0))),
            "get_frame_stats" => {
                let windows = self.windows();
                Ok(Value::Array(
                    windows
                        .iter()
                        .map(|w| {
                            let t = w.renderer_diagnostics().last_layout_pass_timing();
                            let id = self.node_id(w);
                            let mut stats = self.frames[&id].snapshot();
                            stats["node_id"] = json!(id);
                            stats["layout_pass_count"] = json!(t.pass_counter);
                            stats["layout_ms"] = json!(t.elapsed.as_secs_f64() * 1000.0);
                            stats
                        })
                        .collect(),
                ))
            }
            _ => Err(format!("Unknown tool: {name}")),
        }
    }
}

/// Executes an MCP tool on the shared inspector of this thread ([`Inspector::shared`]). The
/// inspector is held only while it reads or changes the objects: an input tool resolves its node,
/// releases the inspector and then delivers the input, so that the handlers of the application,
/// which may run a nested dispatcher frame, can reach the inspector again (through another
/// server, for example).
pub fn execute(name: &str, args: &Value, permissions: Permissions) -> InspectResult {
    Dispatcher::ui_thread().verify_access();
    validate(name, args)?;
    let shared = Inspector::shared();
    if is_input(name) {
        let object = {
            let mut inspector = shared
                .try_borrow_mut()
                .map_err(|_| "The inspector is busy".to_owned())?;
            inspector.prepare_input(args, permissions)?
        };
        return catch(|| {
            input::execute(name, &object, args, permissions)?;
            Ok(json!({"ok":true}))
        });
    }
    let mut inspector = shared
        .try_borrow_mut()
        .map_err(|_| "The inspector is busy".to_owned())?;
    catch(|| inspector.execute_inner(name, args, permissions))
}

fn is_input(name: &str) -> bool {
    matches!(
        name,
        "click" | "type_text" | "press_key" | "scroll" | "invoke_pattern"
    )
}

fn catch(f: impl FnOnce() -> InspectResult) -> InspectResult {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|error| {
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("UI operation panicked");
        Err(message.to_owned())
    })
}

fn properties_json(object: &FerroObject, filter: Option<&Value>) -> Value {
    let names = filter.and_then(Value::as_array);
    Value::Array(
        Inspector::properties(object)
            .into_iter()
            .filter(|p| names.is_none_or(|names| names.iter().any(|n| n == &p.name)))
            .map(|p| {
                json!({"name":p.name,"type":p.property.property_type_name(),"value":boxed(Some(p.value.clone())),"read_only":p.read_only,"priority":p.priority_name(),"binding":p.has_binding,"overridden_current_value":p.overridden_current_value})
            })
            .collect(),
    )
}

/// The name of the property as the tools and selectors use it: `Owner.Name` when attached.
pub fn property_name(p: &FerroProperty) -> String {
    if p.is_attached() {
        format!("{}.{}", p.owner_type().name(), p.name())
    } else {
        p.name().to_owned()
    }
}

/// The text of a value as the framework displays it.
pub fn display_value(value: Option<&BoxedValue>) -> String {
    value
        .cloned()
        .and_then(ValueTypes::normalize)
        .as_ref()
        .map(|v| ValueTypes::to_display_string(Some(v)))
        .unwrap_or_default()
}

fn rect(r: Rect) -> Value {
    json!({"x":r.x,"y":r.y,"width":r.width,"height":r.height})
}
fn boxed(value: Option<BoxedValue>) -> Value {
    let Some(value) = value.and_then(ValueTypes::normalize) else {
        return Value::Null;
    };
    macro_rules! scalar {($($t:ty),*) => {$(if let Some(v) = value.downcast_ref::<$t>() {return json!(v);})*};}
    scalar!(bool, String, i32, i64, u32, u64, usize);
    if let Some(v) = value.downcast_ref::<f64>() {
        return if v.is_finite() {
            json!(v)
        } else {
            json!(v.to_string())
        };
    }
    if let Some(v) = value.downcast_ref::<f32>() {
        return if v.is_finite() {
            json!(v)
        } else {
            json!(v.to_string())
        };
    }
    ValueTypes::try_to_string(&*value)
        .map(Value::String)
        .unwrap_or_else(|| json!({"opaque":true}))
}

// Style-only nesting and incomplete combinators panic in the framework's selector constructors
// or matcher. Validate the public grammar before constructing a selector from a remote request.
fn validate_selector(syntax: &[SelectorSyntax]) -> Result<(), String> {
    let mut term = false;
    for item in syntax {
        match item {
            SelectorSyntax::Nesting => {
                return Err("Style nesting selectors have no parent style in a query".into())
            }
            SelectorSyntax::Child
            | SelectorSyntax::Descendant
            | SelectorSyntax::Template
            | SelectorSyntax::Comma => {
                if !term {
                    return Err("Selector combinator requires a preceding selector".into());
                }
                term = false;
            }
            SelectorSyntax::Not { argument } => {
                validate_selector(argument)?;
                term = true;
            }
            _ => term = true,
        }
    }
    if !term {
        return Err("Selector requires a final selector term".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
