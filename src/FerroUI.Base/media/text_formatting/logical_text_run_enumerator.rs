use std::cell::Ref;
use std::rc::Rc;

use crate::media::text_formatting::indexed_text_run::IndexedTextRun;
use crate::media::text_formatting::text_line_impl::TextLineImpl;
use crate::media::text_formatting::{TextLine, TextRun};

/// Walks the runs of a [`TextLine`] in **logical** (source-text) order. This
/// is the order that splits and length-based offsets are defined in, and is
/// what every `TextCollapsingProperties::collapse` implementation needs to
/// see — unlike [`TextLine::text_runs`], which exposes the post-BiDi *visual*
/// ordering used for rendering.
///
/// When the line has been finalized (the normal case after
/// `TextLineImpl::finalize_line`), the enumerator iterates over the indexed
/// text runs — a level-resolved table that maps each run back to its original
/// logical position. If the line hasn't been finalized (or the line is not a
/// `TextLineImpl`), it falls back to the raw [`TextLine::text_runs`] list.
pub(crate) struct LogicalTextRunEnumerator<'a> {
    text_runs: Option<Ref<'a, [Rc<dyn TextRun>]>>,
    indexed_text_runs: Option<Rc<[IndexedTextRun]>>,
    step: i32,
    end: i32,
    index: i32,
    count: i32,
}

impl<'a> LogicalTextRunEnumerator<'a> {
    pub fn new(line: &'a dyn TextLine) -> Self {
        Self::with_direction(line, false)
    }

    pub fn with_direction(line: &'a dyn TextLine, backward: bool) -> Self {
        let indexed_text_runs =
            line.as_any().downcast_ref::<TextLineImpl>().and_then(|line_impl| line_impl.indexed_text_runs());

        let mut text_runs = None;
        let mut count = 0i32;

        match indexed_text_runs {
            Some(ref indexed) if !indexed.is_empty() => {
                count = indexed.len() as i32;
            }
            _ => {
                let runs = line.text_runs();

                if !runs.is_empty() {
                    count = runs.len() as i32;
                    text_runs = Some(runs);
                }
            }
        }

        let indexed_text_runs = if text_runs.is_none() && count > 0 { indexed_text_runs } else { None };

        let (step, end, index) = if backward { (-1, -1, count) } else { (1, count, -1) };

        Self { text_runs, indexed_text_runs, step, end, index, count }
    }

    /// The number of runs.
    pub fn count(&self) -> i32 {
        self.count
    }

    pub fn move_next(&mut self) -> Option<Rc<dyn TextRun>> {
        self.index += self.step;

        if self.index == self.end {
            return None;
        }

        if let Some(indexed_text_runs) = &self.indexed_text_runs {
            return indexed_text_runs[self.index as usize].text_run.borrow().clone();
        }

        if let Some(text_runs) = &self.text_runs {
            return Some(text_runs[self.index as usize].clone());
        }

        None
    }
}
