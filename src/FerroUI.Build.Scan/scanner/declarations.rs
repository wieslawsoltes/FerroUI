//! The readers of the bodies of the declaration macros (`ferro_class!`,
//! `ferro_static_type!`, `ferro_properties!` / `ferro_property!`,
//! `ferro_class_info!`, `ferro_markup_type!`, `ferro_markup_enum!`): the
//! grammar the macros accept (`ferroui_base::type_system`,
//! `ferroui_base::ferro_property`, `ferroui_base::metadata::markup_macros`;
//! PORTING-GUIDE.md, "Markup metadata"), read from the tokens of an
//! invocation. Nothing is expanded and nothing is resolved here: types and
//! callables stay tokens, with the place they were written at.
//!
//! A form a reader does not know is an error with its line, never a guess.

use proc_macro2::{Delimiter, TokenTree};

use super::tokens::{
    generic_type, group_of, ident_of, is_arrow, is_path_separator, is_punct, line_of, split_types, tokens_of, Cursor, ParseError, Tokens, TypeEnd,
};
use crate::model::{RegistrationModel, TypeKind};

/// The declaration macros: the ones an invocation of is counted and read.
pub(crate) const DECLARATION_MACROS: &[&str] =
    &["ferro_class", "ferro_static_type", "ferro_properties", "ferro_property", "ferro_class_info", "ferro_markup_type", "ferro_markup_enum"];

/// What one invocation of a declaration macro declares.
pub(crate) enum Declaration {
    /// `ferro_class!(Name: Base ..)`; `virtual_trait` is the trait of the virtual members
    /// the class introduces (`virtuals NameImpl: ..`).
    Class { name: String, base: Tokens, virtual_trait: Option<String> },
    /// `ferro_static_type!(Name)`.
    StaticType { name: String },
    /// `ferro_properties! { impl Owner .. { .. } }`, or one `ferro_property!`.
    /// `function_of` is the type of the `impl` block an accessor is written in, when it is
    /// not the owner (`ferro_property!(for Owner; ..)` among the members of another type).
    Properties { owner: String, accessors: Vec<Accessor>, function_of: Option<String> },
    /// `ferro_class_info!(Name { new: .., interfaces: [..], markup: {..} })`.
    ClassInfo { name: String, new: Option<Tokens>, interfaces: Vec<Tokens>, markup: Option<MarkupBody> },
    /// `ferro_markup_type!(kind Type [as "Name"] { .. })`.
    MarkupType { kind: TypeKind, type_: Tokens, is_dyn: bool, name: Option<String>, body: MarkupBody },
    /// `ferro_markup_enum!([flags] Name { members } [, { .. }])`.
    MarkupEnum { name: String, flags: bool, members: Vec<EnumMember>, body: MarkupBody },
}

/// The accessor of a registered property: `vis fn name() -> Type { body }`.
pub(crate) struct Accessor {
    pub name: String,
    pub visibility: String,
    pub return_type: Tokens,
    pub body: Tokens,
    pub line: usize,
}

pub(crate) struct EnumMember {
    pub name: String,
    /// `Member = Variant` of a plain enumeration.
    pub variant: Option<String>,
    /// `Member = expression` of a set of flags.
    pub value: Option<Tokens>,
}

pub(crate) enum RawValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(String),
    Str(String),
    Type(Tokens),
    Array(Vec<RawValue>),
}

pub(crate) struct RawAttribute {
    pub name: String,
    pub arguments: Vec<RawValue>,
    pub properties: Vec<(String, RawValue)>,
}

pub(crate) struct RawParameter {
    pub name: Option<String>,
    pub type_: Tokens,
    pub attributes: Vec<RawAttribute>,
}

/// A constructor, a method, a static field or an event, as declared.
#[derive(Default)]
pub(crate) struct RawMember {
    pub name: String,
    pub parameters: Vec<RawParameter>,
    pub return_type: Option<Tokens>,
    pub is_static: bool,
    pub fallible: bool,
    pub attributes: Vec<RawAttribute>,
    pub callable: Tokens,
}

/// An accessor of a plain property: whether it is declared with `try_`, and the callable.
pub(crate) struct RawAccessor {
    pub fallible: bool,
    pub callable: Tokens,
}

pub(crate) struct RawProperty {
    pub name: String,
    pub parameters: Vec<RawParameter>,
    pub type_: Tokens,
    pub getter: Option<RawAccessor>,
    pub setter: Option<RawAccessor>,
    pub attributes: Vec<RawAttribute>,
}

/// The parts of the body of markup metadata (`markup: {..}`, the body of
/// `ferro_markup_type!`, the second group of `ferro_markup_enum!`).
#[derive(Default)]
pub(crate) struct MarkupBody {
    pub namespace: Option<String>,
    pub handles: Vec<Tokens>,
    pub this: Option<Tokens>,
    pub base: Option<Tokens>,
    pub interfaces: Vec<Tokens>,
    pub generic: Option<(String, Vec<Tokens>)>,
    pub content: Option<String>,
    pub parse: Option<Tokens>,
    pub constructors: Vec<RawMember>,
    pub properties: Vec<RawProperty>,
    pub static_properties: Vec<RawProperty>,
    pub indexers: Vec<RawProperty>,
    pub property_attributes: Vec<(String, Vec<RawAttribute>)>,
    pub notify_property_changed: Option<Tokens>,
    pub type_info: Option<Tokens>,
    pub methods: Vec<RawMember>,
    pub fields: Vec<RawMember>,
    pub events: Vec<RawMember>,
    pub attributes: Vec<RawAttribute>,
}

/// Reads the invocation of the declaration macro `name` with the tokens `tokens`, which
/// starts on `line`.
pub(crate) fn read_declaration(name: &str, tokens: &[TokenTree], line: usize) -> Result<Declaration, ParseError> {
    let mut cursor = Cursor::new(tokens, line);
    match name {
        "ferro_class" => read_class(&mut cursor),
        "ferro_static_type" => {
            let name = cursor.take_ident("the name of the type")?;
            end(&cursor)?;
            Ok(Declaration::StaticType { name })
        }
        "ferro_properties" => read_properties(&mut cursor),
        "ferro_property" => {
            let (owner, accessor) = read_property(&mut cursor)?;
            match owner {
                Some(owner) => Ok(Declaration::Properties { owner, accessors: vec![accessor], function_of: None }),
                None => Err(ParseError { line, message: "a `ferro_property!` without `for Owner;` outside an `impl` block has no owner".to_string() }),
            }
        }
        "ferro_class_info" => read_class_info(&mut cursor),
        "ferro_markup_type" => read_markup_type(&mut cursor),
        "ferro_markup_enum" => read_markup_enum(&mut cursor),
        other => Err(ParseError { line, message: format!("`{other}!` is not a declaration macro") }),
    }
}

fn end(cursor: &Cursor) -> Result<(), ParseError> {
    if cursor.is_end() {
        Ok(())
    } else {
        cursor.error("the end of the declaration")
    }
}

/// `Name: Base` or `Name: Base, virtuals Impl: ParentImpl { fn member(this, ..) -> T; .. }`.
fn read_class(cursor: &mut Cursor) -> Result<Declaration, ParseError> {
    let name = cursor.take_ident("the name of the class")?;
    cursor.expect_punct(':')?;
    let position = cursor.position();
    let base = match cursor.next() {
        Some(token @ TokenTree::Ident(_)) => vec![token.clone()],
        _ => {
            cursor.rewind(position);
            return cursor.error("the base class (one identifier)");
        }
    };
    if cursor.is_end() {
        return Ok(Declaration::Class { name, base, virtual_trait: None });
    }
    cursor.expect_punct(',')?;
    if !cursor.eat_ident("virtuals") {
        return cursor.error("`virtuals`");
    }
    let virtual_trait = cursor.take_ident("the name of the trait of the virtual members")?;
    cursor.expect_punct(':')?;
    // The parent trait is a path; the members are in the braces after it.
    while !cursor.is_end() && !cursor.is_group(Delimiter::Brace) {
        cursor.next();
    }
    cursor.take_group(Delimiter::Brace, "the virtual members in braces")?;
    end(cursor)?;
    Ok(Declaration::Class { name, base, virtual_trait: Some(virtual_trait) })
}

/// `impl Owner [, also [paths]] { accessors }` or `impl Owner, fn part { accessors }`.
fn read_properties(cursor: &mut Cursor) -> Result<Declaration, ParseError> {
    if !cursor.eat_ident("impl") {
        return cursor.error("`impl`");
    }
    let owner = cursor.take_ident("the owner of the properties")?;
    if cursor.eat_punct(',') {
        if cursor.eat_ident("also") {
            cursor.take_group(Delimiter::Bracket, "the list of `also`")?;
        } else if cursor.eat_ident("fn") {
            cursor.take_ident("the name of the registration function")?;
        } else {
            return cursor.error("`also [..]` or `fn name`");
        }
    }
    let (body, line) = cursor.take_group(Delimiter::Brace, "the accessors in braces")?;
    end(cursor)?;
    let mut accessors = Vec::new();
    let mut body = Cursor::new(&body, line);
    while !body.is_end() {
        // An accessor is written directly or wrapped: `ferro_property!( .. );`.
        let wrapped = matches!(body.peek(), Some(TokenTree::Ident(_)))
            && body.peek_at(1).is_some_and(|token| is_punct(token, '!'))
            && body.peek_at(2).is_some_and(|token| group_of(token, Delimiter::Parenthesis).is_some());
        if wrapped {
            body.next();
            body.next();
            let (inner, line) = body.take_group(Delimiter::Parenthesis, "the accessor in parentheses")?;
            let mut inner = Cursor::new(&inner, line);
            accessors.push(read_accessor(&mut inner)?);
            end(&inner)?;
            body.expect_punct(';')?;
        } else {
            accessors.push(read_accessor(&mut body)?);
        }
    }
    Ok(Declaration::Properties { owner, accessors, function_of: None })
}

/// The tokens of `ferro_property!`: `[for Owner;] accessor`.
pub(crate) fn read_property(cursor: &mut Cursor) -> Result<(Option<String>, Accessor), ParseError> {
    let owner = if cursor.eat_ident("for") {
        let owner = cursor.take_ident("the owner of the property")?;
        cursor.expect_punct(';')?;
        Some(owner)
    } else {
        None
    };
    let accessor = read_accessor(cursor)?;
    end(cursor)?;
    Ok((owner, accessor))
}

/// `#[..]* vis fn name() -> Type { body }`.
fn read_accessor(cursor: &mut Cursor) -> Result<Accessor, ParseError> {
    cursor.skip_attributes();
    let line = cursor.line();
    let visibility = cursor.take_visibility();
    if !cursor.eat_ident("fn") {
        return cursor.error("`fn` (an accessor of a property: `vis fn name() -> Type { .. }`)");
    }
    let name = cursor.take_ident("the name of the accessor")?;
    let (parameters, _) = cursor.take_group(Delimiter::Parenthesis, "`()`")?;
    if !parameters.is_empty() {
        return Err(ParseError { line, message: format!("the accessor `{name}` has parameters") });
    }
    cursor.expect_arrow('-')?;
    let return_type = cursor.take_type(TypeEnd { brace: true, ..TypeEnd::default() }, "the type of the property definition")?.to_vec();
    let (body, _) = cursor.take_group(Delimiter::Brace, "the body of the accessor")?;
    Ok(Accessor { name, visibility, return_type, body, line })
}

/// `Name { new: callable, interfaces: [I, J => cast], markup: { .. } }`, the parts in any order.
fn read_class_info(cursor: &mut Cursor) -> Result<Declaration, ParseError> {
    let name = cursor.take_ident("the name of the class")?;
    let (parts, line) = cursor.take_group(Delimiter::Brace, "the parts in braces")?;
    end(cursor)?;
    let mut parts = Cursor::new(&parts, line);
    let (mut new, mut interfaces, mut markup) = (None, Vec::new(), None);
    while !parts.is_end() {
        let part = parts.take_ident("`new`, `interfaces` or `markup`")?;
        parts.expect_punct(':')?;
        match part.as_str() {
            "new" => new = Some(parts.take_expression("the constructor")?.to_vec()),
            "interfaces" => {
                let (list, line) = parts.take_group(Delimiter::Bracket, "the interfaces in brackets")?;
                let mut list = Cursor::new(&list, line);
                while !list.is_end() {
                    interfaces.push(list.take_type(TypeEnd::default(), "an interface handle")?.to_vec());
                    if list.eat_arrow('=') {
                        list.take_expression("the cast of the interface")?;
                    }
                    separator(&mut list)?;
                }
            }
            "markup" => {
                let (body, line) = parts.take_group(Delimiter::Brace, "the markup metadata in braces")?;
                markup = Some(read_markup_body(&body, line)?);
            }
            _ => {
                parts.rewind(parts.position().saturating_sub(2));
                return parts.error("`new`, `interfaces` or `markup`");
            }
        }
        separator(&mut parts)?;
    }
    Ok(Declaration::ClassInfo { name, new, interfaces, markup })
}

/// A comma, unless the list ends.
fn separator(cursor: &mut Cursor) -> Result<(), ParseError> {
    if cursor.is_end() {
        Ok(())
    } else {
        cursor.expect_punct(',')
    }
}

/// `kind Type { .. }`, `kind Type as "Name" { .. }`, `kind dyn Trait as "Name" { .. }`.
fn read_markup_type(cursor: &mut Cursor) -> Result<Declaration, ParseError> {
    let kind = match cursor.take_ident("`class`, `struct`, `interface` or `static`")?.as_str() {
        "class" => TypeKind::Class,
        "struct" => TypeKind::Struct,
        "interface" => TypeKind::Interface,
        "static" => TypeKind::Static,
        _ => {
            cursor.rewind(0);
            return cursor.error("`class`, `struct`, `interface` or `static`");
        }
    };
    let is_dyn = cursor.eat_ident("dyn");
    let type_ = cursor.take_type(TypeEnd { brace: true, as_: true, ..TypeEnd::default() }, "the type")?.to_vec();
    let name = if cursor.eat_ident("as") { Some(take_text(cursor, "the markup name (a string)")?) } else { None };
    if is_dyn && name.is_none() {
        return cursor.error("`as \"Name\"` (a contract `dyn Trait` states its markup name)");
    }
    let (body, line) = cursor.take_group(Delimiter::Brace, "the body in braces")?;
    end(cursor)?;
    Ok(Declaration::MarkupType { kind, type_, is_dyn, name, body: read_markup_body(&body, line)? })
}

/// `Name { A, B = Variant }` or `flags Name { A = Name::A }`, then `, { parts }`.
fn read_markup_enum(cursor: &mut Cursor) -> Result<Declaration, ParseError> {
    // `flags` is the form only when a name follows: an enumeration may be named `flags`.
    let flags = cursor.is_ident("flags") && matches!(cursor.peek_at(1), Some(TokenTree::Ident(_)));
    if flags {
        cursor.next();
    }
    let name = cursor.take_ident("the name of the enumeration")?;
    let (list, line) = cursor.take_group(Delimiter::Brace, "the members in braces")?;
    let mut list = Cursor::new(&list, line);
    let mut members = Vec::new();
    while !list.is_end() {
        let member = list.take_ident("the name of a member")?;
        let (mut variant, mut value) = (None, None);
        if flags {
            list.expect_punct('=')?;
            value = Some(list.take_expression("the value of the member")?.to_vec());
        } else if list.eat_punct('=') {
            variant = Some(list.take_ident("the Rust variant of the member")?);
        }
        members.push(EnumMember { name: member, variant, value });
        separator(&mut list)?;
    }
    let mut body = MarkupBody::default();
    if cursor.eat_punct(',') {
        let (parts, line) = cursor.take_group(Delimiter::Brace, "the parts in braces")?;
        body = read_markup_body(&parts, line)?;
    }
    end(cursor)?;
    Ok(Declaration::MarkupEnum { name, flags, members, body })
}

/// The text of the string literal at the cursor.
fn take_text(cursor: &mut Cursor, what: &str) -> Result<String, ParseError> {
    if let Some(TokenTree::Literal(literal)) = cursor.peek() {
        if let syn::Lit::Str(text) = syn::Lit::new(literal.clone()) {
            cursor.next();
            return Ok(text.value());
        }
    }
    cursor.error(what)
}

/// The tokens of the group in brackets at the cursor, and its line.
fn take_list(cursor: &mut Cursor, what: &str) -> Result<(Tokens, usize), ParseError> {
    cursor.take_group(Delimiter::Bracket, what)
}

/// The parts of markup metadata: `name: value, ..` in any order.
pub(crate) fn read_markup_body(tokens: &[TokenTree], line: usize) -> Result<MarkupBody, ParseError> {
    let mut body = MarkupBody::default();
    let mut cursor = Cursor::new(tokens, line);
    while !cursor.is_end() {
        let part_line = cursor.line();
        let part = cursor.take_ident("the name of a part")?;
        cursor.expect_punct(':')?;
        match part.as_str() {
            "namespace" => body.namespace = Some(take_text(&mut cursor, "the namespace (a string)")?),
            "handles" => {
                let (list, line) = take_list(&mut cursor, "the handles in brackets")?;
                body.handles = split_types(&list, line)?;
            }
            "interfaces" => {
                let (list, line) = take_list(&mut cursor, "the interfaces in brackets")?;
                body.interfaces = split_types(&list, line)?;
            }
            "this" => body.this = Some(cursor.take_type(TypeEnd::default(), "the instance type")?.to_vec()),
            "base" => body.base = Some(cursor.take_type(TypeEnd::default(), "the base type")?.to_vec()),
            "notify_property_changed" => {
                body.notify_property_changed = Some(cursor.take_type(TypeEnd::default(), "the notifying type")?.to_vec());
            }
            "type_info" => body.type_info = Some(cursor.take_type(TypeEnd::default(), "the runtime type")?.to_vec()),
            "generic" => {
                let definition = take_text(&mut cursor, "the generic definition (a string)")?;
                let (list, line) = take_list(&mut cursor, "the type arguments in brackets")?;
                body.generic = Some((definition, split_types(&list, line)?));
            }
            "content" => body.content = Some(cursor.take_ident("the name of the content property")?),
            "parse" => body.parse = Some(cursor.take_expression("the parse function")?.to_vec()),
            "attributes" => {
                let (list, line) = take_list(&mut cursor, "the attributes in brackets")?;
                body.attributes = read_attributes(&list, line)?;
            }
            "constructors" => {
                let (list, line) = take_list(&mut cursor, "the constructors in brackets")?;
                body.constructors = read_constructors(&list, line)?;
            }
            "properties" => {
                let (list, line) = take_list(&mut cursor, "the properties in brackets")?;
                body.properties = read_properties_list(&list, line, false)?;
            }
            "static_properties" => {
                let (list, line) = take_list(&mut cursor, "the static properties in brackets")?;
                body.static_properties = read_properties_list(&list, line, false)?;
            }
            "indexers" => {
                let (list, line) = take_list(&mut cursor, "the indexers in brackets")?;
                body.indexers = read_properties_list(&list, line, true)?;
            }
            "property_attributes" => {
                let (list, line) = take_list(&mut cursor, "the attributes of properties in brackets")?;
                let mut list = Cursor::new(&list, line);
                while !list.is_end() {
                    let name = list.take_ident("the name of a registered property")?;
                    list.expect_punct(':')?;
                    let (attributes, line) = take_list(&mut list, "the attributes in brackets")?;
                    body.property_attributes.push((name, read_attributes(&attributes, line)?));
                    separator(&mut list)?;
                }
            }
            "methods" => {
                let (list, line) = take_list(&mut cursor, "the methods in brackets")?;
                body.methods = read_methods(&list, line)?;
            }
            "fields" => {
                let (list, line) = take_list(&mut cursor, "the fields in brackets")?;
                let mut list = Cursor::new(&list, line);
                while !list.is_end() {
                    let name = list.take_ident("the name of a field")?;
                    list.expect_punct(':')?;
                    let type_ = list.take_type(TypeEnd::default(), "the type of the field")?.to_vec();
                    list.expect_arrow('=')?;
                    let callable = list.take_expression("the callable of the field")?.to_vec();
                    body.fields.push(RawMember { name, return_type: Some(type_), is_static: true, callable, ..RawMember::default() });
                    separator(&mut list)?;
                }
            }
            "events" => {
                let (list, line) = take_list(&mut cursor, "the events in brackets")?;
                let mut list = Cursor::new(&list, line);
                while !list.is_end() {
                    let fallible = is_try(&mut list);
                    let name = list.take_ident("the name of an event")?;
                    let (arguments, line) = list.take_group(Delimiter::Parenthesis, "the arguments of the handler in parentheses")?;
                    let parameters = split_types(&arguments, line)?.into_iter().map(positional).collect();
                    list.expect_arrow('=')?;
                    let callable = list.take_expression("the callable that subscribes")?.to_vec();
                    body.events.push(RawMember { name, parameters, fallible, callable, ..RawMember::default() });
                    separator(&mut list)?;
                }
            }
            other => {
                return Err(ParseError { line: part_line, message: format!("`{other}` is not a part of markup metadata") });
            }
        }
        separator(&mut cursor)?;
    }
    Ok(body)
}

fn positional(type_: Tokens) -> RawParameter {
    RawParameter { name: None, type_, attributes: Vec::new() }
}

/// `try` before a member: the member is declared fallible.
fn is_try(cursor: &mut Cursor) -> bool {
    cursor.eat_ident("try")
}

/// `(A, B) => callable`, `try (A) => callable`, `(name: A [Attribute], other: B) => callable`.
fn read_constructors(tokens: &[TokenTree], line: usize) -> Result<Vec<RawMember>, ParseError> {
    let mut cursor = Cursor::new(tokens, line);
    let mut constructors = Vec::new();
    while !cursor.is_end() {
        let fallible = is_try(&mut cursor);
        let (list, line) = cursor.take_group(Delimiter::Parenthesis, "the parameters of a constructor in parentheses")?;
        let named = matches!(list.first(), Some(TokenTree::Ident(_))) && list.get(1).is_some_and(|token| is_punct(token, ':')) && !is_path_separator(&list, 1);
        let parameters = if named {
            let mut list = Cursor::new(&list, line);
            let mut parameters = Vec::new();
            while !list.is_end() {
                let name = list.take_ident("the name of a parameter")?;
                list.expect_punct(':')?;
                let type_ = list.take_type(TypeEnd { bracket: true, ..TypeEnd::default() }, "the type of the parameter")?.to_vec();
                let attributes = if list.is_group(Delimiter::Bracket) {
                    let (attributes, line) = take_list(&mut list, "the attributes in brackets")?;
                    read_attributes(&attributes, line)?
                } else {
                    Vec::new()
                };
                parameters.push(RawParameter { name: Some(name), type_, attributes });
                separator(&mut list)?;
            }
            parameters
        } else {
            split_types(&list, line)?.into_iter().map(positional).collect()
        };
        cursor.expect_arrow('=')?;
        let callable = cursor.take_expression("the callable of the constructor")?.to_vec();
        constructors.push(RawMember { parameters, is_static: true, fallible, callable, ..RawMember::default() });
        separator(&mut cursor)?;
    }
    Ok(constructors)
}

/// `Name: T { get: c, set: c } [Attribute]`, or, for an indexer, `(A) -> T { .. } [Attribute]`.
fn read_properties_list(tokens: &[TokenTree], line: usize, indexers: bool) -> Result<Vec<RawProperty>, ParseError> {
    let mut cursor = Cursor::new(tokens, line);
    let mut properties = Vec::new();
    while !cursor.is_end() {
        let (name, parameters) = if indexers {
            let (list, line) = cursor.take_group(Delimiter::Parenthesis, "the parameters of an indexer in parentheses")?;
            cursor.expect_arrow('-')?;
            (String::new(), split_types(&list, line)?.into_iter().map(positional).collect())
        } else {
            let name = cursor.take_ident("the name of a property")?;
            cursor.expect_punct(':')?;
            (name, Vec::new())
        };
        let type_ = cursor.take_type(TypeEnd { brace: true, ..TypeEnd::default() }, "the type of the property")?.to_vec();
        let (accessors, line) = cursor.take_group(Delimiter::Brace, "the accessors in braces")?;
        let mut accessors = Cursor::new(&accessors, line);
        let (mut getter, mut setter) = (None, None);
        while !accessors.is_end() {
            let position = accessors.position();
            let accessor = accessors.take_ident("`get`, `set`, `try_get` or `try_set`")?;
            accessors.expect_punct(':')?;
            let callable = accessors.take_expression("the callable of the accessor")?.to_vec();
            match accessor.as_str() {
                "get" => getter = Some(RawAccessor { fallible: false, callable }),
                "try_get" => getter = Some(RawAccessor { fallible: true, callable }),
                "set" => setter = Some(RawAccessor { fallible: false, callable }),
                "try_set" => setter = Some(RawAccessor { fallible: true, callable }),
                _ => {
                    accessors.rewind(position);
                    return accessors.error("`get`, `set`, `try_get` or `try_set`");
                }
            }
            separator(&mut accessors)?;
        }
        let attributes = if cursor.is_group(Delimiter::Bracket) {
            let (attributes, line) = take_list(&mut cursor, "the attributes in brackets")?;
            read_attributes(&attributes, line)?
        } else {
            Vec::new()
        };
        properties.push(RawProperty { name, parameters, type_, getter, setter, attributes });
        separator(&mut cursor)?;
    }
    Ok(properties)
}

/// `[static] [try] fn Name(A, B) [-> R] => callable`, where the callable is a path or an
/// expression in parentheses followed by `[attributes]`, or any expression.
fn read_methods(tokens: &[TokenTree], line: usize) -> Result<Vec<RawMember>, ParseError> {
    let mut cursor = Cursor::new(tokens, line);
    let mut methods = Vec::new();
    while !cursor.is_end() {
        let is_static = cursor.eat_ident("static");
        let fallible = is_try(&mut cursor);
        if !cursor.eat_ident("fn") {
            return cursor.error("`fn` (a method: `[static] [try] fn Name(A) -> R => callable`)");
        }
        let name = cursor.take_ident("the name of a method")?;
        let (list, line) = cursor.take_group(Delimiter::Parenthesis, "the parameters of the method in parentheses")?;
        let parameters = split_types(&list, line)?.into_iter().map(positional).collect();
        let return_type = if cursor.eat_arrow('-') { Some(cursor.take_type(TypeEnd::default(), "the return type")?.to_vec()) } else { None };
        cursor.expect_arrow('=')?;
        let start = cursor.position();
        let mut attributes = Vec::new();
        let path = cursor.take_plain_path().map(<[TokenTree]>::to_vec);
        let callable = if path.is_some() && cursor.is_group(Delimiter::Bracket) {
            let (list, line) = take_list(&mut cursor, "the attributes in brackets")?;
            attributes = read_attributes(&list, line)?;
            path.unwrap_or_default()
        } else {
            cursor.rewind(start);
            let parenthesised = cursor.is_group(Delimiter::Parenthesis) && cursor.peek_at(1).is_some_and(|token| group_of(token, Delimiter::Bracket).is_some());
            if parenthesised {
                let (expression, _) = cursor.take_group(Delimiter::Parenthesis, "the callable in parentheses")?;
                let (list, line) = take_list(&mut cursor, "the attributes in brackets")?;
                attributes = read_attributes(&list, line)?;
                expression
            } else {
                cursor.take_expression("the callable of the method")?.to_vec()
            }
        };
        methods.push(RawMember { name, parameters, return_type, is_static, fallible, attributes, callable });
        separator(&mut cursor)?;
    }
    Ok(methods)
}

/// `Name, Name("text", 1, type(T), Key = value, ["a", "b"])`.
pub(crate) fn read_attributes(tokens: &[TokenTree], line: usize) -> Result<Vec<RawAttribute>, ParseError> {
    let mut cursor = Cursor::new(tokens, line);
    let mut attributes = Vec::new();
    while !cursor.is_end() {
        let name = cursor.take_ident("the name of an attribute")?;
        let mut attribute = RawAttribute { name, arguments: Vec::new(), properties: Vec::new() };
        if cursor.is_group(Delimiter::Parenthesis) {
            let (list, line) = cursor.take_group(Delimiter::Parenthesis, "the arguments in parentheses")?;
            let mut list = Cursor::new(&list, line);
            while !list.is_end() {
                // `Key = value`; `type(T)` is a value, not a key.
                let named = matches!(list.peek(), Some(TokenTree::Ident(_))) && list.peek_at(1).is_some_and(|token| is_punct(token, '='));
                if named {
                    let key = list.take_ident("the name of an argument")?;
                    list.expect_punct('=')?;
                    attribute.properties.push((key, read_value(&mut list)?));
                } else {
                    attribute.arguments.push(read_value(&mut list)?);
                }
                separator(&mut list)?;
            }
        }
        attributes.push(attribute);
        separator(&mut cursor)?;
    }
    Ok(attributes)
}

/// A literal, `null`, `type(T)` or an array of those.
fn read_value(cursor: &mut Cursor) -> Result<RawValue, ParseError> {
    const EXPECTED: &str = "a literal, `null`, `type(T)` or an array of those";
    if cursor.is_group(Delimiter::Bracket) {
        let (list, line) = take_list(cursor, EXPECTED)?;
        let mut list = Cursor::new(&list, line);
        let mut items = Vec::new();
        while !list.is_end() {
            items.push(read_value(&mut list)?);
            separator(&mut list)?;
        }
        return Ok(RawValue::Array(items));
    }
    if cursor.eat_ident("null") {
        return Ok(RawValue::Null);
    }
    if cursor.eat_ident("true") {
        return Ok(RawValue::Bool(true));
    }
    if cursor.eat_ident("false") {
        return Ok(RawValue::Bool(false));
    }
    if cursor.is_ident("type") && cursor.peek_at(1).is_some_and(|token| group_of(token, Delimiter::Parenthesis).is_some()) {
        cursor.next();
        let (type_, _) = cursor.take_group(Delimiter::Parenthesis, EXPECTED)?;
        return Ok(RawValue::Type(type_));
    }
    let start = cursor.position();
    let negative = cursor.eat_punct('-');
    if let Some(TokenTree::Literal(literal)) = cursor.peek() {
        let value = match syn::Lit::new(literal.clone()) {
            syn::Lit::Str(text) if !negative => Some(RawValue::Str(text.value())),
            syn::Lit::Char(character) if !negative => Some(RawValue::Int(character.value() as i64)),
            syn::Lit::Bool(value) if !negative => Some(RawValue::Bool(value.value())),
            syn::Lit::Int(integer) => integer.base10_parse::<i64>().ok().map(|value| RawValue::Int(if negative { -value } else { value })),
            syn::Lit::Float(float) => Some(RawValue::Float(format!("{}{}", if negative { "-" } else { "" }, float.base10_digits()))),
            _ => None,
        };
        if let Some(value) = value {
            cursor.next();
            return Ok(value);
        }
    }
    cursor.rewind(start);
    cursor.error(EXPECTED)
}

/// What the body of the accessor of a registered property does, as far as its tokens say.
pub(crate) struct Registration {
    pub kind: RegistrationModel,
    /// The text of the first argument of `FerroProperty::register*`.
    pub name: Option<String>,
    /// The type arguments of `FerroProperty::register*::<..>`.
    pub type_arguments: Vec<Tokens>,
    /// The accessor the body calls (`Decorator::child_property`), for an added owner and
    /// an alias.
    pub source: Option<Tokens>,
    /// The type argument of `.add_owner*::<Owner>`.
    pub added_owner: Option<Tokens>,
    pub assign_binding: bool,
    pub inherits: bool,
    /// The registration of a direct property states `None` where the setter goes.
    pub read_only: bool,
}

/// Reads the body of an accessor: the registration is found wherever the body states it
/// (`let property = FerroProperty::register..; ..; property`).
pub(crate) fn read_registration(body: &[TokenTree]) -> Registration {
    let mut registration = Registration {
        kind: RegistrationModel::Unknown,
        name: None,
        type_arguments: Vec::new(),
        source: None,
        added_owner: None,
        assign_binding: calls_with_true(body, &["assign_binding", "set_assign_binding"]),
        inherits: calls_with_true(body, &["inherits"]),
        read_only: states_no_setter(body),
    };
    if let Some((name, type_arguments)) = find_register(body) {
        registration.kind = RegistrationModel::Declared;
        registration.name = name;
        registration.type_arguments = type_arguments;
    } else if let Some((source, added_owner)) = find_add_owner(body) {
        registration.kind = RegistrationModel::AddedOwner;
        registration.source = Some(source);
        registration.added_owner = added_owner;
    } else if let Some(source) = alias_of(body) {
        registration.kind = RegistrationModel::Alias;
        registration.source = Some(source);
    }
    registration
}

/// Whether `tokens`, at any depth, register a direct property without a setter:
/// `register_direct*::<..>("Name", getter, None, ..)`, or `.add_owner::<..>(getter, None, ..)`
/// (the owner a direct property is added to states its own accessors; the owner of a
/// styled property states none, so its call has no second argument).
fn states_no_setter(tokens: &[TokenTree]) -> bool {
    for index in 0..tokens.len() {
        if let TokenTree::Group(group) = &tokens[index] {
            if states_no_setter(&tokens_of(group.stream())) {
                return true;
            }
            continue;
        }
        let Some(name) = ident_of(&tokens[index]) else { continue };
        let position = if name.starts_with("register_direct") {
            2
        } else if name == "add_owner" {
            1
        } else {
            continue;
        };
        // `::<..>` may stand between the name and the arguments.
        let mut after = index + 1;
        if is_path_separator(tokens, after) {
            match angle_group(tokens, after + 2) {
                Some((_, next)) => after = next,
                None => continue,
            }
        }
        let Some(call) = tokens.get(after).and_then(|token| group_of(token, Delimiter::Parenthesis)) else { continue };
        let arguments = tokens_of(call.stream());
        let setter = arguments.split(|token| is_punct(token, ',')).nth(position);
        if setter.is_some_and(|setter| setter.len() == 1 && ident_of(&setter[0]).as_deref() == Some("None")) {
            return true;
        }
    }
    false
}

/// Whether `tokens`, at any depth, call `.name(true)` for one of `names`.
fn calls_with_true(tokens: &[TokenTree], names: &[&str]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| match token {
        TokenTree::Group(group) => calls_with_true(&tokens_of(group.stream()), names),
        TokenTree::Ident(ident) => {
            names.contains(&ident.to_string().as_str())
                && index > 0
                && is_punct(&tokens[index - 1], '.')
                && tokens.get(index + 1).and_then(|next| group_of(next, Delimiter::Parenthesis)).is_some_and(|arguments| {
                    let arguments = tokens_of(arguments.stream());
                    arguments.len() == 1 && ident_of(&arguments[0]).as_deref() == Some("true")
                })
        }
        _ => false,
    })
}

/// The tokens between `<` at `tokens[index]` and the `>` that closes it, and the index
/// after that `>`.
fn angle_group(tokens: &[TokenTree], index: usize) -> Option<(&[TokenTree], usize)> {
    if !tokens.get(index).is_some_and(|token| is_punct(token, '<')) {
        return None;
    }
    let mut depth = 0usize;
    let mut at = index;
    while at < tokens.len() {
        if is_arrow(tokens, at, '-') {
            at += 2;
            continue;
        }
        if is_punct(&tokens[at], '<') {
            depth += 1;
        } else if is_punct(&tokens[at], '>') {
            depth -= 1;
            if depth == 0 {
                return Some((&tokens[index + 1..at], at + 1));
            }
        }
        at += 1;
    }
    None
}

/// `FerroProperty::register*::<A, B>("Name", ..)`, at any depth: the name (when the first
/// argument is a string literal) and the type arguments.
fn find_register(tokens: &[TokenTree]) -> Option<(Option<String>, Vec<Tokens>)> {
    for index in 0..tokens.len() {
        if let TokenTree::Group(group) = &tokens[index] {
            if let Some(found) = find_register(&tokens_of(group.stream())) {
                return Some(found);
            }
            continue;
        }
        let is_register = ident_of(&tokens[index]).as_deref() == Some("FerroProperty")
            && is_path_separator(tokens, index + 1)
            && tokens.get(index + 3).and_then(ident_of).is_some_and(|function| function.starts_with("register"))
            && is_path_separator(tokens, index + 4);
        if !is_register {
            continue;
        }
        let Some((arguments, after)) = angle_group(tokens, index + 6) else { continue };
        let Some(call) = tokens.get(after).and_then(|token| group_of(token, Delimiter::Parenthesis)) else { continue };
        let name = match tokens_of(call.stream()).first() {
            Some(TokenTree::Literal(literal)) => match syn::Lit::new(literal.clone()) {
                syn::Lit::Str(text) => Some(text.value()),
                _ => None,
            },
            _ => None,
        };
        return Some((name, split_types(arguments, 0).unwrap_or_default()));
    }
    None
}

/// `Type::accessor().add_owner*::<Owner>(..)`, at any depth: the path of the accessor and
/// the owner.
fn find_add_owner(tokens: &[TokenTree]) -> Option<(Tokens, Option<Tokens>)> {
    for index in 0..tokens.len() {
        if let TokenTree::Group(group) = &tokens[index] {
            if let Some(found) = find_add_owner(&tokens_of(group.stream())) {
                return Some(found);
            }
            continue;
        }
        let is_method = index > 0
            && is_punct(&tokens[index - 1], '.')
            && ident_of(&tokens[index]).is_some_and(|method| method == "add_owner" || method == "add_owner_with");
        if !is_method {
            continue;
        }
        // Before the `.`: the call of the accessor, `path ()`.
        if index < 3 || group_of(&tokens[index - 2], Delimiter::Parenthesis).is_none() {
            continue;
        }
        let end = index - 2;
        let mut start = end;
        while start > 0 && matches!(tokens[start - 1], TokenTree::Ident(_)) {
            start -= 1;
            if start >= 2 && is_path_separator(tokens, start - 2) {
                start -= 2;
            } else {
                break;
            }
        }
        if start == end {
            continue;
        }
        let owner = if is_path_separator(tokens, index + 1) { angle_group(tokens, index + 3).map(|(owner, _)| owner.to_vec()) } else { None };
        return Some((tokens[start..end].to_vec(), owner));
    }
    None
}

/// The path of the accessor, when the body is the call of an accessor and nothing else
/// (`Self::placeholder_text_property()`).
fn alias_of(body: &[TokenTree]) -> Option<Tokens> {
    let (call, path) = body.split_last()?;
    let call = group_of(call, Delimiter::Parenthesis)?;
    if !call.stream().is_empty() || path.is_empty() {
        return None;
    }
    let mut cursor = Cursor::new(path, 0);
    let taken = cursor.take_plain_path()?;
    (cursor.is_end() && taken.len() > 1).then(|| path.to_vec())
}

/// The kind and the value type of a registered property, from the type of its accessor
/// (`StyledProperty<T>`, `AttachedProperty<T>`, `DirectProperty<Owner, T>`); nothing for
/// any other type.
pub(crate) fn property_type(return_type: &[TokenTree]) -> Option<(crate::model::RegisteredKind, Tokens)> {
    use crate::model::RegisteredKind;
    let (head, mut arguments) = generic_type(return_type)?;
    let (kind, count) = match head.last()?.as_str() {
        "StyledProperty" => (RegisteredKind::Styled, 1),
        "AttachedProperty" => (RegisteredKind::Attached, 1),
        "DirectProperty" => (RegisteredKind::Direct, 2),
        _ => return None,
    };
    if arguments.len() != count {
        return None;
    }
    Some((kind, arguments.pop()?))
}

/// The line of the first token, or `line` when there is none.
pub(crate) fn first_line(tokens: &[TokenTree], line: usize) -> usize {
    tokens.first().map_or(line, line_of)
}
