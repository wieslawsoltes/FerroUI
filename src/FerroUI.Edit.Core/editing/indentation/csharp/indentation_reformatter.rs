use super::super::IndentationError;
use super::IDocumentAccessor;
use crate::document::{CharacterClass, TextUtilities};
use std::rc::Rc;

pub(super) struct IndentationSettings<'a> {
    pub indent_string: &'a str,
    pub leave_empty_lines: bool,
}

/// Braces, parentheses and brackets each retain their surrounding indentation.
/// Unbraced statements and continuations are tracked separately within a block.
#[derive(Clone)]
struct Block {
    outer_indent: Rc<str>,
    inner_indent: Rc<str>,
    last_word: Rc<str>,
    bracket: char,
    continuation: bool,
    one_line_block: usize,
    previous_one_line_block: usize,
    start_line: usize,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            outer_indent: Rc::from(""),
            inner_indent: Rc::from(""),
            last_word: Rc::from(""),
            bracket: '{',
            continuation: false,
            one_line_block: 0,
            previous_one_line_block: 0,
            start_line: 0,
        }
    }
}

impl Block {
    fn reset_one_line_block(&mut self) {
        self.previous_one_line_block = self.one_line_block;
        self.one_line_block = 0;
    }

    fn indent(&mut self, indentation: &str) {
        self.outer_indent = self.inner_indent.clone();
        let mut inner = self.inner_indent.to_string();
        inner.push_str(indentation);
        self.inner_indent = Rc::from(inner);
        self.continuation = false;
        self.reset_one_line_block();
        self.last_word = Rc::from("");
    }
}

pub(super) struct IndentationReformatter {
    word_builder: String,
    blocks: Vec<Block>,
    block: Block,
    in_string: bool,
    in_char: bool,
    verbatim: bool,
    escape: bool,
    line_comment: bool,
    block_comment: bool,
    last_real_char: char,
}

impl Default for IndentationReformatter {
    fn default() -> Self {
        Self {
            word_builder: String::new(),
            blocks: Vec::new(),
            block: Block::default(),
            in_string: false,
            in_char: false,
            verbatim: false,
            escape: false,
            line_comment: false,
            block_comment: false,
            last_real_char: ' ',
        }
    }
}

impl IndentationReformatter {
    pub(super) fn reformat(
        &mut self,
        document: &mut dyn IDocumentAccessor,
        settings: &IndentationSettings<'_>,
    ) -> Result<(), IndentationError> {
        *self = Self::default();
        while document.move_next()? {
            self.step(document, settings)?;
        }
        Ok(())
    }

    fn finish_word(&mut self) {
        if !self.word_builder.is_empty() {
            self.block.last_word = Rc::from(self.word_builder.as_str());
            self.word_builder.clear();
        }
    }

    fn step(
        &mut self,
        document: &mut dyn IDocumentAccessor,
        settings: &IndentationSettings<'_>,
    ) -> Result<(), IndentationError> {
        if settings.leave_empty_lines && document.text().is_empty() {
            return Ok(());
        }
        let mut line = document.text().trim_start().to_owned();
        if line.is_empty() {
            if self.block_comment || self.in_string && self.verbatim {
                return Ok(());
            }
            let mut indent = self.block.inner_indent.to_string();
            indent.push_str(&settings.indent_string.repeat(self.block.one_line_block));
            if document.text() != indent {
                document.set_text(indent)?;
            }
            return Ok(());
        }

        if trim_end(document)? {
            line = document.text().trim_start().to_owned();
        }
        let mut old_block = self.block.clone();
        let start_in_comment = self.block_comment;
        let start_in_string = self.in_string && self.verbatim;

        self.line_comment = false;
        self.in_char = false;
        self.escape = false;
        if !self.verbatim {
            self.in_string = false;
        }
        self.last_real_char = '\n';

        // Columns and lookahead count UTF-16 code units, including both halves
        // of supplementary scalars, just like document offsets.
        let units: Vec<u16> = line.encode_utf16().collect();
        let mut current = ' ';
        for (index, &unit) in units.iter().enumerate() {
            if self.line_comment {
                break;
            }
            let previous = current;
            current = char::from_u32(unit as u32).unwrap_or('\u{fffd}');
            let next = units.get(index + 1).map_or('\n', |&unit| {
                char::from_u32(unit as u32).unwrap_or('\u{fffd}')
            });
            if self.escape {
                self.escape = false;
                continue;
            }

            match current {
                '/' => {
                    if self.block_comment && previous == '*' {
                        self.block_comment = false;
                    }
                    if !self.in_string && !self.in_char {
                        if !self.block_comment && next == '/' {
                            self.line_comment = true;
                        }
                        if !self.line_comment && next == '*' {
                            self.block_comment = true;
                        }
                    }
                }
                '#' => {
                    if !(self.in_char || self.block_comment || self.in_string) {
                        self.line_comment = true;
                    }
                }
                '"' => {
                    if !(self.in_char || self.line_comment || self.block_comment) {
                        self.in_string = !self.in_string;
                        if !self.in_string && self.verbatim {
                            if next == '"' {
                                self.escape = true;
                                self.in_string = true;
                            } else {
                                self.verbatim = false;
                            }
                        } else if self.in_string && previous == '@' {
                            self.verbatim = true;
                        }
                    }
                }
                '\'' => {
                    if !(self.in_string || self.line_comment || self.block_comment) {
                        self.in_char = !self.in_char;
                    }
                }
                '\\' => {
                    if self.in_string && !self.verbatim || self.in_char {
                        self.escape = true;
                    }
                }
                _ => {}
            }

            if self.line_comment || self.block_comment || self.in_string || self.in_char {
                self.finish_word();
                continue;
            }
            if !current.is_whitespace()
                && current != '['
                && current != '/'
                && self.block.bracket == '{'
            {
                self.block.continuation = true;
            }
            if is_letter_or_digit(unit) {
                self.word_builder.push(current);
            } else {
                self.finish_word();
            }

            match current {
                '{' => {
                    self.block.reset_one_line_block();
                    self.blocks.push(self.block.clone());
                    self.block.start_line = document.line_number();
                    if self.block.last_word.as_ref() == "switch" {
                        self.block.indent(&settings.indent_string.repeat(2));
                    } else {
                        self.block.indent(settings.indent_string);
                    }
                    self.block.bracket = '{';
                }
                '}' => {
                    while self.block.bracket != '{' {
                        let Some(block) = self.blocks.pop() else {
                            break;
                        };
                        self.block = block;
                    }
                    if let Some(block) = self.blocks.pop() {
                        self.block = block;
                        self.block.continuation = false;
                        self.block.reset_one_line_block();
                    }
                }
                '(' | '[' => {
                    self.blocks.push(self.block.clone());
                    if self.block.start_line == document.line_number() {
                        self.block.inner_indent = self.block.outer_indent.clone();
                    } else {
                        self.block.start_line = document.line_number();
                    }
                    let mut extra = settings.indent_string.repeat(old_block.one_line_block);
                    if old_block.continuation {
                        extra.push_str(settings.indent_string);
                    }
                    if index == units.len() - 1 {
                        extra.push_str(settings.indent_string);
                    } else {
                        extra.push_str(&" ".repeat(index + 1));
                    }
                    self.block.indent(&extra);
                    self.block.bracket = current;
                }
                ')' => {
                    if self.block.bracket == '(' {
                        if let Some(block) = self.blocks.pop() {
                            self.block = block;
                            if is_single_statement_keyword(&self.block.last_word) {
                                self.block.continuation = false;
                            }
                        }
                    }
                }
                ']' => {
                    if self.block.bracket == '[' {
                        if let Some(block) = self.blocks.pop() {
                            self.block = block;
                        }
                    }
                }
                ';' | ',' => {
                    self.block.continuation = false;
                    self.block.reset_one_line_block();
                }
                ':' => {
                    if is_label(&line, &self.block.last_word) {
                        self.block.continuation = false;
                        self.block.reset_one_line_block();
                    }
                }
                _ => {}
            }
            if !current.is_whitespace() {
                self.last_real_char = current;
            }
        }
        self.finish_word();

        let first = line.chars().next().expect("nonempty line after trimming");
        if start_in_string || start_in_comment && first != '*' {
            return Ok(());
        }
        if document.text().starts_with("//\t") || document.text() == "//" {
            return Ok(());
        }

        let mut indent = if first == '}' {
            old_block.reset_one_line_block();
            old_block.continuation = false;
            old_block.outer_indent.to_string()
        } else {
            old_block.inner_indent.to_string()
        };
        if !indent.is_empty()
            && (old_block.bracket == '(' && first == ')'
                || old_block.bracket == '[' && first == ']')
        {
            indent.pop();
        }

        if first == ':' {
            old_block.continuation = true;
        } else if self.last_real_char == ':'
            && indent.encode_utf16().count() >= settings.indent_string.len()
        {
            if is_label(&line, &self.block.last_word) {
                for _ in 0..settings.indent_string.len() {
                    indent.pop();
                }
            }
        } else if self.last_real_char == ')' {
            if is_single_statement_keyword(&self.block.last_word) {
                self.block.one_line_block += 1;
            }
        } else if self.last_real_char == 'e' && self.block.last_word.as_ref() == "else" {
            self.block.one_line_block = self.block.previous_one_line_block.max(1);
            self.block.continuation = false;
            old_block.one_line_block = self.block.one_line_block - 1;
        }

        if document.is_read_only() {
            if !old_block.continuation
                && old_block.one_line_block == 0
                && old_block.start_line == self.block.start_line
                && self.block.start_line < document.line_number()
                && self.last_real_char != ':'
            {
                let text = document.text();
                let leading_length = text.len() - text.trim_start().len();
                let mut existing = text[..leading_length].to_owned();
                if start_in_comment && existing.ends_with(' ') {
                    existing.pop();
                }
                self.block.inner_indent = Rc::from(existing);
            }
            return Ok(());
        }

        if first != '{' {
            if first != ')' && old_block.continuation && old_block.bracket == '{' {
                indent.push_str(settings.indent_string);
            }
            indent.push_str(&settings.indent_string.repeat(old_block.one_line_block));
        }
        if start_in_comment {
            indent.push(' ');
        }
        indent.push_str(&line);
        if document.text() != indent {
            document.set_text(indent)?;
        }
        Ok(())
    }
}

fn is_letter_or_digit(unit: u16) -> bool {
    if unit <= 0x7f {
        (unit as u8).is_ascii_alphanumeric()
    } else {
        TextUtilities::get_character_class(unit) == CharacterClass::IdentifierPart
    }
}

fn is_label(line: &str, last_word: &str) -> bool {
    last_word == "case"
        || line.starts_with("case ")
        || line
            .strip_prefix(last_word)
            .is_some_and(|tail| tail.starts_with(':'))
}

fn is_single_statement_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "if" | "for" | "while" | "do" | "foreach" | "using" | "lock"
    )
}

fn trim_end(document: &mut dyn IDocumentAccessor) -> Result<bool, IndentationError> {
    let line = document.text();
    if !line.chars().last().is_some_and(char::is_whitespace)
        || line.ends_with("// ")
        || line.ends_with("* ")
    {
        return Ok(false);
    }
    document.set_text(line.trim_end().to_owned())?;
    Ok(true)
}
