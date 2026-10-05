//! Port of `Parsers/CommaSeparatedParenthesesTreeParser.cs`.

use std::fmt;

pub struct CommaSeparatedParenthesesTreeParser;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node {
    pub value: Option<String>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseException {
    pub message: String,
}

impl ParseException {
    fn new(message: &str, position: usize) -> Self {
        Self {
            message: format!("{message} at position {position}"),
        }
    }
}

impl fmt::Display for ParseException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseException {}

#[derive(Default)]
struct ArenaNode {
    value: Option<String>,
    children: Vec<usize>,
}

impl CommaSeparatedParenthesesTreeParser {
    pub fn parse(s: &str) -> Result<Vec<Node>, ParseException> {
        // The upstream parser links mutable nodes together; an index arena keeps the same shape.
        let mut arena: Vec<ArenaNode> = Vec::new();
        let new_node = |arena: &mut Vec<ArenaNode>| {
            arena.push(ArenaNode::default());
            arena.len() - 1
        };

        let mut stack: Vec<usize> = Vec::new();
        let mut current = new_node(&mut arena);

        // Initial parser state with initial node added to the root parent
        stack.push(current);
        current = new_node(&mut arena);
        arena[stack[0]].children.push(current);

        let mut after_closing = false;
        // Positions are UTF-16 code unit offsets, as upstream.
        let mut c = 0usize;
        for ch in s.chars() {
            let position = c;
            c += ch.len_utf16();
            if ch == ',' {
                current = new_node(&mut arena);
                let top = *stack
                    .last()
                    .ok_or_else(|| ParseException::new("Unmatched ')'", position))?;
                arena[top].children.push(current);
            } else if after_closing && ch.is_whitespace() {
                continue;
            } else if ch == ')' {
                current = stack
                    .pop()
                    .ok_or_else(|| ParseException::new("Unmatched ')'", position))?;
                if stack.is_empty() {
                    return Err(ParseException::new("Unmatched ')'", position));
                }
            } else if after_closing {
                return Err(ParseException::new("Invalid character after ')'", position));
            } else if ch == '(' {
                stack.push(current);
                current = new_node(&mut arena);
                let top = stack[stack.len() - 1];
                arena[top].children.push(current);
            } else {
                arena[current]
                    .value
                    .get_or_insert_with(String::new)
                    .push(ch);
            }

            after_closing = ch == ')';
        }

        fn build(arena: &[ArenaNode], index: usize) -> Node {
            Node {
                value: arena[index].value.clone(),
                children: arena[index]
                    .children
                    .iter()
                    .map(|c| build(arena, *c))
                    .collect(),
            }
        }

        // Final state: initial node at the top of the stack
        let top = stack.pop().unwrap_or(0);
        Ok(arena[top]
            .children
            .iter()
            .map(|c| build(&arena, *c))
            .collect())
    }
}
