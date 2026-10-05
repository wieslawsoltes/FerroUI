//! Hand-written recursive-descent parser for MicroCom IDL.

use std::fmt;

use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(source: &str) -> Result<Idl, ParseError> {
    let mut p = Parser { src: source, pos: 0 };
    p.parse_file()
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn error<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        let before = &self.src[..self.pos.min(self.src.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.rfind('\n').map_or(self.pos, |i| self.pos - i - 1) + 1;
        Err(ParseError { line, column, message: message.into() })
    }

    /// Skips whitespace, `// ...` and `/* ... */`.
    fn skip_trivia(&mut self) -> Result<(), ParseError> {
        loop {
            let rest = self.rest();
            if let Some(c) = rest.chars().next() {
                if c.is_whitespace() {
                    self.pos += c.len_utf8();
                    continue;
                }
            }
            if rest.starts_with("//") {
                self.pos += rest.find('\n').unwrap_or(rest.len());
                continue;
            }
            if rest.starts_with("/*") {
                match rest.find("*/") {
                    Some(end) => self.pos += end + 2,
                    None => return self.error("unterminated comment"),
                }
                continue;
            }
            return Ok(());
        }
    }

    fn eat(&mut self, c: char) -> Result<bool, ParseError> {
        self.skip_trivia()?;
        if self.peek() == Some(c) {
            self.pos += c.len_utf8();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, c: char) -> Result<(), ParseError> {
        if self.eat(c)? {
            Ok(())
        } else {
            let found = self.peek().map_or("end of file".to_string(), |c| format!("'{c}'"));
            self.error(format!("expected '{c}', found {found}"))
        }
    }

    fn try_ident(&mut self) -> Result<Option<String>, ParseError> {
        self.skip_trivia()?;
        let rest = self.rest();
        let len = rest.find(|c| !is_ident_char(c)).unwrap_or(rest.len());
        if len == 0 {
            return Ok(None);
        }
        self.pos += len;
        Ok(Some(rest[..len].to_string()))
    }

    fn ident(&mut self, what: &str) -> Result<String, ParseError> {
        match self.try_ident()? {
            Some(i) => Ok(i),
            None => self.error(format!("expected {what}")),
        }
    }

    fn parse_file(&mut self) -> Result<Idl, ParseError> {
        let mut idl = Idl::default();
        loop {
            self.skip_trivia()?;
            if self.eof() {
                return Ok(idl);
            }
            if self.peek() == Some('@') {
                idl.directives.push(self.parse_directive()?);
                continue;
            }
            let attributes = self.parse_attributes()?;
            let keyword = self.ident("'enum', 'struct' or 'interface'")?;
            match keyword.as_str() {
                "enum" => idl.enums.push(self.parse_enum(attributes)?),
                "struct" => idl.structs.push(self.parse_struct(attributes)?),
                "interface" => idl.interfaces.push(self.parse_interface(attributes)?),
                other => return self.error(format!("unexpected '{other}'")),
            }
        }
    }

    /// `@name value-to-end-of-line` or `@name @@ multi-line text @@`.
    fn parse_directive(&mut self) -> Result<Directive, ParseError> {
        self.pos += 1; // '@'
        let rest = self.rest();
        let name_len = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        let name = rest[..name_len].to_string();
        if name.is_empty() {
            return self.error("expected directive name after '@'");
        }
        self.pos += name_len;
        let line_end = self.rest().find('\n').unwrap_or(self.rest().len());
        let line = &self.rest()[..line_end];
        if let Some(start) = line.find("@@") {
            self.pos += start + 2;
            let Some(end) = self.rest().find("@@") else {
                return self.error("unterminated '@@' block");
            };
            let value = self.rest()[..end].trim().replace("\r\n", "\n");
            self.pos += end + 2;
            Ok(Directive { name, value })
        } else {
            let value = line.trim().to_string();
            self.pos += line_end;
            Ok(Directive { name, value })
        }
    }

    /// Zero or more `[a, b(value)]` groups.
    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, ParseError> {
        let mut out = Vec::new();
        while self.eat('[')? {
            let rest = self.rest();
            let Some(end) = rest.find(']') else {
                return self.error("unterminated attribute list");
            };
            let body = &rest[..end];
            self.pos += end + 1;
            // Split on commas that are not inside parentheses.
            let mut depth = 0;
            let mut start = 0;
            let mut parts = Vec::new();
            for (i, c) in body.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    ',' if depth == 0 => {
                        parts.push(&body[start..i]);
                        start = i + 1;
                    }
                    _ => {}
                }
            }
            parts.push(&body[start..]);
            for part in parts {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                match part.find('(') {
                    Some(open) if part.ends_with(')') => out.push(Attribute {
                        name: part[..open].trim().to_string(),
                        value: Some(part[open + 1..part.len() - 1].trim().to_string()),
                    }),
                    _ => out.push(Attribute { name: part.to_string(), value: None }),
                }
            }
        }
        Ok(out)
    }

    fn parse_enum(&mut self, attributes: Vec<Attribute>) -> Result<Enum, ParseError> {
        let name = self.ident("enum name")?;
        self.expect('{')?;
        let mut members = Vec::new();
        loop {
            if self.eat('}')? {
                break;
            }
            let member = self.ident("enum member name")?;
            let value = if self.eat('=')? {
                self.skip_trivia()?;
                let rest = self.rest();
                let Some(end) = rest.find([',', '}']) else {
                    return self.error("unterminated enum");
                };
                let text = rest[..end].trim();
                if text.is_empty() {
                    return self.error("expected enum member value");
                }
                self.pos += end;
                Some(text.to_string())
            } else {
                None
            };
            members.push(EnumMember { name: member, value });
            if !self.eat(',')? {
                self.expect('}')?;
                break;
            }
        }
        self.eat(';')?;
        Ok(Enum { attributes, name, members })
    }

    fn parse_struct(&mut self, attributes: Vec<Attribute>) -> Result<Struct, ParseError> {
        let name = self.ident("struct name")?;
        self.expect('{')?;
        let mut fields = Vec::new();
        while !self.eat('}')? {
            // `type name[, name...];`
            let (ty, first) = self.parse_type_and_name("field")?;
            fields.push(Field { ty: ty.clone(), name: first });
            while self.eat(',')? {
                let n = self.ident("field name")?;
                fields.push(Field { ty: ty.clone(), name: n });
            }
            self.expect(';')?;
        }
        self.eat(';')?;
        Ok(Struct { attributes, name, fields })
    }

    /// Parses `ident+ ('*')* ('&')? ident` where the final identifier is the
    /// declared name and everything before it is the type.
    fn parse_type_and_name(&mut self, what: &str) -> Result<(TypeRef, String), ParseError> {
        let mut words = vec![self.ident(&format!("{what} type"))?];
        let mut pointers = 0;
        let mut reference = false;
        loop {
            if self.eat('*')? {
                if reference {
                    return self.error("pointer to reference is not supported");
                }
                pointers += 1;
            } else if self.eat('&')? {
                reference = true;
            } else if let Some(word) = self.try_ident()? {
                if pointers > 0 || reference {
                    // First identifier after the pointer/reference markers is the name.
                    let ty = TypeRef { name: words.join(" "), pointers, reference };
                    return Ok((ty, word));
                }
                words.push(word);
            } else {
                break;
            }
        }
        if pointers > 0 || reference || words.len() < 2 {
            return self.error(format!("expected {what} name"));
        }
        let name = words.pop().unwrap();
        Ok((TypeRef { name: words.join(" "), pointers, reference }, name))
    }

    fn parse_interface(&mut self, attributes: Vec<Attribute>) -> Result<Interface, ParseError> {
        let name = self.ident("interface name")?;
        let base = if self.eat(':')? { Some(self.ident("base interface name")?) } else { None };
        self.expect('{')?;
        let mut methods = Vec::new();
        while !self.eat('}')? {
            let method_attrs = self.parse_attributes()?;
            let (return_type, method_name) = self.parse_type_and_name("method")?;
            self.expect('(')?;
            let mut params = Vec::new();
            if !self.eat(')')? {
                loop {
                    let param_attrs = self.parse_attributes()?;
                    let (ty, pname) = self.parse_type_and_name("parameter")?;
                    params.push(Param { attributes: param_attrs, ty, name: pname });
                    if self.eat(',')? {
                        continue;
                    }
                    self.expect(')')?;
                    break;
                }
            }
            self.expect(';')?;
            methods.push(Method { attributes: method_attrs, return_type, name: method_name, params });
        }
        self.eat(';')?;
        Ok(Interface { attributes, name, base, methods })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
@clr-namespace Foo.Bar
@clr-map bool int
@clr-map long IntPtr
@cpp-preamble @@
#pragma once
#include "com.h"
@@

enum E { A, B = 5, C = A | B, }
[class-enum]
enum F { X = -1, Y }

struct S { double X, Y; void* Data; byte B; }

// line comment
[uuid(809c652e-7396-11d2-9771-00a0c9b4d50c)]
interface IRoot : IUnknown
{
    /** doc
     * comment */
    HRESULT Get(IRoot**ppv);
    [intptr]void* Handle([intptr] void* a, [const] char* text, [const] S& s);
    bool Flag();
}

[uuid(e5aca675-02b7-4129-aa79-d6e417210bda), cpp-virtual-inherits]
interface IChild : IRoot { }

[uuid(722aad20-a87b-4ce5-b50f-f05cfa4cda39)]
interface INoBase { unsigned int Count(); }
"#;

    #[test]
    fn parses_sample() {
        let idl = parse(SAMPLE).unwrap();
        assert_eq!(idl.directive("clr-namespace"), Some("Foo.Bar"));
        assert_eq!(idl.directive_values("clr-map").collect::<Vec<_>>(), ["bool int", "long IntPtr"]);
        assert_eq!(idl.directive("cpp-preamble"), Some("#pragma once\n#include \"com.h\""));

        assert_eq!(idl.enums.len(), 2);
        assert_eq!(idl.enums[0].members[1].value.as_deref(), Some("5"));
        assert_eq!(idl.enums[0].members[2].value.as_deref(), Some("A | B"));
        assert!(idl.enums[1].is_class_enum());
        assert_eq!(idl.enums[1].members[0].value.as_deref(), Some("-1"));

        let s = &idl.structs[0];
        assert_eq!(s.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["X", "Y", "Data", "B"]);
        assert_eq!(s.fields[2].ty, TypeRef { name: "void".into(), pointers: 1, reference: false });

        let root = &idl.interfaces[0];
        assert_eq!(root.uuid(), Some("809c652e-7396-11d2-9771-00a0c9b4d50c"));
        assert_eq!(root.methods.len(), 3);
        assert_eq!(root.methods[0].params[0].ty.pointers, 2);
        assert_eq!(root.methods[0].params[0].name, "ppv");
        let handle = &root.methods[1];
        assert!(has_attr(&handle.attributes, "intptr"));
        assert_eq!(handle.return_type.pointers, 1);
        assert!(handle.params[1].is_const());
        assert!(handle.params[2].ty.reference);

        assert!(idl.interfaces[1].cpp_virtual_inherits());
        assert_eq!(idl.interfaces[1].base.as_deref(), Some("IRoot"));
        assert_eq!(idl.interfaces[2].base, None);
        assert_eq!(idl.interfaces[2].methods[0].return_type.name, "unsigned int");
    }

    #[test]
    fn reports_position() {
        let err = parse("enum E {\n  A = ,\n}").unwrap_err();
        assert_eq!(err.line, 2);
    }
}
