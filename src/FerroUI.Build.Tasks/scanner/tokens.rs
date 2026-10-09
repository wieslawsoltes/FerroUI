//! Token trees of a declaration: a cursor over them, the readers of a type
//! and of an expression, and their canonical text.

use proc_macro2::{Delimiter, Group, Spacing, TokenStream, TokenTree};

/// The tokens of one level of a token stream.
pub(crate) type Tokens = Vec<TokenTree>;

pub(crate) fn tokens_of(stream: TokenStream) -> Tokens {
    stream.into_iter().collect()
}

/// The line (from 1) a token starts on, in the text it was read from.
pub(crate) fn line_of(token: &TokenTree) -> usize {
    match token {
        TokenTree::Group(group) => group.span_open().start().line,
        other => other.span().start().line,
    }
}

pub(crate) fn is_punct(token: &TokenTree, character: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == character)
}

/// Whether `token` is the first of two punctuation characters written together (`::`, `->`).
fn is_joint(token: &TokenTree, character: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == character && punct.spacing() == Spacing::Joint)
}

pub(crate) fn ident_of(token: &TokenTree) -> Option<String> {
    match token {
        TokenTree::Ident(ident) => Some(ident.to_string()),
        _ => None,
    }
}

pub(crate) fn group_of(token: &TokenTree, delimiter: Delimiter) -> Option<&Group> {
    match token {
        TokenTree::Group(group) if group.delimiter() == delimiter => Some(group),
        _ => None,
    }
}

/// Whether `tokens[index..]` starts with `::`.
pub(crate) fn is_path_separator(tokens: &[TokenTree], index: usize) -> bool {
    index + 1 < tokens.len() && is_joint(&tokens[index], ':') && is_punct(&tokens[index + 1], ':')
}

/// Whether `tokens[index..]` starts with `->` (`first` is `-`) or `=>` (`first` is `=`).
pub(crate) fn is_arrow(tokens: &[TokenTree], index: usize, first: char) -> bool {
    index + 1 < tokens.len() && is_joint(&tokens[index], first) && is_punct(&tokens[index + 1], '>')
}

/// An error of a reader of a declaration: the line and what was expected.
#[derive(Debug)]
pub(crate) struct ParseError {
    pub line: usize,
    pub message: String,
}

/// Where a type ends, besides `,`, `;`, `=>` and the end of the tokens, which always end it.
#[derive(Clone, Copy, Default)]
pub(crate) struct TypeEnd {
    /// A group in braces ends the type (`Name: T { get: .. }`).
    pub brace: bool,
    /// A group in brackets that follows a complete type ends it (`name: T [Attribute]`).
    pub bracket: bool,
    /// The word `as` ends the type (`dyn IBrush as "IBrush"`).
    pub as_: bool,
}

/// A cursor over the tokens of one level.
pub(crate) struct Cursor<'a> {
    tokens: &'a [TokenTree],
    at: usize,
    /// The line of the group the tokens are in: the line of an error at the end.
    line: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(tokens: &'a [TokenTree], line: usize) -> Self {
        Self { tokens, at: 0, line }
    }

    pub fn is_end(&self) -> bool {
        self.at >= self.tokens.len()
    }

    pub fn peek(&self) -> Option<&'a TokenTree> {
        self.tokens.get(self.at)
    }

    pub fn peek_at(&self, offset: usize) -> Option<&'a TokenTree> {
        self.tokens.get(self.at + offset)
    }

    pub fn next(&mut self) -> Option<&'a TokenTree> {
        let token = self.tokens.get(self.at);
        if token.is_some() {
            self.at += 1;
        }
        token
    }

    pub fn position(&self) -> usize {
        self.at
    }

    pub fn rewind(&mut self, position: usize) {
        self.at = position;
    }

    /// The tokens from `position` to the cursor.
    pub fn since(&self, position: usize) -> &'a [TokenTree] {
        &self.tokens[position..self.at]
    }

    /// The line of the token at the cursor, or of the last token at the end.
    pub fn line(&self) -> usize {
        self.peek().or_else(|| self.tokens.last()).map_or(self.line, line_of)
    }

    pub fn error<T>(&self, expected: &str) -> Result<T, ParseError> {
        let found = match self.peek() {
            Some(token) => format!("`{}`", text_of(std::slice::from_ref(token)).chars().take(40).collect::<String>()),
            None => "the end".to_string(),
        };
        Err(ParseError { line: self.line(), message: format!("expected {expected}, found {found}") })
    }

    pub fn is_ident(&self, name: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(ident)) if ident == name)
    }

    pub fn eat_ident(&mut self, name: &str) -> bool {
        let found = self.is_ident(name);
        if found {
            self.at += 1;
        }
        found
    }

    pub fn take_ident(&mut self, what: &str) -> Result<String, ParseError> {
        match self.peek().and_then(ident_of) {
            Some(name) => {
                self.at += 1;
                Ok(name)
            }
            None => self.error(what),
        }
    }

    pub fn is_punct(&self, character: char) -> bool {
        self.peek().is_some_and(|token| is_punct(token, character))
    }

    pub fn eat_punct(&mut self, character: char) -> bool {
        let found = self.is_punct(character);
        if found {
            self.at += 1;
        }
        found
    }

    pub fn expect_punct(&mut self, character: char) -> Result<(), ParseError> {
        if self.eat_punct(character) {
            Ok(())
        } else {
            self.error(&format!("`{character}`"))
        }
    }

    /// `->` or `=>`.
    pub fn eat_arrow(&mut self, first: char) -> bool {
        let found = is_arrow(self.tokens, self.at, first);
        if found {
            self.at += 2;
        }
        found
    }

    pub fn expect_arrow(&mut self, first: char) -> Result<(), ParseError> {
        if self.eat_arrow(first) {
            Ok(())
        } else {
            self.error(&format!("`{first}>`"))
        }
    }

    pub fn eat_path_separator(&mut self) -> bool {
        let found = is_path_separator(self.tokens, self.at);
        if found {
            self.at += 2;
        }
        found
    }

    pub fn is_group(&self, delimiter: Delimiter) -> bool {
        self.peek().is_some_and(|token| group_of(token, delimiter).is_some())
    }

    /// The tokens of the group at the cursor and its line.
    pub fn take_group(&mut self, delimiter: Delimiter, what: &str) -> Result<(Tokens, usize), ParseError> {
        match self.peek().and_then(|token| group_of(token, delimiter).map(|group| (group, line_of(token)))) {
            Some((group, line)) => {
                self.at += 1;
                Ok((tokens_of(group.stream()), line))
            }
            None => self.error(what),
        }
    }

    /// Skips attributes (`#[..]`, which is also what a doc comment is).
    pub fn skip_attributes(&mut self) {
        while self.is_punct('#') && self.peek_at(1).is_some_and(|token| group_of(token, Delimiter::Bracket).is_some()) {
            self.at += 2;
        }
    }

    /// A visibility (`pub`, `pub(crate)`), as written; empty when there is none.
    pub fn take_visibility(&mut self) -> String {
        if !self.is_ident("pub") {
            return String::new();
        }
        let start = self.at;
        self.at += 1;
        if self.is_group(Delimiter::Parenthesis) {
            self.at += 1;
        }
        text_of(&self.tokens[start..self.at])
    }

    /// A path of identifiers (`Type::function`), when one starts at the cursor.
    pub fn take_plain_path(&mut self) -> Option<&'a [TokenTree]> {
        let start = self.at;
        if !matches!(self.peek(), Some(TokenTree::Ident(_))) {
            return None;
        }
        self.at += 1;
        while is_path_separator(self.tokens, self.at) && matches!(self.tokens.get(self.at + 2), Some(TokenTree::Ident(_))) {
            self.at += 3;
        }
        Some(&self.tokens[start..self.at])
    }

    /// The tokens of a type: up to `,`, `;`, `=>`, the end, or what `end` adds, outside
    /// angle brackets.
    pub fn take_type(&mut self, end: TypeEnd, what: &str) -> Result<&'a [TokenTree], ParseError> {
        let start = self.at;
        let mut depth = 0usize;
        while let Some(token) = self.peek() {
            if depth == 0 {
                if is_punct(token, ',') || is_punct(token, ';') || is_arrow(self.tokens, self.at, '=') {
                    break;
                }
                if end.brace && group_of(token, Delimiter::Brace).is_some() {
                    break;
                }
                if end.as_ && self.is_ident("as") {
                    break;
                }
                if end.bracket && group_of(token, Delimiter::Bracket).is_some() && is_complete_type(&self.tokens[start..self.at]) {
                    break;
                }
            }
            if is_arrow(self.tokens, self.at, '-') {
                self.at += 2;
                continue;
            }
            if is_punct(token, '<') {
                depth += 1;
            } else if is_punct(token, '>') {
                depth = depth.saturating_sub(1);
            }
            self.at += 1;
        }
        if self.at == start {
            return self.error(what);
        }
        Ok(&self.tokens[start..self.at])
    }

    /// The tokens of an expression: up to a `,` (or `;`) that is not inside the parameters
    /// of a leading closure or a turbofish.
    pub fn take_expression(&mut self, what: &str) -> Result<&'a [TokenTree], ParseError> {
        let start = self.at;
        self.eat_ident("move");
        if self.is_punct('|') {
            // The parameters of a closure: `||` or `|a, b: T|`.
            self.at += 1;
            while let Some(token) = self.next() {
                if is_punct(token, '|') {
                    break;
                }
            }
        }
        let mut depth = 0usize;
        while let Some(token) = self.peek() {
            if depth == 0 && (is_punct(token, ',') || is_punct(token, ';')) {
                break;
            }
            if is_arrow(self.tokens, self.at, '-') || is_arrow(self.tokens, self.at, '=') {
                self.at += 2;
                continue;
            }
            if depth == 0 {
                if is_path_separator(self.tokens, self.at) && self.tokens.get(self.at + 2).is_some_and(|next| is_punct(next, '<')) {
                    depth = 1;
                    self.at += 3;
                    continue;
                }
            } else if is_punct(token, '<') {
                depth += 1;
            } else if is_punct(token, '>') {
                depth -= 1;
            }
            self.at += 1;
        }
        if self.at == start {
            return self.error(what);
        }
        Ok(&self.tokens[start..self.at])
    }
}

/// Whether the tokens are a type that a group in brackets cannot continue: they end with
/// a name, `>` or a group, and not with `&`, a lifetime or `mut`.
fn is_complete_type(tokens: &[TokenTree]) -> bool {
    match tokens.last() {
        Some(TokenTree::Ident(ident)) => {
            let lifetime = tokens.len() >= 2 && is_punct(&tokens[tokens.len() - 2], '\'');
            !lifetime && !matches!(ident.to_string().as_str(), "mut" | "dyn" | "const")
        }
        Some(TokenTree::Group(_)) => true,
        Some(token) => is_punct(token, '>'),
        None => false,
    }
}

/// The parts of `tokens` between the commas that are outside angle brackets, each read as
/// a type; an empty part at the end (a trailing comma) is left out.
pub(crate) fn split_types(tokens: &[TokenTree], line: usize) -> Result<Vec<Tokens>, ParseError> {
    let mut cursor = Cursor::new(tokens, line);
    let mut types = Vec::new();
    while !cursor.is_end() {
        types.push(cursor.take_type(TypeEnd::default(), "a type")?.to_vec());
        if !cursor.is_end() {
            cursor.expect_punct(',')?;
        }
    }
    Ok(types)
}

/// The head of a generic type and its arguments: `DirectProperty<O, T>` gives
/// (`[DirectProperty]`, `[O, T]`). The head is the segments of the path, the last one the
/// name. Nothing when the tokens are not a path followed by one argument list.
pub(crate) fn generic_type(tokens: &[TokenTree]) -> Option<(Vec<String>, Vec<Tokens>)> {
    let mut head = Vec::new();
    let mut index = 0;
    if is_path_separator(tokens, index) {
        head.push(String::new());
        index += 2;
    }
    loop {
        head.push(ident_of(tokens.get(index)?)?);
        index += 1;
        if is_path_separator(tokens, index) && matches!(tokens.get(index + 2), Some(TokenTree::Ident(_))) {
            index += 2;
        } else {
            break;
        }
    }
    if index == tokens.len() {
        return Some((head, Vec::new()));
    }
    if !is_punct(&tokens[index], '<') || !is_punct(tokens.last()?, '>') {
        return None;
    }
    let arguments = split_types(&tokens[index + 1..tokens.len() - 1], 0).ok()?;
    Some((head, arguments))
}

/// A piece of the text of tokens: what is printed between two places where a space may go.
pub(crate) enum Piece {
    /// An identifier, a keyword, a literal, a lifetime or a whole path.
    Word(String),
    /// A punctuation mark, or two written together (`::`, `->`, `=>`).
    Mark(&'static str, String),
    Group(Delimiter, Vec<Piece>),
}

/// The words that are not the head of a path in a type or an expression.
const KEYWORDS: &[&str] = &[
    "_", "as", "const", "dyn", "extern", "fn", "for", "impl", "in", "let", "move", "mut", "ref", "return", "static", "unsafe", "where", "true",
    "false", "if", "else", "match", "loop", "while", "break", "continue",
];

/// The pieces of `tokens`. A path (`a::b::C`, `::a::B`) is one word: `path` is called with
/// its segments (a leading `::` is an empty first segment) and gives the text of the word;
/// without `path` the word is the path as written.
pub(crate) fn pieces(tokens: &[TokenTree], path: &mut dyn FnMut(&[String]) -> String) -> Vec<Piece> {
    let mut result: Vec<Piece> = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        match token {
            TokenTree::Group(group) => {
                result.push(Piece::Group(group.delimiter(), pieces(&tokens_of(group.stream()), path)));
                index += 1;
            }
            TokenTree::Literal(literal) => {
                result.push(Piece::Word(literal.to_string()));
                index += 1;
            }
            TokenTree::Punct(punct) => {
                let character = punct.as_char();
                if character == '\'' {
                    if let Some(name) = tokens.get(index + 1).and_then(ident_of) {
                        result.push(Piece::Word(format!("'{name}")));
                        index += 2;
                        continue;
                    }
                }
                if is_path_separator(tokens, index) {
                    // `::` after a name or `>` continues what is before it; anywhere else it
                    // starts a path from the crate root of the edition (`::std::rc::Rc`).
                    let continues = matches!(result.last(), Some(Piece::Word(_)) | Some(Piece::Group(..)))
                        || matches!(result.last(), Some(Piece::Mark(mark, _)) if *mark == ">");
                    if !continues && matches!(tokens.get(index + 2), Some(TokenTree::Ident(_))) {
                        let (segments, next) = path_segments(tokens, index + 2);
                        let mut rooted = vec![String::new()];
                        rooted.extend(segments);
                        result.push(Piece::Word(path(&rooted)));
                        index = next;
                    } else {
                        result.push(Piece::Mark("::", "::".to_string()));
                        index += 2;
                    }
                    continue;
                }
                if is_arrow(tokens, index, '-') {
                    result.push(Piece::Mark("->", " -> ".to_string()));
                    index += 2;
                    continue;
                }
                if is_arrow(tokens, index, '=') {
                    result.push(Piece::Mark("=>", " => ".to_string()));
                    index += 2;
                    continue;
                }
                let (mark, text): (&'static str, String) = match character {
                    ',' => (",", ", ".to_string()),
                    ';' => (";", "; ".to_string()),
                    ':' => (":", ": ".to_string()),
                    '=' if punct.spacing() == Spacing::Alone => ("=", " = ".to_string()),
                    '+' => ("+", " + ".to_string()),
                    '>' => (">", ">".to_string()),
                    other => ("", other.to_string()),
                };
                // A trailing comma of a list of type arguments is not part of the text
                // (`Map<K, V,>` written over several lines is `Map<K, V>`).
                if mark == ">" && matches!(result.last(), Some(Piece::Mark(last, _)) if *last == ",") {
                    result.pop();
                }
                result.push(Piece::Mark(mark, text));
                index += 1;
            }
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let after_separator = matches!(result.last(), Some(Piece::Mark(mark, _)) if *mark == "::");
                let binding = tokens.get(index + 1).is_some_and(|next| matches!(next, TokenTree::Punct(punct) if punct.as_char() == '=' && punct.spacing() == Spacing::Alone));
                if KEYWORDS.contains(&name.as_str()) || after_separator || binding {
                    result.push(Piece::Word(name));
                    index += 1;
                } else {
                    let (segments, next) = path_segments(tokens, index);
                    result.push(Piece::Word(path(&segments)));
                    index = next;
                }
            }
        }
    }
    result
}

/// The segments of the path of identifiers that starts at `tokens[index]`, and the index
/// after it.
fn path_segments(tokens: &[TokenTree], mut index: usize) -> (Vec<String>, usize) {
    let mut segments = Vec::new();
    while let Some(name) = tokens.get(index).and_then(ident_of) {
        segments.push(name);
        index += 1;
        if is_path_separator(tokens, index) && matches!(tokens.get(index + 2), Some(TokenTree::Ident(_))) {
            index += 2;
        } else {
            break;
        }
    }
    (segments, index)
}

/// The text of pieces: no space but between two words, after `,` `;` `:` and around `->`,
/// `=>`, `=`, `+`; a trailing comma of a group is left out.
pub(crate) fn text_of_pieces(pieces: &[Piece]) -> String {
    let mut text = String::new();
    write_pieces(&mut text, pieces);
    text.trim_end().to_string()
}

fn write_pieces(text: &mut String, pieces: &[Piece]) {
    let mut after_word = false;
    for piece in pieces {
        match piece {
            Piece::Word(word) => {
                if after_word || (word == "as" && !text.ends_with(' ')) {
                    text.push(' ');
                }
                text.push_str(word);
                after_word = true;
            }
            Piece::Mark(_, mark) => {
                if mark.starts_with(' ') && text.ends_with(' ') {
                    text.push_str(mark.trim_start());
                } else {
                    text.push_str(mark);
                }
                after_word = false;
            }
            Piece::Group(delimiter, inner) => {
                let (open, close) = match delimiter {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                if after_word && matches!(delimiter, Delimiter::Bracket | Delimiter::Brace) {
                    text.push(' ');
                }
                text.push_str(open);
                let mut inside = String::new();
                write_pieces(&mut inside, inner);
                let inside = inside.trim_end();
                text.push_str(inside.strip_suffix(',').unwrap_or(inside));
                text.push_str(close);
                after_word = false;
            }
        }
    }
}

/// The canonical text of tokens, with every path as written.
pub(crate) fn text_of(tokens: &[TokenTree]) -> String {
    text_of_pieces(&pieces(tokens, &mut |segments: &[String]| segments.join("::")))
}

/// The segments of `tokens` when they are a path of identifiers, turbofish arguments left
/// out (`FerroList::<T>::new` gives `FerroList`, `new`); a leading `::` is an empty first
/// segment. Nothing for any other expression.
pub(crate) fn plain_path(tokens: &[TokenTree]) -> Option<Vec<String>> {
    let mut segments = Vec::new();
    let mut index = 0;
    if is_path_separator(tokens, index) {
        segments.push(String::new());
        index += 2;
    }
    loop {
        segments.push(ident_of(tokens.get(index)?)?);
        index += 1;
        if index == tokens.len() {
            return Some(segments);
        }
        if !is_path_separator(tokens, index) {
            return None;
        }
        index += 2;
        if tokens.get(index).is_some_and(|token| is_punct(token, '<')) {
            // A turbofish: skip to the `>` that closes it.
            let mut depth = 0usize;
            loop {
                let token = tokens.get(index)?;
                if is_arrow(tokens, index, '-') {
                    index += 2;
                    continue;
                }
                if is_punct(token, '<') {
                    depth += 1;
                } else if is_punct(token, '>') {
                    depth -= 1;
                }
                index += 1;
                if depth == 0 {
                    break;
                }
            }
            if index == tokens.len() {
                return Some(segments);
            }
            if !is_path_separator(tokens, index) {
                return None;
            }
            index += 2;
        }
    }
}

/// Calls `found` with the name and the line of every invocation `name!(..)` of one of
/// `names` in `tokens`, at any depth.
pub(crate) fn invocations(tokens: &[TokenTree], names: &[&str], found: &mut dyn FnMut(&str, usize)) {
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Ident(ident) => {
                let is_invocation = tokens.get(index + 1).is_some_and(|next| is_punct(next, '!'))
                    && matches!(tokens.get(index + 2), Some(TokenTree::Group(_)));
                if is_invocation {
                    let name = ident.to_string();
                    if names.contains(&name.as_str()) {
                        found(&name, line_of(token));
                    }
                }
            }
            TokenTree::Group(group) => invocations(&tokens_of(group.stream()), names, found),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Tokens {
        tokens_of(text.parse().expect("tokens"))
    }

    /// Not from upstream: the text of a type does not depend on how it is spaced.
    #[test]
    fn type_text_is_canonical() {
        for (written, expected) in [
            ("Option < Rc < dyn IBrush > >", "Option<Rc<dyn IBrush>>"),
            ("& 'static FerroProperty", "&'static FerroProperty"),
            ("&'static [u8]", "&'static [u8]"),
            ("& mut [ T ; 4 ]", "&mut [T; 4]"),
            ("Rc<dyn Fn( A , B ,) -> C + 'static>", "Rc<dyn Fn(A, B) -> C + 'static>"),
            (":: std :: rc :: Rc < crate :: a :: B >", "::std::rc::Rc<crate::a::B>"),
            ("FerroList<Ref<Control>>", "FerroList<Ref<Control>>"),
            ("(A,B)", "(A, B)"),
            ("Map<\n    K,\n    Rc<dyn V>,\n>", "Map<K, Rc<dyn V>>"),
            ("Option<Map<K, V,>,>", "Option<Map<K, V>>"),
            ("Box<dyn Iterator<Item = u8>>", "Box<dyn Iterator<Item = u8>>"),
            ("<T as Trait>::Output", "<T as Trait>::Output"),
        ] {
            assert_eq!(text_of(&tokens(written)), expected, "{written}");
        }
    }

    /// Not from upstream: a path is handed over whole, with a leading `::` as an empty
    /// segment; keywords, lifetimes and the names of associated types are not paths.
    #[test]
    fn paths_of_a_type_are_found() {
        let mut found = Vec::new();
        let text = text_of_pieces(&pieces(
            &tokens("Rc<dyn a::IBrush + 'static>, ::std::Vec<&'a mut b::C>, Box<dyn Iterator<Item = d::E>>, Foo<A>::Bar"),
            &mut |segments: &[String]| {
                found.push(segments.join("::"));
                format!("<{}>", segments.join("."))
            },
        ));
        assert_eq!(found, ["Rc", "a::IBrush", "::std::Vec", "b::C", "Box", "Iterator", "d::E", "Foo", "A"]);
        assert_eq!(text, "<Rc><dyn <a.IBrush> + 'static>, <.std.Vec><&'a mut <b.C>>, <Box><dyn <Iterator><Item = <d.E>>>, <Foo><<A>>::Bar");
    }

    /// Not from upstream: a type ends where the declaration forms end it.
    #[test]
    fn type_reader_stops_where_the_forms_end_a_type() {
        let source = tokens("Option<(A, B)> { get: x } FerroList<A, B> => y, &'static [T] [Attribute] fn(A) -> B, dyn IBrush as \"IBrush\"");
        let mut cursor = Cursor::new(&source, 1);
        let end = TypeEnd { brace: true, bracket: true, as_: true };
        assert_eq!(text_of(cursor.take_type(end, "a type").expect("a type")), "Option<(A, B)>");
        assert!(cursor.is_group(Delimiter::Brace));
        cursor.next();
        assert_eq!(text_of(cursor.take_type(end, "a type").expect("a type")), "FerroList<A, B>");
        assert!(cursor.eat_arrow('='));
        assert_eq!(text_of(cursor.take_expression("an expression").expect("an expression")), "y");
        assert!(cursor.eat_punct(','));
        assert_eq!(text_of(cursor.take_type(end, "a type").expect("a type")), "&'static [T]");
        assert!(cursor.is_group(Delimiter::Bracket));
        cursor.next();
        assert_eq!(text_of(cursor.take_type(end, "a type").expect("a type")), "fn(A) -> B");
        assert!(cursor.eat_punct(','));
        assert_eq!(text_of(cursor.take_type(end, "a type").expect("a type")), "dyn IBrush");
        assert!(cursor.eat_ident("as"));
        assert!(cursor.take_type(end, "a type").is_ok());
        assert!(cursor.is_end());
        let error = cursor.take_type(end, "a type").expect_err("nothing is left");
        assert_eq!(error.message, "expected a type, found the end");
    }

    /// Not from upstream: an expression ends at the comma after it, not at one of the
    /// parameters of a closure or of a turbofish.
    #[test]
    fn expression_reader_keeps_closures_and_turbofish_whole() {
        let source = tokens("|this: &Ref<Window>, handler: MarkupDelegate| this.add(handler), Map::<A, B>::new, move || 1, Type::function");
        let mut cursor = Cursor::new(&source, 1);
        let mut expressions = Vec::new();
        while !cursor.is_end() {
            expressions.push(cursor.take_expression("an expression").expect("an expression").to_vec());
            cursor.eat_punct(',');
        }
        assert_eq!(expressions.len(), 4);
        assert_eq!(plain_path(&expressions[0]), None);
        assert_eq!(plain_path(&expressions[1]), Some(vec!["Map".to_string(), "new".to_string()]));
        assert_eq!(plain_path(&expressions[2]), None);
        assert_eq!(plain_path(&expressions[3]), Some(vec!["Type".to_string(), "function".to_string()]));
        assert_eq!(text_of(&expressions[3]), "Type::function");
    }

    /// Not from upstream: the arguments of a generic type, and the invocations of a macro
    /// at any depth with their lines.
    #[test]
    fn generic_arguments_and_invocations_are_found() {
        let (head, arguments) = generic_type(&tokens("DirectProperty<Border, Option<Rc<dyn IBrush>>>")).expect("a generic type");
        assert_eq!(head, ["DirectProperty"]);
        assert_eq!(arguments.iter().map(|argument| text_of(argument)).collect::<Vec<_>>(), ["Border", "Option<Rc<dyn IBrush>>"]);
        assert_eq!(generic_type(&tokens("a::B")).map(|(head, arguments)| (head, arguments.len())), Some((vec!["a".to_string(), "B".to_string()], 0)));
        assert!(generic_type(&tokens("&'static T")).is_none());

        let mut found = Vec::new();
        invocations(
            &tokens("impl X {\n ferro_property!(fn a() -> T { other!(ferro_class!(A: B)) });\n }\n ferro_class ! { C: D }\n ferro_class; not_ferro_class!(E: F);"),
            &["ferro_class", "ferro_property"],
            &mut |name, line| found.push((name.to_string(), line)),
        );
        assert_eq!(found, [("ferro_property".to_string(), 2), ("ferro_class".to_string(), 2), ("ferro_class".to_string(), 4)]);
    }
}
