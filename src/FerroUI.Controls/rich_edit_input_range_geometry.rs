//! Visible-story geometry for platform input without changing editing state.

use super::RichEditBoxTextInputMethodClient;
use crate::rich_text::{EditorSelection, RichTextEditor};
use crate::rich_text_layout::{RichTextLayout, RichTextLine};
use crate::{RichEditBox, RichTextBlock, RichTextBlockImplExt};
use ferroui_base::input::text_input::{
    TextInputMethodClient, TextInputMethodError, TextInputMethodRangeGeometry,
    TextInputMethodTextBounds, TextSelection,
};
use ferroui_base::media::CharacterHit;
use ferroui_base::{Matrix, Point, Rect, Ref, WeakRef};
use std::collections::HashSet;
use std::ops::Range;
use std::rc::{Rc, Weak};

struct PageSnapshot {
    view: WeakRef<RichTextBlock>,
    layout: Weak<RichTextLayout>,
    generation: u64,
    next: Option<WeakRef<RichTextBlock>>,
    origin: Point,
    bounds: Rect,
    to_owner: Matrix,
}

struct GeometrySnapshot {
    owner: WeakRef<RichEditBox>,
    editor: Weak<RichTextEditor>,
    client: Weak<dyn TextInputMethodClient>,
    mutation: u64,
    selection: EditorSelection,
    composing: bool,
    pages: Vec<PageSnapshot>,
}

impl GeometrySnapshot {
    fn owner_is_current(&self, owner: &RichEditBox, editor: &Rc<RichTextEditor>) -> bool {
        Weak::ptr_eq(&self.editor, &Rc::downgrade(editor))
            && Rc::ptr_eq(editor, &owner.editor())
            && editor.document().mutation_version() == self.mutation
            && editor.selection() == self.selection
            && editor.is_composing() == self.composing
            && self
                .client
                .upgrade()
                .is_some_and(|client| Rc::ptr_eq(&client, &owner.text_input_method_client()))
            && owner.is_attached_to_visual_tree()
            && owner.is_effectively_visible()
    }

    fn is_current(&self) -> bool {
        let Some(owner) = self.owner.upgrade() else {
            return false;
        };
        let Some(editor) = self.editor.upgrade() else {
            return false;
        };
        if !self.owner_is_current(&owner, &editor)
            || self.pages.first().and_then(|page| page.view.upgrade())
                != Some(owner.text_presenter().upcast())
        {
            return false;
        }
        let document = editor.document();
        for page in &self.pages {
            let Some(view) = page.view.upgrade() else {
                return false;
            };
            let next = view.overflow_content_target().map(Ref::upcast);
            let next_matches = match &page.next {
                Some(expected) => expected
                    .upgrade()
                    .is_some_and(|expected| Some(expected) == next),
                None => next.is_none(),
            };
            if !next_matches
                || view.document() != document
                || !view.is_attached_to_visual_tree()
                || !view.is_effectively_visible()
                || view.visual_root() != owner.visual_root()
                || view.bounds() != page.bounds
                || view.text_origin() != page.origin
                || view.transform_to_visual(&owner) != Some(page.to_owner)
            {
                return false;
            }
        }
        // Text-origin and transform services may call application code. End
        // with only model identity and cached-layout checks, never reformatting
        // a view while trying to validate the preceding geometry.
        self.owner_is_current(&owner, &editor)
            && self.pages.iter().all(|page| {
                page.view.upgrade().is_some_and(|view| {
                    view.document() == document
                        && view.page_layout_generation() == page.generation
                        && view.cached_page_layout().is_some_and(|layout| {
                            Weak::ptr_eq(&page.layout, &Rc::downgrade(&layout))
                        })
                })
            })
    }
}

fn line_caret(entry: &RichTextLine, position: usize, width: f64) -> Rect {
    let local = entry.display_position(position - entry.paragraph_offset);
    let x = entry
        .line
        .get_distance_from_character_hit(CharacterHit::new(local as i32));
    Rect::new(entry.origin.x + x, entry.allocation_y, width, entry.advance)
}

/// A verified source insertion position, not the layout's clamped fallback.
fn caret_for_range(layout: &RichTextLayout, position: usize) -> Option<Rect> {
    let first = layout.lines.first()?;
    if position < first.start()
        || position > layout.next.position
        || position == layout.next.position && layout.has_overflow
    {
        return None;
    }
    let entry = layout
        .lines
        .iter()
        .find(|entry| {
            entry
                .visible_ranges()
                .iter()
                .any(|range| range.contains(&position))
        })
        .or_else(|| {
            layout.lines.iter().find(|entry| {
                entry
                    .visible_ranges()
                    .iter()
                    .any(|range| position == range.end)
            })
        })?;
    Some(line_caret(entry, position, 1.0))
}

fn page_range_bounds(
    layout: &RichTextLayout,
    requested: &Range<usize>,
    covered: &mut Vec<Range<usize>>,
) -> Result<Vec<Rect>, TextInputMethodError> {
    let mut result = Vec::new();
    for entry in &layout.lines {
        let visible = entry.visible_ranges();
        for range in &visible {
            let start = requested.start.max(range.start);
            let end = requested.end.min(range.end);
            if start >= end {
                continue;
            }
            let local_start = entry.display_position(start - entry.paragraph_offset);
            let local_end = entry.display_position(end - entry.paragraph_offset);
            let bounds = entry
                .line
                .get_text_bounds(local_start as i32, (local_end - local_start) as i32);
            if bounds.is_empty() {
                return Err(TextInputMethodError::Unavailable);
            }
            result.extend(
                bounds
                    .into_iter()
                    .map(|bounds| entry.range_bounds(bounds.rectangle())),
            );
            covered.push(start..end);
        }
        // Paragraph objects contribute one real UTF-16 separator. Its text
        // geometry is a zero-width line-end box, not the wider selection mark.
        let end = entry.paragraph_offset + entry.paragraph_length;
        if requested.contains(&end)
            && end < layout.source.length
            && entry.end() == end
            && visible.iter().any(|range| range.end == end)
        {
            result.push(line_caret(entry, end, 0.0));
            covered.push(end..end + 1);
        }
    }
    Ok(result)
}

fn fully_covered(requested: &Range<usize>, covered: &mut [Range<usize>]) -> bool {
    covered.sort_unstable_by_key(|range| range.start);
    let mut position = requested.start;
    for range in covered {
        if range.start > position {
            return false;
        }
        position = position.max(range.end);
        if position >= requested.end {
            return true;
        }
    }
    false
}

impl RichEditBoxTextInputMethodClient {
    pub(super) fn query_text_range_geometry(
        &self,
        range: TextSelection,
    ) -> Result<TextInputMethodRangeGeometry, TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        let editor = owner.editor();
        let document = editor.document();
        if range.start < 0 || range.end < range.start || range.end as usize > document.text_length()
        {
            return Err(TextInputMethodError::InvalidRange);
        }
        let requested = range.start as usize..range.end as usize;
        let mut snapshot = GeometrySnapshot {
            owner: owner.downgrade(),
            editor: Rc::downgrade(&editor),
            client: Rc::downgrade(&owner.text_input_method_client()),
            mutation: document.mutation_version(),
            selection: editor.selection(),
            composing: editor.is_composing(),
            pages: Vec::new(),
        };
        let first: Ref<RichTextBlock> = owner.text_presenter().upcast();
        if !snapshot.owner_is_current(&owner, &editor)
            || !owner.text_presenter().has_measured_layout()
            || first.previous_arrange().is_none()
        {
            return Err(TextInputMethodError::Unavailable);
        }
        let mut current = Some(first);
        let mut visited = HashSet::new();
        let mut source = None;
        let mut covered = Vec::new();
        let mut fragments = Vec::new();
        while let Some(view) = current {
            if !visited.insert(&*view as *const RichTextBlock as usize)
                || view.document() != document
                || !view.is_attached_to_visual_tree()
                || !view.is_effectively_visible()
                || view.visual_root() != owner.visual_root()
                || view.previous_arrange().is_none()
            {
                return Err(TextInputMethodError::Unavailable);
            }
            let generation = view.page_layout_generation();
            let layout = view.page_layout();
            if generation != view.page_layout_generation()
                || !snapshot.owner_is_current(&owner, &editor)
                || view.document() != document
                || layout.source.length != document.text_length()
                || source
                    .as_ref()
                    .is_some_and(|source| !Rc::ptr_eq(source, &layout.source))
            {
                return Err(TextInputMethodError::StateChanged);
            }
            source.get_or_insert_with(|| layout.source.clone());
            let origin = view.text_origin();
            let to_owner = view
                .transform_to_visual(&owner)
                .ok_or(TextInputMethodError::Unavailable)?;
            let next = view
                .overflow_content_target()
                .map(Ref::upcast::<RichTextBlock>);
            snapshot.pages.push(PageSnapshot {
                view: view.downgrade(),
                layout: Rc::downgrade(&layout),
                generation,
                next: next.as_ref().map(Ref::downgrade),
                origin,
                bounds: view.bounds(),
                to_owner,
            });
            let bounds = if requested.is_empty() {
                caret_for_range(&layout, requested.start)
                    .into_iter()
                    .collect()
            } else {
                page_range_bounds(&layout, &requested, &mut covered)?
            };
            let transform = Matrix::create_translation(origin.x, origin.y) * to_owner;
            fragments.extend(
                bounds.into_iter().map(|rectangle| {
                    TextInputMethodTextBounds::with_transform(rectangle, transform)
                }),
            );
            if requested.is_empty() && !fragments.is_empty()
                || !requested.is_empty() && fully_covered(&requested, &mut covered)
            {
                break;
            }
            current = next;
        }
        if !snapshot.is_current() {
            return Err(TextInputMethodError::StateChanged);
        }
        if fragments.is_empty() || !requested.is_empty() && !fully_covered(&requested, &mut covered)
        {
            return Err(TextInputMethodError::Unavailable);
        }
        let geometry =
            TextInputMethodRangeGeometry::new(range, fragments, move || snapshot.is_current())?;
        if !geometry.is_current() {
            return Err(TextInputMethodError::StateChanged);
        }
        Ok(geometry)
    }
}
