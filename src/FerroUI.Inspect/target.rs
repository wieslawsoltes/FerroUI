//! One top level (a window or a popup) as the target of the Chrome DevTools Protocol endpoint
//! (`ferroui-devtools`). A target is a view of the shared [`Inspector`]: the node IDs, selectors,
//! properties, styles, layout boxes, screenshots and highlights are the inspector's; the target
//! keeps its root, the tree it shows (visual or logical), which nodes are in that tree, and its
//! highlight. Every operation checks that the node is in the current tree, so a control that has
//! been detached cannot be driven through its ID until it is attached again. UI thread only.
use crate::{display_value, Highlight, Inspector, NodeId, TreeKind};
use ferroui_base::{FerroObject, FerroPropertyRegistry, Rect, Ref, StyledElement, Visual, WeakRef};
use ferroui_controls::TopLevel;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub type InspectResult<T> = Result<T, String>;

#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub name: String,
    /// Name and value pairs: `class`, `id` and `Name` (the name of the element), `Text`,
    /// `IsEnabled` and `IsVisible`.
    pub attributes: Vec<String>,
    pub children: Vec<NodeId>,
}

#[derive(Clone, Debug)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub priority: String,
    /// Whether the value is a local value (CSS's inline style).
    pub local: bool,
    pub read_only: bool,
}

#[derive(Clone, Debug)]
pub struct MatchedStyle {
    pub selector: String,
    pub properties: Vec<(String, String)>,
}

pub use crate::BoxModel;

pub struct Target {
    inspector: Rc<RefCell<Inspector>>,
    root: WeakRef<TopLevel>,
    /// The tree of the snapshots: visual or logical.
    pub tree_kind: TreeKind,
    members: HashSet<NodeId>,
    highlight: Option<Highlight>,
}

impl Target {
    /// A target on the shared inspector of this thread.
    pub fn new(root: &Ref<TopLevel>) -> Self {
        Self::with_inspector(Inspector::shared(), root)
    }

    pub fn with_inspector(inspector: Rc<RefCell<Inspector>>, root: &Ref<TopLevel>) -> Self {
        Self {
            inspector,
            root: root.downgrade(),
            tree_kind: TreeKind::Visual,
            members: HashSet::new(),
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
        self.inspector.borrow_mut().node_id(element)
    }

    pub fn snapshot(&mut self) -> InspectResult<Vec<Node>> {
        let root = self.root()?.upcast::<FerroObject>();
        let tree = if self.tree_kind == TreeKind::Logical {
            TreeKind::Logical
        } else {
            TreeKind::Visual
        };
        let mut inspector = self.inspector.borrow_mut();
        let mut result = Vec::new();
        let mut pending = vec![(root, None)];
        let mut seen = HashSet::new();
        while let Some((object, parent)) = pending.pop() {
            let Some(element) = object.cast::<StyledElement>() else {
                continue;
            };
            let id = inspector.node_id(&object);
            if !seen.insert(id) {
                continue;
            }
            let children: Vec<_> = inspector
                .children(&object, tree)
                .into_iter()
                .filter(|c| c.cast::<StyledElement>().is_some())
                .collect();
            let child_ids = children.iter().map(|c| inspector.node_id(c)).collect();
            let mut attributes = vec!["class".into(), element.classes().snapshot().join(" ")];
            if let Some(name) = element.name() {
                attributes.extend(["id".into(), name.clone(), "Name".into(), name]);
            }
            for name in ["Text", "IsEnabled", "IsVisible"] {
                if let Some(p) =
                    FerroPropertyRegistry::instance().find_registered(element.get_type(), name)
                {
                    attributes.extend([
                        name.into(),
                        display_value(Some(&element.get_value_untyped(p))),
                    ]);
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
        self.members = seen;
        Ok(result)
    }

    /// The element of a node of the current tree.
    pub fn element(&mut self, id: NodeId) -> InspectResult<Ref<StyledElement>> {
        self.snapshot()?;
        if !self.members.contains(&id) {
            return Err(format!("Unknown or detached node {id}"));
        }
        self.inspector
            .borrow()
            .resolve(id)?
            .cast::<StyledElement>()
            .ok_or_else(|| format!("Node {id} is not an element"))
    }

    /// The nodes below `scope` (the whole tree without one) that match the selector, in document
    /// order. Uses the framework's selector syntax and matching rules (including logical ancestry).
    pub fn query(&mut self, scope: Option<NodeId>, query: &str) -> InspectResult<Vec<NodeId>> {
        let nodes = self.snapshot()?;
        if let Some(scope) = scope {
            self.element(scope)?;
        }
        let inspector = self.inspector.borrow();
        let selector = inspector.parse_selector(query)?;
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
                let Some(element) = inspector
                    .resolve(node.id)
                    .ok()
                    .and_then(|o| o.cast::<StyledElement>())
                else {
                    continue;
                };
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
        Ok(Inspector::properties(&element)
            .into_iter()
            .map(|p| Property {
                value: display_value(Some(&p.value)),
                priority: p.priority_name(),
                local: p.is_local(),
                read_only: p.read_only,
                name: p.name,
            })
            .collect())
    }

    /// `class` replaces the classes (not the pseudo-classes); any other name sets the local value
    /// of a property from text. The name of an element cannot change after it has been styled.
    pub fn set_attribute(&mut self, id: NodeId, name: &str, value: &str) -> InspectResult<()> {
        let element = self.element(id)?;
        match name {
            "id" | "Name" => Err("Name is immutable after styling".into()),
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
                Ok(())
            }
            _ => {
                Inspector::set_property(&element, name, Some(Rc::new(value.to_owned()))).map(|_| ())
            }
        }
    }

    /// The active frames that apply to the node, with their setters.
    pub fn matched_styles(&mut self, id: NodeId) -> InspectResult<Vec<MatchedStyle>> {
        let element = self.element(id)?;
        Ok(Inspector::styles(&element)
            .into_iter()
            .filter(|frame| frame.active)
            .map(|frame| MatchedStyle {
                selector: frame.selector.unwrap_or(frame.kind),
                properties: frame
                    .setters
                    .into_iter()
                    .map(|(name, value)| (name, display_value(value.as_ref())))
                    .collect(),
            })
            .collect())
    }

    pub fn box_model(&mut self, id: NodeId) -> InspectResult<BoxModel> {
        let visual = self
            .element(id)?
            .cast::<Visual>()
            .ok_or("Node is not a visual")?;
        Inspector::box_model(&visual, &self.root()?.upcast())
    }

    /// The bounds of the top level, from its origin.
    pub fn bounds(&self) -> InspectResult<Rect> {
        Ok(Rect::from_size(self.root()?.bounds().size()))
    }

    /// A PNG of the whole top level.
    pub fn screenshot(&self) -> InspectResult<Vec<u8>> {
        Ok(Inspector::screenshot(&self.root()?.upcast(), 1.0)?.data)
    }

    pub fn hide_highlight(&mut self) {
        self.highlight = None;
    }

    pub fn highlight(&mut self, id: NodeId) -> InspectResult<()> {
        self.hide_highlight();
        let visual = self
            .element(id)?
            .cast::<Visual>()
            .ok_or("Node is not a visual")?;
        self.highlight = Some(self.inspector.borrow_mut().highlight(&visual)?);
        Ok(())
    }
}
