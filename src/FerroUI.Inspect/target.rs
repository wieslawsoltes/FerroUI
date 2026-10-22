//! Typed inspection of one top level (a window or a popup), the target of the Chrome DevTools
//! Protocol endpoint (`ferroui-devtools`). UI-thread-only. Handles are weak, ids are never
//! recycled, and every operation checks membership in the current tree so detached controls cannot
//! be driven by stale ids. The JSON tools of the crate root serve the MCP server instead.
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::media::{Colors, SolidColorBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{ControlTheme, Style};
use ferroui_base::{
    BoxedValue, FerroPropertyRegistry, PixelSize, Rect, Ref, StyledElement, Thickness, TypeInfo,
    Vector, Visual, WeakRef,
};
use ferroui_controls::primitives::{AdornerLayer, TemplatedControl};
use ferroui_controls::{Border, TopLevel};
use ferroui_markup::markup::parsers::{SelectorGrammar, SelectorParser, SelectorSyntax};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub type NodeId = u64;
pub type InspectResult<T> = Result<T, String>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeKind {
    #[default]
    Visual,
    Logical,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub name: String,
    pub attributes: Vec<String>,
    pub children: Vec<NodeId>,
}

#[derive(Clone, Debug)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub priority: String,
    pub read_only: bool,
}

#[derive(Clone, Debug)]
pub struct MatchedStyle {
    pub selector: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct BoxModel {
    pub content: Rect,
    pub padding: Rect,
    pub border: Rect,
    pub margin: Rect,
}

/// One top-level window (or popup). Keep this on the UI thread.
pub struct Inspector {
    root: WeakRef<TopLevel>,
    next_id: NodeId,
    handles: HashMap<NodeId, WeakRef<StyledElement>>,
    identities: HashMap<usize, NodeId>,
    overlay: Option<WeakRef<StyledElement>>,
    pub tree_kind: TreeKind,
    highlight: Option<Rc<dyn IDisposable>>,
}

impl Inspector {
    pub fn new(root: &Ref<TopLevel>) -> Self {
        Self {
            root: root.downgrade(),
            next_id: 2,
            handles: HashMap::new(),
            identities: HashMap::new(),
            overlay: None,
            tree_kind: TreeKind::Visual,
            highlight: None,
        }
    }

    pub fn root(&self) -> InspectResult<Ref<TopLevel>> {
        self.root
            .upgrade()
            .filter(|root| root.platform_impl().is_some())
            .ok_or_else(|| "Target has closed".into())
    }

    pub fn id_for(&mut self, element: &Ref<StyledElement>) -> NodeId {
        let identity = &**element as *const StyledElement as usize;
        if let Some(id) = self.identities.get(&identity) {
            if self.handles.get(id).and_then(WeakRef::upgrade).as_ref() == Some(element) {
                return *id;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.handles.insert(id, element.downgrade());
        self.identities.insert(identity, id);
        id
    }

    pub fn snapshot(&mut self) -> InspectResult<Vec<Node>> {
        let root = self.root()?.upcast::<StyledElement>();
        let mut result = Vec::new();
        let mut pending = vec![(root, None)];
        let mut seen = HashSet::new();
        while let Some((element, parent)) = pending.pop() {
            let id = self.id_for(&element);
            if !seen.insert(id) {
                continue;
            }
            let children: Vec<Ref<StyledElement>> = match self.tree_kind {
                TreeKind::Logical => element.logical_children().to_vec(),
                TreeKind::Visual => element
                    .cast::<Visual>()
                    .map(|v| {
                        v.visual_children()
                            .to_vec()
                            .into_iter()
                            .map(|v| v.upcast())
                            .collect()
                    })
                    .unwrap_or_default(),
            };
            let children: Vec<_> = children
                .into_iter()
                .filter(|c| self.overlay.as_ref().and_then(WeakRef::upgrade).as_ref() != Some(c))
                .collect();
            let child_ids = children.iter().map(|c| self.id_for(c)).collect();
            let mut attributes = vec!["class".into(), element.classes().snapshot().join(" ")];
            if let Some(name) = element.name() {
                attributes.extend(["id".into(), name.clone(), "Name".into(), name]);
            }
            for name in ["Text", "IsEnabled", "IsVisible"] {
                if let Some(p) =
                    FerroPropertyRegistry::instance().find_registered(element.get_type(), name)
                {
                    attributes.extend([name.into(), display(Some(&element.get_value_untyped(p)))]);
                }
            }
            result.push(Node {
                id,
                parent,
                name: element.get_type().name().into(),
                attributes,
                children: child_ids,
            });
            pending.extend(children.into_iter().rev().map(|c| (c, Some(id))));
        }
        self.handles.retain(|id, _| seen.contains(id));
        self.identities.retain(|_, id| seen.contains(id));
        Ok(result)
    }

    pub fn element(&mut self, id: NodeId) -> InspectResult<Ref<StyledElement>> {
        self.snapshot()?;
        self.handles
            .get(&id)
            .and_then(WeakRef::upgrade)
            .ok_or_else(|| format!("Unknown or detached node {id}"))
    }

    /// Uses the framework's selector parser and matching rules (including logical ancestry).
    pub fn query(&mut self, scope: Option<NodeId>, query: &str) -> InspectResult<Vec<NodeId>> {
        let nodes = self.snapshot()?;
        if let Some(scope) = scope {
            self.element(scope)?;
        }
        if query.trim().is_empty() || query.len() > 4096 {
            return Err("Selector must contain 1..4096 bytes".into());
        }
        let mut nesting = 0usize;
        for character in query.chars() {
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
        validate_selector(&SelectorGrammar::parse(query).map_err(|e| e.to_string())?)?;
        let mut types: HashMap<String, &'static TypeInfo> = TypeInfo::registered_types()
            .into_iter()
            .map(|t| (t.name().into(), t))
            .collect();
        for weak in self.handles.values() {
            if let Some(element) = weak.upgrade() {
                let mut ty = Some(element.get_type());
                while let Some(t) = ty {
                    types.insert(t.name().into(), t);
                    ty = t.base_type();
                }
            }
        }
        let selector = SelectorParser::new(move |_, name| types.get(name).copied())
            .parse(query)
            .map_err(|e| e.to_string())?;
        let mut descendants = HashSet::new();
        let mut found = Vec::new();
        for node in nodes {
            let in_scope = scope.is_none()
                || node.parent == scope
                || node.parent.is_some_and(|p| descendants.contains(&p));
            if in_scope {
                descendants.insert(node.id);
            }
            if in_scope && Some(node.id) != scope {
                let element = self.handles[&node.id].upgrade().unwrap();
                if selector
                    .as_ref()
                    .is_none_or(|s| s.match_(&element, None, false).is_match())
                {
                    found.push(node.id);
                }
            }
        }
        Ok(found)
    }

    pub fn properties(&mut self, id: NodeId) -> InspectResult<Vec<Property>> {
        let element = self.element(id)?;
        let mut result = Vec::new();
        for p in FerroPropertyRegistry::instance()
            .get_registered(element.get_type())
            .iter()
        {
            let d = element.get_diagnostic(p);
            result.push(Property {
                name: p.name().into(),
                value: display(Some(d.value())),
                priority: format!("{:?}", d.priority()),
                read_only: p.is_read_only(),
            });
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    pub fn set_attribute(&mut self, id: NodeId, name: &str, value: &str) -> InspectResult<()> {
        let element = self.element(id)?;
        match name {
            "id" | "Name" => return Err("Name is immutable after styling".into()),
            "class" => {
                let classes = element.classes();
                // Pseudo-classes are framework state, not editable CSS classes.
                if value.split_whitespace().any(|c| c.starts_with(':')) {
                    return Err("Pseudo-classes are read-only".into());
                }
                for class in classes.snapshot().iter().filter(|c| !c.starts_with(':')) {
                    classes.remove(class);
                }
                classes.add_range(value.split_whitespace());
                return Ok(());
            }
            _ => {}
        }
        let p = FerroPropertyRegistry::instance()
            .find_registered(element.get_type(), name)
            .ok_or_else(|| format!("Unknown property {name}"))?;
        if p.is_read_only() {
            return Err(format!("{name} is read-only"));
        }
        let text: BoxedValue = Rc::new(value.to_owned());
        let converted = ValueTypes::try_convert(
            Some(&text),
            ValueType::new(p.property_type(), p.property_type_name()),
        )
        .flatten()
        .ok_or_else(|| format!("Invalid value for {name}"))?;
        if !p.is_valid_value(converted.as_any()) {
            return Err(format!("Invalid value for {name}"));
        }
        element.set_value_untyped(p, converted.as_any(), BindingPriority::LocalValue);
        Ok(())
    }

    pub fn matched_styles(&mut self, id: NodeId) -> InspectResult<Vec<MatchedStyle>> {
        let element = self.element(id)?;
        Ok(element
            .get_value_store_diagnostic()
            .applied_frames()
            .iter()
            .filter(|f| f.is_active())
            .map(|f| {
                let source = f.source().and_then(|s| ValueTypes::as_object(&*s));
                let selector = source
                    .as_ref()
                    .and_then(|s| s.cast::<Style>())
                    .and_then(|s| s.selector())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        source.and_then(|s| s.cast::<ControlTheme>()).map(|t| {
                            format!(
                                "Theme {}",
                                t.target_type().map(TypeInfo::name).unwrap_or_default()
                            )
                        })
                    })
                    .unwrap_or_else(|| format!("{:?}", f.type_()));
                MatchedStyle {
                    selector,
                    properties: f
                        .values()
                        .iter()
                        .map(|v| (v.property.name().into(), display(v.value.as_ref())))
                        .collect(),
                }
            })
            .collect())
    }

    pub fn box_model(&mut self, id: NodeId) -> InspectResult<BoxModel> {
        let visual = self
            .element(id)?
            .cast::<Visual>()
            .ok_or("Node is not a visual")?;
        let root = self.root()?;
        let transform = visual
            .transform_to_visual(&root)
            .ok_or("Node has no transform to target")?;
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

    pub fn screenshot(&self) -> InspectResult<Vec<u8>> {
        let root = self.root()?;
        let size = root.bounds().size();
        if size.width <= 0.0 || size.height <= 0.0 || size.width * size.height > 32_000_000.0 {
            return Err("Invalid or oversized screenshot dimensions".into());
        }
        let bitmap = RenderTargetBitmap::with_dpi(
            PixelSize::new(size.width.ceil() as i32, size.height.ceil() as i32),
            Vector::new(96.0, 96.0),
        );
        bitmap.render(&root.upcast());
        let mut bytes = Vec::new();
        let result = bitmap.save(&mut bytes, &PngBitmapEncoderOptions::DEFAULT.into());
        bitmap.dispose();
        result.map_err(|e| e.to_string())?;
        Ok(bytes)
    }

    pub fn hide_highlight(&mut self) {
        self.overlay = None;
        if let Some(highlight) = self.highlight.take() {
            highlight.dispose();
        }
    }

    pub fn highlight(&mut self, id: NodeId) -> InspectResult<()> {
        self.hide_highlight();
        let visual = self
            .element(id)?
            .cast::<Visual>()
            .ok_or("Node is not a visual")?;
        let layer = AdornerLayer::get_adorner_layer(&visual).ok_or("Node has no adorner layer")?;
        let border = Border::new();
        border.set_is_hit_test_visible(false);
        border.set_border_thickness(Thickness::uniform(2.0));
        border.set_border_brush(Some(
            SolidColorBrush::with_color(Colors::DODGER_BLUE).into(),
        ));
        AdornerLayer::set_adorned_element(&border, visual);
        self.overlay = Some(border.clone().upcast::<StyledElement>().downgrade());
        layer.children().add(border.clone());
        self.highlight = Some(Disposable::create(move || {
            layer.children().remove(&border);
        }));
        Ok(())
    }
}

impl Drop for Inspector {
    fn drop(&mut self) {
        self.hide_highlight();
    }
}

fn display(value: Option<&BoxedValue>) -> String {
    let value = value.cloned().and_then(ValueTypes::normalize);
    value
        .as_ref()
        .map(|v| ValueTypes::to_display_string(Some(v)))
        .unwrap_or_default()
}

// Style-only nesting and incomplete combinators panic in the framework's selector constructors
// or matcher. Validate the public grammar before constructing a selector from a remote request.
fn validate_selector(syntax: &[SelectorSyntax]) -> InspectResult<()> {
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
