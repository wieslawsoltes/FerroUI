//! Enumerate logical controls and template instances without crossing their source scopes.
use ferroui_base::{controls::NameScope, BoxedValue, Ref, StyledElement};
use ferroui_markup_xaml::{reload_templates::Declaration, FromXamlObject};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

pub(crate) struct Control {
    pub identity: Vec<String>,
    pub name: String,
    pub element: Ref<StyledElement>,
    pub source: Rc<str>,
}
fn address(element: &StyledElement) -> usize {
    element as *const StyledElement as usize
}
fn template(node: roxmltree::Node<'_, '_>) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some("https://github.com/ferroui")
        && matches!(
            node.tag_name().name(),
            "ControlTemplate" | "DataTemplate" | "TreeDataTemplate" | "ItemsPanelTemplate"
        )
}
fn definition(node: roxmltree::Node<'_, '_>) -> Vec<String> {
    const X: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
    let mut path = Vec::new();
    for node in node.ancestors().filter(|node| node.is_element()) {
        let tag = node.tag_name().name();
        let key = node
            .attribute((crate::KEY_NAMESPACE, "Key"))
            .or_else(|| node.attribute((X, "Key")))
            .or_else(|| node.attribute((X, "Name")))
            .or_else(|| node.attribute("Name"));
        if let Some(key) = key {
            path.push(format!("{tag}#{key}"));
        } else if let Some(property) = node.attribute("Property").filter(|_| tag == "Setter") {
            path.push(format!("Setter:{property}"));
        } else if let Some(selector) = node.attribute("Selector").filter(|_| tag == "Style") {
            path.push(format!("Style:{selector}"));
        } else if tag.contains('.')
            || template(node)
            || node.parent().is_some_and(|parent| parent.is_root())
        {
            path.push(format!(
                "{{{}}}{tag}",
                node.tag_name().namespace().unwrap_or("")
            ));
        }
    }
    path.reverse();
    path
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
/// Keep in-scope namespaces and remove nested template declarations. A name in
/// another template must never affect an authored-value or identity decision.
fn fragment(source: &str, node: roxmltree::Node<'_, '_>) -> Rc<str> {
    let range = node.range();
    let mut text = source[range.clone()].to_owned();
    let mut excluded: Vec<_> = node
        .descendants()
        .skip(1)
        .filter(|child| template(*child))
        .filter(|child| {
            !child
                .ancestors()
                .skip(1)
                .take_while(|parent| *parent != node)
                .any(template)
        })
        .map(|child| child.range())
        .collect();
    excluded.sort_by_key(|range| range.start);
    for child in excluded.into_iter().rev() {
        text.replace_range(child.start - range.start..child.end - range.start, "");
    }
    let namespaces = node
        .namespaces()
        .map(|ns| match ns.name() {
            Some(name) => format!(" xmlns:{name}=\"{}\"", escape(ns.uri())),
            None => format!(" xmlns=\"{}\"", escape(ns.uri())),
        })
        .collect::<String>();
    Rc::from(format!("<ReloadState{namespaces}>{text}</ReloadState>"))
}
fn declaration_scope(
    declaration: &Declaration,
    main_uri: Option<&str>,
    main: &str,
) -> Option<(String, Rc<str>)> {
    let mut uri = declaration.uri.as_deref().map(crate::document_key);
    if uri.is_none() {
        if let Some(source) = &declaration.source {
            let normalized = crate::identity::runtime_source(main).ok();
            if &**source != main && normalized.as_deref() != Some(&**source) {
                return None;
            }
            uri = main_uri.map(crate::document_key);
        }
    }
    let source = declaration.source.clone().or_else(|| {
        uri.as_ref()
            .and_then(|uri| ferroui_markup_xaml::source_overlay::compiled_source(uri))
            .map(Rc::from)
            .or_else(|| {
                (uri.as_deref() == main_uri.map(crate::document_key).as_deref())
                    .then(|| Rc::from(main))
            })
    })?;
    let xml = roxmltree::Document::parse(&source).ok()?;
    let line = usize::try_from(declaration.line).ok()?.checked_sub(1)?;
    let column = usize::try_from(declaration.column).ok()?.checked_sub(1)?;
    // XamlX uses IXmlLineInfo coordinates: UTF-16 columns and XML line
    // endings (including a lone CR), not UTF-8 byte offsets.
    let mut starts = vec![0];
    let bytes = source.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' || (*byte == b'\r' && bytes.get(index + 1) != Some(&b'\n')) {
            starts.push(index + 1);
        }
    }
    let start = *starts.get(line)?;
    let end = starts.get(line + 1).copied().unwrap_or(source.len());
    let mut units = 0;
    let offset = source[start..end]
        .char_indices()
        .find_map(|(offset, character)| {
            let matches = units == column;
            units += character.len_utf16();
            matches.then_some(start + offset)
        })?;
    let node = xml
        .descendants()
        .filter(|node| node.is_element() && node.range().contains(&offset))
        .last()?
        .ancestors()
        .find(|node| template(*node))?;
    let path = definition(node);
    if xml
        .descendants()
        .filter(|node| template(*node) && definition(*node) == path)
        .count()
        != 1
    {
        return None;
    }
    Some((
        format!(
            "template:{}:{path:?}",
            uri.as_deref().unwrap_or("@document")
        ),
        fragment(&source, node),
    ))
}
fn children(element: &StyledElement, visual: bool) -> Vec<Ref<StyledElement>> {
    let mut result = element.logical_children().to_vec();
    if visual {
        if let Some(visual) = element.to_ref().cast::<ferroui_base::Visual>() {
            result.extend(
                visual
                    .visual_children()
                    .to_vec()
                    .into_iter()
                    .filter_map(|child| child.cast::<StyledElement>()),
            );
        }
    }
    result
}
struct Instance {
    root: Ref<StyledElement>,
    key: String,
    source: Rc<str>,
}
struct Walk {
    assigned: HashSet<usize>,
    owners: HashMap<usize, Vec<String>>,
    templates: HashSet<usize>,
    pending_scopes: HashSet<usize>,
    result: Vec<Control>,
}
impl Walk {
    fn scope(
        &mut self,
        root: Ref<StyledElement>,
        base: Vec<String>,
        source: Rc<str>,
        visual: bool,
    ) {
        let mut pending = vec![(root, base, None, true)];
        while let Some((element, mut scope_path, parent_scope, is_root)) = pending.pop() {
            let id = address(&element);
            if (!is_root && self.templates.contains(&id)) || !self.assigned.insert(id) {
                continue;
            }
            let own_scope = NameScope::get_name_scope(&element);
            let name = crate::identity::control_name(&element);
            let identity = name
                .as_ref()
                .and_then(|name| crate::identity::identity(&source, name));
            if !is_root && own_scope.is_some() && own_scope != parent_scope {
                let Some(owner) = &identity else {
                    continue;
                };
                scope_path.push(owner.clone());
            }
            let scope = own_scope.or(parent_scope);
            pending.extend(
                children(&element, visual)
                    .into_iter()
                    .map(|child| (child, scope_path.clone(), scope.clone(), false)),
            );
            if let Some(identity) = identity {
                scope_path.push(identity);
                self.owners.insert(id, scope_path.clone());
                self.result.push(Control {
                    identity: scope_path,
                    name: name.unwrap(),
                    element,
                    source: source.clone(),
                });
            } else if is_root {
                scope_path.push("@root".into());
                self.owners.insert(id, scope_path);
            }
        }
    }
    fn owner(&self, instance: &StyledElement) -> Option<Vec<String>> {
        let parent = |element: &StyledElement| {
            element
                .to_ref()
                .cast::<ferroui_base::Visual>()
                .and_then(|visual| visual.visual_parent())
                .and_then(|value| value.cast::<StyledElement>())
                .or_else(|| element.parent())
        };
        let mut next = instance
            .templated_parent()
            .and_then(|value| value.cast::<StyledElement>())
            .or_else(|| parent(instance));
        let mut visited = HashSet::new();
        while let Some(element) = next {
            let id = address(&element);
            if !visited.insert(id) {
                return None;
            }
            if let Some(owner) = self.owners.get(&id) {
                return Some(owner.clone());
            }
            // A nested declaration must wait for its outer instance to establish
            // its scope. Unknown library templates may be crossed while finding
            // the declared presenter, but their parts are never captured.
            if self.pending_scopes.contains(&id) {
                return None;
            }
            next = parent(&element);
        }
        None
    }
}
pub(crate) fn has_focus(root: &BoxedValue) -> bool {
    let Some(root) = Ref::<StyledElement>::from_xaml_object(root) else {
        return false;
    };
    let mut pending = vec![root];
    let mut seen = HashSet::new();
    while let Some(element) = pending.pop() {
        if !seen.insert(address(&element)) {
            continue;
        }
        if element
            .cast::<ferroui_base::input::InputElement>()
            .is_some_and(|input| input.is_focused())
        {
            return true;
        }
        pending.extend(children(&element, true));
    }
    false
}

pub(crate) fn controls(root: &BoxedValue, uri: Option<&str>, source: &str) -> Vec<Control> {
    let Some(root) = Ref::<StyledElement>::from_xaml_object(root) else {
        return Vec::new();
    };
    let mut pending = vec![root.clone()];
    let mut seen = HashSet::new();
    let mut instances = Vec::new();
    let mut templates = HashSet::new();
    let mut resolved = HashMap::new();
    while let Some(element) = pending.pop() {
        if !seen.insert(address(&element)) {
            continue;
        }
        pending.extend(children(&element, true));
        if let Some(declaration) = ferroui_markup_xaml::reload_templates::declaration(&element) {
            templates.insert(address(&element));
            let cache_key = (
                declaration.uri.clone(),
                declaration.line,
                declaration.column,
                declaration
                    .source
                    .as_ref()
                    .map(|source| source.as_ptr() as usize),
            );
            let scope = resolved
                .entry(cache_key)
                .or_insert_with(|| declaration_scope(&declaration, uri, source));
            if let Some((key, source)) = scope {
                instances.push(Instance {
                    root: element,
                    key: key.clone(),
                    source: source.clone(),
                });
            }
        }
    }
    let xml = roxmltree::Document::parse(source).ok();
    let source = xml
        .as_ref()
        .map(|xml| fragment(source, xml.root_element()))
        .unwrap_or_else(|| Rc::from(source));
    let mut walk = Walk {
        assigned: HashSet::new(),
        owners: HashMap::new(),
        templates,
        pending_scopes: instances
            .iter()
            .map(|instance| address(&instance.root))
            .collect(),
        result: Vec::new(),
    };
    walk.scope(root, Vec::new(), source, false);
    loop {
        let mut remaining = Vec::new();
        let mut progressed = false;
        for instance in instances {
            if let Some(mut owner) = walk.owner(&instance.root) {
                owner.push(instance.key);
                walk.scope(instance.root, owner, instance.source, true);
                progressed = true;
            } else {
                remaining.push(instance);
            }
        }
        if !progressed {
            break;
        }
        instances = remaining;
    }
    walk.result
}
