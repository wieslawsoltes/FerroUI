//! Deterministic, immutable design records. No application viewmodel constructor or
//! setter is invoked to populate sample data; ordinary preview bindings read this plugin.
use crate::design_records::{DesignRecord, SampleValue, register_accessors};
use ferroui_base::data::BindingPriority;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{BoxedValue, FerroObject, FerroPropertyRegistry, Ref, StyledElement, Visual};
use ferroui_controls::Control;
use ferroui_designer_core::{Document, MarkupValue, Node, NodeId, SourceSpan};
use ferroui_inspect::{Inspector, TreeKind};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub const MAX_SAMPLE_ITEMS: usize = 200;
const MAX_FIELDS: usize = 128;
const MAX_PATH_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SampleKind {
    Text,
    Boolean,
    Number,
}

#[derive(Clone, Debug)]
pub(crate) struct SampleRequest {
    pub node: NodeId,
    pub property: String,
    pub count: usize,
    fields: BTreeMap<String, SampleKind>,
    fixture: Option<DesignRecord>,
    /// Anonymous records have runtime members. The application retains its compiled
    /// bindings; only the temporary sample-data projection uses reflection access.
    pub binding_replacements: Vec<(SourceSpan, String)>,
}

pub(crate) fn sample_count(
    document: &Document,
    node: NodeId,
    value: &str,
) -> Result<Option<usize>, String> {
    let Ok(MarkupValue::Extension(extension)) = MarkupValue::parse(value) else {
        return Ok(None);
    };
    let Some((prefix, name)) = extension.name.split_once(':') else {
        return Ok(None);
    };
    if !matches!(name, "SampleData" | "SampleDataExtension")
        || document
            .ast()
            .namespaces(node)
            .get(prefix)
            .map(String::as_str)
            != Some(crate::design_time::DESIGN_NAMESPACE)
    {
        return Ok(None);
    }
    let mut count = 5;
    for argument in &extension.arguments {
        if argument.name.as_deref() != Some("ItemCount") {
            return Err("Design SampleData accepts only ItemCount.".into());
        }
        let MarkupValue::Literal(value) = &argument.value else {
            return Err("Sample ItemCount must be an integer.".into());
        };
        count = value
            .parse::<usize>()
            .map_err(|_| "Sample ItemCount must be an integer.")?;
    }
    if count > MAX_SAMPLE_ITEMS {
        return Err(format!(
            "Sample ItemCount must be between 0 and {MAX_SAMPLE_ITEMS}."
        ));
    }
    Ok(Some(count))
}

pub(crate) fn request(
    document: &Document,
    node: NodeId,
    property: &str,
    count: usize,
    fixture: Option<DesignRecord>,
    notices: &mut Vec<String>,
) -> SampleRequest {
    let mut fields = BTreeMap::new();
    let mut binding_replacements = Vec::new();
    let owner = document.ast().node(node).unwrap();
    let mut roots = vec![node];
    if property == "ItemsSource" {
        // Inline templates and column bindings are in the same source subtree. A
        // referenced local DataTemplate can be found without loading application code.
        if let Some(attribute) = owner.attribute_info("ItemTemplate") {
            if let Ok(MarkupValue::Extension(extension)) = attribute.markup() {
                let (prefix, local) = extension
                    .name
                    .split_once(':')
                    .unwrap_or(("", &extension.name));
                if local == "StaticResource"
                    && document
                        .ast()
                        .namespaces(node)
                        .get(prefix)
                        .is_some_and(|uri| uri == "https://github.com/ferroui")
                {
                    if let Some(MarkupValue::Literal(key)) =
                        extension.arguments.first().map(|argument| &argument.value)
                    {
                        if let Ok(resources) =
                            ferroui_designer_tools::visible_component_resources(document, node)
                        {
                            if let Some(resource) = resources
                                .iter()
                                .find(|resource| resource.key.as_deref() == Some(key))
                            {
                                roots.push(resource.node);
                            }
                        }
                    }
                }
            }
        }
    }
    for root in roots {
        for element in document
            .ast()
            .subtree(root)
            .filter(|element| element.is_element())
        {
            if fixture.is_some() && !inherits_fixture(document, element, node) {
                continue;
            }
            for attribute in &element.attributes {
                if attribute.namespace_uri.as_deref() == Some(crate::design_time::DESIGN_NAMESPACE)
                {
                    continue;
                }
                let Ok(MarkupValue::Extension(extension)) = attribute.markup() else {
                    continue;
                };
                let (prefix, name) = extension
                    .name
                    .split_once(':')
                    .unwrap_or(("", &extension.name));
                if !matches!(name, "Binding" | "ReflectionBinding" | "CompiledBinding")
                    || document
                        .ast()
                        .namespaces(element.id)
                        .get(prefix)
                        .is_none_or(|uri| uri != "https://github.com/ferroui")
                {
                    continue;
                }
                if extension.arguments.iter().any(|argument| {
                    matches!(
                        argument.name.as_deref(),
                        Some("Source" | "ElementName" | "RelativeSource")
                    )
                }) {
                    continue;
                }
                if name != "ReflectionBinding" {
                    let qualified = if prefix.is_empty() {
                        "ReflectionBinding".to_owned()
                    } else {
                        format!("{prefix}:ReflectionBinding")
                    };
                    let tail = attribute.value[1..].trim_start();
                    let expression = format!("{{{qualified}{}", &tail[extension.name.len()..]);
                    binding_replacements.push((
                        attribute.value_span,
                        expression
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('"', "&quot;")
                            .replace('\'', "&apos;"),
                    ));
                }
                if fixture.is_some() {
                    continue;
                }
                let Some(MarkupValue::Literal(path)) = extension
                    .arguments
                    .iter()
                    .find(|argument| {
                        argument.name.is_none() || argument.name.as_deref() == Some("Path")
                    })
                    .map(|argument| &argument.value)
                else {
                    continue;
                };
                if path.is_empty() || path == "." {
                    continue;
                }
                if !simple_path(path) {
                    notices.push(format!("Sample data leaves complex binding path '{path}' to explicit design values."));
                    continue;
                }
                if fields.len() >= MAX_FIELDS {
                    continue;
                }
                let kind = property_kind(document, element, attribute);
                fields.entry(path.clone()).or_insert(kind);
            }
        }
    }
    SampleRequest {
        node,
        property: property.into(),
        count,
        fields,
        fixture,
        binding_replacements,
    }
}

fn inherits_fixture(document: &Document, element: &Node, root: NodeId) -> bool {
    let mut current = element;
    while current.id != root {
        let owns_context = current.attributes.iter().any(|attribute| {
            let name = attribute.name.rsplit(':').next().unwrap_or("");
            let namespace = attribute.namespace_uri.as_deref();
            (name == "DataContext" || name == "Design.DataContext")
                && (namespace.is_none()
                    || namespace == current.namespace_uri.as_deref()
                    || namespace == Some(crate::design_time::DESIGN_NAMESPACE))
        }) || document.ast().element_children(current.id).into_iter().any(
            |id| {
                document
                    .ast()
                    .node(id)
                    .is_some_and(|child| child.local_name().ends_with(".DataContext"))
            },
        );
        if owns_context {
            return false;
        }
        let Some(parent) = current.parent.and_then(|id| document.ast().node(id)) else {
            return false;
        };
        current = parent;
    }
    true
}

fn simple_path(path: &str) -> bool {
    path.len() <= 1024
        && path.split('.').count() <= MAX_PATH_DEPTH
        && path.split('.').all(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .is_some_and(|character| character.is_alphabetic() || character == '_')
                && characters.all(|character| character.is_alphanumeric() || character == '_')
        })
}

fn property_kind(
    document: &Document,
    node: &Node,
    attribute: &ferroui_designer_core::Attribute,
) -> SampleKind {
    let property = attribute.name.rsplit(':').next().unwrap_or(&attribute.name);
    let native = if let Some((owner, property)) = property.split_once('.') {
        let mut declared = node.clone();
        declared.name = owner.into();
        declared.namespace_uri = attribute
            .namespace_uri
            .clone()
            .or_else(|| document.ast().namespaces(node.id).get("").cloned());
        crate::design_time::native_type_for_node(&declared).and_then(|type_| {
            let registry = FerroPropertyRegistry::instance();
            let ordinary = registry.get_registered(type_);
            let attached = registry.get_registered_attached(type_);
            ordinary
                .iter()
                .chain(attached.iter())
                .copied()
                .find(|candidate| candidate.name() == property)
        })
    } else {
        crate::design_time::native_type_for_node(node)
            .and_then(|type_| FerroPropertyRegistry::instance().find_registered(type_, property))
    };
    let type_id = native.map(|property| property.property_type());
    let is = |type_| type_id == Some(type_);
    if is(std::any::TypeId::of::<bool>()) || is(std::any::TypeId::of::<Option<bool>>()) {
        SampleKind::Boolean
    } else if [
        std::any::TypeId::of::<u8>(),
        std::any::TypeId::of::<u16>(),
        std::any::TypeId::of::<u32>(),
        std::any::TypeId::of::<u64>(),
        std::any::TypeId::of::<i8>(),
        std::any::TypeId::of::<i16>(),
        std::any::TypeId::of::<i32>(),
        std::any::TypeId::of::<i64>(),
        std::any::TypeId::of::<f32>(),
        std::any::TypeId::of::<f64>(),
        std::any::TypeId::of::<usize>(),
        std::any::TypeId::of::<isize>(),
    ]
    .into_iter()
    .any(is)
    {
        SampleKind::Number
    } else {
        SampleKind::Text
    }
}

impl SampleRequest {
    pub fn value(&self) -> BoxedValue {
        register_accessors();
        if let Some(fixture) = &self.fixture {
            Rc::new(fixture.clone())
        } else if self.property == "DataContext" {
            self.record(0)
        } else {
            Rc::new(Some(ferroui_controls::ItemsSource::from_items(
                (0..self.count).map(|row| Some(self.record(row))),
            )))
        }
    }
    fn record(&self, row: usize) -> BoxedValue {
        if self.fields.is_empty() {
            return Rc::new(format!("Sample item {}", row + 1));
        }
        let mut record = DesignRecord::new();
        for (path, kind) in &self.fields {
            let parts = path.split('.').collect::<Vec<_>>();
            let value = match kind {
                SampleKind::Boolean => SampleValue::Boolean(row.is_multiple_of(2)),
                SampleKind::Number => SampleValue::Number((row + 1) as f64 * 10.0),
                SampleKind::Text => SampleValue::Text(format!(
                    "{} {}",
                    display_name(parts.last().unwrap()),
                    row + 1
                )),
            };
            record.insert(&parts, value);
        }
        Rc::new(record)
    }
}

pub(crate) fn apply(visual: &Ref<Visual>, sample: &SampleRequest) -> Result<(), String> {
    let value = sample.value();
    if sample.property == "DataContext" {
        let element = visual
            .cast::<StyledElement>()
            .ok_or("Design sample data requires a styled element.")?;
        element.set_data_context(Some(value));
        return Ok(());
    }
    let property = FerroPropertyRegistry::instance()
        .find_registered(visual.get_type(), &sample.property)
        .ok_or_else(|| {
            format!(
                "{} has no registered {} property for sample data.",
                visual.get_type().name(),
                sample.property
            )
        })?;
    let target = ValueType::new(property.property_type(), property.property_type_name());
    let converted = ValueTypes::try_convert(Some(&value), target)
        .flatten()
        .ok_or_else(|| {
            format!(
                "{} needs explicit design-time items for its custom collection type.",
                visual.get_type().name()
            )
        })?;
    visual.set_value_untyped(property, converted.as_any(), BindingPriority::LocalValue);
    Ok(())
}

/// State belongs to one accepted native graph. Newly realized template instances are
/// sampled once when the designer refreshes its runtime map; existing instances keep
/// their temporary interaction state. The global budget also bounds nested collections.
pub(crate) struct SampleState {
    applied: BTreeSet<(u64, usize)>,
    remaining: usize,
    budget_notice: bool,
}
impl Default for SampleState {
    fn default() -> Self {
        Self {
            applied: BTreeSet::new(),
            remaining: 4096,
            budget_notice: false,
        }
    }
}

pub(crate) fn apply_projection(
    root: &Ref<Control>,
    projection: &crate::design_time::DesignTimeProjection,
    state: &mut SampleState,
) -> Vec<String> {
    if projection.samples.is_empty() {
        return Vec::new();
    }
    let root_value: BoxedValue = Rc::new(root.clone());
    let provenance = XamlSourceInfo::get_xaml_source_info(&root_value);
    let mut targets = BTreeMap::<(String, usize), Vec<(usize, NodeId)>>::new();
    let mut positions = projection
        .document
        .ast()
        .elements()
        .map(|node| (node.span.start, node.id, node.local_name().to_owned()))
        .collect::<Vec<_>>();
    let mut by_node = BTreeMap::<NodeId, Vec<usize>>::new();
    for (index, sample) in projection.samples.iter().enumerate() {
        by_node.entry(sample.node).or_default().push(index);
    }
    positions.sort_by_key(|(offset, _, _)| *offset);
    let (mut offset, mut line, mut column) = (0usize, 1usize, 1usize);
    for (start, index, type_name) in positions {
        for character in projection.document.valid_source()[offset..start].chars() {
            if character == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        offset = start;
        targets
            .entry((type_name, line))
            .or_default()
            .push((column, index));
    }
    let mut notices = Vec::new();
    let inspector = Inspector::shared();
    let root_id = inspector.borrow_mut().node_id(root);
    for _ in 0..16 {
        let mut pending = vec![root.clone().upcast::<FerroObject>()];
        let mut visited = BTreeSet::new();
        let mut changed = false;
        while let Some(object) = pending.pop() {
            let object_id = inspector.borrow_mut().node_id(&object);
            if !visited.insert(object_id) {
                continue;
            }
            for tree in [TreeKind::Logical, TreeKind::Visual] {
                pending.extend(inspector.borrow().children(&object, tree));
            }
            let Some(visual) = object.cast::<Visual>() else {
                continue;
            };
            let value: BoxedValue = Rc::new(object.clone());
            let info = XamlSourceInfo::get_xaml_source_info(&value);
            if info
                .as_ref()
                .zip(provenance.as_ref())
                .is_some_and(|(info, root)| info.source_uri() != root.source_uri())
            {
                continue;
            }
            let indices = if object_id == root_id {
                projection
                    .samples
                    .iter()
                    .enumerate()
                    .filter_map(|(index, sample)| {
                        (sample.node == projection.document.ast().root()).then_some(index)
                    })
                    .collect::<Vec<_>>()
            } else if let Some(info) = info {
                targets
                    .get(&(
                        object.get_type().name().to_owned(),
                        info.line_number().max(1) as usize,
                    ))
                    .map(|candidates| {
                        let nearest = candidates
                            .iter()
                            .map(|(column, _)| {
                                column.abs_diff(info.line_position().max(1) as usize)
                            })
                            .min();
                        candidates
                            .iter()
                            .filter(|(column, _)| {
                                Some(column.abs_diff(info.line_position().max(1) as usize))
                                    == nearest
                            })
                            .flat_map(|(_, node)| by_node.get(node).into_iter().flatten().copied())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            for index in indices {
                if !state.applied.insert((object_id, index)) {
                    continue;
                }
                let mut sample = projection.samples[index].clone();
                let requested = if sample.property == "DataContext" {
                    1
                } else {
                    sample.count
                };
                if requested > state.remaining {
                    if !state.budget_notice {
                        notices.push("Nested sample data reached the 4096-record preview limit; remaining collections are truncated.".into());
                        state.budget_notice = true;
                    }
                    if sample.property == "DataContext" {
                        continue;
                    }
                    sample.count = state.remaining;
                }
                state.remaining = state.remaining.saturating_sub(requested);
                if let Err(error) = apply(&visual, &sample) {
                    notices.push(error);
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
        root.update_layout();
    }
    notices
}
fn display_name(name: &str) -> String {
    let mut result = String::new();
    for (index, character) in name.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            result.push(' ');
        }
        result.push(character);
    }
    result
}
