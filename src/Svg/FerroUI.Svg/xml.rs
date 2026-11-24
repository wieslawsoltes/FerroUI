//! Deterministic, conservative processing of the SVG emitted by Skia.
//!
//! This module deliberately does not flatten transforms, merge paths, change
//! fill rules, or reconstruct text. Those operations can change rendering or
//! destroy the structure designers need to edit. Resource interning compares
//! complete definitions and their inherited context, with equality checks
//! after hashing. Numerical rounding is explicit and never touches path data
//! or transformation matrices.

use roxmltree::{Document, Node, ParsingOptions};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};
use std::hash::{Hash, Hasher};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
const MAX_DEPTH: usize = 256;
const MAX_NODES: u32 = 2_000_000;

/// Serialization and lossless resource-optimization controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlOptions {
    /// Optional decimal places for scalar and coordinate-list attributes.
    /// `None` retains their original values. Paths and matrices are always
    /// retained exactly; rounding them can change topology or amplify error.
    pub precision: Option<u8>,
    /// Indent structural XML without inserting whitespace into text runs.
    pub pretty: bool,
    /// Prefix for deterministic generated resource identifiers.
    pub id_prefix: String,
    /// Intern identical resources with equivalent inherited context.
    pub deduplicate: bool,
}

impl Default for XmlOptions {
    fn default() -> Self {
        Self {
            precision: None,
            pretty: false,
            id_prefix: "svg".into(),
            deduplicate: true,
        }
    }
}

/// Statistics from one optimization pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XmlStats {
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub definitions_deduplicated: usize,
    pub elements: usize,
    /// Embedded image elements remaining after resource interning.
    pub image_elements: usize,
}

/// An optimized, self-contained SVG document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptimizedSvg {
    pub svg: String,
    pub stats: XmlStats,
}

/// An invalid document, unsafe resource, or invalid processing option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlError {
    message: String,
}

impl XmlError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for XmlError {}

/// A technical illustration treatment applied to visible graphics only.
/// Definitions, clips, masks and filter inputs retain their original meaning.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum InspectionStyle {
    #[default]
    Normal,
    XRay {
        fill_opacity: f64,
        stroke: String,
        stroke_width: f64,
        stroke_opacity: f64,
    },
    Wireframe {
        stroke: String,
        stroke_width: f64,
        stroke_opacity: f64,
    },
}

/// Validate and optimize internally generated SVG. IDs beginning with
/// `ferro-` are semantic identifiers and are preserved, including references
/// to them. All other IDs are reassigned by encounter order, removing Skia's
/// process-global clip-generation IDs from the result.
///
/// The accepted subset is inert: scripts, animation, foreign objects, DTDs,
/// processing instructions, style sheets, event handlers and external image
/// resources are rejected. Embedded PNG, JPEG, GIF and WebP images are allowed.
pub fn optimize(svg: &str, options: &XmlOptions) -> Result<OptimizedSvg, XmlError> {
    validate_prefix(&options.id_prefix)?;
    if options.precision.is_some_and(|precision| precision > 12) {
        return Err(XmlError::new(
            "SVG numeric precision must be between 0 and 12",
        ));
    }
    let mut root = parse(svg)?;
    let ids = collect_ids(&root)?;
    validate_references(&root, &ids)?;
    let names = assign_ids(&root, &options.id_prefix, true);
    rewrite(&mut root, &names)?;
    if let Some(precision) = options.precision {
        quantize(&mut root, precision)?;
    }

    let definitions_deduplicated = if options.deduplicate {
        let mut aliases = HashMap::new();
        let mut resources = HashMap::new();
        find_duplicates(&root, &mut Vec::new(), false, &mut resources, &mut aliases);
        drop_duplicates(&mut root, &aliases);
        if !aliases.is_empty() {
            rewrite(&mut root, &aliases)?;
            // Removing resources can leave holes in encounter-order identifiers.
            // Empty-def pruning cannot: it only removes elements without attributes.
            // Reassign only when an ID-bearing resource was actually removed.
            let names = assign_ids(&root, &options.id_prefix, true);
            rewrite(&mut root, &names)?;
        }
        aliases.len()
    } else {
        0
    };

    let mut result = String::with_capacity(svg.len());
    let mut elements = XmlStats::default();
    write_element(
        &root,
        &[],
        options.pretty,
        false,
        0,
        &mut result,
        &mut elements,
    );
    if options.pretty {
        result.push('\n');
    }
    let stats = XmlStats {
        input_bytes: svg.len(),
        output_bytes: result.len(),
        definitions_deduplicated,
        elements: elements.elements,
        image_elements: elements.image_elements,
    };
    Ok(OptimizedSvg { svg: result, stats })
}

/// Extract the contents of a complete SVG document for composition into
/// another SVG, giving *every* identifier a private namespace. All namespace
/// declarations required by the resulting fragment are retained, so several
/// recordings can be joined without collisions or unbound prefixes.
///
/// The coordinates remain document coordinates; this does not apply a source
/// root `viewBox` viewport mapping. The caller owns the destination viewport.
/// Root presentation attributes are retained in an enclosing group when
/// needed. `style` changes visible graphics while preserving resource shapes.
#[cfg(test)]
pub fn fragment(svg: &str, prefix: &str, style: &InspectionStyle) -> Result<String, XmlError> {
    fragment_impl(svg, prefix, style, false)
}

/// Prepare a fragment produced by the pinned Skia SVG writer. Skia emits the
/// image paint's alpha as `fill-opacity` on its image `<use>` elements, where
/// SVG does not apply it to pixels. Normalize that producer-specific encoding
/// before ordinary composition; generic SVG input retains its own semantics.
pub(crate) fn skia_fragment(
    svg: &str,
    prefix: &str,
    style: &InspectionStyle,
) -> Result<String, XmlError> {
    fragment_impl(svg, prefix, style, true)
}

fn fragment_impl(
    svg: &str,
    prefix: &str,
    style: &InspectionStyle,
    normalize_skia_images: bool,
) -> Result<String, XmlError> {
    validate_prefix(prefix)?;
    validate_inspection(style)?;
    let mut root = parse(svg)?;
    let ids = collect_ids(&root)?;
    validate_references(&root, &ids)?;
    if normalize_skia_images {
        normalize_skia_image_opacity(&mut root)?;
    }
    let names = assign_ids(&root, prefix, false);
    rewrite(&mut root, &names)?;
    if !matches!(style, InspectionStyle::Normal) {
        let mut image_bounds = HashMap::new();
        let mut definitions = Vec::new();
        if matches!(style, InspectionStyle::Wireframe { .. }) {
            collect_image_bounds(&root, prefix, style, &mut image_bounds, &mut definitions);
        }
        inspect(&mut root, style, &image_bounds, false);
        if !definitions.is_empty() {
            root.children.insert(
                0,
                Child::Element(Element {
                    qualified: qualified_name_for(&root.namespaces, SVG_NS, "defs", false),
                    local: "defs".into(),
                    namespace: SVG_NS.into(),
                    namespaces: root.namespaces.clone(),
                    attributes: Vec::new(),
                    children: definitions,
                }),
            );
        }
    }
    root.attributes.retain(|attribute| {
        attribute.namespace.is_some()
            || !matches!(
                attribute.local.as_str(),
                "width" | "height" | "viewBox" | "preserveAspectRatio" | "version" | "x" | "y"
            )
    });
    let mut result = String::with_capacity(svg.len());
    let mut elements = XmlStats::default();
    if root.attributes.is_empty() {
        for child in &root.children {
            write_child(child, &[], false, false, 0, &mut result, &mut elements);
        }
    } else {
        root.local = "g".into();
        root.qualified = qualified_name_for(&root.namespaces, SVG_NS, "g", false);
        write_element(&root, &[], false, false, 0, &mut result, &mut elements);
    }
    Ok(result)
}

fn normalize_skia_image_opacity(root: &mut Element) -> Result<(), XmlError> {
    fn collect_images(element: &Element, images: &mut HashSet<String>) {
        if element.namespace == SVG_NS && element.local == "image" {
            if let Some(id) = element.id() {
                images.insert(id.into());
            }
        }
        for child in &element.children {
            if let Child::Element(child) = child {
                collect_images(child, images);
            }
        }
    }

    fn alpha(value: &str) -> Result<f64, XmlError> {
        value
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
            .ok_or_else(|| XmlError::new("Skia image opacity must be finite and between 0 and 1"))
    }

    fn visit(element: &mut Element, images: &HashSet<String>) -> Result<(), XmlError> {
        let image_use = element.namespace == SVG_NS
            && element.local == "use"
            && element
                .href()
                .and_then(|href| href.trim().strip_prefix('#'))
                .is_some_and(|id| images.contains(id));
        if image_use {
            if let Some(paint_alpha) = element.get("fill-opacity") {
                // The native writer uses presentation attributes, never inline
                // CSS. Do not silently override an unexpected CSS cascade.
                if element
                    .get("style")
                    .is_some_and(|style| !style.trim().is_empty())
                {
                    return Err(XmlError::new(
                        "Skia image paint alpha cannot be combined with inline CSS",
                    ));
                }
                let opacity = alpha(paint_alpha)?
                    * element
                        .get("opacity")
                        .map(alpha)
                        .transpose()?
                        .unwrap_or(1.0);
                element.set("opacity", opacity.to_string());
                element.attributes.retain(|attribute| {
                    attribute.namespace.is_some() || attribute.local != "fill-opacity"
                });
            }
        }
        for child in &mut element.children {
            if let Child::Element(child) = child {
                visit(child, images)?;
            }
        }
        Ok(())
    }

    let mut images = HashSet::new();
    collect_images(root, &mut images);
    if images.is_empty() {
        return Ok(());
    }
    visit(root, &images)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Attribute {
    qualified: String,
    local: String,
    namespace: Option<String>,
    value: String,
}

#[derive(Clone, Debug)]
struct Element {
    qualified: String,
    local: String,
    namespace: String,
    namespaces: Vec<(String, String)>,
    attributes: Vec<Attribute>,
    children: Vec<Child>,
}

#[derive(Clone, Debug)]
enum Child {
    Element(Element),
    Text(String),
}

impl Element {
    fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.namespace.is_none() && a.local == name)
            .map(|a| a.value.as_str())
    }
    fn id(&self) -> Option<&str> {
        self.get("id")
    }
    fn href(&self) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| {
                a.local == "href"
                    && (a.namespace.is_none() || a.namespace.as_deref() == Some(XLINK_NS))
            })
            .map(|a| a.value.as_str())
    }
    fn set(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        // Inline style takes precedence over a presentation attribute. Remove
        // only the property being replaced, retaining unrelated declarations.
        if let Some(style) = self
            .attributes
            .iter_mut()
            .find(|a| a.namespace.is_none() && a.local == "style")
        {
            style.value = without_css_property(&style.value, name);
        }
        self.attributes
            .retain(|a| a.local != "style" || !a.value.trim().is_empty());
        if let Some(attribute) = self
            .attributes
            .iter_mut()
            .find(|a| a.namespace.is_none() && a.local == name)
        {
            attribute.value = value;
        } else {
            self.attributes.push(Attribute {
                qualified: name.into(),
                local: name.into(),
                namespace: None,
                value,
            });
        }
    }
    fn remove(&mut self, name: &str) {
        self.attributes.retain(|a| a.local != name);
    }
}

fn parse(svg: &str) -> Result<Element, XmlError> {
    let document = Document::parse_with_options(
        svg,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_NODES,
        },
    )
    .map_err(|error| XmlError::new(format!("Invalid SVG XML: {error}")))?;
    if document.descendants().any(|node| node.is_pi()) {
        return Err(XmlError::new("SVG processing instructions are not allowed"));
    }
    let root = document.root_element();
    if !root.has_tag_name((SVG_NS, "svg")) {
        return Err(XmlError::new("Expected an SVG root in the SVG namespace"));
    }
    read_element(root, 0, false, false)
}

fn read_element(
    node: Node<'_, '_>,
    depth: usize,
    preserve: bool,
    metadata: bool,
) -> Result<Element, XmlError> {
    if depth > MAX_DEPTH {
        return Err(XmlError::new("SVG nesting exceeds 256 elements"));
    }
    let local = node.tag_name().name();
    let namespace = node.tag_name().namespace().unwrap_or("");
    let metadata = metadata || (namespace == SVG_NS && local == "metadata");
    if namespace == SVG_NS {
        if !allowed_element(local) {
            return Err(XmlError::new(format!(
                "Unsupported active SVG element: {local}"
            )));
        }
    } else if !metadata
        || matches!(
            local.to_ascii_lowercase().as_str(),
            "script" | "iframe" | "object" | "embed" | "style"
        )
    {
        return Err(XmlError::new(format!(
            "Foreign SVG element outside metadata: {local}"
        )));
    }
    let namespaces = node
        .namespaces()
        .filter(|n| n.name() != Some("xml"))
        .map(|n| (n.name().unwrap_or("").to_owned(), n.uri().to_owned()))
        .collect::<Vec<_>>();
    let qualified = qualified_name_for(&namespaces, namespace, local, false);
    let preserve = preserve
        || matches!(
            local,
            "text" | "tspan" | "textPath" | "title" | "desc" | "metadata"
        )
        || node.attribute((XML_NS, "space")) == Some("preserve");
    let mut attributes = Vec::with_capacity(node.attributes().len());
    for attribute in node.attributes() {
        validate_attribute(
            local,
            attribute.name(),
            attribute.namespace(),
            attribute.value(),
        )?;
        attributes.push(Attribute {
            qualified: qualified_name_for(
                &namespaces,
                attribute.namespace().unwrap_or(""),
                attribute.name(),
                true,
            ),
            local: attribute.name().into(),
            namespace: attribute.namespace().map(str::to_owned),
            value: attribute.value().into(),
        });
    }
    attributes.sort_by(|a, b| a.qualified.cmp(&b.qualified));
    let mut children = Vec::new();
    for child in node.children() {
        if child.is_element() {
            children.push(Child::Element(read_element(
                child,
                depth + 1,
                preserve,
                metadata,
            )?));
        } else if let Some(text) = child.text().filter(|_| child.is_text()) {
            if preserve || !text.trim().is_empty() {
                children.push(Child::Text(text.into()));
            }
        }
    }
    Ok(Element {
        qualified,
        local: local.into(),
        namespace: namespace.into(),
        namespaces,
        attributes,
        children,
    })
}

fn allowed_element(name: &str) -> bool {
    matches!(
        name,
        "svg"
            | "g"
            | "defs"
            | "symbol"
            | "use"
            | "path"
            | "rect"
            | "circle"
            | "ellipse"
            | "line"
            | "polyline"
            | "polygon"
            | "text"
            | "tspan"
            | "textPath"
            | "image"
            | "clipPath"
            | "mask"
            | "linearGradient"
            | "radialGradient"
            | "stop"
            | "pattern"
            | "filter"
            | "feBlend"
            | "feColorMatrix"
            | "feComponentTransfer"
            | "feComposite"
            | "feConvolveMatrix"
            | "feDiffuseLighting"
            | "feDisplacementMap"
            | "feDistantLight"
            | "feDropShadow"
            | "feFlood"
            | "feFuncA"
            | "feFuncB"
            | "feFuncG"
            | "feFuncR"
            | "feGaussianBlur"
            | "feImage"
            | "feMerge"
            | "feMergeNode"
            | "feMorphology"
            | "feOffset"
            | "fePointLight"
            | "feSpecularLighting"
            | "feSpotLight"
            | "feTile"
            | "feTurbulence"
            | "marker"
            | "title"
            | "desc"
            | "metadata"
            | "a"
            | "switch"
            | "view"
    )
}

fn qualified_name_for(
    namespaces: &[(String, String)],
    uri: &str,
    local: &str,
    attribute: bool,
) -> String {
    if uri.is_empty() {
        return local.into();
    }
    if uri == XML_NS {
        return format!("xml:{local}");
    }
    if let Some((prefix, _)) = namespaces
        .iter()
        .find(|(prefix, value)| value == uri && (!attribute || !prefix.is_empty()))
    {
        if !prefix.is_empty() {
            return format!("{prefix}:{local}");
        }
    }
    local.into()
}

fn validate_prefix(prefix: &str) -> Result<(), XmlError> {
    if prefix.is_empty()
        || !prefix.as_bytes()[0].is_ascii_alphabetic() && !prefix.starts_with('_')
        || !prefix
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
    {
        return Err(XmlError::new(
            "SVG ID prefixes must start with a letter or underscore and contain only letters, digits, '.', '_' or '-'",
        ));
    }
    Ok(())
}

fn url_attribute(name: &str) -> bool {
    matches!(
        name,
        "fill"
            | "stroke"
            | "clip-path"
            | "mask"
            | "filter"
            | "style"
            | "cursor"
            | "marker"
            | "marker-start"
            | "marker-mid"
            | "marker-end"
    )
}

fn validate_attribute(
    element: &str,
    name: &str,
    namespace: Option<&str>,
    value: &str,
) -> Result<(), XmlError> {
    if name.to_ascii_lowercase().starts_with("on") {
        return Err(XmlError::new(format!(
            "SVG event attributes are not allowed: {name}"
        )));
    }
    if namespace == Some(XML_NS) && name == "base" {
        return Err(XmlError::new("SVG xml:base is not allowed"));
    }
    if name == "href" && (namespace.is_none() || namespace == Some(XLINK_NS)) {
        let value = value.trim();
        let embedded = matches!(element, "image" | "feImage")
            && ["png", "jpeg", "gif", "webp"]
                .iter()
                .any(|mime| value.starts_with(&format!("data:image/{mime};base64,")));
        let link =
            element == "a" && (value.starts_with("https://") || value.starts_with("http://"));
        if !value.starts_with('#') && !embedded && !link {
            return Err(XmlError::new(
                "External or active SVG resources are not allowed",
            ));
        }
    }
    if url_attribute(name) {
        // CSS escapes can disguise URLs or obsolete executable expressions.
        if value.contains('\\')
            || value.to_ascii_lowercase().contains("expression(")
            || value.contains('@')
        {
            return Err(XmlError::new("Escaped or active SVG CSS is not allowed"));
        }
        visit_urls(value, |_, _, _| Ok(()))?;
    }
    Ok(())
}

fn collect_ids(root: &Element) -> Result<HashSet<String>, XmlError> {
    fn visit(element: &Element, result: &mut HashSet<String>) -> Result<(), XmlError> {
        if let Some(id) = element.id() {
            if id.is_empty() || id.chars().any(char::is_whitespace) || !result.insert(id.into()) {
                return Err(XmlError::new(format!(
                    "Invalid or duplicate SVG identifier: {id}"
                )));
            }
        }
        for child in &element.children {
            if let Child::Element(child) = child {
                visit(child, result)?;
            }
        }
        Ok(())
    }
    let mut result = HashSet::new();
    visit(root, &mut result)?;
    Ok(result)
}

fn validate_references(element: &Element, ids: &HashSet<String>) -> Result<(), XmlError> {
    let check = |id: &str| {
        if !ids.contains(id) {
            Err(XmlError::new(format!("Unresolved SVG reference: #{id}")))
        } else {
            Ok(())
        }
    };
    for attribute in &element.attributes {
        if attribute.local == "href"
            && (attribute.namespace.is_none() || attribute.namespace.as_deref() == Some(XLINK_NS))
        {
            if let Some(id) = attribute.value.trim().strip_prefix('#') {
                check(id)?;
            }
        } else if url_attribute(&attribute.local) {
            visit_urls(&attribute.value, |_, _, id| check(id))?;
        }
    }
    for child in &element.children {
        if let Child::Element(child) = child {
            validate_references(child, ids)?;
        }
    }
    Ok(())
}

fn assign_ids(root: &Element, prefix: &str, preserve_semantic: bool) -> HashMap<String, String> {
    fn visit(element: &Element, ids: &mut Vec<String>) {
        if let Some(id) = element.id() {
            ids.push(id.into());
        }
        for child in &element.children {
            if let Child::Element(child) = child {
                visit(child, ids);
            }
        }
    }
    let mut ids = Vec::new();
    visit(root, &mut ids);
    let reserved = if preserve_semantic {
        ids.iter()
            .filter(|id| id.starts_with("ferro-"))
            .cloned()
            .collect::<HashSet<_>>()
    } else {
        HashSet::new()
    };
    let mut result = HashMap::with_capacity(ids.len());
    let mut next = 0;
    for id in ids {
        if reserved.contains(&id) {
            result.insert(id.clone(), id);
        } else {
            loop {
                let replacement = format!("{prefix}-{next}");
                next += 1;
                if !reserved.contains(&replacement) {
                    result.insert(id, replacement);
                    break;
                }
            }
        }
    }
    result
}

fn rewrite(element: &mut Element, names: &HashMap<String, String>) -> Result<(), XmlError> {
    for attribute in &mut element.attributes {
        if attribute.namespace.is_none() && attribute.local == "id" {
            if let Some(replacement) = names.get(&attribute.value) {
                if replacement != &attribute.value {
                    attribute.value = replacement.clone();
                }
            }
        } else if attribute.local == "href"
            && (attribute.namespace.is_none() || attribute.namespace.as_deref() == Some(XLINK_NS))
        {
            if let Some(id) = attribute.value.trim().strip_prefix('#') {
                if let Some(replacement) = names.get(id) {
                    if attribute.value.strip_prefix('#') != Some(replacement.as_str()) {
                        attribute.value = format!("#{replacement}");
                    }
                }
            }
        } else if url_attribute(&attribute.local) {
            if let Cow::Owned(replacement) = rewrite_urls(&attribute.value, names)? {
                attribute.value = replacement;
            }
        } else if matches!(
            attribute.local.as_str(),
            "aria-labelledby" | "aria-describedby"
        ) {
            attribute.value = attribute
                .value
                .split_whitespace()
                .map(|id| names.get(id).map(String::as_str).unwrap_or(id))
                .collect::<Vec<_>>()
                .join(" ");
        }
    }
    for child in &mut element.children {
        if let Child::Element(child) = child {
            rewrite(child, names)?;
        }
    }
    Ok(())
}

/// Visit CSS URL tokens without copying their text. The callback receives the
/// complete token's start/end byte offsets and its borrowed fragment ID.
/// #fff is a color, #a and #ab are distinct IDs, and quoted references are valid.
fn visit_urls<'a>(
    value: &'a str,
    mut visit: impl FnMut(usize, usize, &'a str) -> Result<(), XmlError>,
) -> Result<(), XmlError> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index + 3 <= bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            index = css_comment_end(value, index)?;
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"') {
            index = css_string_end(value, index)?;
            continue;
        }
        if !bytes[index..index + 3].eq_ignore_ascii_case(b"url")
            || (index > 0
                && (bytes[index - 1].is_ascii_alphanumeric()
                    || matches!(bytes[index - 1], b'_' | b'-')
                    || !bytes[index - 1].is_ascii()))
        {
            index += 1;
            continue;
        }
        let mut open = index + 3;
        while open < bytes.len() && bytes[open].is_ascii_whitespace() {
            open += 1;
        }
        if bytes.get(open) != Some(&b'(') {
            index += 3;
            continue;
        }
        let mut start = open + 1;
        while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
            start += 1;
        }
        let (target, end) = if matches!(bytes.get(start), Some(b'\'' | b'"')) {
            let close = css_string_end(value, start)?;
            let mut end = close;
            while bytes.get(end).is_some_and(u8::is_ascii_whitespace) {
                end += 1;
            }
            if bytes.get(end) != Some(&b')') {
                return Err(XmlError::new("Invalid quoted SVG URL reference"));
            }
            (&value[start + 1..close - 1], end)
        } else {
            let end = value[start..]
                .find(')')
                .map(|offset| start + offset)
                .ok_or_else(|| XmlError::new("Unterminated SVG URL reference"))?;
            let target = value[start..end].trim();
            if target.contains(['\'', '"', '(']) {
                return Err(XmlError::new("Invalid unquoted SVG URL reference"));
            }
            (target, end)
        };
        let id = target
            .strip_prefix('#')
            .filter(|id| !id.is_empty() && !id.chars().any(char::is_whitespace))
            .ok_or_else(|| XmlError::new("SVG URL resources must be local fragment references"))?;
        // Validate the original target before ID reassignment can make it safe.
        // Canonical URL output has no CSS escapes, so one quote type must remain
        // available as a delimiter when quoted syntax is needed.
        if id.contains('\'') && id.contains('"') {
            return Err(XmlError::new(
                "SVG URL identifiers cannot contain both quote types",
            ));
        }
        visit(index, end + 1, id)?;
        index = end + 1;
    }
    Ok(())
}

/// Preserve already-canonical attributes in place. Most paint attributes have
/// no resource reference; even referenced attributes often need no changes in
/// the alias/final-ID passes. Allocate once, at the first changed token only.
fn rewrite_urls<'a>(
    value: &'a str,
    names: &HashMap<String, String>,
) -> Result<Cow<'a, str>, XmlError> {
    let mut output: Option<String> = None;
    let mut copied = 0;
    visit_urls(value, |start, end, id| {
        let replacement = names.get(id).map(String::as_str).unwrap_or(id);
        // Semantic IDs are retained verbatim and may contain parentheses or
        // quotes. Keep the CSS reference quoted when unquoted syntax is invalid.
        let (prefix, suffix) = if replacement.contains(['\'', '"', '(', ')']) {
            if !replacement.contains('\'') {
                ("url('#", "')")
            } else if !replacement.contains('"') {
                ("url(\"#", "\")")
            } else {
                return Err(XmlError::new(
                    "SVG URL identifiers cannot contain both quote types",
                ));
            }
        } else {
            ("url(#", ")")
        };
        if value[start..end]
            .strip_prefix(prefix)
            .and_then(|target| target.strip_suffix(suffix))
            == Some(replacement)
        {
            return Ok(());
        }
        let output = output.get_or_insert_with(|| String::with_capacity(value.len()));
        output.push_str(&value[copied..start]);
        output.push_str(prefix);
        output.push_str(replacement);
        output.push_str(suffix);
        copied = end;
        Ok(())
    })?;
    if let Some(mut output) = output {
        output.push_str(&value[copied..]);
        Ok(Cow::Owned(output))
    } else {
        Ok(Cow::Borrowed(value))
    }
}

fn css_comment_end(value: &str, start: usize) -> Result<usize, XmlError> {
    value[start + 2..]
        .find("*/")
        .map(|end| start + 2 + end + 2)
        .ok_or_else(|| XmlError::new("Unterminated SVG CSS comment"))
}

fn css_string_end(value: &str, start: usize) -> Result<usize, XmlError> {
    // CSS escapes are rejected by validate_attribute before tokenization.
    let end = value[start + 1..]
        .find(value.as_bytes()[start] as char)
        .map(|end| start + 1 + end + 1)
        .ok_or_else(|| XmlError::new("Unterminated SVG CSS string"))?;
    // A raw newline ends a CSS string with a bad-string token. Treating it as
    // part of a quoted literal could otherwise conceal a following active URL.
    if value[start + 1..end - 1].contains(['\n', '\r', '\u{c}']) {
        return Err(XmlError::new("Newlines are not allowed in SVG CSS strings"));
    }
    Ok(end)
}

fn numeric_attribute(name: &str) -> bool {
    matches!(
        name,
        "x" | "y"
            | "x1"
            | "y1"
            | "x2"
            | "y2"
            | "cx"
            | "cy"
            | "r"
            | "rx"
            | "ry"
            | "width"
            | "height"
            | "dx"
            | "dy"
            | "points"
            | "stroke-width"
            | "stroke-miterlimit"
            | "opacity"
            | "fill-opacity"
            | "stroke-opacity"
            | "stop-opacity"
            | "offset"
            | "font-size"
            | "textLength"
            | "stdDeviation"
            | "baseFrequency"
            | "scale"
            | "slope"
            | "intercept"
            | "amplitude"
            | "exponent"
    )
}

fn quantize(element: &mut Element, precision: u8) -> Result<(), XmlError> {
    for attribute in &mut element.attributes {
        if element.namespace == SVG_NS
            && attribute.namespace.is_none()
            && numeric_attribute(&attribute.local)
        {
            attribute.value = quantize_numbers(&attribute.value, precision)?;
        }
    }
    for child in &mut element.children {
        if let Child::Element(child) = child {
            quantize(child, precision)?;
        }
    }
    Ok(())
}

fn quantize_numbers(value: &str, precision: u8) -> Result<String, XmlError> {
    let bytes = value.as_bytes();
    let mut result = String::with_capacity(value.len());
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        if matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let mut digits = 0;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
            digits += 1;
        }
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            let character = value[start..].chars().next().expect("valid UTF-8 input");
            result.push(character);
            index = start + character.len_utf8();
            continue;
        }
        if matches!(bytes.get(index), Some(b'e' | b'E')) {
            let mut end = index + 1;
            if matches!(bytes.get(end), Some(b'+' | b'-')) {
                end += 1;
            }
            let exponent_start = end;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end != exponent_start {
                index = end;
            }
        }
        let token = &value[start..index];
        let number: f64 = token
            .parse()
            .map_err(|_| XmlError::new("Invalid SVG number"))?;
        if !number.is_finite() {
            return Err(XmlError::new("Non-finite SVG number"));
        }
        // Avoid inflating scientific notation and rounding huge magnitudes.
        if number.abs() >= 1e16 {
            result.push_str(token);
        } else {
            let mut formatted = format!("{:.*}", usize::from(precision), number);
            if formatted.contains('.') {
                while formatted.ends_with('0') {
                    formatted.pop();
                }
                if formatted.ends_with('.') {
                    formatted.pop();
                }
            }
            if formatted == "-0" {
                formatted = "0".into();
            }
            result.push_str(&formatted);
        }
        // Decimal points and signs can delimit adjacent SVG number tokens.
        // Formatting removes leading '+' and rounded negative zero, so all
        // such boundaries need an explicit separator, including after a
        // scientific-notation token that was deliberately kept unchanged.
        if matches!(bytes.get(index), Some(b'.' | b'+' | b'-')) {
            result.push(' ');
        }
    }
    Ok(result)
}

type Context = Vec<(String, String)>;
type ResourceTable<'a> = HashMap<u64, Vec<(&'a Element, Context)>>;

fn find_duplicates<'a>(
    element: &'a Element,
    context: &mut Context,
    in_defs: bool,
    resources: &mut ResourceTable<'a>,
    aliases: &mut HashMap<String, String>,
) {
    let resource = element.namespace == SVG_NS
        && match element.local.as_str() {
            "linearGradient" | "radialGradient" | "clipPath" => true,
            "image" => in_defs,
            _ => false,
        };
    if resource
        && element.id().is_some_and(|id| !id.starts_with("ferro-"))
        && !descendant_has_id(element)
    {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        context.hash(&mut hasher);
        hash_definition(element, true, &mut hasher);
        let bucket = resources.entry(hasher.finish()).or_default();
        if let Some((existing, _)) = bucket.iter().find(|(existing, inherited)| {
            inherited == context && same_definition(existing, element, true)
        }) {
            aliases.insert(element.id().unwrap().into(), existing.id().unwrap().into());
        } else {
            bucket.push((element, context.clone()));
        }
    }
    // Leaf images and paths often carry the largest strings in the document.
    // Nothing can inherit from a leaf, so do not copy their payload into the
    // ancestor-context key (particularly base64 image data).
    if !element
        .children
        .iter()
        .any(|child| matches!(child, Child::Element(_)))
    {
        return;
    }
    let length = context.len();
    if element.local == "svg" {
        context.push(("viewport".into(), "svg".into()));
    }
    for attribute in &element.attributes {
        if attribute.local == "id"
            || attribute.local.starts_with("data-")
            || attribute.namespace.as_deref() == Some("http://www.inkscape.org/namespaces/inkscape")
        {
            continue;
        }
        context.push((
            format!(
                "{}:{}",
                attribute.namespace.as_deref().unwrap_or(""),
                attribute.local
            ),
            attribute.value.clone(),
        ));
    }
    for child in &element.children {
        if let Child::Element(child) = child {
            find_duplicates(
                child,
                context,
                in_defs || element.local == "defs",
                resources,
                aliases,
            );
        }
    }
    context.truncate(length);
}

fn descendant_has_id(element: &Element) -> bool {
    element.children.iter().any(|child| match child {
        Child::Element(child) => child.id().is_some() || descendant_has_id(child),
        Child::Text(_) => false,
    })
}

fn definition_attributes(element: &Element, root: bool) -> impl Iterator<Item = &Attribute> {
    element
        .attributes
        .iter()
        .filter(move |a| !(root && a.namespace.is_none() && a.local == "id"))
}

fn hash_definition(element: &Element, root: bool, hasher: &mut impl Hasher) {
    element.namespace.hash(hasher);
    element.local.hash(hasher);
    for attribute in definition_attributes(element, root) {
        attribute.namespace.hash(hasher);
        attribute.local.hash(hasher);
        attribute.value.hash(hasher);
    }
    for child in &element.children {
        match child {
            Child::Element(child) => {
                0u8.hash(hasher);
                hash_definition(child, false, hasher);
            }
            Child::Text(text) => {
                1u8.hash(hasher);
                text.hash(hasher);
            }
        }
    }
}

fn same_definition(first: &Element, second: &Element, root: bool) -> bool {
    first.namespace == second.namespace
        && first.local == second.local
        && definition_attributes(first, root)
            .map(|a| (&a.namespace, &a.local, &a.value))
            .eq(definition_attributes(second, root).map(|a| (&a.namespace, &a.local, &a.value)))
        && first.children.len() == second.children.len()
        && first
            .children
            .iter()
            .zip(&second.children)
            .all(|(a, b)| match (a, b) {
                (Child::Element(a), Child::Element(b)) => same_definition(a, b, false),
                (Child::Text(a), Child::Text(b)) => a == b,
                _ => false,
            })
}

fn drop_duplicates(element: &mut Element, aliases: &HashMap<String, String>) {
    element.children.retain_mut(|child| {
        if let Child::Element(child) = child {
            if child.id().is_some_and(|id| aliases.contains_key(id)) {
                return false;
            }
            drop_duplicates(child, aliases);
            if child.local == "defs" && child.children.is_empty() && child.attributes.is_empty() {
                return false;
            }
        }
        true
    });
}

fn validate_inspection(style: &InspectionStyle) -> Result<(), XmlError> {
    let (stroke, width, opacity) = match style {
        InspectionStyle::Normal => return Ok(()),
        InspectionStyle::XRay {
            fill_opacity,
            stroke,
            stroke_width,
            stroke_opacity,
        } => {
            if !fill_opacity.is_finite() || !(0.0..=1.0).contains(fill_opacity) {
                return Err(XmlError::new(
                    "X-ray fill opacity must be finite and between 0 and 1",
                ));
            }
            (stroke, stroke_width, stroke_opacity)
        }
        InspectionStyle::Wireframe {
            stroke,
            stroke_width,
            stroke_opacity,
        } => (stroke, stroke_width, stroke_opacity),
    };
    if !width.is_finite() || *width < 0.0 {
        return Err(XmlError::new(
            "Inspection stroke width must be finite and non-negative",
        ));
    }
    if !opacity.is_finite() || !(0.0..=1.0).contains(opacity) {
        return Err(XmlError::new(
            "Inspection stroke opacity must be finite and between 0 and 1",
        ));
    }
    validate_attribute("path", "stroke", None, stroke)?;
    if stroke.trim().is_empty()
        || stroke.to_ascii_lowercase().contains("url")
        || stroke.contains([';', '{', '}'])
    {
        return Err(XmlError::new(
            "Inspection stroke must be a color, not a resource or CSS declaration",
        ));
    }
    Ok(())
}

fn without_css_property(style: &str, property: &str) -> String {
    let mut result = String::with_capacity(style.len());
    let mut parentheses = 0usize;
    let mut start = 0;
    let mut append = |declaration: &str| {
        let mut name = String::new();
        let mut index = 0;
        while index < declaration.len() {
            if declaration.as_bytes()[index..].starts_with(b"/*") {
                // Comments separate tokens; do not join two partial names into
                // a different declaration when removing them for comparison.
                name.push(' ');
                index = css_comment_end(declaration, index).unwrap_or(declaration.len());
                continue;
            }
            let character = declaration[index..].chars().next().unwrap();
            if character == ':' {
                break;
            }
            name.push(character);
            index += character.len_utf8();
        }
        if name.trim().eq_ignore_ascii_case(property) {
            return;
        }
        if !result.is_empty() {
            result.push(';');
        }
        result.push_str(declaration);
    };
    let mut index = 0;
    while index < style.len() {
        if style.as_bytes()[index..].starts_with(b"/*") {
            index = css_comment_end(style, index).unwrap_or(style.len());
            continue;
        }
        match style.as_bytes()[index] {
            b'\'' | b'"' => {
                index = css_string_end(style, index).unwrap_or(style.len());
                continue;
            }
            b'(' => parentheses += 1,
            b')' => parentheses = parentheses.saturating_sub(1),
            b';' if parentheses == 0 => {
                append(&style[start..index]);
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    append(&style[start..]);
    result
}

fn collect_image_bounds(
    element: &Element,
    prefix: &str,
    style: &InspectionStyle,
    images: &mut HashMap<String, String>,
    definitions: &mut Vec<Child>,
) {
    if element.namespace == SVG_NS && element.local == "image" {
        if let Some(id) = element.id() {
            // Keep <use> as <use>: its x/y add a translation, its width/height
            // do not resize image references, and both source and instance
            // transforms/graphical effects must survive. A separate rectangle
            // resource gives SVG's own instance layout those same semantics.
            // The original image remains available to clips, masks and filters.
            let bounds_id = format!("{prefix}-image-bounds-{}", definitions.len());
            let mut bounds = Element {
                qualified: qualified_name_for(&element.namespaces, SVG_NS, "rect", false),
                local: "rect".into(),
                namespace: SVG_NS.into(),
                namespaces: element.namespaces.clone(),
                attributes: element
                    .attributes
                    .iter()
                    .filter(|a| !matches!(a.local.as_str(), "href" | "preserveAspectRatio"))
                    .cloned()
                    .collect(),
                // Descriptive children belong to the original image. Copying
                // their IDs would create duplicate identifiers in the fragment.
                children: Vec::new(),
            };
            bounds.set("id", bounds_id.clone());
            inspect(&mut bounds, style, &HashMap::new(), false);
            images.insert(id.into(), bounds_id);
            definitions.push(Child::Element(bounds));
        }
    }
    for child in &element.children {
        if let Child::Element(child) = child {
            collect_image_bounds(child, prefix, style, images, definitions);
        }
    }
}

fn inspect(
    element: &mut Element,
    style: &InspectionStyle,
    images: &HashMap<String, String>,
    resource: bool,
) {
    let resource = resource
        || matches!(
            element.local.as_str(),
            "defs"
                | "clipPath"
                | "mask"
                | "filter"
                | "linearGradient"
                | "radialGradient"
                | "pattern"
                | "marker"
                | "symbol"
                | "metadata"
        );
    if !resource && element.namespace == SVG_NS {
        let image = element.local == "image";
        let referenced_image = if element.local == "use" {
            element
                .href()
                .and_then(|href| href.strip_prefix('#'))
                .and_then(|id| images.get(id))
        } else {
            None
        };
        if matches!(style, InspectionStyle::Wireframe { .. }) {
            if let Some(bounds_id) = referenced_image {
                for attribute in &mut element.attributes {
                    if attribute.local == "href"
                        && (attribute.namespace.is_none()
                            || attribute.namespace.as_deref() == Some(XLINK_NS))
                    {
                        attribute.value = format!("#{bounds_id}");
                    }
                }
            }
            if image {
                element.local = "rect".into();
                element.qualified = qualified_name_for(&element.namespaces, SVG_NS, "rect", false);
                element.remove("href");
                element.remove("preserveAspectRatio");
            }
        }
        if matches!(
            element.local.as_str(),
            "path"
                | "rect"
                | "circle"
                | "ellipse"
                | "line"
                | "polyline"
                | "polygon"
                | "text"
                | "tspan"
                | "textPath"
        ) {
            let (stroke, width, opacity) = match style {
                InspectionStyle::Normal => unreachable!(),
                InspectionStyle::XRay {
                    fill_opacity,
                    stroke,
                    stroke_width,
                    stroke_opacity,
                } => {
                    element.set("fill-opacity", fill_opacity.to_string());
                    (stroke, stroke_width, stroke_opacity)
                }
                InspectionStyle::Wireframe {
                    stroke,
                    stroke_width,
                    stroke_opacity,
                } => {
                    element.set("fill", "none");
                    (stroke, stroke_width, stroke_opacity)
                }
            };
            element.set("stroke", stroke.clone());
            element.set("stroke-width", width.to_string());
            element.set("stroke-opacity", opacity.to_string());
            element.set("vector-effect", "non-scaling-stroke");
        } else if image || element.local == "use" {
            if let InspectionStyle::XRay { fill_opacity, .. } = style {
                // Multiplication must preserve inline CSS precedence, percentage
                // values and !important, as well as presentation attributes.
                // A compositing group avoids parsing or overriding their paint.
                let wrapper = Element {
                    qualified: qualified_name_for(&element.namespaces, SVG_NS, "g", false),
                    local: "g".into(),
                    namespace: SVG_NS.into(),
                    namespaces: element.namespaces.clone(),
                    attributes: vec![Attribute {
                        qualified: "opacity".into(),
                        local: "opacity".into(),
                        namespace: None,
                        value: fill_opacity.to_string(),
                    }],
                    children: Vec::new(),
                };
                let original = std::mem::replace(element, wrapper);
                element.children.push(Child::Element(original));
                // The original is intentionally unchanged; inspecting it again
                // would apply the same opacity multiplier recursively.
                return;
            }
        }
    }
    for child in &mut element.children {
        if let Child::Element(child) = child {
            inspect(child, style, images, resource);
        }
    }
}

fn escape(output: &mut String, value: &str, attribute: bool) {
    let mut copied = 0;
    for (index, byte) in value.bytes().enumerate() {
        let replacement = match byte {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' if attribute => "&quot;",
            b'\n' if attribute => "&#10;",
            b'\t' if attribute => "&#9;",
            b'\r' => "&#13;",
            _ => continue,
        };
        // Every escape byte is ASCII, hence both ends of these slices are
        // UTF-8 boundaries. Large path/image payloads usually need one copy.
        output.push_str(&value[copied..index]);
        output.push_str(replacement);
        copied = index + 1;
    }
    output.push_str(&value[copied..]);
}

fn write_child(
    child: &Child,
    inherited: &[(String, String)],
    pretty: bool,
    inline: bool,
    depth: usize,
    output: &mut String,
    elements: &mut XmlStats,
) {
    match child {
        Child::Text(text) => escape(output, text, false),
        Child::Element(element) => {
            write_element(element, inherited, pretty, inline, depth, output, elements)
        }
    }
}

fn write_element(
    element: &Element,
    inherited: &[(String, String)],
    pretty: bool,
    inline: bool,
    depth: usize,
    output: &mut String,
    elements: &mut XmlStats,
) {
    elements.elements += 1;
    if element.namespace == SVG_NS && element.local == "image" {
        elements.image_elements += 1;
    }
    if pretty && !inline {
        for _ in 0..depth {
            output.push_str("  ");
        }
    }
    output.push('<');
    output.push_str(&element.qualified);
    for (prefix, uri) in &element.namespaces {
        if inherited.iter().any(|(p, u)| p == prefix && u == uri) {
            continue;
        }
        output.push_str(" xmlns");
        if !prefix.is_empty() {
            output.push(':');
            output.push_str(prefix);
        }
        output.push_str("=\"");
        escape(output, uri, true);
        output.push('"');
    }
    for attribute in &element.attributes {
        write!(output, " {}=\"", attribute.qualified).expect("writing to String");
        escape(output, &attribute.value, true);
        output.push('"');
    }
    if element.children.is_empty() {
        output.push_str("/>");
        return;
    }
    output.push('>');
    let text_context = inline
        || matches!(
            element.local.as_str(),
            "text" | "tspan" | "textPath" | "title" | "desc" | "metadata"
        )
        || element.attributes.iter().any(|a| {
            a.namespace.as_deref() == Some(XML_NS) && a.local == "space" && a.value == "preserve"
        })
        || element
            .children
            .iter()
            .any(|child| matches!(child, Child::Text(_)));
    if pretty && !text_context {
        output.push('\n');
    }
    for child in &element.children {
        write_child(
            child,
            &element.namespaces,
            pretty,
            text_context,
            depth + 1,
            output,
            elements,
        );
        if pretty && !text_context {
            output.push('\n');
        }
    }
    if pretty && !text_context {
        for _ in 0..depth {
            output.push_str("  ");
        }
    }
    output.push_str("</");
    output.push_str(&element.qualified);
    output.push('>');
}

#[cfg(test)]
#[path = "xml_tests.rs"]
mod tests;
