use crate::document::{DocumentError, SimpleSegment, TextDocument};

pub trait ICompletionData {
    fn text(&self) -> &str;
    fn description(&self) -> &str;
    fn priority(&self) -> f64 {
        0.0
    }
    fn complete(
        &self,
        document: &TextDocument,
        segment: SimpleSegment,
    ) -> Result<(), DocumentError> {
        document.replace(segment.offset, segment.length, self.text())
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompletionItem {
    pub text: String,
    pub insertion_text: Option<String>,
    pub description: String,
    pub priority: f64,
}
impl CompletionItem {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
}
impl ICompletionData for CompletionItem {
    fn text(&self) -> &str {
        &self.text
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn complete(
        &self,
        document: &TextDocument,
        segment: SimpleSegment,
    ) -> Result<(), DocumentError> {
        document.replace(
            segment.offset,
            segment.length,
            self.insertion_text.as_deref().unwrap_or(&self.text),
        )
    }
}

/// Completion filtering and ranking independent of a popup implementation.
/// The list retains item identity across filtering and keyboard navigation.
#[derive(Clone, Debug, Default)]
pub struct CompletionList {
    data: Vec<CompletionItem>,
    visible: Vec<usize>,
    selected: Option<usize>,
    pub is_filtering: bool,
}
impl CompletionList {
    pub fn new(data: Vec<CompletionItem>) -> Self {
        let visible = (0..data.len()).collect();
        let selected = if data.is_empty() { None } else { Some(0) };
        Self {
            data,
            visible,
            selected,
            is_filtering: true,
        }
    }
    pub fn completion_data(&self) -> &[CompletionItem] {
        &self.data
    }
    pub fn current_list(&self) -> impl ExactSizeIterator<Item = &CompletionItem> {
        self.visible.iter().map(|i| &self.data[*i])
    }
    /// Original item indices in display order, for hosts with rich item data.
    pub fn current_indices(&self) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.visible.iter().copied()
    }
    pub fn selected_data_index(&self) -> Option<usize> {
        self.selected.and_then(|index| self.visible.get(index)).copied()
    }
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.selected
            .and_then(|i| self.visible.get(i))
            .map(|i| &self.data[*i])
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }
    pub fn select_index(&mut self, index: usize) {
        self.selected = (!self.visible.is_empty()).then(|| index.min(self.visible.len() - 1));
    }
    pub fn move_selection(&mut self, delta: isize, wrap: bool) {
        if self.visible.is_empty() {
            self.selected = None;
            return;
        }
        let current = self.selected.unwrap_or(0) as isize;
        self.selected = Some(if wrap {
            (current + delta).rem_euclid(self.visible.len() as isize) as usize
        } else {
            (current + delta).clamp(0, self.visible.len() as isize - 1) as usize
        });
    }
    pub fn select_item(&mut self, query: &str) {
        let previous = self.selected.and_then(|i| self.visible.get(i)).copied();
        let mut scored: Vec<_> = self
            .data
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let score = match_quality(&item.text, query, self.is_filtering);
                (score > 0 || !self.is_filtering).then_some((i, score))
            })
            .collect();
        scored.sort_by(|(ai, a), (bi, b)| {
            b.cmp(a)
                .then_with(|| self.data[*bi].priority.total_cmp(&self.data[*ai].priority))
                .then_with(|| ai.cmp(bi))
        });
        self.visible = scored.into_iter().map(|(i, _)| i).collect();
        self.selected = previous.and_then(|p| self.visible.iter().position(|i| *i == p));
        if self.selected.is_none() || !query.is_empty() {
            self.selected = (!self.visible.is_empty()).then_some(0);
        }
    }
}
fn match_quality(text: &str, query: &str, substring: bool) -> u8 {
    if query.is_empty() {
        return 1;
    }
    if text == query {
        return 8;
    }
    if text.starts_with(query) {
        return 7;
    }
    let folded = text.to_lowercase();
    let query_folded = query.to_lowercase();
    if folded == query_folded {
        return 6;
    }
    if folded.starts_with(&query_folded) {
        return 5;
    }
    if !substring {
        return 0;
    }
    let mut initials = String::new();
    let mut word_start = true;
    let characters: Vec<_> = text.chars().collect();
    for (i, ch) in characters.iter().copied().enumerate() {
        let next_lower = characters.get(i + 1).is_some_and(|c| c.is_lowercase());
        let prev_lower = i > 0 && characters[i - 1].is_lowercase();
        if ch.is_alphanumeric() && (word_start || ch.is_uppercase() && (prev_lower || next_lower)) {
            initials.extend(ch.to_lowercase());
        }
        word_start = !ch.is_alphanumeric();
    }
    if initials.starts_with(&query_folded) {
        return 4;
    }
    if text.contains(query) {
        return 3;
    }
    if folded.contains(&query_folded) {
        return 2;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranking_and_camelcase_filtering() {
        let mut list = CompletionList::new(
            ["XmlReader", "XR", "Other", "xmlReader"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        list.select_item("XR");
        assert_eq!(
            list.current_list()
                .map(|i| i.text.as_str())
                .collect::<Vec<_>>(),
            vec!["XR", "XmlReader", "xmlReader"]
        );
        list.move_selection(-1, true);
        assert_eq!(list.selected_item().unwrap().text, "xmlReader");
        list.select_item("missing");
        assert_eq!(list.current_list().len(), 0);
        assert!(list.selected_item().is_none());
    }
}
