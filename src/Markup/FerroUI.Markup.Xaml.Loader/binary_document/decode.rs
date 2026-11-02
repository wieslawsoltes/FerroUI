use super::*;
use std::{collections::HashMap, rc::Rc, sync::Arc};
use xamlx::ast::*;

struct Entry {
    node: Rc<dyn IXamlAstNode>,
    expansion: Expansion,
    used: bool,
}

struct Reader<'a> {
    input: &'a [u8],
    strings: Vec<&'a str>,
    scopes: Vec<Arc<XamlNamespaceScope>>,
    nodes: Vec<Entry>,
    expanded_text: usize,
    alias_entries_left: usize,
    expansion: Expansion,
}

pub(super) fn read_uint(input: &mut &[u8]) -> XamlResult<u32> {
    let mut v = 0;
    for shift in (0..32).step_by(7) {
        let (&b, rest) = input
            .split_first()
            .ok_or_else(|| invalid("truncated integer"))?;
        *input = rest;
        if (shift == 28 && b > 15) || (shift > 0 && b == 0) {
            return Err(invalid("invalid unsigned integer"));
        }
        v |= u32::from(b & 127) << shift;
        if b < 128 {
            return Ok(v);
        }
    }
    Err(invalid("integer overflow"))
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> XamlResult<&'a [u8]> {
        let (value, rest) = self
            .input
            .split_at_checked(n)
            .ok_or_else(|| invalid("truncated image"))?;
        self.input = rest;
        Ok(value)
    }

    fn uint(&mut self) -> XamlResult<u32> {
        read_uint(&mut self.input)
    }

    fn count(&mut self, min_bytes: usize) -> XamlResult<usize> {
        let n = self.uint()? as usize;
        if n > MAX_NODES || n > self.input.len() / min_bytes {
            return Err(invalid("invalid count"));
        }
        Ok(n)
    }

    fn flag(&mut self) -> XamlResult<bool> {
        match self.take(1)? {
            [0] => Ok(false),
            [1] => Ok(true),
            _ => Err(invalid("invalid flag")),
        }
    }

    fn text_at(&mut self, index: usize) -> XamlResult<&'a str> {
        let s = *self
            .strings
            .get(index)
            .ok_or_else(|| invalid("string index out of range"))?;
        self.expanded_text = self
            .expanded_text
            .checked_sub(s.len())
            .ok_or_else(|| invalid("expanded strings exceed limit"))?;
        Ok(s)
    }

    fn text(&mut self) -> XamlResult<&'a str> {
        let idx = self.uint()? as usize;
        self.text_at(idx)
    }

    fn optional(&mut self) -> XamlResult<Option<&'a str>> {
        match self.uint()? {
            0 => Ok(None),
            n => self.text_at(n as usize - 1).map(Some),
        }
    }

    fn aliases(&mut self) -> XamlResult<HashMap<String, String>> {
        let count = self.count(2)?;
        self.alias_entries_left = self
            .alias_entries_left
            .checked_sub(count)
            .ok_or_else(|| invalid("namespace maps exceed limit"))?;
        let mut aliases = HashMap::with_capacity(count);
        for _ in 0..count {
            if aliases
                .insert(self.text()?.to_owned(), self.text()?.to_owned())
                .is_some()
            {
                return Err(invalid("duplicate namespace prefix"));
            }
        }
        Ok(aliases)
    }

    fn reference<T: ?Sized + XamlAstCast>(&mut self) -> XamlResult<Rc<T>> {
        let back = self.uint()? as usize;
        let index = self
            .nodes
            .len()
            .checked_sub(back)
            .and_then(|n| n.checked_sub(1))
            .ok_or_else(|| invalid("invalid backwards node reference"))?;
        let entry = &mut self.nodes[index];
        let result = entry
            .node
            .cast::<T>()
            .ok_or_else(|| invalid("node in wrong role"))?;
        // The parser shares XML type references between an object, its members and
        // generic arguments. Other nodes are mutable occurrences owned by one parent.
        if entry.used && !entry.node.is::<XamlAstXmlTypeReference>() {
            return Err(invalid("shared mutable occurrence"));
        }
        entry.used = true;
        self.expansion.include(entry.expansion)?;
        Ok(result)
    }

    fn list<T: ?Sized + XamlAstCast>(&mut self) -> XamlResult<Vec<Rc<T>>> {
        let n = self.count(1)?;
        (0..n).map(|_| self.reference()).collect()
    }

    fn record(
        &mut self,
        tag: u8,
        line: i32,
        pos: i32,
        scope: u32,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let scope = match scope {
            0 => None,
            n => Some(
                self.scopes
                    .get(n as usize - 1)
                    .ok_or_else(|| invalid("scope index out of range"))?
                    .clone(),
            ),
        };
        let li = XamlLineInfo::new(line, pos).with_namespace_scope(scope);
        match tag {
            TYPE => {
                let ns = self.optional()?;
                let name = self.text()?;
                let extension = self.flag()?;
                let ty =
                    XamlAstXmlTypeReference::with_generic_arguments(&li, ns, name, self.list()?);
                ty.is_markup_extension.set(extension);
                Ok(ty)
            }
            PROPERTY => Ok(XamlAstNamePropertyReference::new(
                &li,
                self.reference()?,
                self.text()?,
                self.reference()?,
            )),
            TEXT => {
                let n = XamlAstTextNode::new(&li, self.text()?, self.flag()?);
                n.type_.replace(self.reference()?);
                Ok(n)
            }
            OBJECT => {
                let obj = XamlAstObjectNode::new(&li, self.reference()?);
                obj.arguments.replace(self.list()?);
                let children: Vec<Rc<dyn IXamlAstNode>> = self.list()?;
                if children.iter().any(|c| {
                    !c.is::<dyn IXamlAstValueNode>() && !c.is::<dyn IXamlAstManipulationNode>()
                }) {
                    return Err(invalid("invalid object child"));
                }
                obj.children.replace(children);
                Ok(obj)
            }
            ASSIGN => {
                let property = self.reference()?;
                let attribute = self.flag()?;
                Ok(XamlAstXamlPropertyValueNode::with_values(
                    &li,
                    property,
                    self.list()?,
                    attribute,
                ))
            }
            DIRECTIVE => Ok(XamlAstXmlDirective::new(
                &li,
                self.optional()?,
                self.text()?,
                self.list()?,
            )),
            _ => Err(invalid("unknown node opcode")),
        }
    }
}

struct Columns<'a> {
    tags: &'a [u8],
    lines: &'a [u8],
    positions: &'a [u8],
    scopes: &'a [u8],
    last_line: i32,
}

impl<'a> Columns<'a> {
    fn read(r: &mut Reader<'a>, count: usize) -> XamlResult<Self> {
        let lines = r.uint()? as usize;
        let positions = r.uint()? as usize;
        let scopes = r.uint()? as usize;
        Ok(Self {
            tags: r.take(count)?,
            lines: r.take(lines)?,
            positions: r.take(positions)?,
            scopes: r.take(scopes)?,
            last_line: 0,
        })
    }

    fn next(&mut self, i: usize) -> XamlResult<(u8, i32, i32, u32)> {
        let n = read_uint(&mut self.lines)?;
        let delta = ((n >> 1) as i32) ^ -((n & 1) as i32);
        self.last_line = self
            .last_line
            .checked_add(delta)
            .filter(|&v| v >= 0)
            .ok_or_else(|| invalid("source line overflow"))?;
        let pos = i32::try_from(read_uint(&mut self.positions)?)
            .map_err(|_| invalid("source column overflow"))?;
        Ok((
            self.tags[i],
            self.last_line,
            pos,
            read_uint(&mut self.scopes)?,
        ))
    }

    fn exhausted(&self) -> bool {
        self.lines.is_empty() && self.positions.is_empty() && self.scopes.is_empty()
    }
}

// Return validated, uncompressed operand bytes while preserving the graph's
// row/column layout. Comparisons can reuse this without constructing an AST.
pub(super) fn payload(bytes: &[u8]) -> XamlResult<std::borrow::Cow<'_, [u8]>> {
    let size = expanded_size(bytes)? - 6;
    match bytes[5] & 1 {
        0 => Ok(std::borrow::Cow::Borrowed(&bytes[6..])),
        _ => {
            let data = &bytes[10..];
            let mut buffer = vec![0; size];
            let mut z = flate2::Decompress::new(true);
            let status = z
                .decompress(data, &mut buffer, flate2::FlushDecompress::Finish)
                .map_err(|_| invalid("invalid compressed payload"))?;
            if status != flate2::Status::StreamEnd
                || z.total_out() != size as u64
                || z.total_in() != data.len() as u64
            {
                return Err(invalid("compressed payload length mismatch"));
            }
            Ok(std::borrow::Cow::Owned(buffer))
        }
    }
}

/// Decode and validate a complete syntax graph. No framework types are accessed or
/// application constructors called until the caller transforms the returned document.
pub fn decode(bytes: &[u8]) -> XamlResult<XamlDocument> {
    let payload = payload(bytes)?;
    let mut r = Reader {
        input: &payload,
        strings: vec![],
        scopes: vec![],
        nodes: vec![],
        expanded_text: MAX_EXPANDED_TEXT,
        alias_entries_left: MAX_NODES,
        expansion: Expansion::LEAF,
    };
    let strings = r.count(1)?;
    r.strings.reserve(strings);
    for _ in 0..strings {
        let len = r.uint()? as usize;
        let text = std::str::from_utf8(r.take(len)?).map_err(|_| invalid("malformed UTF-8"))?;
        r.strings.push(text);
    }
    let mut document = XamlDocument::new();
    document.document = r.optional()?.map(str::to_owned);
    document.namespace_aliases = r.aliases()?;
    let scopes = r.count(1)?;
    for _ in 0..scopes {
        let aliases = r.aliases()?;
        r.scopes.push(XamlNamespaceScope::new(None, aliases));
    }
    let nodes = r.count(5)?;
    let mut columns = (bytes[5] >= 2)
        .then(|| Columns::read(&mut r, nodes))
        .transpose()?;
    r.nodes.reserve(nodes);
    for i in 0..nodes {
        r.expansion = Expansion::LEAF;
        let (tag, line, pos, scope) = if let Some(columns) = &mut columns {
            columns.next(i)?
        } else {
            (
                r.take(1)?[0],
                i32::try_from(r.uint()?).map_err(|_| invalid("source line overflow"))?,
                i32::try_from(r.uint()?).map_err(|_| invalid("source column overflow"))?,
                r.uint()?,
            )
        };
        let node = r.record(tag, line, pos, scope)?;
        r.nodes.push(Entry {
            node,
            expansion: r.expansion,
            used: false,
        });
    }
    if columns.as_ref().is_some_and(|c| !c.exhausted()) {
        return Err(invalid("trailing bytes in source columns"));
    }
    let root = r.nodes.pop().ok_or_else(|| invalid("missing root"))?.node;
    if !r.input.is_empty() || !root.is::<XamlAstObjectNode>() || r.nodes.iter().any(|n| !n.used) {
        return Err(invalid("invalid root, unreachable nodes or trailing bytes"));
    }
    document.set_root(root);
    Ok(document)
}
