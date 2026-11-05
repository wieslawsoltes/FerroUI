//! The path grammar of `{x:Bind}` (docs/porting/x-bind.md, step X2): WinUI's
//! `BindingPath.g4` (`src/XamlCompiler/BuildTasks/BindingPath/BindingPath.g4` of
//! microsoft-ui-xaml at the commit the NOTICE.md of `ferroui-markup-xaml` pins), parsed into a
//! syntax tree that `x_bind::transform_path` resolves against the root source.
//!
//! Not a port of an upstream file: WinUI generates its parser with ANTLR; this is a recursive
//! descent parser of the same grammar.
//!
//! ```text
//! path     : IDENTIFIER                      a member of the root source, or a named element
//!          | path '.' IDENTIFIER             a member
//!          | static_type '.' IDENTIFIER      a static member (ns:Type.Member)
//!          | path '[' Digits ']'             an integer indexer
//!          | path '[' QuotedString ']'       a string indexer
//!          | path '.' attached_expr          path.(ns:Owner.Property)
//!          | cast_expr path?                 (ns:Type)path: casts the leaf of the path
//!          | '(' cast_expr path ')'          ((ns:Type)path): a cast inside a longer path
//!          | function                        Format(a, b)
//!          | path '.' function               path.Format(a, b)
//!          | static_type '.' function        ns:Type.Format(a, b)
//! argument : path | x:True | x:False | x:Null | number | 'text' | "text"
//! ```
//!
//! WinUI's grammar rejects an attached property at the start of a path
//! (`BindAssignment_CastCannotStartWithAttachedProperty`); FerroUI XAML reads it as an attached
//! property of the root source (`{x:Bind (Grid.Column)}`, a superset that step X1 introduced).
//! A function is only allowed at the end of a path, and not as an argument
//! (`FunctionNotLeaf`, `FunctionAsParameter` of WinUI's parser).

use std::fmt;

/// A parsed `{x:Bind}` path.
#[derive(Clone, Debug, PartialEq)]
pub enum XBindPath {
    /// `name`: a member of the root source, or a named element.
    Identifier(String),
    /// `path.name`.
    Member(Box<XBindPath>, String),
    /// `ns:Type.Member`: a static property.
    Static { type_name: String, member: String },
    /// `path[0]`.
    Index(Box<XBindPath>, i32),
    /// `path['key']`.
    Key(Box<XBindPath>, String),
    /// `path.(ns:Owner.Property)`: an attached property, or a member of the type the value is
    /// cast to (an interface member); without a path, an attached property of the root source.
    Attached { path: Option<Box<XBindPath>>, type_name: String, property: String },
    /// `(ns:Type)path` and `((ns:Type)path)`; without a path, the cast of the root source.
    Cast { type_name: String, path: Option<Box<XBindPath>> },
    /// `Name(arguments)` on the root source, on the value of a path, or on a type (static).
    Function { owner: XBindFunctionOwner, name: String, arguments: Vec<XBindArgument> },
}

/// What a function is called on.
#[derive(Clone, Debug, PartialEq)]
pub enum XBindFunctionOwner {
    /// The root source: `Format(Name)`.
    Root,
    /// The value of a path: `Model.Format(Name)`.
    Path(Box<XBindPath>),
    /// A type, for a static function: `local:Helpers.Format(Name)`.
    Static(String),
}

/// An argument of a function.
#[derive(Clone, Debug, PartialEq)]
pub enum XBindArgument {
    /// A path read from the root source.
    Path(XBindPath),
    /// `x:True` / `x:False`.
    Bool(bool),
    /// A decimal number as written: `-?Digits('.'Digits)?`.
    Number(String),
    /// A quoted text, unescaped (`^'` and `^"` are the quotes).
    String(String),
    /// `x:Null`.
    Null,
}

impl XBindPath {
    /// Whether the path ends with a function.
    pub fn is_function(&self) -> bool {
        matches!(self, XBindPath::Function { .. })
    }
}

impl fmt::Display for XBindPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XBindPath::Identifier(name) => f.write_str(name),
            XBindPath::Member(path, name) => write!(f, "{path}.{name}"),
            XBindPath::Static { type_name, member } => write!(f, "{type_name}.{member}"),
            XBindPath::Index(path, index) => write!(f, "{path}[{index}]"),
            XBindPath::Key(path, key) => write!(f, "{path}['{key}']"),
            XBindPath::Attached { path: Some(path), type_name, property } => write!(f, "{path}.({type_name}.{property})"),
            XBindPath::Attached { path: None, type_name, property } => write!(f, "({type_name}.{property})"),
            XBindPath::Cast { type_name, path: Some(path) } => write!(f, "(({type_name}){path})"),
            XBindPath::Cast { type_name, path: None } => write!(f, "({type_name})"),
            XBindPath::Function { owner, name, arguments } => {
                match owner {
                    XBindFunctionOwner::Root => {}
                    XBindFunctionOwner::Path(path) => write!(f, "{path}.")?,
                    XBindFunctionOwner::Static(type_name) => write!(f, "{type_name}.")?,
                }
                let arguments: Vec<String> = arguments.iter().map(ToString::to_string).collect();
                write!(f, "{name}({})", arguments.join(", "))
            }
        }
    }
}

impl fmt::Display for XBindArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XBindArgument::Path(path) => write!(f, "{path}"),
            XBindArgument::Bool(true) => f.write_str("x:True"),
            XBindArgument::Bool(false) => f.write_str("x:False"),
            XBindArgument::Number(number) => f.write_str(number),
            XBindArgument::String(text) => write!(f, "'{text}'"),
            XBindArgument::Null => f.write_str("x:Null"),
        }
    }
}

/// Parses an `{x:Bind}` path. The error is the reason, without the path.
pub fn parse(text: &str) -> Result<XBindPath, String> {
    let mut parser = Parser { chars: text.chars().collect(), position: 0 };
    let path = parser.path()?;
    parser.skip_whitespace();
    if parser.position < parser.chars.len() {
        return Err(parser.syntax_error());
    }
    Ok(path)
}

struct Parser {
    chars: Vec<char>,
    position: usize,
}

fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_identifier_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.position + offset).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.position += 1;
        }
    }

    /// Consumes `c` after any whitespace.
    fn eat(&mut self, c: char) -> bool {
        self.skip_whitespace();
        if self.peek() == Some(c) {
            self.position += 1;
            return true;
        }
        false
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        match self.eat(c) {
            true => Ok(()),
            false => Err(self.syntax_error()),
        }
    }

    fn rest(&self) -> String {
        self.chars[self.position.min(self.chars.len())..].iter().collect()
    }

    /// WinUI's `SyntaxError`: `Syntax error at '...'`.
    fn syntax_error(&self) -> String {
        match self.position < self.chars.len() {
            true => format!("Syntax error at '{}'", self.rest()),
            false => "Syntax error at the end of the path".to_string(),
        }
    }

    fn starts_with(&self, text: &str) -> bool {
        let mut offset = 0;
        for c in text.chars() {
            if self.peek_at(offset) != Some(c) {
                return false;
            }
            offset += 1;
        }
        true
    }

    fn identifier(&mut self) -> Option<String> {
        self.skip_whitespace();
        if !self.peek().is_some_and(is_identifier_start) {
            return None;
        }
        let start = self.position;
        while self.peek().is_some_and(is_identifier_part) {
            self.position += 1;
        }
        Some(self.chars[start..self.position].iter().collect())
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        self.identifier().ok_or_else(|| self.syntax_error())
    }

    /// `[ns:]Type` followed by an optional `.Member`.
    fn qualified_name(&mut self) -> Result<(String, Option<String>), String> {
        let first = self.expect_identifier()?;
        let type_name = match self.peek() {
            Some(':') => {
                self.position += 1;
                let name = self.expect_identifier()?;
                format!("{first}:{name}")
            }
            _ => first,
        };
        let member = match self.eat('.') {
            true => Some(self.expect_identifier()?),
            false => None,
        };
        Ok((type_name, member))
    }

    /// Whether a path (or one of the forms that are not x:Bind syntax) starts here.
    fn at_path_start(&mut self) -> bool {
        self.skip_whitespace();
        self.peek().is_some_and(|c| is_identifier_start(c) || matches!(c, '(' | '$' | '#' | '!' | '^'))
    }

    fn path(&mut self) -> Result<XBindPath, String> {
        let primary = self.primary()?;
        self.postfix(primary)
    }

    fn primary(&mut self) -> Result<XBindPath, String> {
        self.skip_whitespace();
        match self.peek() {
            None => Err("the path is empty".to_string()),
            Some('$') => {
                self.position += 1;
                let name = self.identifier().unwrap_or_default();
                Err(format!("'${name}' is not x:Bind syntax"))
            }
            Some('#') => Err("'#name' is not x:Bind syntax; x:Bind names an element without '#'".to_string()),
            Some('!') => Err("'!' is not x:Bind syntax; use a function binding".to_string()),
            Some('^') => Err("the stream operator '^' is not x:Bind syntax".to_string()),
            Some('(') => {
                self.position += 1;
                self.skip_whitespace();
                if self.peek() == Some('(') {
                    // `((ns:Type)path)`: a cast inside a longer path.
                    let inner = self.path()?;
                    if !matches!(inner, XBindPath::Cast { path: Some(_), .. }) {
                        return Err(self.syntax_error());
                    }
                    self.expect(')')?;
                    return Ok(inner);
                }
                let (type_name, member) = self.qualified_name()?;
                self.expect(')')?;
                if let Some(property) = member {
                    return Ok(XBindPath::Attached { path: None, type_name, property });
                }
                // `(ns:Type)path` casts the leaf of the whole path that follows.
                let path = match self.at_path_start() {
                    true => {
                        let path = self.path()?;
                        if path.is_function() {
                            return Err(function_not_leaf());
                        }
                        Some(Box::new(path))
                    }
                    false => None,
                };
                Ok(XBindPath::Cast { type_name, path })
            }
            Some(c) if is_identifier_start(c) => {
                let first = self.expect_identifier()?;
                if self.peek() == Some(':') && self.peek_at(1).is_some_and(is_identifier_start) {
                    // `ns:Type.Member` or `ns:Type.Function(...)`.
                    self.position += 1;
                    let name = self.expect_identifier()?;
                    let type_name = format!("{first}:{name}");
                    self.expect('.')?;
                    let member = self.expect_identifier()?;
                    if self.eat_open_parenthesis() {
                        let arguments = self.arguments()?;
                        return Ok(XBindPath::Function { owner: XBindFunctionOwner::Static(type_name), name: member, arguments });
                    }
                    return Ok(XBindPath::Static { type_name, member });
                }
                if self.eat_open_parenthesis() {
                    let arguments = self.arguments()?;
                    return Ok(XBindPath::Function { owner: XBindFunctionOwner::Root, name: first, arguments });
                }
                Ok(XBindPath::Identifier(first))
            }
            Some(_) => Err(self.syntax_error()),
        }
    }

    fn eat_open_parenthesis(&mut self) -> bool {
        self.eat('(')
    }

    fn postfix(&mut self, mut path: XBindPath) -> Result<XBindPath, String> {
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('.') => {
                    if path.is_function() {
                        return Err(function_not_leaf());
                    }
                    self.position += 1;
                    if self.eat('(') {
                        let (type_name, member) = self.qualified_name()?;
                        let Some(property) = member else {
                            return Err(format!(
                                "a cast is written before the path it casts: ({type_name})path; an attached property is path.(Owner.Property)"
                            ));
                        };
                        self.expect(')')?;
                        path = XBindPath::Attached { path: Some(Box::new(path)), type_name, property };
                        continue;
                    }
                    let name = self.expect_identifier()?;
                    if self.eat_open_parenthesis() {
                        let arguments = self.arguments()?;
                        path = XBindPath::Function { owner: XBindFunctionOwner::Path(Box::new(path)), name, arguments };
                    } else {
                        path = XBindPath::Member(Box::new(path), name);
                    }
                }
                Some('[') => {
                    if path.is_function() {
                        return Err(function_not_leaf());
                    }
                    self.position += 1;
                    self.skip_whitespace();
                    path = match self.peek() {
                        Some(c) if c.is_ascii_digit() => {
                            let digits = self.digits();
                            let index = digits.parse::<i32>().map_err(|_| format!("the index {digits} is out of range"))?;
                            XBindPath::Index(Box::new(path), index)
                        }
                        Some('\'' | '"') => XBindPath::Key(Box::new(path), self.quoted()?),
                        _ => return Err("Expecting a digit or a quoted key in an indexer.".to_string()),
                    };
                    self.expect(']')?;
                }
                _ => return Ok(path),
            }
        }
    }

    fn digits(&mut self) -> String {
        let start = self.position;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.position += 1;
        }
        self.chars[start..self.position].iter().collect()
    }

    /// A quoted text: `'...'` or `"..."`, where `^'` and `^"` are quotes.
    fn quoted(&mut self) -> Result<String, String> {
        let Some(quote) = self.peek() else {
            return Err(self.syntax_error());
        };
        self.position += 1;
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Err("Unclosed quote".to_string()),
                Some('^') if matches!(self.peek_at(1), Some('\'' | '"')) => {
                    text.extend(self.peek_at(1));
                    self.position += 2;
                }
                Some(c) if c == quote => {
                    self.position += 1;
                    return Ok(text);
                }
                Some(c) => {
                    text.push(c);
                    self.position += 1;
                }
            }
        }
    }

    /// The arguments of a function, after its `(`.
    fn arguments(&mut self) -> Result<Vec<XBindArgument>, String> {
        let mut arguments = Vec::new();
        if self.eat(')') {
            return Ok(arguments);
        }
        loop {
            arguments.push(self.argument()?);
            if self.eat(',') {
                continue;
            }
            self.expect(')')?;
            return Ok(arguments);
        }
    }

    fn keyword(&mut self, keyword: &str) -> bool {
        let after = self.peek_at(keyword.chars().count());
        if self.starts_with(keyword) && !after.is_some_and(is_identifier_part) {
            self.position += keyword.chars().count();
            return true;
        }
        false
    }

    fn argument(&mut self) -> Result<XBindArgument, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('\'' | '"') => return Ok(XBindArgument::String(self.quoted()?)),
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let mut number = String::new();
                if c == '-' {
                    number.push('-');
                    self.position += 1;
                }
                let digits = self.digits();
                if digits.is_empty() {
                    return Err("Expecting a digit.".to_string());
                }
                number.push_str(&digits);
                if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                    self.position += 1;
                    number.push('.');
                    number.push_str(&self.digits());
                }
                return Ok(XBindArgument::Number(number));
            }
            _ => {}
        }
        if self.keyword("x:True") {
            return Ok(XBindArgument::Bool(true));
        }
        if self.keyword("x:False") {
            return Ok(XBindArgument::Bool(false));
        }
        if self.keyword("x:Null") {
            return Ok(XBindArgument::Null);
        }
        let path = self.path()?;
        if path.is_function() {
            return Err("Functions cannot have other functions as parameters.".to_string());
        }
        Ok(XBindArgument::Path(path))
    }
}

/// WinUI's `FunctionNotLeaf`.
fn function_not_leaf() -> String {
    "Functions are only supported at the end of the bind path.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(name: &str) -> Box<XBindPath> {
        Box::new(XBindPath::Identifier(name.to_string()))
    }

    #[test]
    fn members_static_members_and_indexers() {
        assert_eq!(parse("Model.Name").unwrap(), XBindPath::Member(identifier("Model"), "Name".to_string()));
        assert_eq!(
            parse("local:Helpers.Prefix").unwrap(),
            XBindPath::Static { type_name: "local:Helpers".to_string(), member: "Prefix".to_string() }
        );
        assert_eq!(parse("Items[0]").unwrap(), XBindPath::Index(identifier("Items"), 0));
        assert_eq!(parse("Map['a key']").unwrap(), XBindPath::Key(identifier("Map"), "a key".to_string()));
        assert_eq!(parse("Map[\"it^'s\"]").unwrap(), XBindPath::Key(identifier("Map"), "it's".to_string()));
        assert_eq!(parse(" Model . Items [ 2 ] ").unwrap().to_string(), "Model.Items[2]");
    }

    #[test]
    fn casts_and_attached_properties() {
        assert_eq!(
            parse("(local:Model)Tag.Name").unwrap(),
            XBindPath::Cast {
                type_name: "local:Model".to_string(),
                path: Some(Box::new(XBindPath::Member(identifier("Tag"), "Name".to_string()))),
            }
        );
        assert_eq!(parse("((local:Model)Tag).Name").unwrap().to_string(), "((local:Model)Tag).Name");
        assert_eq!(parse("(x:String)").unwrap(), XBindPath::Cast { type_name: "x:String".to_string(), path: None });
        assert_eq!(
            parse("editor.(Grid.Row)").unwrap(),
            XBindPath::Attached { path: Some(identifier("editor")), type_name: "Grid".to_string(), property: "Row".to_string() }
        );
        assert_eq!(
            parse("(Grid.Column)").unwrap(),
            XBindPath::Attached { path: None, type_name: "Grid".to_string(), property: "Column".to_string() }
        );
        assert_eq!(parse("Tag.(local:INamed.Name)").unwrap().to_string(), "Tag.(local:INamed.Name)");
    }

    #[test]
    fn functions_and_their_arguments() {
        assert_eq!(
            parse("local:Helpers.Describe('a, b', -2.5, x:True, x:Null, 3, Model.Name)").unwrap(),
            XBindPath::Function {
                owner: XBindFunctionOwner::Static("local:Helpers".to_string()),
                name: "Describe".to_string(),
                arguments: vec![
                    XBindArgument::String("a, b".to_string()),
                    XBindArgument::Number("-2.5".to_string()),
                    XBindArgument::Bool(true),
                    XBindArgument::Null,
                    XBindArgument::Number("3".to_string()),
                    XBindArgument::Path(XBindPath::Member(identifier("Model"), "Name".to_string())),
                ],
            }
        );
        assert_eq!(parse("Format()").unwrap().to_string(), "Format()");
        assert_eq!(parse("Model.Format(Name)").unwrap().to_string(), "Model.Format(Name)");
        // A keyword is a whole token: `x:Nullable` is a type name.
        assert_eq!(parse("Format(x:Nullable.Value)").unwrap().to_string(), "Format(x:Nullable.Value)");
    }

    #[test]
    fn errors() {
        assert_eq!(parse("Format().Length").unwrap_err(), "Functions are only supported at the end of the bind path.");
        assert_eq!(parse("Format(Other())").unwrap_err(), "Functions cannot have other functions as parameters.");
        assert_eq!(parse("$self.Tag").unwrap_err(), "'$self' is not x:Bind syntax");
        assert!(parse("Format(Name").unwrap_err().starts_with("Syntax error"));
        assert!(parse("Items[x]").unwrap_err().contains("Expecting a digit"));
        assert!(parse("Name Other").unwrap_err().starts_with("Syntax error"));
    }
}
