use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::media::text_formatting::{ShapedTextRun, TextRun};
use crate::media::FlowDirection;

/// Caches shaped text runs and bidi processing results to avoid redundant
/// shaping when only the paragraph width constraint changes (e.g., between
/// Measure and Arrange).
///
/// Uses an inline single-entry store for the common case of a single
/// paragraph, and only promotes to a dictionary when multiple entries are
/// added.
///
/// This API is in preview and subject to change without deprecation.
#[derive(Default)]
pub struct TextRunCache {
    state: RefCell<State>,
}

#[derive(Default)]
struct State {
    // Single-entry inline store (avoids a map allocation for the common single-paragraph case).
    single: Option<(i32, CachedShapingResult)>,
    // Multi-entry store (only allocated when 2+ distinct keys are added).
    entries: Option<HashMap<i32, CachedShapingResult>>,
}

impl TextRunCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Invalidates all cached entries and disposes their shaped buffers.
    pub fn invalidate(&self) {
        let mut state = self.state.borrow_mut();

        if let Some((_, value)) = state.single.take() {
            Self::dispose_cached_runs(&value);
            return;
        }

        let Some(entries) = &mut state.entries else {
            return;
        };

        for entry in entries.values() {
            Self::dispose_cached_runs(entry);
        }

        entries.clear();
    }

    /// Invalidates all cached entries at or after the specified text source index.
    pub fn invalidate_from(&self, text_source_index: i32) {
        let mut state = self.state.borrow_mut();

        if let Some((key, value)) = &state.single {
            if *key >= text_source_index {
                Self::dispose_cached_runs(value);
                state.single = None;
            }

            return;
        }

        let Some(entries) = &mut state.entries else {
            return;
        };

        entries.retain(|key, result| {
            if *key >= text_source_index {
                Self::dispose_cached_runs(result);
                false
            } else {
                true
            }
        });
    }

    /// Tries to retrieve cached shaped runs for the given text source index.
    pub(crate) fn try_get_shaped_runs(&self, first_text_source_index: i32) -> Option<CachedShapingResult> {
        let state = self.state.borrow();

        if let Some((key, value)) = &state.single {
            if *key == first_text_source_index {
                return Some(value.clone());
            }
        }

        state.entries.as_ref().and_then(|entries| entries.get(&first_text_source_index).cloned())
    }

    /// Adds shaped runs to the cache for the given text source index. The cache takes
    /// its own reference to each [`ShapedTextRun`]; the caller retains its
    /// original references unchanged.
    pub(crate) fn add(&self, first_text_source_index: i32, result: CachedShapingResult) {
        Self::add_ref_shaped_runs(&result.shaped_runs);

        let mut state = self.state.borrow_mut();

        if let Some(entries) = &mut state.entries {
            if let Some(existing) = entries.insert(first_text_source_index, result) {
                Self::dispose_cached_runs(&existing);
            }

            return;
        }

        match state.single.take() {
            None => {
                state.single = Some((first_text_source_index, result));
            }
            Some((single_key, single_value)) if single_key == first_text_source_index => {
                Self::dispose_cached_runs(&single_value);
                state.single = Some((first_text_source_index, result));
            }
            Some((single_key, single_value)) => {
                // Second distinct key: promote to dictionary.
                let mut entries = HashMap::new();
                entries.insert(single_key, single_value);
                entries.insert(first_text_source_index, result);
                state.entries = Some(entries);
            }
        }
    }

    fn add_ref_shaped_runs(runs: &[Rc<dyn TextRun>]) {
        for run in runs {
            if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                shaped.add_reference();
            }
        }
    }

    /// Releases the cache.
    pub fn dispose(&self) {
        self.invalidate();
        self.state.borrow_mut().entries = None;
    }

    fn dispose_cached_runs(result: &CachedShapingResult) {
        for run in result.shaped_runs.iter() {
            if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                shaped.dispose();
            }
        }
    }

    /// Whether the multi-entry store has been allocated.
    #[cfg(test)]
    pub(crate) fn has_entries_map(&self) -> bool {
        self.state.borrow().entries.is_some()
    }
}

/// Stores the result of text shaping for a paragraph segment starting at a
/// given text source index.
#[derive(Clone)]
pub(crate) struct CachedShapingResult {
    /// The shaped text runs (output of the shaping step).
    pub shaped_runs: Rc<[Rc<dyn TextRun>]>,
    /// The resolved flow direction for the paragraph.
    pub resolved_flow_direction: FlowDirection,
    /// The end of line marker, if any.
    pub text_end_of_line: Option<Rc<dyn TextRun>>,
    /// The total text source length consumed.
    pub text_source_length: i32,
}

impl CachedShapingResult {
    pub fn new(
        shaped_runs: Rc<[Rc<dyn TextRun>]>,
        resolved_flow_direction: FlowDirection,
        text_end_of_line: Option<Rc<dyn TextRun>>,
        text_source_length: i32,
    ) -> Self {
        Self { shaped_runs, resolved_flow_direction, text_end_of_line, text_source_length }
    }
}
