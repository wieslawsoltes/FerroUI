use super::*;
use std::{collections::HashMap, rc::Rc, sync::Arc};
use xamlx::ast::*;

type Node = Rc<dyn IXamlAstNode>;

// A record refers to children that precede it in the stream. Building the postorder
// iteratively preserves node identity without recursing down user-supplied markup.
enum Field {
    Uint(u32),
    Node(Node),
}

fn children<T: ?Sized + XamlAstCast>(fields: &mut Vec<Field>, list: &[Rc<T>]) -> XamlResult<()> {
    if list.len() > MAX_NODES {
        return Err(invalid("invalid count"));
    }
    fields.push(Field::Uint(count(list.len())?));
    fields.extend(list.iter().map(|n| Field::Node(n.as_node())));
    Ok(())
}

fn count(n: usize) -> XamlResult<u32> {
    u32::try_from(n).map_err(|_| invalid("count exceeds the wire range"))
}

fn put(out: &mut Vec<u8>, mut n: u32) {
    while n >= 128 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
}

#[derive(Default)]
struct Encoder {
    strings: HashMap<String, u32>,
    string_pool: Vec<u8>,
    expanded_text: usize,
    alias_entries: usize,
    scopes: HashMap<usize, u32>,
    scope_pool: Vec<u8>,
    ids: HashMap<usize, u32>,
    expansions: Vec<Expansion>,
    records: Vec<u8>,
    columns: Option<Columns>,
}

// In a theme, source sites, opcodes and operand sequences each repeat at
// different intervals. Keep each independently compressible without dropping
// diagnostics or changing the ordering or identity of nodes.
#[derive(Default)]
struct Columns {
    tags: Vec<u8>,
    lines: Vec<u8>,
    positions: Vec<u8>,
    scopes: Vec<u8>,
    last_line: i32,
}

impl Columns {
    fn push(&mut self, tag: u8, line: i32, pos: u32, scope: u32) {
        let delta = line - self.last_line;
        self.last_line = line;
        self.tags.push(tag);
        put(&mut self.lines, ((delta << 1) ^ (delta >> 31)) as u32);
        put(&mut self.positions, pos);
        put(&mut self.scopes, scope);
    }

    fn byte_len(&self) -> usize {
        self.tags.len() + self.lines.len() + self.positions.len() + self.scopes.len()
    }
}

impl Encoder {
    fn string(&mut self, s: &str) -> XamlResult<u32> {
        self.expanded_text = self
            .expanded_text
            .checked_add(s.len())
            .filter(|&total| total <= MAX_EXPANDED_TEXT)
            .ok_or_else(|| invalid("expanded strings exceed limit"))?;
        if let Some(&index) = self.strings.get(s) {
            return Ok(index);
        }
        if self.strings.len() >= MAX_NODES || s.len() + self.string_pool.len() > MAX_DOCUMENT_BYTES
        {
            return Err(invalid("string table exceeds limit"));
        }
        let id = count(self.strings.len())?;
        put(&mut self.string_pool, count(s.len())?);
        self.string_pool.extend_from_slice(s.as_bytes());
        self.strings.insert(s.to_owned(), id);
        Ok(id)
    }

    fn optional(&mut self, s: Option<&str>) -> XamlResult<u32> {
        s.map(|s| self.string(s).map(|n| n + 1))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    fn aliases(&mut self, aliases: &HashMap<String, String>) -> XamlResult<Vec<u8>> {
        self.alias_entries = self
            .alias_entries
            .checked_add(aliases.len())
            .filter(|&total| total <= MAX_NODES)
            .ok_or_else(|| invalid("namespace maps exceed limit"))?;
        let mut pairs: Vec<_> = aliases.iter().collect();
        pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
        let mut bytes = Vec::new();
        put(&mut bytes, count(pairs.len())?);
        for (k, v) in pairs {
            put(&mut bytes, self.string(k)?);
            put(&mut bytes, self.string(v)?);
        }
        Ok(bytes)
    }

    fn scope(&mut self, node: &Node) -> XamlResult<u32> {
        let Some(scope) = node.namespace_scope() else {
            return Ok(0);
        };
        let ptr = Arc::as_ptr(&scope) as usize;
        if let Some(&id) = self.scopes.get(&ptr) {
            return Ok(id);
        }
        let bytes = self.aliases(scope.aliases())?;
        self.scope_pool.extend(bytes);
        let id = count(self.scopes.len())? + 1;
        self.scopes.insert(ptr, id);
        Ok(id)
    }

    fn fields(&mut self, n: &Node) -> XamlResult<(u8, Vec<Field>)> {
        use Field::{Node as Ref, Uint as U};
        let mut body = Vec::new();
        let tag = if let Some(t) = n.cast::<XamlAstXmlTypeReference>() {
            body.extend([
                U(self.optional(t.xml_namespace.borrow().as_deref())?),
                U(self.string(&t.name.borrow())?),
                U(t.is_markup_extension.get() as u32),
            ]);
            children(&mut body, &t.generic_arguments.borrow())?;
            TYPE
        } else if let Some(p) = n.cast::<XamlAstNamePropertyReference>() {
            body.extend([
                Ref(p.declaring_type.borrow().as_node()),
                U(self.string(&p.name.borrow())?),
                Ref(p.target_type.borrow().as_node()),
            ]);
            PROPERTY
        } else if let Some(t) = n.cast::<XamlAstTextNode>() {
            body.extend([
                U(self.string(&t.text.borrow())?),
                U(t.preserve_whitespace as u32),
                Ref(t.type_.borrow().as_node()),
            ]);
            TEXT
        } else if let Some(o) = n.cast::<XamlAstObjectNode>() {
            body.push(Ref(o.type_.borrow().as_node()));
            children(&mut body, &o.arguments.borrow())?;
            let list = o.children.borrow();
            if list.iter().any(|n| {
                !n.is::<dyn IXamlAstValueNode>() && !n.is::<dyn IXamlAstManipulationNode>()
            }) {
                return Err(invalid("invalid object child"));
            }
            children(&mut body, &list)?;
            OBJECT
        } else if let Some(p) = n.cast::<XamlAstXamlPropertyValueNode>() {
            body.extend([
                Ref(p.property.borrow().as_node()),
                U(p.is_attribute_syntax as u32),
            ]);
            children(&mut body, &p.values.borrow())?;
            ASSIGN
        } else if let Some(d) = n.cast::<XamlAstXmlDirective>() {
            body.extend([
                U(self.optional(d.namespace.borrow().as_deref())?),
                U(self.string(&d.name.borrow())?),
            ]);
            children(&mut body, &d.values.borrow())?;
            DIRECTIVE
        } else {
            return Err(invalid(&format!("{} is not a parser node", n.type_name())));
        };
        Ok((tag, body))
    }

    fn graph(&mut self, root: Node) -> XamlResult<()> {
        struct Frame {
            node: Node,
            tag: u8,
            fields: Vec<Field>,
            next: usize,
        }
        let mut stack = Vec::<Frame>::new();
        let mut pending = Some(root);
        while let Some(n) = pending.take() {
            if stack.len() >= MAX_DEPTH || self.ids.len() >= MAX_NODES {
                return Err(invalid("syntax graph exceeds limit"));
            }
            let ptr = Rc::as_ptr(&n) as *const () as usize;
            if self.ids.insert(ptr, u32::MAX).is_some() {
                return Err(invalid("cyclic syntax"));
            }
            let (tag, fields) = self.fields(&n)?;
            stack.push(Frame {
                node: n,
                tag,
                fields,
                next: 0,
            });
            while let Some(frame) = stack.last_mut() {
                while let Some(field) = frame.fields.get(frame.next) {
                    frame.next += 1;
                    if let Field::Node(child) = field {
                        let ptr = Rc::as_ptr(child) as *const () as usize;
                        match self.ids.get(&ptr) {
                            Some(&u32::MAX) => return Err(invalid("cyclic syntax")),
                            None => {
                                pending = Some(child.clone());
                                break;
                            }
                            Some(_) if !child.is::<XamlAstXmlTypeReference>() => {
                                return Err(invalid("shared mutable occurrence"));
                            }
                            Some(_) => {}
                        }
                    }
                }
                if pending.is_some() {
                    break;
                }
                let frame = stack.pop().unwrap();
                let id = count(self.expansions.len())?;
                let mut expansion = Expansion::LEAF;
                let ptr = Rc::as_ptr(&frame.node) as *const () as usize;
                self.ids.insert(ptr, id);
                let scope = self.scope(&frame.node)?;
                let line = frame.node.line();
                let pos = u32::try_from(frame.node.position())
                    .map_err(|_| invalid("negative source position"))?;
                let unsigned_line =
                    u32::try_from(line).map_err(|_| invalid("negative source line"))?;
                if let Some(columns) = &mut self.columns {
                    columns.push(frame.tag, line, pos, scope);
                } else {
                    self.records.push(frame.tag);
                    for v in [unsigned_line, pos, scope] {
                        put(&mut self.records, v);
                    }
                }
                for field in frame.fields {
                    let v = match field {
                        Field::Uint(v) => v,
                        Field::Node(n) => {
                            let child = self.ids[&(Rc::as_ptr(&n) as *const () as usize)];
                            expansion.include(self.expansions[child as usize])?;
                            id - child - 1
                        }
                    };
                    put(&mut self.records, v);
                }
                self.expansions.push(expansion);
                if self.records.len()
                    + self.scope_pool.len()
                    + self.string_pool.len()
                    + self.columns.as_ref().map_or(0, Columns::byte_len)
                    > MAX_DOCUMENT_BYTES
                {
                    return Err(invalid("image too large"));
                }
            }
        }
        Ok(())
    }
}

/// Encode syntax returned by the XML parser. Transformed CLR nodes are rejected.
pub fn encode(document: &XamlDocument) -> XamlResult<Vec<u8>> {
    compress(encode_stored(document)?, flate2::Compression::fast())
}

pub(super) fn encode_columnar(document: &XamlDocument) -> XamlResult<Vec<u8>> {
    compress(write_syntax(document, true)?, flate2::Compression::best())
}

fn compress(mut stored: Vec<u8>, level: flate2::Compression) -> XamlResult<Vec<u8>> {
    use flate2::{Compress, FlushCompress, Status};
    // Reserve only the byte budget at which storing the compressed image pays off.
    // Incompressible input fills this buffer and immediately takes the stored path.
    if stored.len() >= 1024 {
        let payload = &stored[6..];
        let budget = stored.len() - stored.len() / 8 - 11;
        let mut compressed = Vec::with_capacity(budget);
        let status = Compress::new(level, true)
            .compress_vec(payload, &mut compressed, FlushCompress::Finish)
            .map_err(|_| invalid("compression failed"))?;
        if status == Status::StreamEnd && compressed.len() <= budget {
            let size = count(payload.len())?;
            stored.truncate(6);
            stored[5] |= 1;
            stored.extend_from_slice(&size.to_le_bytes());
            stored.extend(compressed);
        }
    }
    Ok(stored)
}

/// Encode an uncompressed FXD document for in-memory transfer or outer compression.
pub fn encode_stored(document: &XamlDocument) -> XamlResult<Vec<u8>> {
    write_syntax(document, false)
}

fn write_syntax(document: &XamlDocument, columnar: bool) -> XamlResult<Vec<u8>> {
    let root = document.root()?;
    if !root.is::<XamlAstObjectNode>() {
        return Err(invalid("root is not an object"));
    }
    let mut enc = Encoder {
        columns: columnar.then(Columns::default),
        ..Encoder::default()
    };
    let name = enc.optional(document.document.as_deref())?;
    let aliases = enc.aliases(&document.namespace_aliases)?;
    enc.graph(root)?;
    let mut header = MAGIC.to_vec();
    header.extend([VERSION, if columnar { 2 } else { 0 }]);
    put(&mut header, count(enc.strings.len())?);
    let mut meta = Vec::new();
    put(&mut meta, name);
    meta.extend(aliases);
    put(&mut meta, count(enc.scopes.len())?);
    put(&mut enc.scope_pool, count(enc.ids.len())?);
    let mut sections = vec![header, enc.string_pool, meta, enc.scope_pool];
    if let Some(columns) = enc.columns {
        // One tag per node. Only the three varint lanes need explicit lengths.
        let mut lengths = Vec::new();
        for n in [
            columns.lines.len(),
            columns.positions.len(),
            columns.scopes.len(),
        ] {
            put(&mut lengths, count(n)?);
        }
        sections.extend([
            lengths,
            columns.tags,
            columns.lines,
            columns.positions,
            columns.scopes,
        ]);
    }
    sections.push(enc.records);
    let size: usize = sections.iter().map(Vec::len).sum();
    if size > MAX_DOCUMENT_BYTES {
        return Err(invalid("image too large"));
    }
    let mut out = Vec::with_capacity(size);
    for section in sections {
        out.extend(section);
    }
    Ok(out)
}
