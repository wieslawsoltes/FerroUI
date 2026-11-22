const QUOTE: u16 = b'"' as u16;
const APOSTROPHE: u16 = b'\'' as u16;
const DOLLAR: u16 = b'$' as u16;
const AT: u16 = b'@' as u16;
const OPEN_BRACE: u16 = b'{' as u16;
const CLOSE_BRACE: u16 = b'}' as u16;
const SLASH: u16 = b'/' as u16;
const BACKSLASH: u16 = b'\\' as u16;
const STAR: u16 = b'*' as u16;
const COLON: u16 = b':' as u16;
const SPACE: u16 = b' ' as u16;

#[derive(Clone, Copy)]
enum StringKind {
    Regular,
    Verbatim,
    Raw { quotes: usize, multiline: bool },
}

#[derive(Clone, Copy)]
struct Literal {
    kind: StringKind,
    dollars: usize,
}

#[derive(Clone, Copy, Default)]
struct Interpolation {
    parentheses: usize,
    brackets: usize,
    braces: usize,
    format: bool,
}

#[derive(Clone, Copy)]
enum Context {
    String(Literal),
    Interpolation(Interpolation),
    Character,
    BlockComment,
}

/// Hides literal and comment syntax from the outer indentation state machine.
/// A stack keeps the enclosing string intact when a hole contains another
/// string or spans lines. The mask retains the original UTF-16 columns.
#[derive(Default)]
pub(super) struct LiteralScanner {
    contexts: Vec<Context>,
}

impl LiteralScanner {
    pub(super) fn in_string(&self) -> bool {
        matches!(self.contexts.first(), Some(Context::String(_)))
    }

    pub(super) fn in_raw_string(&self) -> bool {
        self.contexts.iter().any(|context| {
            matches!(
                context,
                Context::String(Literal {
                    kind: StringKind::Raw { .. },
                    ..
                })
            )
        })
    }

    pub(super) fn in_block_comment(&self) -> bool {
        matches!(self.contexts.last(), Some(Context::BlockComment))
    }

    /// Returns whether the trailing whitespace belongs to an open literal.
    pub(super) fn mask(&mut self, units: &mut [u16]) -> bool {
        self.scan(units);
        let protected_end = self.in_string();
        if matches!(
            self.contexts.last(),
            Some(
                Context::Character
                    | Context::String(Literal {
                        kind: StringKind::Regular
                            | StringKind::Raw {
                                multiline: false,
                                ..
                            },
                        ..
                    })
            )
        ) {
            self.contexts.pop();
        }
        protected_end
    }

    pub(super) fn scan(&mut self, units: &mut [u16]) {
        let mut index = 0;
        while index < units.len() {
            let tail = &units[index..];
            let current = tail[0];
            let next = tail.get(1).copied();
            let (length, visible) = match self.contexts.last().copied() {
                None => self.scan_code(tail),
                Some(Context::String(literal)) => self.scan_string(tail, literal),
                Some(Context::Character) => match current {
                    BACKSLASH => (escape_len(tail), None),
                    APOSTROPHE => self.close_token(1, APOSTROPHE),
                    _ => (1, None),
                },
                Some(Context::BlockComment) if current == STAR && next == Some(SLASH) => {
                    self.close_token(2, SLASH)
                }
                Some(Context::BlockComment) => (1, None),
                Some(Context::Interpolation(hole)) => self.scan_interpolation(tail, hole),
            };
            let end = index + length;
            units[index..end].fill(SPACE);
            if let Some(unit) = visible {
                units[end - 1] = unit;
            }
            index = end;
        }
    }

    fn close_token(&mut self, length: usize, unit: u16) -> (usize, Option<u16>) {
        self.contexts.pop();
        (length, self.contexts.is_empty().then_some(unit))
    }

    fn scan_code(&mut self, tail: &[u16]) -> (usize, Option<u16>) {
        let current = tail[0];
        if tail.starts_with(&[SLASH, SLASH]) || current == b'#' as u16 {
            return (tail.len(), None);
        }
        if tail.starts_with(&[SLASH, STAR]) {
            self.contexts.push(Context::BlockComment);
            return (2, None);
        }
        if current == APOSTROPHE {
            self.contexts.push(Context::Character);
            return (1, None);
        }
        let dollars = if current == DOLLAR {
            run(tail, DOLLAR)
        } else {
            0
        };
        if let Some((length, literal)) = string_start(tail, dollars) {
            self.contexts.push(Context::String(literal));
            return (length, None);
        }
        (dollars.max(1), self.contexts.is_empty().then_some(current))
    }

    fn scan_string(&mut self, tail: &[u16], literal: Literal) -> (usize, Option<u16>) {
        match tail[0] {
            BACKSLASH if matches!(literal.kind, StringKind::Regular) => (escape_len(tail), None),
            QUOTE => match literal.kind {
                StringKind::Verbatim if tail.get(1) == Some(&QUOTE) => (2, None),
                StringKind::Raw { quotes, .. } => {
                    let length = run(tail, QUOTE);
                    if length >= quotes {
                        self.close_token(length, QUOTE)
                    } else {
                        (length, None)
                    }
                }
                _ => self.close_token(1, QUOTE),
            },
            OPEN_BRACE if literal.dollars != 0 => {
                let length = if matches!(literal.kind, StringKind::Raw { .. }) {
                    let length = run(tail, OPEN_BRACE);
                    if length < literal.dollars {
                        return (length, None);
                    }
                    length
                } else if tail.get(1) == Some(&OPEN_BRACE) {
                    return (2, None);
                } else {
                    1
                };
                self.contexts
                    .push(Context::Interpolation(Interpolation::default()));
                (length, None)
            }
            _ => (1, None),
        }
    }

    fn scan_interpolation(
        &mut self,
        tail: &[u16],
        mut hole: Interpolation,
    ) -> (usize, Option<u16>) {
        let Some(Context::String(literal)) = self.contexts.get(self.contexts.len() - 2).copied()
        else {
            unreachable!("interpolations are always started inside a string");
        };
        let current = tail[0];
        if current == CLOSE_BRACE && hole.braces == 0 {
            let length = run(tail, CLOSE_BRACE);
            if length >= literal.dollars {
                self.contexts.pop();
                return (literal.dollars, None);
            }
            return (length, None);
        }
        if hole.format {
            return if current == BACKSLASH && matches!(literal.kind, StringKind::Regular) {
                (escape_len(tail), None)
            } else {
                (1, None)
            };
        }
        match current {
            OPEN_BRACE => hole.braces += 1,
            CLOSE_BRACE => hole.braces -= 1,
            x if x == b'(' as u16 => hole.parentheses += 1,
            x if x == b')' as u16 => hole.parentheses = hole.parentheses.saturating_sub(1),
            x if x == b'[' as u16 => hole.brackets += 1,
            x if x == b']' as u16 => hole.brackets = hole.brackets.saturating_sub(1),
            COLON if tail.get(1) == Some(&COLON) => return (2, None),
            COLON if hole.braces == 0 && hole.parentheses == 0 && hole.brackets == 0 => {
                hole.format = true
            }
            _ => return self.scan_code(tail),
        }
        *self.contexts.last_mut().unwrap() = Context::Interpolation(hole);
        (1, None)
    }
}

fn string_start(tail: &[u16], dollar_count: usize) -> Option<(usize, Literal)> {
    let (prefix, dollars, verbatim) = match tail[0] {
        QUOTE => (0, 0, false),
        AT if tail.starts_with(&[AT, DOLLAR, QUOTE]) => (2, 1, true),
        AT if tail.get(1) == Some(&QUOTE) => (1, 0, true),
        DOLLAR if tail.starts_with(&[DOLLAR, AT, QUOTE]) => (2, 1, true),
        DOLLAR => (dollar_count, dollar_count, false),
        _ => return None,
    };
    if tail.get(prefix) != Some(&QUOTE) {
        return None;
    }
    if verbatim {
        return Some((
            prefix + 1,
            Literal {
                kind: StringKind::Verbatim,
                dollars,
            },
        ));
    }
    let quotes = run(&tail[prefix..], QUOTE);
    if quotes >= 3 {
        let length = prefix + quotes;
        let multiline = tail[length..]
            .iter()
            .all(|unit| char::from_u32(u32::from(*unit)).is_some_and(char::is_whitespace));
        return Some((
            length,
            Literal {
                kind: StringKind::Raw { quotes, multiline },
                dollars,
            },
        ));
    }
    (dollars <= 1).then_some((
        prefix + 1,
        Literal {
            kind: StringKind::Regular,
            dollars,
        },
    ))
}

fn run(tail: &[u16], unit: u16) -> usize {
    tail.iter().take_while(|&&value| value == unit).count()
}

fn escape_len(tail: &[u16]) -> usize {
    tail.len().min(2)
}
