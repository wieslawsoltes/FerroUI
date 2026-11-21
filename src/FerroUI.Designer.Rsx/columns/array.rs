//! Checked array mutations that leave surrounding Rust and separator trivia intact.
use crate::{animation::syntax::span, source_array::ArraySlot, source_edits::line_prefix};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::ExprArray;

pub(super) struct ColumnArray {
    append: ArraySlot,
    members: Vec<Member>,
}

struct Member {
    expression: SourceSpan,
    comma: Option<SourceSpan>,
    /// Include leading comments and any comment before the separator in a move.
    item: SourceSpan,
}

impl ColumnArray {
    pub(super) fn read(array: &ExprArray) -> Self {
        let mut start = array.bracket_token.span.open().byte_range().end;
        let members = array
            .elems
            .pairs()
            .map(|pair| {
                let expression = span(*pair.value());
                let comma = pair.punct().map(|comma| span(*comma));
                let item =
                    SourceSpan::new(start, comma.map_or(expression.end, |comma| comma.start));
                start = comma.map_or(expression.end, |comma| comma.end);
                Member {
                    expression,
                    comma,
                    item,
                }
            })
            .collect();
        Self {
            append: ArraySlot::at(array),
            members,
        }
    }

    pub(super) fn insert(
        &self,
        source: &str,
        index: usize,
        expression: &str,
    ) -> Result<Vec<TextPatch>, String> {
        let count = self.members.len();
        if index == count {
            return self
                .append
                .insert(source, 0, "columns", |_, _| expression.into());
        }
        let member = self
            .members
            .get(index)
            .ok_or("Choose a position in this column list")?;
        let prefix = line_prefix(source, Some(member.expression.start));
        let prefix = if prefix.is_empty() { " " } else { prefix };
        Ok(vec![TextPatch {
            span: SourceSpan::new(member.item.start, member.item.start),
            replacement: format!("{prefix}{expression},"),
        }])
    }

    pub(super) fn move_member(
        &self,
        source: &str,
        from: usize,
        to: usize,
    ) -> Result<Vec<TextPatch>, String> {
        if to >= self.members.len() {
            return Err("Choose a position in this column list".into());
        }
        if from == to {
            return Ok(Vec::new());
        }
        let first = from.min(to);
        let last = from.max(to);
        let slots = &self.members[first..=last];
        let mut order: Vec<_> = slots.iter().collect();
        let moving = order.remove(from - first);
        order.insert(to - first, moving);
        Ok(slots
            .iter()
            .zip(order)
            .map(|(slot, value)| TextPatch {
                span: slot.item,
                replacement: source[value.item.range()].to_owned(),
            })
            .collect())
    }

    pub(super) fn remove(&self, index: usize) -> Vec<TextPatch> {
        let member = &self.members[index];
        let separator = member.comma.or_else(|| {
            index
                .checked_sub(1)
                .and_then(|previous| self.members[previous].comma)
        });
        std::iter::once(member.expression)
            .chain(separator)
            .map(|span| TextPatch {
                span,
                replacement: String::new(),
            })
            .collect()
    }
}
