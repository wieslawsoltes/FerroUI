//! Source-backed independent animation programs. Identity is the document's stable Animation
//! syntax ID; display names are ordinary XML comments, so generated markup needs no designer
//! runtime types and unknown payloads remain fully editable.
use crate::{require, AnimationError, AnimationImport, ImportOptions, Result, Timeline};
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const NATIVE_XMLNS: &str = "https://github.com/ferroui";
const NAME_MARKER: &str = "ferroui-designer-program:";
const ID_MARKER: &str = "ferroui-designer-program-id:";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationProgram {
    pub id: NodeId,
    /// Persisted UUID for source/collaboration identity; legacy source is stamped on first edit.
    #[serde(default)]
    pub source_key: Option<String>,
    pub name: String,
    pub owner: NodeId,
    pub style: Option<NodeId>,
    pub selector: String,
    pub targets: Vec<NodeId>,
    pub timeline: Option<Timeline>,
    pub codec: Option<String>,
    pub diagnostics: Vec<String>,
    pub source: String,
    pub span: SourceSpan,
}
impl AnimationProgram {
    pub fn primary_target(&self) -> NodeId {
        self.targets.first().copied().unwrap_or(self.owner)
    }
}

/// A custom codec owns the complete payload it recognizes. `encode` receives the original
/// source, allowing a plugin to retain custom animator configuration while changing keyframes.
/// An unsuccessful decode never changes the document or removes the opaque program.
pub trait AnimationCodec {
    fn id(&self) -> &str;
    fn can_decode(&self, source: &str) -> bool;
    fn decode(
        &self,
        source: &str,
        target: NodeId,
        options: &ImportOptions,
    ) -> Result<AnimationImport>;
    fn encode(&self, original: &str, timeline: &Timeline, target: NodeId) -> Result<String>;
}
pub struct AnimationCodecRegistry {
    codecs: Vec<Box<dyn AnimationCodec>>,
}
impl Default for AnimationCodecRegistry {
    fn default() -> Self {
        Self {
            codecs: vec![Box::new(NativeCodec)],
        }
    }
}
impl AnimationCodecRegistry {
    pub fn register(&mut self, codec: Box<dyn AnimationCodec>) {
        self.codecs.retain(|existing| existing.id() != codec.id());
        self.codecs.insert(0, codec);
    }
    pub fn codec_ids(&self) -> impl Iterator<Item = &str> {
        self.codecs.iter().map(|codec| codec.id())
    }
    pub fn decode(&self, program: &mut AnimationProgram, source: &str, options: &ImportOptions) {
        program.timeline = None;
        program.codec = None;
        let mut failures = Vec::new();
        for codec in &self.codecs {
            if !codec.can_decode(source) {
                continue;
            }
            match codec.decode(source, program.primary_target(), options) {
                Ok(import) => {
                    program.timeline = Some(import.timeline);
                    program.codec = Some(codec.id().to_owned());
                    program.diagnostics.extend(import.warnings);
                    return;
                }
                Err(error) => failures.push(format!("{}: {error}", codec.id())),
            }
        }
        program.diagnostics.extend(failures);
        if program.diagnostics.is_empty() {
            program.diagnostics.push(
                "No animation codec recognizes this payload; its source remains editable".into(),
            );
        }
    }
    pub fn encode(
        &self,
        program: &AnimationProgram,
        original: &str,
        timeline: &Timeline,
    ) -> Result<String> {
        let id = program.codec.as_deref().ok_or_else(|| {
            AnimationError("Select a codec or edit this opaque program's source".into())
        })?;
        let codec = self
            .codecs
            .iter()
            .find(|codec| codec.id() == id)
            .ok_or_else(|| AnimationError(format!("Animation codec {id} is not registered")))?;
        let generated = codec.encode(original, timeline, program.primary_target())?;
        let original_document = Document::new(original).map_err(core_error)?;
        let root = original_document
            .ast()
            .node(original_document.ast().root())
            .expect("root");
        let name = root
            .children
            .iter()
            .filter_map(|id| original_document.ast().node(*id))
            .filter(|node| node.kind == NodeKind::Comment)
            .find_map(|node| {
                node.text
                    .as_deref()
                    .and_then(|text| text.trim().strip_prefix(NAME_MARKER))
                    .and_then(decode_text)
            });
        let generated = if let Some(name) = name {
            named_source(&generated, &name)?
        } else {
            generated
        };
        let generated = stamp_identity(
            &generated,
            source_identity(&original_document, root.id).as_deref(),
            false,
        )?;
        let mut generated = Document::new(generated).map_err(core_error)?;
        if let Some(role) = crate::states::state_program_role(&original_document, root.id) {
            let target = generated.ast().root();
            if crate::states::state_program_role(&generated, target).is_none() {
                generated
                    .apply(
                        "Preserve state program ownership",
                        vec![Edit::InsertChild {
                            parent: target,
                            index: 0,
                            source: format!("<!--{}{role}-->", crate::states::STATE_PROGRAM_MARKER),
                        }],
                    )
                    .map_err(core_error)?;
            }
        }
        Ok(generated.source().into())
    }
}
struct NativeCodec;
impl AnimationCodec for NativeCodec {
    fn id(&self) -> &str {
        "ferroui.native"
    }
    fn can_decode(&self, source: &str) -> bool {
        roxmltree::Document::parse(source).is_ok_and(|document| {
            let root = document.root_element();
            root.tag_name().name() == "Animation"
                && crate::xaml::native_namespace(root.tag_name().namespace())
        })
    }
    fn decode(
        &self,
        source: &str,
        target: NodeId,
        options: &ImportOptions,
    ) -> Result<AnimationImport> {
        let mut document = Document::new(source).map_err(core_error)?;
        let root = document.ast().node(document.ast().root()).expect("root");
        let edits = root
            .attributes
            .iter()
            .filter(|attribute| xaml_annotation(attribute))
            .map(|attribute| Edit::RemoveAttribute {
                node: root.id,
                name: attribute.name.clone(),
            })
            .collect::<Vec<_>>();
        document
            .apply("Read animation identity", edits)
            .map_err(core_error)?;
        Timeline::import_xaml(document.source(), target, options)
    }
    fn encode(&self, original: &str, timeline: &Timeline, target: NodeId) -> Result<String> {
        let mut generated = timeline.to_xaml(target)?;
        // Timeline edits may normalize keyframe formatting, but authored comments are retained.
        if let Ok(document) = Document::new(original) {
            let comments = document
                .ast()
                .nodes()
                .filter(|node| node.kind == NodeKind::Comment)
                .map(|node| &original[node.span.start..node.span.end])
                .collect::<Vec<_>>()
                .join("\n");
            if !comments.is_empty() {
                let offset = generated.find('>').expect("generated XML") + 1;
                generated.insert_str(offset, &format!("\n{comments}"));
            }
        }
        let original_document = Document::new(original).map_err(core_error)?;
        let original_root = original_document
            .ast()
            .node(original_document.ast().root())
            .expect("root");
        let mut output = Document::new(generated).map_err(core_error)?;
        let edits = original_root
            .attributes
            .iter()
            .filter(|attribute| attribute.is_namespace() || xaml_annotation(attribute))
            .map(|attribute| Edit::SetAttribute {
                node: output.ast().root(),
                name: attribute.name.clone(),
                value: attribute.value.clone(),
            })
            .collect::<Vec<_>>();
        output
            .apply("Retain animation identity", edits)
            .map_err(core_error)?;
        Ok(output.source().into())
    }
}
fn xaml_identity(attribute: &ferroui_designer_core::Attribute) -> bool {
    attribute.namespace_uri.as_deref() == Some("http://schemas.microsoft.com/winfx/2006/xaml")
        && matches!(attribute.name.rsplit(':').next(), Some("Key" | "Name"))
}

fn xaml_annotation(attribute: &ferroui_designer_core::Attribute) -> bool {
    xaml_identity(attribute)
        || (attribute.namespace_uri.as_deref()
            == Some("http://schemas.microsoft.com/winfx/2006/xaml")
            && attribute.name.rsplit(':').next() == Some("SetterTargetType"))
}

pub struct AnimationPrograms;
impl AnimationPrograms {
    /// Enumerate every Animation, including resource, nested-style and custom payload programs.
    /// Hosts resolve selectors against their live tree and then invoke the codec registry.
    pub fn scan(document: &Document) -> Vec<AnimationProgram> {
        let mut nodes = document
            .ast()
            .elements()
            .filter(|node| node.local_name() == "Animation")
            .collect::<Vec<_>>();
        nodes.sort_by_key(|node| node.span.start);
        nodes
            .into_iter()
            .map(|node| {
                let mut style = None;
                let mut owner = document.ast().root();
                let mut current = node.parent;
                let mut animation_collection = false;
                let mut resource = false;
                while let Some(id) = current {
                    let Some(parent) = document.ast().node(id) else {
                        break;
                    };
                    if parent.local_name() == "Style.Animations" {
                        animation_collection = true;
                    }
                    if parent.local_name().ends_with(".Resources") {
                        resource = true;
                        if let Some(host) = parent
                            .parent
                            .and_then(|id| document.ast().node(id))
                            .filter(|node| {
                                !matches!(
                                    node.local_name(),
                                    "Style" | "Styles" | "ControlTheme" | "ResourceDictionary"
                                )
                            })
                        {
                            owner = host.id;
                            break;
                        }
                    }
                    if style.is_none()
                        && animation_collection
                        && !resource
                        && parent.local_name() == "Style"
                    {
                        style = Some(id);
                    }
                    if parent.local_name().ends_with(".Styles") {
                        if let Some(id) = parent.parent {
                            owner = id;
                        }
                        break;
                    }
                    current = parent.parent;
                }
                let selector = style
                    .and_then(|id| document.ast().node(id))
                    .and_then(|node| node.attribute("Selector"))
                    .unwrap_or("")
                    .to_owned();
                let name = node
                    .children
                    .iter()
                    .filter_map(|id| document.ast().node(*id))
                    .find_map(|child| {
                        child
                            .text
                            .as_deref()
                            .and_then(|text| text.trim().strip_prefix(NAME_MARKER))
                            .and_then(decode_text)
                    })
                    .or_else(|| {
                        node.attributes
                            .iter()
                            .find(|attribute| xaml_identity(attribute))
                            .map(|attribute| attribute.value.clone())
                    })
                    .unwrap_or_else(|| format!("Animation {}", node.id));
                AnimationProgram {
                    id: node.id,
                    source_key: source_identity(document, node.id),
                    name,
                    owner,
                    style,
                    selector,
                    targets: Vec::new(),
                    timeline: None,
                    codec: None,
                    diagnostics: Vec::new(),
                    source: document.valid_source()[node.span.start..node.span.end].into(),
                    span: node.span,
                }
            })
            .collect()
    }
    pub fn create(
        document: &Document,
        owner: NodeId,
        name: &str,
        selector: &str,
        timeline: &Timeline,
    ) -> Result<Vec<Edit>> {
        validate_name(name)?;
        require(
            !selector.trim().is_empty(),
            "An animation program needs a selector",
        )?;
        let source = named_source(&timeline.to_xaml(owner)?, name)?;
        let style = format!(
            "<Style Selector=\"{}\"><Style.Animations>{source}</Style.Animations></Style>",
            escape(selector)
        );
        insert_styles(document, owner, &style)
    }
    pub fn remove(document: &Document, id: NodeId) -> Result<Vec<Edit>> {
        animation_node(document, id)?;
        Ok(vec![Edit::RemoveNode { node: id }])
    }
    pub fn rename(document: &Document, id: NodeId, name: &str) -> Result<Vec<Edit>> {
        validate_name(name)?;
        let node = animation_node(document, id)?;
        let comment = format!("<!--{NAME_MARKER}{}-->", encode_text(name));
        let edit = if let Some(marker) = node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find(|node| {
                node.kind == NodeKind::Comment
                    && node
                        .text
                        .as_deref()
                        .is_some_and(|text| text.trim().starts_with(NAME_MARKER))
            }) {
            Edit::Text {
                span: marker.span,
                replacement: comment,
            }
        } else if node.self_closing {
            Edit::Text {
                span: SourceSpan::new(node.opening_span.end - 2, node.opening_span.end),
                replacement: format!(">{comment}</{}>", node.name),
            }
        } else {
            Edit::Text {
                span: SourceSpan::new(node.opening_span.end, node.opening_span.end),
                replacement: comment,
            }
        };
        let mut edits = vec![edit];
        if source_identity(document, id).is_none() {
            let mut staged = document.clone();
            staged
                .apply("Name animation", edits.clone())
                .map_err(core_error)?;
            let node = animation_node(&staged, id)?;
            edits.push(Edit::Text {
                span: SourceSpan::new(node.opening_span.end, node.opening_span.end),
                replacement: format!("<!--{ID_MARKER}{}-->", uuid::Uuid::new_v4()),
            });
        }
        Ok(edits)
    }
    pub fn duplicate(document: &Document, id: NodeId, name: &str) -> Result<Vec<Edit>> {
        let node = animation_node(document, id)?;
        let parent = node.parent.ok_or_else(|| {
            AnimationError("Cannot duplicate a document-root program in place".into())
        })?;
        let source = stamp_identity(&named_source(&fragment(document, id)?, name)?, None, true)?;
        let mut copy = Document::new(source).map_err(core_error)?;
        let ownership = copy
            .ast()
            .node(copy.ast().root())
            .expect("root")
            .children
            .iter()
            .filter_map(|id| copy.ast().node(*id))
            .filter(|node| {
                node.kind == NodeKind::Comment
                    && node.text.as_deref().is_some_and(|text| {
                        text.trim().starts_with(crate::states::STATE_PROGRAM_MARKER)
                    })
            })
            .map(|node| Edit::RemoveNode { node: node.id })
            .collect();
        copy.apply("Make duplicate program independent", ownership)
            .map_err(core_error)?;
        let mut attributes = Vec::new();
        for attribute in node
            .attributes
            .iter()
            .filter(|attribute| xaml_identity(attribute))
        {
            let key = attribute.name.rsplit(':').next() == Some("Key");
            let existing = document
                .ast()
                .elements()
                .filter(|candidate| !key || candidate.parent == Some(parent))
                .flat_map(|candidate| candidate.attributes.iter())
                .filter(|candidate| {
                    xaml_identity(candidate)
                        && candidate.name.rsplit(':').next() == attribute.name.rsplit(':').next()
                })
                .map(|attribute| attribute.value.as_str())
                .collect::<BTreeSet<_>>();
            let stem = if key {
                name.to_owned()
            } else {
                format!("{}Copy", attribute.value)
            };
            let mut value = stem.clone();
            let mut suffix = 1;
            while existing.contains(value.as_str()) {
                value = format!("{stem}{suffix}");
                suffix += 1;
            }
            if value.starts_with('{') {
                value.insert_str(0, "{}");
            }
            attributes.push(Edit::SetAttribute {
                node: copy.ast().root(),
                name: attribute.name.clone(),
                value,
            });
        }
        copy.apply("Allocate copied runtime keys", attributes)
            .map_err(core_error)?;
        let source = copy.source().to_owned();
        let index = document
            .ast()
            .element_children(parent)
            .iter()
            .position(|existing| *existing == id)
            .unwrap_or(0)
            + 1;
        Ok(vec![Edit::InsertChild {
            parent,
            index,
            source,
        }])
    }
    pub fn replace_source(document: &Document, id: NodeId, source: &str) -> Result<Vec<Edit>> {
        let node = animation_node(document, id)?;
        validate_fragment(document, id, source, Some("Animation"))?;
        // The supplied fragment may rely on inherited prefixes; wrap it only for the source-ID patch.
        let declarations = document
            .ast()
            .namespaces(id)
            .into_iter()
            .map(|(prefix, uri)| {
                format!(
                    " {}=\"{}\"",
                    if prefix.is_empty() {
                        "xmlns".into()
                    } else {
                        format!("xmlns:{prefix}")
                    },
                    escape(&uri)
                )
            })
            .collect::<String>();
        let wrapper = format!("<DesignerFragment{declarations}>");
        let mut parsed =
            Document::new(format!("{wrapper}{source}</DesignerFragment>")).map_err(core_error)?;
        let child = parsed.ast().element_children(parsed.ast().root())[0];
        if let Some(role) = crate::states::state_program_role(document, id) {
            if crate::states::state_program_role(&parsed, child).is_none() {
                parsed
                    .apply(
                        "Preserve state ownership",
                        vec![Edit::InsertChild {
                            parent: child,
                            index: 0,
                            source: format!("<!--{}{role}-->", crate::states::STATE_PROGRAM_MARKER),
                        }],
                    )
                    .map_err(core_error)?;
            }
        }
        let source = stamp_identity_at(
            &parsed,
            child,
            source_identity(document, id).as_deref(),
            false,
        )?;
        Ok(vec![Edit::Text {
            span: node.span,
            replacement: source,
        }])
    }
    /// Isolate this program in its own sibling style before changing a shared style selector.
    /// MoveNode pins the original Animation identity; existing setters and other animations stay.
    pub fn set_selector(document: &Document, id: NodeId, selector: &str) -> Result<Vec<Edit>> {
        require(
            !selector.trim().is_empty(),
            "An animation program needs a selector",
        )?;
        if source_identity(document, id).is_none() {
            let node = animation_node(document, id)?;
            let mut edits = Self::replace_source(
                document,
                id,
                &document.valid_source()[node.span.start..node.span.end],
            )?;
            let mut staged = document.clone();
            staged
                .apply("Stamp animation identity", edits.clone())
                .map_err(core_error)?;
            edits.extend(Self::set_selector(&staged, id, selector)?);
            return Ok(edits);
        }
        let program = Self::scan(document)
            .into_iter()
            .find(|program| program.id == id)
            .ok_or_else(|| AnimationError("Unknown animation program".into()))?;
        let style_id = program.style.ok_or_else(|| {
            AnimationError(
                "Move this resource animation into a Style before assigning a selector".into(),
            )
        })?;
        let style = document.ast().node(style_id).expect("style");
        let animations = style
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find(|node| node.local_name().ends_with(".Animations"));
        let simple = animations
            .is_some_and(|animations| document.ast().element_children(animations.id) == [id])
            && document
                .ast()
                .element_children(style_id)
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .all(|node| {
                    node.local_name().ends_with(".Animations")
                        || node.local_name().ends_with(".Resources")
                });
        if simple {
            return Ok(vec![Edit::SetAttribute {
                node: style_id,
                name: "Selector".into(),
                value: selector.into(),
            }]);
        }
        let parent = style.parent.ok_or_else(|| {
            AnimationError("A root Style cannot be split; create a Styles container first".into())
        })?;
        let resources = style
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .filter(|node| node.local_name().ends_with(".Resources"))
            .map(|node| fragment(document, node.id))
            .collect::<Result<Vec<_>>>()?
            .join("");
        let source = native_fragment(
            document,
            parent,
            &format!(
                "<Style Selector=\"{}\">{resources}<Style.Animations/></Style>",
                escape(selector)
            ),
        )?;
        let insert = Edit::InsertChild {
            parent,
            index: document
                .ast()
                .element_children(parent)
                .iter()
                .position(|node| *node == style_id)
                .unwrap_or(0)
                + 1,
            source,
        };
        let previous = document
            .ast()
            .nodes()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();
        let mut candidate = document.clone();
        candidate
            .apply("Stage isolated animation style", vec![insert.clone()])
            .map_err(core_error)?;
        let destination = candidate
            .ast()
            .elements()
            .find(|node| !previous.contains(&node.id) && node.local_name().ends_with(".Animations"))
            .ok_or_else(|| AnimationError("New animation collection is missing".into()))?
            .id;
        Ok(vec![
            insert,
            Edit::MoveNode {
                node: id,
                parent: destination,
                index: 0,
            },
        ])
    }
}

fn source_identity(document: &Document, id: NodeId) -> Option<String> {
    document
        .ast()
        .node(id)?
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| node.kind == NodeKind::Comment)
        .find_map(|node| {
            node.text
                .as_deref()
                .and_then(|text| text.trim().strip_prefix(ID_MARKER))
                .and_then(|value| uuid::Uuid::parse_str(value).ok())
                .map(|id| id.to_string())
        })
}
fn stamp_identity(source: &str, preferred: Option<&str>, fresh: bool) -> Result<String> {
    let document = Document::new(source).map_err(core_error)?;
    stamp_identity_at(&document, document.ast().root(), preferred, fresh)
}
fn stamp_identity_at(
    document: &Document,
    id: NodeId,
    preferred: Option<&str>,
    fresh: bool,
) -> Result<String> {
    let node = animation_node(document, id)?;
    let identity = if fresh {
        uuid::Uuid::new_v4().to_string()
    } else {
        preferred
            .map(str::to_owned)
            .or_else(|| source_identity(document, id))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    };
    let comment = format!("<!--{ID_MARKER}{identity}-->");
    let mut source = document.valid_source()[node.span.start..node.span.end].to_owned();
    if let Some(marker) = node
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .find(|node| {
            node.kind == NodeKind::Comment
                && node
                    .text
                    .as_deref()
                    .is_some_and(|text| text.trim().starts_with(ID_MARKER))
        })
    {
        source.replace_range(
            marker.span.start - node.span.start..marker.span.end - node.span.start,
            &comment,
        );
    } else if node.self_closing {
        source.replace_range(
            node.opening_span.end - node.span.start - 2..node.opening_span.end - node.span.start,
            &format!(">{comment}</{}>", node.name),
        );
    } else {
        source.insert_str(node.opening_span.end - node.span.start, &comment);
    }
    Ok(source)
}

pub(crate) fn animation_node(
    document: &Document,
    id: NodeId,
) -> Result<&ferroui_designer_core::Node> {
    let node = document
        .ast()
        .node(id)
        .ok_or_else(|| AnimationError("Unknown animation program".into()))?;
    require(
        node.local_name() == "Animation",
        "Expected an Animation source node",
    )?;
    Ok(node)
}
pub(crate) fn core_error(error: impl std::fmt::Display) -> AnimationError {
    AnimationError(error.to_string())
}
pub(crate) fn validate_name(value: &str) -> Result<()> {
    require(
        !value.trim().is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control),
        "A name must be nonempty, at most 1024 bytes, and contain no control characters",
    )
}
pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}
pub(crate) fn encode_text(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(crate) fn decode_text(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = value
        .as_bytes()
        .chunks(2)
        .map(|bytes| {
            std::str::from_utf8(bytes)
                .ok()
                .and_then(|text| u8::from_str_radix(text, 16).ok())
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}
pub(crate) fn named_source(source: &str, name: &str) -> Result<String> {
    validate_name(name)?;
    let mut document = Document::new(source).map_err(core_error)?;
    let edits = AnimationPrograms::rename(&document, document.ast().root(), name)?;
    document
        .apply("Name animation", edits)
        .map_err(core_error)?;
    Ok(document.source().into())
}
/// Extract a standalone XML subtree with its inherited namespace declarations intact.
pub fn fragment(document: &Document, id: NodeId) -> Result<String> {
    let node = document
        .ast()
        .node(id)
        .ok_or_else(|| AnimationError("Unknown source node".into()))?;
    let mut source = document.valid_source()[node.span.start..node.span.end].to_owned();
    let mut declarations = String::new();
    for (prefix, uri) in document.ast().namespaces(id) {
        let name = if prefix.is_empty() {
            "xmlns".into()
        } else {
            format!("xmlns:{prefix}")
        };
        if node.attribute(&name).is_none() {
            declarations.push_str(&format!(" {name}=\"{}\"", escape(&uri)));
        }
    }
    source.insert_str(node.name_span.end - node.span.start, &declarations);
    Ok(source)
}
pub(crate) fn validate_fragment(
    document: &Document,
    at: NodeId,
    source: &str,
    root: Option<&str>,
) -> Result<()> {
    let declarations = document
        .ast()
        .namespaces(at)
        .into_iter()
        .map(|(prefix, uri)| {
            format!(
                " {}=\"{}\"",
                if prefix.is_empty() {
                    "xmlns".into()
                } else {
                    format!("xmlns:{prefix}")
                },
                escape(&uri)
            )
        })
        .collect::<String>();
    let parsed = Document::new(format!(
        "<DesignerFragment{declarations}>{source}</DesignerFragment>"
    ))
    .map_err(core_error)?;
    let children = parsed.ast().element_children(parsed.ast().root());
    require(
        children.len() == 1,
        "Provide exactly one XML object subtree",
    )?;
    if let Some(root) = root {
        require(
            parsed.ast().node(children[0]).expect("child").local_name() == root,
            format!("Expected {root} markup"),
        )?;
    }
    Ok(())
}
/// Qualify generated native markup using the source's namespace prefix, without rewriting it.
pub fn native_fragment(document: &Document, at: NodeId, source: &str) -> Result<String> {
    let namespaces = document.ast().namespaces(at);
    if namespaces.get("").is_some_and(|uri| uri == NATIVE_XMLNS) {
        return Ok(source.into());
    }
    // Parse in an inherited namespace context. Repeated composition (an already-prefixed
    // behavior inside a new property element) must not require declarations on every object.
    let declarations = namespaces
        .iter()
        .filter(|(prefix, _)| !prefix.is_empty())
        .map(|(prefix, uri)| format!(" xmlns:{prefix}=\"{}\"", escape(uri)))
        .collect::<String>();
    let wrapper = format!("<DesignerFragment xmlns=\"{NATIVE_XMLNS}\"{declarations}>");
    let parsed =
        Document::new(format!("{wrapper}{source}</DesignerFragment>")).map_err(core_error)?;
    let roots = parsed.ast().element_children(parsed.ast().root());
    require(
        roots.len() == 1,
        "Provide exactly one generated XML subtree",
    )?;
    let Some((prefix, _)) = namespaces
        .iter()
        .find(|(_, uri)| uri.as_str() == NATIVE_XMLNS)
    else {
        let root = parsed.ast().node(roots[0]).expect("root");
        let mut output = source.to_owned();
        if !root.name.contains(':')
            && root.namespace_uri.as_deref() == Some(NATIVE_XMLNS)
            && root.attribute("xmlns").is_none()
        {
            output.insert_str(
                root.name_span.end - wrapper.len(),
                &format!(" xmlns=\"{NATIVE_XMLNS}\""),
            );
        }
        return Ok(output);
    };
    let mut patches = Vec::new();
    for node in parsed.ast().elements().filter(|node| {
        node.id != parsed.ast().root()
            && !node.name.contains(':')
            && node.namespace_uri.as_deref() == Some(NATIVE_XMLNS)
    }) {
        patches.push((
            SourceSpan::new(
                node.name_span.start - wrapper.len(),
                node.name_span.end - wrapper.len(),
            ),
            format!("{prefix}:{}", node.name),
        ));
        if let Some(closing) = node.closing_span {
            patches.push((
                SourceSpan::new(
                    closing.start + 2 - wrapper.len(),
                    closing.start + 2 + node.name.len() - wrapper.len(),
                ),
                format!("{prefix}:{}", node.name),
            ));
        }
    }
    patches.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut output = source.to_owned();
    for (span, text) in patches {
        output.replace_range(span.start..span.end, &text);
    }
    Ok(output)
}
pub(crate) fn insert_styles(document: &Document, owner: NodeId, source: &str) -> Result<Vec<Edit>> {
    let node = document
        .ast()
        .node(owner)
        .ok_or_else(|| AnimationError("Unknown style owner".into()))?;
    let source = native_fragment(document, owner, source)?;
    if node.local_name() == "Styles" || node.local_name().ends_with(".Styles") {
        return Ok(vec![Edit::InsertChild {
            parent: owner,
            index: document.ast().element_children(owner).len(),
            source,
        }]);
    }
    if let Some(styles) = node
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .find(|node| node.local_name().ends_with(".Styles"))
    {
        Ok(vec![Edit::InsertChild {
            parent: styles.id,
            index: document.ast().element_children(styles.id).len(),
            source,
        }])
    } else {
        Ok(vec![Edit::InsertChild {
            parent: owner,
            index: 0,
            source: format!("<{}.Styles>{source}</{}.Styles>", node.name, node.name),
        }])
    }
}

/// Allocate fresh persistent animation keys in a copied subtree. Ordinary renames must not
/// call this: keys intentionally survive named-reference and source formatting changes.
pub fn rekey_copied_programs(source: &str) -> Result<String> {
    let mut document = Document::new(source).map_err(core_error)?;
    let mut edits = document
        .ast()
        .nodes()
        .filter(|node| {
            node.kind == NodeKind::Comment
                && node
                    .text
                    .as_deref()
                    .is_some_and(|text| text.trim().starts_with(ID_MARKER))
        })
        .map(|node| Edit::Text {
            span: node.span,
            replacement: format!("<!--{ID_MARKER}{}-->", uuid::Uuid::new_v4()),
        })
        .collect::<Vec<_>>();
    edits.sort_by_key(|edit| match edit {
        Edit::Text { span, .. } => std::cmp::Reverse(span.start),
        _ => std::cmp::Reverse(0),
    });
    document
        .apply("Allocate copied animation identities", edits)
        .map_err(core_error)?;
    Ok(document.source().into())
}

pub(crate) fn subtree(document: &Document, root: NodeId) -> Vec<&ferroui_designer_core::Node> {
    let mut pending = vec![root];
    let mut nodes = Vec::new();
    while let Some(id) = pending.pop() {
        if let Some(node) = document.ast().node(id) {
            nodes.push(node);
            pending.extend(node.children.iter().rev());
        }
    }
    nodes
}
