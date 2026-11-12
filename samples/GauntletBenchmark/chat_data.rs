//! The chat beside the patchwork, as the benchmark's shared C# file makes it bit for bit: an
//! agent's answers arrive from a data thread as tokens, several a frame, and the screen shows them
//! as they come.
//!
//! Integer arithmetic follows the 32-bit signed arithmetic of the shared C# file. The texts of the
//! turn are ASCII, so their lengths and indices in bytes are the lengths and indices of the C#
//! strings in UTF-16 code units.

use std::sync::LazyLock;

/// One block of a turn: the text of a user, heading, paragraph or code block, the rows of a table
/// (whose first row is its header) or the labels of the buttons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChatBlock {
    User(&'static str),
    Heading(&'static str),
    Paragraph(&'static str),
    Code(&'static str),
    Table(&'static [&'static [&'static str]]),
    Gif,
    Buttons(&'static [&'static str]),
}

/// A token: its number from 1, the chat's last block and the tokens of that block that arrived.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ChatToken {
    pub count: i32,
    pub block: i32,
    pub shown: i32,
}

/// What a stretch of code is, for its color.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodeKind {
    Keyword,
    Text,
    Number,
    Comment,
}

/// A colored stretch of code: where it starts and ends, and its kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CodeSpan {
    pub start: usize,
    pub end: usize,
    pub kind: CodeKind,
}

/// The chat's sizes, colors, turn and per-token arithmetic.
pub struct ChatData;

impl ChatData {
    pub const WIDTH: f32 = 320.0;
    pub const TOKENS_PER_FRAME: i32 = 4;
    pub const TOKEN_CHARS: i32 = 6;
    pub const GIF_HOLD: i32 = 2;
    pub const STATUS_HEIGHT: f32 = 28.0;
    pub const SPINNER_SIZE: f32 = 16.0;
    pub const SPIN_DEGREES: f32 = 12.0;
    pub const SHIMMER_STEP: f32 = 6.0;
    pub const SHIMMER_SPAN: f32 = 120.0;
    pub const GIF_SIZE: f32 = 64.0;
    pub const TEXT: f32 = 12.0;
    pub const HEADING_TEXT: f32 = 14.0;
    pub const CODE_TEXT: f32 = 11.0;
    pub const BACKGROUND_ARGB: u32 = 0xFF14182B;
    pub const USER_ARGB: u32 = 0xFF2B3A67;
    pub const PANEL_ARGB: u32 = 0xFF0B0E1A;
    pub const SHIMMER_DIM_ARGB: u32 = 0xFF64748B;
    pub const SHIMMER_BRIGHT_ARGB: u32 = 0xFFFFFFFF;

    /// The color of code of a kind.
    pub fn code_argb(kind: CodeKind) -> u32 {
        match kind {
            CodeKind::Keyword => 0xFFC084FC,
            CodeKind::Text => 0xFF86EFAC,
            CodeKind::Number => 0xFFFDBA74,
            CodeKind::Comment => 0xFF64748B,
        }
    }

    /// One turn of the chat.
    pub const TURN: &'static [ChatBlock] = &[
        ChatBlock::User("Why does the list stutter when one row changes?"),
        ChatBlock::Heading("What happens in a frame"),
        ChatBlock::Paragraph(concat!(
            "The list stutters because a small change starts a large amount of work. The row ",
            "that changed shows a counter, and the counter gets a new value on every frame. ",
            "The row measures its text again, which is right, because the text is new. The ",
            "row then tells its parent that its size may have changed. The parent does not ",
            "check whether the size really changed. It measures and places every one of its ",
            "children again, the ones on the screen and the ones kept ready beyond its edges. ",
            "Each child measures its own text again, although none of that text changed. A ",
            "frame that should cost one row costs the whole list, and the cost grows with ",
            "the number of rows, not with the size of the change.",
        )),
        ChatBlock::Paragraph(concat!(
            "The profile shows the same thing from the other side. Most of the frame goes to ",
            "text shaping, and nearly all of the shaped strings are equal to the strings of ",
            "the frame before. The cache of shaped text does not help, because the list asks ",
            "for each string at a width that differs by a fraction of a point from the width ",
            "of the last frame, so every lookup misses. Drawing takes a small part of the ",
            "frame and does not change, because the picture is almost the same.",
        )),
        ChatBlock::Heading("Three things to change"),
        ChatBlock::Paragraph(concat!(
            "- Give the row a fixed height. A row whose size cannot change needs no new ",
            "layout of its parent, so the work stops at the row.",
        )),
        ChatBlock::Paragraph(concat!(
            "- Round the width that the list passes to its rows to whole points. Equal ",
            "strings then meet equal widths, and the cache of shaped text answers.",
        )),
        ChatBlock::Paragraph(concat!(
            "- Read the counter in the draw phase where the toolkit allows it. A value ",
            "that changes only what is drawn should not start composition or layout.",
        )),
        ChatBlock::Code(concat!(
            "// Measure only the row that changed.\n",
            "fn relayout(rows: &mut [Row], dirty: usize) -> f32 {\n",
            "    let row = &mut rows[dirty];\n",
            "    if row.width == 320.0 {\n",
            "        return row.height;\n",
            "    }\n",
            "    row.label = \"measured\";\n",
            "    row.height = row.measure(320.0);\n",
            "    row.height\n",
            "}\n",
            "\n",
            "// A row of fixed height ends the pass.\n",
            "fn place(rows: &mut [Row], first: usize) {\n",
            "    let mut top = 0.0;\n",
            "    for row in &mut rows[first..] {\n",
            "        row.top = top;\n",
            "        top = top + row.height;\n",
            "    }\n",
            "}\n",
        )),
        ChatBlock::Paragraph(concat!(
            "With the boundary in place the frame lays out one row and leaves the others ",
            "alone. I ran the list of two thousand rows before and after the change, on the ",
            "same device and at the same scroll position. These are the passes of one frame, ",
            "as the median of three hundred frames:",
        )),
        ChatBlock::Table(&[
            &["Pass", "Before", "After", "Change"],
            &["Compose", "1.92 ms", "0.31 ms", "-84%"],
            &["Measure", "4.70 ms", "0.22 ms", "-95%"],
            &["Place", "1.18 ms", "0.09 ms", "-92%"],
            &["Draw", "2.46 ms", "2.41 ms", "-2%"],
            &["Frame", "10.26 ms", "3.03 ms", "-70%"],
        ]),
        ChatBlock::Gif,
        ChatBlock::Paragraph(concat!(
            "Drawing is now the largest pass, and it did not move. The next step is to keep ",
            "the drawing of the rows that did not change and to draw only the row with the ",
            "counter. That needs a look at how the list records its rows, which I have not ",
            "read yet. The picture above turns while the benchmark runs; it stops when the ",
            "run ends. Tell me whether to apply the three changes, or to show the diff first.",
        )),
        ChatBlock::Buttons(&["Apply", "Show diff", "Dismiss"]),
        ChatBlock::Paragraph("Waiting for your answer."),
    ];

    fn typed_tokens(text: &str) -> i32 {
        (text.len() as i32 + Self::TOKEN_CHARS - 1) / Self::TOKEN_CHARS
    }

    /// The tokens `block` takes to appear whole.
    pub fn tokens(block: &ChatBlock) -> i32 {
        match block {
            ChatBlock::Heading(text) | ChatBlock::Paragraph(text) | ChatBlock::Code(text) => Self::typed_tokens(text),
            ChatBlock::Table(rows) => rows.len() as i32,
            _ => 1,
        }
    }

    /// What the status line says the agent does while `block` is the last one.
    pub fn doing(block: &ChatBlock) -> &'static str {
        match block {
            ChatBlock::User(_) => "Reading",
            ChatBlock::Heading(_) | ChatBlock::Paragraph(_) => "Writing",
            ChatBlock::Code(_) => "Writing code",
            ChatBlock::Table(_) => "Filling the table",
            ChatBlock::Gif => "Drawing",
            ChatBlock::Buttons(_) => "Waiting",
        }
    }

    /// The token at which each block of the turn starts, and the tokens of the whole turn.
    fn starts() -> &'static (Vec<i32>, i32) {
        static STARTS: LazyLock<(Vec<i32>, i32)> = LazyLock::new(|| {
            let mut starts = Vec::with_capacity(ChatData::TURN.len());
            let mut total = 0;
            for block in ChatData::TURN {
                starts.push(total);
                total += ChatData::tokens(block);
            }
            (starts, total)
        });
        &STARTS
    }

    /// The chat after `count` tokens, `count` at least 1.
    pub fn token_after(count: i32) -> ChatToken {
        let (starts, turn_tokens) = Self::starts();
        let index = count.max(1) - 1;
        let turn = index / turn_tokens;
        let within = index % turn_tokens;
        let mut block = Self::TURN.len() - 1;
        while starts[block] > within {
            block -= 1;
        }
        ChatToken {
            count: count.max(1),
            block: turn * Self::TURN.len() as i32 + block as i32,
            shown: within - starts[block] + 1,
        }
    }

    /// The turn of block `block`.
    pub fn turn_of(block: i32) -> i32 {
        block / Self::TURN.len() as i32
    }

    /// The characters of `text` that `shown` tokens show; `i32::MAX` tokens show all of it.
    pub fn shown_chars(text: &str, shown: i32) -> usize {
        let length = text.len() as i32;
        (if shown >= length { length } else { (shown * Self::TOKEN_CHARS).min(length) }) as usize
    }

    /// The status line's text after token `token`.
    pub fn status(token: ChatToken) -> String {
        let block = &Self::TURN[(token.block % Self::TURN.len() as i32) as usize];
        format!("{}\u{2026} ({:06} tokens)", Self::doing(block), token.count % 1_000_000)
    }

    /// The mark of turn `turn` before a paragraph or a code block.
    pub fn turn_label(turn: i32) -> String {
        format!("#{:05}", turn % 100_000)
    }

    const KEYWORDS: [&'static str; 10] = ["fn", "let", "mut", "if", "for", "in", "return", "usize", "f32", "Row"];

    fn is_alpha(c: u8) -> bool {
        c.is_ascii_lowercase() || c.is_ascii_uppercase()
    }

    fn is_digit(c: u8) -> bool {
        c.is_ascii_digit()
    }

    /// The colored stretches of `code`, in order.
    pub fn code_spans(code: &str) -> Vec<CodeSpan> {
        let bytes = code.as_bytes();
        let mut spans = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            let start = at;
            let c = bytes[at];
            if c == b'/' && at + 1 < bytes.len() && bytes[at + 1] == b'/' {
                at = code[at..].find('\n').map_or(bytes.len(), |end| at + end);
                spans.push(CodeSpan { start, end: at, kind: CodeKind::Comment });
            } else if c == b'"' {
                at = code[at + 1..].find('"').map_or(bytes.len(), |end| at + 1 + end + 1);
                spans.push(CodeSpan { start, end: at, kind: CodeKind::Text });
            } else if Self::is_alpha(c) || c == b'_' {
                while at < bytes.len()
                    && (Self::is_alpha(bytes[at]) || Self::is_digit(bytes[at]) || bytes[at] == b'_')
                {
                    at += 1;
                }
                if Self::KEYWORDS.contains(&&code[start..at]) {
                    spans.push(CodeSpan { start, end: at, kind: CodeKind::Keyword });
                }
            } else if Self::is_digit(c) {
                while at < bytes.len() && (Self::is_digit(bytes[at]) || bytes[at] == b'.') {
                    at += 1;
                }
                spans.push(CodeSpan { start, end: at, kind: CodeKind::Number });
            } else {
                at += 1;
            }
        }
        spans
    }

    /// The frame of the animated picture that frame `frame` of the screen shows, of `frames`
    /// frames.
    pub fn gif_frame(frame: i32, frames: i32) -> i32 {
        frame / Self::GIF_HOLD % frames.max(1)
    }
}
