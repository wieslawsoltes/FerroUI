//! Transport-independent inspection of live FerroUI objects. All Inspector methods run on the
//! UI thread. IDs are weak, stable for an object's lifetime, and never reused within a session.
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
use ferroui_base::styling::StyleBase;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, PixelSize, Rect, Ref,
    StyledElement, TypeInfo, Vector, Visual, WeakRef,
};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::{Application, Control, TopLevel};
use ferroui_markup::markup::parsers::SelectorParser;
pub use logs::{LogCapture, LogGuard};
pub use schema::{tools, validate};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Mutating operations are disabled unless explicitly enabled by the application.
#[derive(Clone, Copy, Debug, Default)]
pub struct Permissions {
    pub allow_property_writes: bool,
    pub allow_input: bool,
}

pub type InspectResult = Result<Value, String>;

pub struct Inspector {
    next_id: u64,
    nodes: HashMap<u64, WeakRef<FerroObject>>,
    addresses: HashMap<usize, u64>,
    roots: Vec<WeakRef<TopLevel>>,
    frames: HashMap<u64, frames::FrameStats>,
    pub permissions: Permissions,
    pub logs: LogCapture,
}

impl Inspector {
    pub fn new(permissions: Permissions) -> Self {
        Dispatcher::ui_thread().verify_access();
        Self {
            next_id: 1,
            nodes: HashMap::new(),
            addresses: HashMap::new(),
            roots: Vec::new(),
            frames: HashMap::new(),
            permissions,
            logs: LogCapture::default(),
        }
    }

    /// Add a top level for a custom lifetime or tests. Desktop windows are discovered automatically.
    pub fn register_root(&mut self, root: &TopLevel) {
        if !self.roots.iter().any(|weak| weak.points_to(&root.to_ref())) {
            self.roots.push(root.to_ref().downgrade());
        }
    }

    pub fn node_id(&mut self, object: &FerroObject) -> u64 {
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

    pub fn resolve(&self, id: u64) -> Result<Ref<FerroObject>, String> {
        self.nodes
            .get(&id)
            .and_then(WeakRef::upgrade)
            .ok_or_else(|| format!("Unknown or expired node_id: {id}"))
    }

    fn windows(&mut self) -> Vec<Ref<TopLevel>> {
        self.nodes.retain(|_, weak| weak.upgrade().is_some());
        self.addresses.retain(|_, id| self.nodes.contains_key(id));
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

    fn visual_object(object: &Ref<FerroObject>) -> Ref<FerroObject> {
        object
            .cast::<ControlAutomationPeer>()
            .and_then(|p| p.try_owner())
            .map(Ref::upcast)
            .unwrap_or_else(|| object.clone())
    }

    fn children(object: &Ref<FerroObject>, tree: &str) -> Vec<Ref<FerroObject>> {
        match tree {
            "automation" => Self::peer(object)
                .map(|p| p.get_children().iter().cloned().map(Ref::upcast).collect())
                .unwrap_or_default(),
            "logical" => object
                .cast::<StyledElement>()
                .map(|o| {
                    o.logical_children()
                        .to_vec()
                        .into_iter()
                        .map(Ref::upcast)
                        .collect()
                })
                .unwrap_or_default(),
            _ => object
                .cast::<Visual>()
                .map(|o| {
                    o.visual_children()
                        .to_vec()
                        .into_iter()
                        .map(Ref::upcast)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn peer(object: &Ref<FerroObject>) -> Result<Ref<AutomationPeer>, String> {
        object
            .cast::<AutomationPeer>()
            .or_else(|| {
                object
                    .cast::<Control>()
                    .map(|c| ControlAutomationPeer::create_peer_for_element(&c))
            })
            .ok_or("Node has no automation peer".into())
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
            result["properties"] = Self::properties(&visual_object, Some(properties));
        }
        result
    }

    fn tree(
        &mut self,
        object: Ref<FerroObject>,
        tree: &str,
        depth: u64,
        properties: Option<&Value>,
        visited: &mut HashSet<u64>,
    ) -> Value {
        let object = if tree == "automation" {
            Self::peer(&object).map(Ref::upcast).unwrap_or(object)
        } else {
            object
        };
        let id = self.node_id(&object);
        if visited.len() >= 10000 || !visited.insert(id) {
            return json!({"node_id":id,"truncated":true});
        }
        let mut result = self.describe(&object, properties);
        let children = Self::children(&object, tree);
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

    fn property(object: &FerroObject, name: &str) -> Result<&'static FerroProperty, String> {
        Self::registered(object)
            .into_iter()
            .find(|p| property_name(p) == name)
            .ok_or_else(|| format!("Unknown property: {name}"))
    }

    fn properties(object: &FerroObject, filter: Option<&Value>) -> Value {
        Value::Array(Self::registered(object).into_iter().filter(|p| filter.and_then(Value::as_array).is_none_or(|names| names.iter().any(|n| n == &property_name(p)))).map(|p| {
            let diag = object.get_diagnostic(p);
            json!({"name":property_name(p),"type":p.property_type_name(),"value":boxed(Some(diag.value().clone())),"read_only":p.is_read_only(),"priority":if p.is_direct() {"Direct".into()} else {format!("{:?}",diag.priority())},"binding":BindingOperations::get_binding_expression_base(object,p).is_some(),"overridden_current_value":diag.is_overridden_current_value()})
        }).collect())
    }

    fn find(&mut self, args: &Value) -> Result<Vec<Ref<FerroObject>>, String> {
        let selector = args["selector"]
            .as_str()
            .map(|s| {
                SelectorParser::new(|ns, name| {
                    if ns.is_empty() {
                        TypeInfo::find_by_name(name)
                    } else {
                        TypeInfo::find(ns, name)
                    }
                })
                .parse(s)
            })
            .transpose()
            .map_err(|e| e.to_string())?
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
                return Err("Search exceeded 10000 nodes; scope it with node_id".into());
            }
            let mut children = Self::children(&object, "visual");
            children.extend(Self::children(&object, "logical"));
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
        Ok(found)
    }

    /// Execute a single UI-thread operation. `wait_for` is a nonblocking condition probe;
    /// the transport polls it until `matched` or its deadline, leaving the UI free to render.
    pub fn execute(&mut self, name: &str, args: &Value) -> InspectResult {
        Dispatcher::ui_thread().verify_access();
        validate(name, args)?;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.execute_inner(name, args)
        }))
        .unwrap_or_else(|error| {
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("UI operation panicked");
            Err(message.to_owned())
        })
    }

    fn execute_inner(&mut self, name: &str, args: &Value) -> InspectResult {
        self.nodes.retain(|_, weak| weak.upgrade().is_some());
        self.addresses.retain(|_, id| self.nodes.contains_key(id));
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
                Ok(Value::Array(
                    roots
                        .into_iter()
                        .map(|root| {
                            self.tree(
                                root,
                                args["tree"].as_str().unwrap_or("visual"),
                                args["depth"].as_u64().unwrap_or(8),
                                args.get("properties"),
                                &mut visited,
                            )
                        })
                        .collect(),
                ))
            }
            "find_elements" => {
                let found = self.find(args)?;
                Ok(Value::Array(
                    found.iter().map(|o| self.describe(o, None)).collect(),
                ))
            }
            "get_properties" => Ok(Self::properties(
                &Self::visual_object(&self.object(args)?),
                args.get("properties"),
            )),
            "set_property" => {
                if !self.permissions.allow_property_writes {
                    return Err("Property writes are disabled".into());
                }
                let object = Self::visual_object(&self.object(args)?);
                let property = Self::property(&object, args["property"].as_str().unwrap())?;
                if property.is_read_only() {
                    return Err("Property is read-only".into());
                }
                if args["clear"].as_bool().unwrap_or(false) {
                    object.clear_value_untyped(property);
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
                    let converted = ValueTypes::try_convert(
                        source.as_ref(),
                        ValueType::new(property.property_type(), property.property_type_name()),
                    )
                    .flatten()
                    .ok_or("Cannot convert value to property type")?;
                    if !property.is_valid_value(converted.as_any()) {
                        return Err("Property validation rejected value".into());
                    }
                    object.set_value_untyped(
                        property,
                        converted.as_any(),
                        BindingPriority::LocalValue,
                    );
                }
                Ok(Self::properties(
                    &object,
                    Some(&json!([property_name(property)])),
                ))
            }
            "get_styles" => {
                let object = Self::visual_object(&self.object(args)?);
                Ok(Value::Array(object.get_value_store_diagnostic().applied_frames().iter().map(|frame| {
                    let source = frame.source();
                    let selector = source.as_ref().and_then(|s|s.downcast_ref::<Ref<StyleBase>>()).and_then(|s|s.cast::<ferroui_base::styling::Style>()).and_then(|s|s.selector()).map(|s|s.to_string_with_owner(None));
                    json!({"kind":format!("{:?}",frame.type_()),"selector":selector,"active":frame.is_active(),"priority":format!("{:?}",frame.priority()),"setters":frame.values().into_iter().map(|v|json!({"property":property_name(v.property),"value":boxed(v.value)})).collect::<Vec<_>>()})
                }).collect()))
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
                    object.resources().keys()
                };
                Ok(Value::Array(keys.into_iter().map(|key| { let value = host.try_find_resource(&key,object.actual_theme_variant().as_ref()); json!({"key":key.as_str().map(str::to_owned).unwrap_or_else(||format!("{key:?}")),"found":value.is_some(),"value":boxed(value.flatten())}) }).collect()))
            }
            "click" | "type_text" | "press_key" | "scroll" | "invoke_pattern" => {
                if !self.permissions.allow_input {
                    return Err("Input is disabled".into());
                }
                let object = self.object(args)?;
                input::execute(name, &object, args, self.permissions)?;
                Ok(json!({"ok":true}))
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
                let scale = args["scale"].as_f64().unwrap_or(1.0);
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
                bitmap.render(&visual);
                let mut png = Vec::new();
                let result = bitmap.save(&mut png, &PngBitmapEncoderOptions::DEFAULT.into());
                bitmap.dispose();
                result.map_err(|e| e.to_string())?;
                Ok(
                    json!({"mimeType":"image/png","data":base64::engine::general_purpose::STANDARD.encode(png),"width":width,"height":height}),
                )
            }
            "wait_for" => {
                let found = self.find(args)?;
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
            "get_logs" => Ok(self.logs.snapshot(args["after"].as_u64().unwrap_or(0))),
            "get_frame_stats" => {
                let windows = self.windows();
                Ok(Value::Array(
                    windows
                        .iter()
                        .map(|w| {
                            let t = w.renderer_diagnostics().last_layout_pass_timing();
                            {
                                let id = self.node_id(w);
                                let mut stats = self.frames[&id].snapshot();
                                stats["node_id"] = json!(id);
                                stats["layout_pass_count"] = json!(t.pass_counter);
                                stats["layout_ms"] = json!(t.elapsed.as_secs_f64() * 1000.0);
                                stats
                            }
                        })
                        .collect(),
                ))
            }
            _ => Err(format!("Unknown tool: {name}")),
        }
    }
}

fn property_name(p: &FerroProperty) -> String {
    if p.is_attached() {
        format!("{}.{}", p.owner_type().name(), p.name())
    } else {
        p.name().to_owned()
    }
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

#[cfg(test)]
mod tests;
