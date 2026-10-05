//! Port of `Parsers/XDocumentXamlParser.cs`.
//!
//! Upstream loads the markup with `System.Xml.Linq` (`XDocument.Load` over an `XmlReader`
//! wrapped in `CompatibleXmlReader`). This port parses the markup with `roxmltree` and applies
//! the same rules while walking the tree; see the crate documentation for the known
//! behavioral differences.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use roxmltree::Node as XmlNode;

use crate::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo,
    XamlAstNamePropertyReference, XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode,
    XamlAstXamlPropertyValueNode, XamlAstXmlDirective, XamlAstXmlTypeReference, XamlDocument,
    XamlLineInfo,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::xaml_namespaces::XamlNamespaces;

use super::compatible_xml_reader::{CompatibleXmlReader, MARKUP_COMPATIBILITY_NAMESPACE};
use super::system_xaml_markup_extension_parser::{MeScannerError, SystemXamlMarkupExtensionParser};
use super::{CommaSeparatedParenthesesTreeParser, Node as TypeArgumentNode};

const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// Elements nested deeper than this are rejected instead of risking a stack overflow in the
/// recursive XML parser, XAML parser and visitors (upstream has no such limit).
pub const MAX_ELEMENT_DEPTH: usize = 256;

#[derive(Debug, Clone, Default)]
pub struct XDocumentXamlParserSettings {
    pub compatible_namespaces: Option<HashMap<String, String>>,
}

pub struct XDocumentXamlParser;

/// `System.Xml.XmlSpace`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum XmlSpace {
    None,
    Default,
    Preserve,
}

/// Maps byte offsets of the source text to 1-based line numbers and 1-based positions
/// (counted in UTF-16 code units, like `IXmlLineInfo`).
struct LineIndex<'input> {
    source: &'input str,
    line_starts: Vec<usize>,
}

impl<'input> LineIndex<'input> {
    fn new(source: &'input str) -> Self {
        let bytes = source.as_bytes();
        let mut line_starts = vec![0];
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => line_starts.push(i + 1),
                b'\r' => {
                    if bytes.get(i + 1) != Some(&b'\n') {
                        line_starts.push(i + 1);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Self {
            source,
            line_starts,
        }
    }

    fn line_info(&self, offset: usize) -> XamlLineInfo {
        let offset = offset.min(self.source.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index - 1,
        };
        let line_start = self.line_starts[line];
        let column = self
            .source
            .get(line_start..offset)
            .map(|s| s.encode_utf16().count())
            .unwrap_or(0);
        XamlLineInfo::new(line as i32 + 1, column as i32 + 1)
    }
}

fn find_from(source: &str, from: usize, pattern: &str) -> Option<usize> {
    source
        .get(from..)
        .and_then(|rest| rest.find(pattern))
        .map(|p| from + p)
}

/// Finds the end (exclusive) of the tag starting at `start`, honoring quoted attribute values.
fn tag_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    let mut quote = 0u8;
    while i < bytes.len() {
        let b = bytes[i];
        if quote != 0 {
            if b == quote {
                quote = 0;
            }
        } else if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b'>' {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

/// Finds the end (exclusive) of the document type declaration starting at `start`.
fn doctype_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = start + 1;
    let mut quote = 0u8;
    let mut brackets = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if quote != 0 {
            if b == quote {
                quote = 0;
            }
        } else if bytes[i..].starts_with(b"<!--") {
            i = find_from(source, i + 4, "-->")
                .map(|p| p + 3)
                .unwrap_or(bytes.len());
            continue;
        } else if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b'[' {
            brackets += 1;
        } else if b == b']' {
            brackets = brackets.saturating_sub(1);
        } else if b == b'>' && brackets == 0 {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

/// A lexical pass over the markup performed before the XML parser runs. It rejects documents
/// whose elements are nested too deeply and locates the document type declaration, which is
/// ignored like upstream does (`DtdProcessing.Ignore`).
fn prescan(source: &str, lines: &LineIndex<'_>) -> XamlResult<Option<std::ops::Range<usize>>> {
    let bytes = source.as_bytes();
    let mut doctype = None;
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &source[i..];
        if rest.starts_with("<!--") {
            i = find_from(source, i + 4, "-->")
                .map(|p| p + 3)
                .unwrap_or(bytes.len());
        } else if rest.starts_with("<![CDATA[") {
            i = find_from(source, i + 9, "]]>")
                .map(|p| p + 3)
                .unwrap_or(bytes.len());
        } else if rest.starts_with("<?") {
            i = find_from(source, i + 2, "?>")
                .map(|p| p + 2)
                .unwrap_or(bytes.len());
        } else if rest.starts_with("<!DOCTYPE") {
            let end = doctype_end(source, i);
            doctype = Some(i..end);
            i = end;
        } else if rest.starts_with("</") {
            depth = depth.saturating_sub(1);
            i = tag_end(bytes, i);
        } else {
            let end = tag_end(bytes, i);
            let self_closing = end >= 2 && bytes[end - 2] == b'/';
            if !self_closing {
                depth += 1;
                if depth > MAX_ELEMENT_DEPTH + 1 {
                    return Err(XamlError::parse_exception(
                        format!(
                            "Elements are nested too deeply (the limit is {MAX_ELEMENT_DEPTH})"
                        ),
                        Some(&lines.line_info(i + 1)),
                    ));
                }
            }
            i = end;
        }
    }
    Ok(doctype)
}

/// An attribute as written in a start tag (including namespace declarations).
struct RawAttribute<'input> {
    qualified_name: &'input str,
    /// Byte offset of the attribute name.
    offset: usize,
}

impl RawAttribute<'_> {
    /// The prefix declared by this attribute when it is a namespace declaration
    /// (`Some("")` for the default namespace).
    fn declared_prefix(&self) -> Option<&str> {
        if self.qualified_name == "xmlns" {
            Some("")
        } else {
            self.qualified_name.strip_prefix("xmlns:")
        }
    }
}

/// Lists the attributes of the start tag beginning at `start` (the offset of its `<`).
/// The document has already been validated by the XML parser.
fn scan_start_tag(source: &str, start: usize) -> Vec<RawAttribute<'_>> {
    let bytes = source.as_bytes();
    let is_space = |b: u8| matches!(b, b' ' | b'\t' | b'\r' | b'\n');
    let mut rv = Vec::new();
    let mut i = start + 1;
    // Element name
    while i < bytes.len() && !is_space(bytes[i]) && bytes[i] != b'>' && bytes[i] != b'/' {
        i += 1;
    }
    loop {
        while i < bytes.len() && is_space(bytes[i]) {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'>' || bytes[i] == b'/' {
            break;
        }
        let name_start = i;
        while i < bytes.len() && !is_space(bytes[i]) && bytes[i] != b'=' {
            i += 1;
        }
        let name_end = i;
        while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\'' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let quote = bytes[i];
        i += 1;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        i += 1;
        if let Some(qualified_name) = source.get(name_start..name_end) {
            rv.push(RawAttribute {
                qualified_name,
                offset: name_start,
            });
        }
    }
    rv
}

/// An attribute visible to the XAML parser: not a namespace declaration and not ignored by the
/// markup-compatibility rules.
struct VisibleAttribute<'a, 'input> {
    namespace: String,
    local_name: &'a str,
    value: &'a str,
    line_info: XamlLineInfo,
    _marker: std::marker::PhantomData<&'input ()>,
}

enum ElementAttribute<'a, 'input> {
    /// An `xmlns`/`xmlns:prefix` declaration at the given position.
    NamespaceDeclaration(XamlLineInfo),
    Attribute(VisibleAttribute<'a, 'input>),
}

struct EnteredElement<'a, 'input> {
    namespace: String,
    attributes: Vec<ElementAttribute<'a, 'input>>,
    pushed_scope: bool,
}

struct ParserContext<'input> {
    lines: LineIndex<'input>,
    source: &'input str,
    compat: RefCell<CompatibleXmlReader>,
}

fn lookup_raw_namespace<'a>(el: XmlNode<'a, '_>, prefix: &str) -> Option<&'a str> {
    match prefix {
        "" => el.lookup_namespace_uri(None),
        "xml" => Some(XML_NAMESPACE),
        "xmlns" => Some(XMLNS_NAMESPACE),
        _ => el.lookup_namespace_uri(Some(prefix)),
    }
}

impl<'input> ParserContext<'input> {
    fn li(&self, node: XmlNode<'_, 'input>) -> XamlLineInfo {
        // Elements are positioned at their name (after '<'), other nodes at their first character.
        let offset = node.range().start + usize::from(node.is_element());
        self.lines.line_info(offset)
    }

    fn mapped(&self, ns: &str) -> String {
        self.compat.borrow_mut().get_mapped(ns)
    }

    /// `XmlReader.LookupNamespace` through the compatibility layer.
    fn lookup_namespace(&self, el: XmlNode<'_, 'input>, prefix: &str) -> Option<String> {
        lookup_raw_namespace(el, prefix).map(|ns| self.mapped(ns))
    }

    /// The namespace of the element as seen through the compatibility layer.
    fn element_namespace(&self, el: XmlNode<'_, 'input>) -> String {
        self.mapped(el.tag_name().namespace().unwrap_or(""))
    }

    /// Whether the reader would skip this element (and its content) entirely.
    fn is_ignored_element(&self, el: XmlNode<'_, 'input>) -> bool {
        let ns = self.element_namespace(el);
        self.compat.borrow().should_ignore(&ns)
    }

    /// `XmlReader.IsEmptyElement`: whether the element is written as `<a/>`.
    fn is_empty_element(&self, el: XmlNode<'_, 'input>) -> bool {
        self.source
            .get(el.range())
            .is_some_and(|text| text.ends_with("/>"))
    }

    /// Whether the compatibility reader hides this child node from the document.
    ///
    /// Ignorable elements are skipped. As upstream, skipping a non-empty element also consumes
    /// the node that follows its end tag (the reader is advanced once more after
    /// `XmlReader.Skip`); the port applies that to text, comments and processing instructions.
    fn is_hidden_child(&self, node: XmlNode<'_, 'input>, skip_following: &mut bool) -> bool {
        let swallow = std::mem::take(skip_following);
        if node.is_element() {
            if self.is_ignored_element(node) {
                *skip_following = !self.is_empty_element(node);
                return true;
            }
            return false;
        }
        swallow
    }

    /// Performs what the compatibility reader does when it reaches a start tag: opens the
    /// `mc:Ignorable` scope and filters the attributes. Must be paired with [`Self::leave_element`].
    fn enter_element<'a>(&self, el: XmlNode<'a, 'input>) -> EnteredElement<'a, 'input> {
        let namespace = self.element_namespace(el);

        let mut pushed_scope = false;
        let ignorable = el.attributes().find(|a| {
            a.name() == "Ignorable"
                && self.mapped(a.namespace().unwrap_or("")) == MARKUP_COMPATIBILITY_NAMESPACE
        });
        if let Some(ignorable) = ignorable {
            let mut compat = self.compat.borrow_mut();
            compat.push_scope(ignorable.value(), |compat, prefix| {
                lookup_raw_namespace(el, prefix).map(|ns| compat.get_mapped(ns))
            });
            pushed_scope = true;
        }

        let raw_attributes = scan_start_tag(self.source, el.range().start);
        let mut xml_attributes = el.attributes();
        let mut attributes = Vec::with_capacity(raw_attributes.len());
        for raw in raw_attributes {
            let line_info = self.lines.line_info(raw.offset);
            if let Some(prefix) = raw.declared_prefix() {
                // Reading the declaration's value maps the declared namespace.
                if let Some(ns) = lookup_raw_namespace(el, prefix) {
                    self.mapped(ns);
                }
                attributes.push(ElementAttribute::NamespaceDeclaration(line_info));
                continue;
            }

            let Some(attribute) = xml_attributes.next() else {
                break;
            };
            let attribute_namespace = self.mapped(attribute.namespace().unwrap_or(""));
            if self.compat.borrow().should_ignore(&attribute_namespace) {
                continue;
            }
            attributes.push(ElementAttribute::Attribute(VisibleAttribute {
                namespace: attribute_namespace,
                local_name: attribute.name(),
                value: attribute.value(),
                line_info,
                _marker: std::marker::PhantomData,
            }));
        }

        EnteredElement {
            namespace,
            attributes,
            pushed_scope,
        }
    }

    fn leave_element(&self, entered: &EnteredElement<'_, 'input>) {
        if entered.pushed_scope {
            self.compat.borrow_mut().pop_scope();
        }
    }

    fn get_type_reference(
        &self,
        el: XmlNode<'_, 'input>,
        namespace: &str,
    ) -> Rc<XamlAstXmlTypeReference> {
        XamlAstXmlTypeReference::new(&self.li(el), Some(namespace), el.tag_name().name())
    }

    fn parse_type_name(
        &self,
        info: &dyn IXamlLineInfo,
        type_name: &str,
        xel: XmlNode<'_, 'input>,
    ) -> XamlResult<Rc<XamlAstXmlTypeReference>> {
        let (_, xmlns_val, name) = self.parse_pair_with_xmlns(info, type_name, xel)?;
        Ok(XamlAstXmlTypeReference::new(info, Some(&xmlns_val), &name))
    }

    /// Returns `(xmlnsKey, xmlnsVal, name)`.
    fn parse_pair_with_xmlns(
        &self,
        _info: &dyn IXamlLineInfo,
        type_name: &str,
        xel: XmlNode<'_, 'input>,
    ) -> XamlResult<(String, String, String)> {
        let trimmed = type_name.trim_matches(char::is_whitespace);
        // As upstream, an unknown prefix resolves to the empty namespace.
        Ok(match trimmed.split_once(':') {
            None => (
                String::new(),
                self.prefix_resolver("", xel),
                trimmed.to_string(),
            ),
            Some((prefix, name)) => (
                prefix.to_string(),
                self.prefix_resolver(prefix, xel),
                name.to_string(),
            ),
        })
    }

    fn prefix_resolver(&self, ns: &str, xel: XmlNode<'_, 'input>) -> String {
        if ns.trim().is_empty() {
            self.lookup_namespace(xel, "").unwrap_or_default()
        } else {
            self.lookup_namespace(xel, ns).unwrap_or_default()
        }
    }

    fn parse_type_arguments(
        &self,
        args: &str,
        xel: XmlNode<'_, 'input>,
        info: &dyn IXamlLineInfo,
    ) -> XamlResult<Vec<Rc<XamlAstXmlTypeReference>>> {
        fn parse<'input>(
            this: &ParserContext<'input>,
            node: &TypeArgumentNode,
            xel: XmlNode<'_, 'input>,
            info: &dyn IXamlLineInfo,
        ) -> XamlResult<Rc<XamlAstXmlTypeReference>> {
            let value = node.value.as_deref().ok_or_else(|| {
                XamlError::internal("NullReferenceException", "Type argument name is missing")
            })?;
            let rv = this.parse_type_name(info, value, xel)?;
            if !node.children.is_empty() {
                let mut generic_arguments = Vec::with_capacity(node.children.len());
                for child in &node.children {
                    generic_arguments.push(parse(this, child, xel, info)?);
                }
                *rv.generic_arguments.borrow_mut() = generic_arguments;
            }
            Ok(rv)
        }

        let tree = CommaSeparatedParenthesesTreeParser::parse(args)
            .map_err(|e| XamlError::parse_exception(e.message, Some(info)))?;
        tree.iter()
            .map(|node| parse(self, node, xel, info))
            .collect()
    }

    fn parse_text_value_or_markup_extension(
        &self,
        ext: &str,
        xel: XmlNode<'_, 'input>,
        info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<dyn IXamlAstValueNode>> {
        let mut ext = ext;
        if ext.starts_with('{') || ext.starts_with("\\{") {
            if let Some(stripped) = ext.strip_prefix("{}") {
                ext = stripped;
            } else {
                let type_resolver = |t: &str| self.parse_type_name(info, t, xel);
                return match SystemXamlMarkupExtensionParser::parse(info, ext, &type_resolver) {
                    Ok(extension_object) => {
                        if let Some(ast_object) = extension_object.cast::<XamlAstObjectNode>() {
                            self.transform_markup_extension_node_properties(&ast_object, xel)?;
                        }

                        Ok(extension_object)
                    }
                    Err(MeScannerError::Parse(parse_ex)) => {
                        Err(XamlError::parse_exception(parse_ex.message, Some(info)))
                    }
                    Err(MeScannerError::Xaml(e)) => Err(e),
                };
            }
        }

        // Do not apply XAML whitespace normalization to attribute values
        Ok(XamlAstTextNode::new(info, ext, true))
    }

    fn transform_markup_extension_node_properties(
        &self,
        ast_object: &Rc<XamlAstObjectNode>,
        xel: XmlNode<'_, 'input>,
    ) -> XamlResult<()> {
        let object_type = ast_object.type_.borrow().clone();
        let xml_type = object_type
            .cast::<XamlAstXmlTypeReference>()
            .ok_or_else(|| {
                XamlError::invalid_cast(format!(
                    "Unable to cast object of type '{}' to type 'XamlAstXmlTypeReference'.",
                    object_type.type_name()
                ))
            })?;

        let remove_child = |prop: &Rc<dyn IXamlAstNode>| {
            let mut children = ast_object.children.borrow_mut();
            if let Some(index) = children.iter().position(|c| c.same_node(prop)) {
                children.remove(index);
            }
        };

        let children = ast_object.children.borrow().clone();
        for prop in children {
            let Some(value_node) = prop.cast::<XamlAstXamlPropertyValueNode>() else {
                continue;
            };
            let Some(prop_name) = value_node.property().cast::<XamlAstNamePropertyReference>()
            else {
                continue;
            };

            let (xmlns_key, xmlns_val, name) =
                self.parse_pair_with_xmlns(&*prop, &prop_name.name(), xel)?;
            let values = value_node.values.borrow().clone();
            if xmlns_val == XamlNamespaces::XAML2006 && name == "TypeArguments" {
                if values.len() != 1 {
                    return Err(XamlError::invalid_operation(if values.is_empty() {
                        "Sequence contains no elements"
                    } else {
                        "Sequence contains more than one element"
                    }));
                }
                let Some(text) = values[0].cast::<XamlAstTextNode>() else {
                    return Err(XamlError::parse_exception(
                        "Unable to resolve TypeArguments. String node with one or multiple type arguments is expected.",
                        Some(&*prop),
                    ));
                };

                let type_arguments = self.parse_type_arguments(&text.text(), xel, &*prop)?;
                xml_type
                    .generic_arguments
                    .borrow_mut()
                    .extend(type_arguments);
                remove_child(&prop);
            } else if !xmlns_key.is_empty() && !name.contains('.') {
                ast_object
                    .children
                    .borrow_mut()
                    .push(XamlAstXmlDirective::new(
                        &*prop,
                        Some(&xmlns_val),
                        &name,
                        values,
                    ));
                remove_child(&prop);
            } else if let Some(child_ast_object) =
                values.first().and_then(|v| v.cast::<XamlAstObjectNode>())
            {
                self.transform_markup_extension_node_properties(&child_ast_object, xel)?;
            }
        }
        Ok(())
    }

    fn parse_new_instance(
        &self,
        el: XmlNode<'_, 'input>,
        root: bool,
        space_mode: XmlSpace,
        depth: usize,
    ) -> XamlResult<Rc<XamlAstObjectNode>> {
        let entered = self.enter_element(el);
        let result = self.parse_new_instance_core(el, &entered, root, space_mode, depth);
        self.leave_element(&entered);
        result
    }

    fn parse_new_instance_core(
        &self,
        el: XmlNode<'_, 'input>,
        entered: &EnteredElement<'_, 'input>,
        root: bool,
        space_mode: XmlSpace,
        depth: usize,
    ) -> XamlResult<Rc<XamlAstObjectNode>> {
        let el_li = self.li(el);
        if depth > MAX_ELEMENT_DEPTH {
            return Err(XamlError::parse_exception(
                format!("Elements are nested too deeply (the limit is {MAX_ELEMENT_DEPTH})"),
                Some(&el_li),
            ));
        }

        let mut space_mode = space_mode;
        let declared_mode = get_declared_whitespace_mode(entered);
        if declared_mode != XmlSpace::None {
            space_mode = declared_mode;
        }

        let local_name = el.tag_name().name();
        if local_name.contains('.') {
            return Err(XamlError::parse_exception(
                "Dots aren't allowed in type names",
                Some(&el_li),
            ));
        }
        let type_ = self.get_type_reference(el, &entered.namespace);
        let i = XamlAstObjectNode::new(&el_li, type_.clone());
        for attr in &entered.attributes {
            let attr = match attr {
                ElementAttribute::NamespaceDeclaration(line_info) => {
                    if !root {
                        return Err(XamlError::parse_exception(
                            "xmlns declarations are only allowed on the root element to preserve memory",
                            Some(line_info),
                        ));
                    }
                    continue;
                }
                ElementAttribute::Attribute(attr) => attr,
            };

            if attr.namespace.starts_with("http://www.w3.org") {
                // Silently ignore all xml-parser related attributes
            }
            // Parse type arguments
            else if attr.namespace == XamlNamespaces::XAML2006
                && attr.local_name == "TypeArguments"
            {
                *type_.generic_arguments.borrow_mut() =
                    self.parse_type_arguments(attr.value, el, &attr.line_info)?;
            }
            // Parse as a directive
            else if !attr.namespace.is_empty() && !attr.local_name.contains('.') {
                i.children.borrow_mut().push(XamlAstXmlDirective::new(
                    &el_li,
                    Some(&attr.namespace),
                    attr.local_name,
                    vec![self.parse_text_value_or_markup_extension(
                        attr.value,
                        el,
                        &attr.line_info,
                    )?],
                ));
            }
            // Parse as a property
            else {
                let mut pname = attr.local_name;
                let mut ptype: Rc<dyn IXamlAstTypeReference> = i.type_.borrow().clone();

                if let Some((type_name, property_name)) = pname.split_once('.') {
                    pname = property_name;
                    let ns = if attr.namespace.is_empty() {
                        self.lookup_namespace(el, "").unwrap_or_default()
                    } else {
                        attr.namespace.clone()
                    };
                    ptype = XamlAstXmlTypeReference::new(&el_li, Some(&ns), type_name);
                }

                i.children
                    .borrow_mut()
                    .push(XamlAstXamlPropertyValueNode::new(
                        &el_li,
                        XamlAstNamePropertyReference::new(&el_li, ptype, pname, type_.clone()),
                        self.parse_text_value_or_markup_extension(attr.value, el, &attr.line_info)?,
                        true,
                    ));
            }
        }

        let mut skip_following = false;
        for node in el.children() {
            if self.is_hidden_child(node, &mut skip_following) {
                continue;
            }

            if node.is_element() {
                let node_local_name = node.tag_name().name();
                if let Some((type_name, property_name)) = node_local_name.split_once('.') {
                    let node_entered = self.enter_element(node);
                    let result = (|| -> XamlResult<Rc<XamlAstXamlPropertyValueNode>> {
                        if !node_entered.attributes.is_empty() {
                            return Err(XamlError::parse_exception(
                                "Attributes aren't allowed on element properties",
                                Some(&self.li(node)),
                            ));
                        }

                        let declaring_type: Rc<dyn IXamlAstTypeReference> = if type_name
                            == type_.name()
                            && Some(node_entered.namespace.as_str())
                                == type_.xml_namespace().as_deref()
                        {
                            type_.clone()
                        } else {
                            XamlAstXmlTypeReference::new(
                                &el_li,
                                Some(&node_entered.namespace),
                                type_name,
                            )
                        };
                        Ok(XamlAstXamlPropertyValueNode::with_values(
                            &el_li,
                            XamlAstNamePropertyReference::new(
                                &el_li,
                                declaring_type,
                                property_name,
                                type_.clone(),
                            ),
                            self.parse_value_node_children(node, space_mode, depth + 1)?,
                            false,
                        ))
                    })();
                    self.leave_element(&node_entered);
                    i.children.borrow_mut().push(result?);
                    continue;
                }
            }

            if let Some(parsed) = self.parse_value_node(node, space_mode, depth + 1)? {
                i.children.borrow_mut().push(parsed);
            }
        }

        Ok(i)
    }

    fn parse_value_node(
        &self,
        node: XmlNode<'_, 'input>,
        space_mode: XmlSpace,
        depth: usize,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        if node.is_element() {
            return Ok(Some(
                self.parse_new_instance(node, false, space_mode, depth)?,
            ));
        }

        if node.is_text() {
            let preserve_whitespace = space_mode == XmlSpace::Preserve;
            return Ok(Some(XamlAstTextNode::new(
                &self.li(node),
                node.text().unwrap_or(""),
                preserve_whitespace,
            )));
        }

        Ok(None)
    }

    fn parse_value_node_children(
        &self,
        parent: XmlNode<'_, 'input>,
        space_mode: XmlSpace,
        depth: usize,
    ) -> XamlResult<Vec<Rc<dyn IXamlAstValueNode>>> {
        let mut lst = Vec::new();
        let mut skip_following = false;
        for n in parent.children() {
            if self.is_hidden_child(n, &mut skip_following) {
                continue;
            }
            if let Some(parsed) = self.parse_value_node(n, space_mode, depth)? {
                lst.push(parsed);
            }
        }
        Ok(lst)
    }
}

/// Get the xml:space mode declared on the element.
fn get_declared_whitespace_mode(entered: &EnteredElement<'_, '_>) -> XmlSpace {
    let declared_mode = entered.attributes.iter().find_map(|a| match a {
        ElementAttribute::Attribute(a)
            if a.namespace == XML_NAMESPACE && a.local_name == "space" =>
        {
            Some(a.value)
        }
        _ => None,
    });
    match declared_mode {
        Some("default") => XmlSpace::Default,
        Some("preserve") => XmlSpace::Preserve,
        _ => XmlSpace::None,
    }
}

impl XDocumentXamlParser {
    pub fn parse(
        s: &str,
        compatibility_mappings: Option<&HashMap<String, String>>,
    ) -> XamlResult<XamlDocument> {
        let lines = LineIndex::new(s);

        // The document type declaration is ignored: it is blanked out (keeping all offsets and
        // line breaks intact) before the XML parser sees the text.
        let without_doctype: Option<String> = prescan(s, &lines)?.map(|range| {
            let mut text = String::with_capacity(s.len());
            text.push_str(&s[..range.start]);
            text.extend(s[range.clone()].bytes().map(|b| {
                if b == b'\n' || b == b'\r' {
                    b as char
                } else {
                    ' '
                }
            }));
            text.push_str(&s[range.end..]);
            text
        });
        let xml_text = without_doctype.as_deref().unwrap_or(s);

        let xml = roxmltree::Document::parse(xml_text).map_err(|e| {
            let pos = e.pos();
            XamlError::xml_exception(e.to_string(), pos.row as i32, pos.col as i32)
        })?;

        let context = ParserContext {
            lines,
            source: xml_text,
            compat: RefCell::new(CompatibleXmlReader::new(compatibility_mappings)),
        };

        let root = xml.root_element();
        if context.is_ignored_element(root) {
            return Err(XamlError::xml_exception("Root element is missing.", 0, 0));
        }

        let mut doc = XamlDocument::new();
        doc.set_root(context.parse_new_instance(root, true, XmlSpace::Default, 0)?);

        for raw in scan_start_tag(xml_text, root.range().start) {
            if let Some(prefix) = raw.declared_prefix() {
                let value = context.lookup_namespace(root, prefix).unwrap_or_default();
                doc.namespace_aliases.insert(prefix.to_string(), value);
            }
        }

        Ok(doc)
    }

    pub fn parse_with_settings(
        s: &str,
        settings: &XDocumentXamlParserSettings,
    ) -> XamlResult<XamlDocument> {
        Self::parse(s, settings.compatible_namespaces.as_ref())
    }
}
