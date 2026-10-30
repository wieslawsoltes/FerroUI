//! Shared field identity and its visible semantic projection. The instruction
//! story is a separately typed service; these ranges remain document UTF-16.
use super::document_snapshot::{slice_inlines, utf16_to_byte};
use super::{
    CharacterFormat, DocumentError, DocumentSnapshot, HyperlinkBehavior, InlineSnapshot,
    InlineSnapshotKind, UnderlineStyle,
};
use std::collections::{HashMap, HashSet};
use std::ops::Range;

/// One link instruction and its visible result. A result can include structural
/// paragraph separators without adding an invisible interactive control there.
/// IDs are local to a document image; insertion remaps a copied image once per
/// field, while restore/history preserve its identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentLinkField {
    pub id: u64,
    pub range: Range<usize>,
    /// Unquoted navigation target, independent of instruction syntax.
    pub target: String,
    pub target_name: Option<String>,
    /// The optional instruction sentinel is not part of NavigateUri.
    pub has_sentinel: bool,
}

#[derive(Clone)]
struct Segment<'a> {
    range: Range<usize>,
    id: u64,
    target: Option<&'a str>,
    target_name: Option<&'a str>,
    template: &'a InlineSnapshot,
}

impl DocumentSnapshot {
    pub(crate) fn structural_paragraph_breaks(&self) -> Vec<usize> {
        let mut offset = 0;
        let mut result = Vec::with_capacity(self.paragraphs.len().saturating_sub(1));
        for (index, paragraph) in self.paragraphs.iter().enumerate() {
            offset += paragraph.text_length();
            if index + 1 < self.paragraphs.len() {
                result.push(offset);
                offset += 1;
            }
        }
        result
    }

    pub(crate) fn field_result_preserves_inline_boundaries(&self, range: Range<usize>) -> bool {
        fn visit(inlines: &[InlineSnapshot], offset: &mut usize, range: &Range<usize>) -> bool {
            for inline in inlines {
                let start = *offset;
                let end = start + inline.text_length();
                if matches!(inline.kind, InlineSnapshotKind::LineBreak)
                    && ((start < range.start && range.start < end)
                        || (start < range.end && range.end < end))
                {
                    return false;
                }
                if let Some(children) = inline.children() {
                    if !visit(children, offset, range) {
                        return false;
                    }
                }
                *offset = end;
            }
            true
        }
        let mut offset = 0;
        for paragraph in &self.paragraphs {
            if !visit(&paragraph.inlines, &mut offset, &range) {
                return false;
            }
            offset += 1;
        }
        true
    }

    pub(crate) fn prune_orphan_field_projections(&mut self) {
        let ids: HashSet<_> = self.link_fields.iter().map(|field| field.id).collect();
        fn visit(inlines: &mut [InlineSnapshot], ids: &HashSet<u64>) {
            for inline in inlines {
                if let InlineSnapshotKind::Hyperlink {
                    field_id: Some(id),
                    inlines,
                    ..
                } = &mut inline.kind
                {
                    if !ids.contains(id) {
                        inline.kind = InlineSnapshotKind::Span(std::mem::take(inlines));
                    }
                }
                if let Some(children) = inline.children_mut() {
                    visit(children, ids);
                }
            }
        }
        for paragraph in &mut self.paragraphs {
            visit(&mut paragraph.inlines, &ids);
        }
    }

    pub(crate) fn validate_link_fields(&self) -> Result<(), DocumentError> {
        let text = self.text();
        let length = text.encode_utf16().count();
        let mut ids = HashSet::new();
        let mut ordered: Vec<_> = self.link_fields.iter().collect();
        ordered.sort_by_key(|field| field.range.start);
        let mut end = 0;
        if self.next_link_field_id == 0 {
            return invalid("The field identity watermark must be nonzero.");
        }
        for field in &ordered {
            if field.id == 0 || field.id >= self.next_link_field_id || !ids.insert(field.id) {
                return invalid(
                    "Field identities must be distinct, nonzero and below their watermark.",
                );
            }
            if field.range.start >= field.range.end
                || field.range.start < end
                || field.range.end > length
            {
                return invalid("Field results must be nonempty, disjoint document ranges.");
            }
            if utf16_to_byte(&text, field.range.start).is_none()
                || utf16_to_byte(&text, field.range.end).is_none()
            {
                return invalid("A field result cannot split a UTF-16 surrogate pair.");
            }
            if field.target.is_empty() || field.target.contains(['\0', '"']) {
                return invalid(
                    "A field target must be nonempty and cannot contain NUL or a quotation mark.",
                );
            }
            end = field.range.end;
        }
        let segments = self.link_segments();
        let fields_by_id: HashMap<_, _> = self
            .link_fields
            .iter()
            .map(|field| (field.id, field))
            .collect();
        let mut segments_by_id: HashMap<u64, Vec<&Segment<'_>>> = HashMap::new();
        for segment in &segments {
            segments_by_id.entry(segment.id).or_default().push(segment);
            let Some(field) = fields_by_id.get(&segment.id) else {
                return invalid("A projected hyperlink refers to an absent field identity.");
            };
            if segment.range.start < field.range.start
                || segment.range.end > field.range.end
                || segment.target != Some(field.target.as_str())
                || segment.target_name != field.target_name.as_deref()
            {
                return invalid(
                    "A projected hyperlink disagrees with its field range or navigation target.",
                );
            }
        }
        let breaks = self.structural_paragraph_breaks();
        for field in &self.link_fields {
            let mut cursor = field.range.start;
            for segment in segments_by_id
                .get(&field.id)
                .into_iter()
                .flat_map(|segments| segments.iter())
                .filter(|segment| !segment.range.is_empty())
            {
                if segment.range.start < cursor {
                    return invalid("Projected field segments cannot overlap or nest.");
                }
                if !only_breaks(cursor..segment.range.start, &breaks) {
                    return invalid(
                        "Visible field text must have a matching semantic hyperlink projection.",
                    );
                }
                cursor = segment.range.end;
            }
            if !only_breaks(cursor..field.range.end, &breaks) {
                return invalid(
                    "Visible field text must have a matching semantic hyperlink projection.",
                );
            }
        }
        Ok(())
    }

    fn link_segments(&self) -> Vec<Segment<'_>> {
        fn visit<'a>(
            inlines: &'a [InlineSnapshot],
            offset: &mut usize,
            result: &mut Vec<Segment<'a>>,
        ) {
            for inline in inlines {
                let start = *offset;
                let end = start + inline.text_length();
                if let InlineSnapshotKind::Hyperlink {
                    field_id: Some(id),
                    navigate_uri,
                    target_name,
                    ..
                } = &inline.kind
                {
                    result.push(Segment {
                        range: start..end,
                        id: *id,
                        target: navigate_uri.as_deref(),
                        target_name: target_name.as_deref(),
                        template: inline,
                    });
                }
                if let Some(children) = inline.children() {
                    visit(children, offset, result);
                }
                *offset = end;
            }
        }
        let mut result = Vec::new();
        let mut offset = 0;
        for paragraph in &self.paragraphs {
            visit(&paragraph.inlines, &mut offset, &mut result);
            offset += 1;
        }
        result.sort_by_key(|segment| segment.range.start);
        result
    }

    /// Normalizes legacy semantic hyperlinks for the projected service only.
    /// Ordinary snapshot/restore APIs keep their existing semantic structure.
    /// This pure image is not installed until an actual field mutation commits.
    pub(crate) fn with_promoted_links(&self, allocation_floor: u64) -> Result<Self, DocumentError> {
        self.validate()?;
        let mut result = self.clone();
        let mut next = result.next_link_field_id.max(allocation_floor);
        fn promote(
            inlines: &mut [InlineSnapshot],
            offset: &mut usize,
            active: bool,
            fields: &mut Vec<DocumentLinkField>,
            next: &mut u64,
        ) -> Result<(), DocumentError> {
            for inline in inlines {
                let start = *offset;
                let end = start + inline.text_length();
                let mut is_field = false;
                if let InlineSnapshotKind::Hyperlink {
                    field_id,
                    navigate_uri,
                    target_name,
                    ..
                } = &mut inline.kind
                {
                    is_field = field_id.is_some();
                    if field_id.is_none() && start < end {
                        if let Some(target) = navigate_uri {
                            if active {
                                return invalid("Nested semantic hyperlinks have no supported field projection.");
                            }
                            if target.is_empty() || target.contains(['\0', '"']) {
                                return invalid(
                                    "This semantic hyperlink has no supported instruction target.",
                                );
                            }
                            let id = allocate(next)?;
                            *field_id = Some(id);
                            fields.push(DocumentLinkField {
                                id,
                                range: start..end,
                                target: target.clone(),
                                target_name: target_name.clone(),
                                has_sentinel: false,
                            });
                            is_field = true;
                        }
                    }
                }
                if let Some(children) = inline.children_mut() {
                    promote(children, offset, active || is_field, fields, next)?;
                }
                *offset = end;
            }
            Ok(())
        }
        let mut offset = 0;
        for paragraph in &mut result.paragraphs {
            promote(
                &mut paragraph.inlines,
                &mut offset,
                false,
                &mut result.link_fields,
                &mut next,
            )?;
            offset += 1;
        }
        result.next_link_field_id = next;
        result.link_fields.sort_by_key(|field| field.range.start);
        result.validate()?;
        Ok(result)
    }

    /// Applies a single validated instruction edit and the matching semantic
    /// projection. Existing segments remain intact on target updates; append
    /// creates only the extra result segments and shares their behavior lists.
    pub(crate) fn set_link_field(
        &mut self,
        id: u64,
        after: Option<DocumentLinkField>,
        next: u64,
    ) -> Result<(), DocumentError> {
        let before = self
            .link_fields
            .iter()
            .find(|field| field.id == id)
            .cloned();
        let template = self
            .link_segments()
            .into_iter()
            .find(|segment| segment.id == id)
            .map(|segment| (*segment.template).clone());
        match (&before, &after) {
            (None, Some(field)) => self.wrap_link_result(field.range.clone(), field, None)?,
            (Some(before), Some(field)) => {
                if field.range.start != before.range.start || field.range.end < before.range.end {
                    return invalid(
                        "An instruction edit cannot shrink or move its visible result.",
                    );
                }
                update_projection(&mut self.paragraphs, id, Some(field));
                if field.range.end > before.range.end {
                    self.wrap_link_result(
                        before.range.end..field.range.end,
                        field,
                        template.as_ref(),
                    )?;
                }
            }
            (Some(_), None) => update_projection(&mut self.paragraphs, id, None),
            (None, None) => return invalid("The field to remove does not exist."),
        }
        self.link_fields.retain(|field| field.id != id);
        if let Some(field) = after {
            self.link_fields.push(field);
        }
        self.link_fields.sort_by_key(|field| field.range.start);
        self.next_link_field_id = next;
        self.validate()
    }

    /// Adds a complete field from a codec or explicit shared model consumer.
    /// Nonempty text is wrapped; covered paragraph marks need no fake inline.
    pub(crate) fn add_link_field(&mut self, field: DocumentLinkField) -> Result<(), DocumentError> {
        self.wrap_link_result(field.range.clone(), &field, None)?;
        self.next_link_field_id = self.next_link_field_id.max(field.id.checked_add(1).ok_or(
            DocumentError::InvalidLinkField("Field identities are exhausted."),
        )?);
        self.link_fields.push(field);
        self.link_fields.sort_by_key(|field| field.range.start);
        self.validate()
    }

    fn wrap_link_result(
        &mut self,
        range: Range<usize>,
        field: &DocumentLinkField,
        template: Option<&InlineSnapshot>,
    ) -> Result<(), DocumentError> {
        let text = self.text();
        if range.start > range.end
            || range.end > self.text_length()
            || utf16_to_byte(&text, range.start).is_none()
            || utf16_to_byte(&text, range.end).is_none()
        {
            return invalid("A projected result must be a scalar-aligned document range.");
        }
        let mut offset = 0;
        for paragraph in &mut self.paragraphs {
            let length = paragraph.text_length();
            let start = range.start.max(offset);
            let end = range.end.min(offset + length);
            if start < end {
                let local = start - offset..end - offset;
                let middle = slice_inlines(&paragraph.inlines, local.clone());
                let mut wrapper = template.cloned().unwrap_or_else(|| InlineSnapshot {
                    format: CharacterFormat::default(),
                    kind: InlineSnapshotKind::Hyperlink {
                        inlines: Vec::new(),
                        field_id: Some(field.id),
                        behavior: HyperlinkBehavior::default(),
                        navigate_uri: Some(field.target.clone()),
                        target_name: field.target_name.clone(),
                        is_enabled: true,
                        is_tab_stop: true,
                        tab_index: 0,
                        is_visited: false,
                        underline_style: UnderlineStyle::Single,
                    },
                });
                if let InlineSnapshotKind::Hyperlink {
                    inlines,
                    field_id,
                    navigate_uri,
                    target_name,
                    ..
                } = &mut wrapper.kind
                {
                    *inlines = middle;
                    *field_id = Some(field.id);
                    *navigate_uri = Some(field.target.clone());
                    *target_name = field.target_name.clone();
                }
                let mut inlines = slice_inlines(&paragraph.inlines, 0..local.start);
                inlines.push(wrapper);
                inlines.extend(slice_inlines(&paragraph.inlines, local.end..length));
                paragraph.inlines = inlines;
            }
            offset += length + 1;
        }
        Ok(())
    }

    /// Reconciles direct low-level tree edits. Ordinary text edits map the
    /// existing span; property edits take their consistent live target. If an
    /// application dismantles a field into incompatible semantic pieces, that
    /// field identity is retired while its visible hyperlinks remain intact.
    /// Other fields keep their identities and exact instruction metadata.
    pub(crate) fn reconcile_external_link_fields(
        &mut self,
        change: Option<&super::DocumentTextChange>,
    ) {
        let segments = self.link_segments();
        let mut segments_by_id: HashMap<u64, Vec<&Segment<'_>>> = HashMap::new();
        for segment in &segments {
            if !segment.range.is_empty() {
                segments_by_id.entry(segment.id).or_default().push(segment);
            }
        }
        let breaks = self.structural_paragraph_breaks();
        let length = self.text_length();
        let mut retained = Vec::new();
        for original in &self.link_fields {
            let mut field = original.clone();
            if let Some(change) = change {
                field.range = change
                    .map_position(super::TextPosition::new(
                        field.range.start,
                        super::TextGravity::Backward,
                    ))
                    .offset
                    ..change
                        .map_position(super::TextPosition::new(
                            field.range.end,
                            super::TextGravity::Forward,
                        ))
                        .offset;
            }
            let parts = segments_by_id.get(&field.id).map_or(&[][..], Vec::as_slice);
            if let Some(first) = parts.first() {
                let Some(target) = first.target else {
                    continue;
                };
                if target.is_empty()
                    || target.contains(['\0', '"'])
                    || parts.iter().any(|part| {
                        part.target != Some(target) || part.target_name != first.target_name
                    })
                {
                    continue;
                }
                field.target = target.to_owned();
                field.target_name = first.target_name.map(str::to_owned);
                let start = first.range.start;
                let end = parts.last().unwrap().range.end;
                if field.range.start > start || !only_breaks(field.range.start..start, &breaks) {
                    field.range.start = start;
                }
                if field.range.end < end || !only_breaks(end..field.range.end, &breaks) {
                    field.range.end = end;
                }
                let mut cursor = field.range.start;
                let mut valid = true;
                for part in parts {
                    if part.range.start < cursor || !only_breaks(cursor..part.range.start, &breaks)
                    {
                        valid = false;
                        break;
                    }
                    cursor = part.range.end;
                }
                if !valid || !only_breaks(cursor..field.range.end, &breaks) {
                    continue;
                }
            } else if !only_breaks(field.range.clone(), &breaks) {
                continue;
            }
            if field.range.start < field.range.end && field.range.end <= length {
                retained.push(field);
            }
        }
        retained.sort_by_key(|field| field.range.start);
        let mut end = 0;
        retained.retain(|field| {
            let valid = field.range.start >= end;
            if valid {
                end = field.range.end;
            }
            valid
        });
        let ids: HashSet<_> = retained.iter().map(|field| field.id).collect();
        fn clear_unknown(inlines: &mut [InlineSnapshot], ids: &HashSet<u64>) {
            for inline in inlines {
                if let InlineSnapshotKind::Hyperlink { field_id, .. } = &mut inline.kind {
                    if field_id.is_some_and(|id| !ids.contains(&id)) {
                        *field_id = None;
                    }
                }
                if let Some(children) = inline.children_mut() {
                    clear_unknown(children, ids);
                }
            }
        }
        for paragraph in &mut self.paragraphs {
            clear_unknown(&mut paragraph.inlines, &ids);
        }
        self.link_fields = retained;
    }

    /// Rebases canonical field spans after a caller has transformed visible
    /// content. Entries represent old UTF-16 boundaries; surrogate interiors may
    /// be None. Collapsed results retire their identity and semantic wrappers,
    /// while ordinary children, formatting and the allocation watermark survive.
    pub(crate) fn remap_link_field_offsets(
        &mut self,
        offsets: &[Option<usize>],
    ) -> Result<(), DocumentError> {
        let transformed_length = self.text_length();
        if offsets.first() != Some(&Some(0)) || offsets.last() != Some(&Some(transformed_length)) {
            return invalid("A field offset map must cover the complete transformed document.");
        }
        let mut previous = 0;
        for offset in offsets.iter().flatten() {
            if *offset < previous || *offset > transformed_length {
                return invalid("A field offset map must preserve boundary order.");
            }
            previous = *offset;
        }
        let mut candidate = self.clone();
        let mut retained = Vec::with_capacity(candidate.link_fields.len());
        for field in &candidate.link_fields {
            let mut field = field.clone();
            let start = offsets.get(field.range.start).copied().flatten().ok_or(
                DocumentError::InvalidLinkField("A field start is absent from the offset map."),
            )?;
            let end = offsets.get(field.range.end).copied().flatten().ok_or(
                DocumentError::InvalidLinkField("A field end is absent from the offset map."),
            )?;
            if start < end {
                field.range = start..end;
                retained.push(field);
            } else {
                update_projection(&mut candidate.paragraphs, field.id, None);
            }
        }
        candidate.link_fields = retained;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// A copied fragment receives new destination identities once per field,
    /// including all paragraph-owned segments of a cross-paragraph result.
    pub(crate) fn remap_link_field_ids(&mut self, next: &mut u64) -> Result<(), DocumentError> {
        let mut ids = HashMap::new();
        for field in &mut self.link_fields {
            let replacement = allocate(next)?;
            ids.insert(field.id, replacement);
            field.id = replacement;
        }
        visit_fields_mut(&mut self.paragraphs, &mut |id, _| {
            if let Some(replacement) = ids.get(id) {
                *id = *replacement;
            }
        });
        self.next_link_field_id = *next;
        self.validate()
    }

    /// Updates the field table after the caller has spliced visible inlines.
    /// Rich insertion owns its own fields. Surviving destination pieces are
    /// split around unrelated inserted content; plain inherited insertion can
    /// explicitly retain an existing identity through its fragment metadata.
    pub(crate) fn replace_link_fields(
        &mut self,
        before: &Self,
        range: Range<usize>,
        fragment: &Self,
        mut next: u64,
    ) -> Result<(), DocumentError> {
        let inserted = fragment.text_length();
        let map_after = |position: usize| position - range.end + range.start + inserted;
        let mut fields = Vec::new();
        let mut retags = Vec::new();
        for field in &before.link_fields {
            if field.range.end <= range.start {
                fields.push(field.clone());
                retags.push((field.id, field.clone()));
            } else if field.range.start >= range.end {
                let mut shifted = field.clone();
                shifted.range = map_after(field.range.start)..map_after(field.range.end);
                retags.push((field.id, shifted.clone()));
                fields.push(shifted);
            } else {
                let has_left = field.range.start < range.start;
                let has_right = field.range.end > range.end;
                let inherited = fragment.link_fields.iter().any(|part| part.id == field.id);
                if has_left {
                    let mut left = field.clone();
                    left.range.end = range.start;
                    retags.push((field.id, left.clone()));
                    fields.push(left);
                }
                if has_right {
                    let mut right = field.clone();
                    right.range = range.start + inserted..map_after(field.range.end);
                    if has_left && inserted != 0 && !inherited {
                        right.id = allocate(&mut next)?;
                    }
                    retags.push((field.id, right.clone()));
                    fields.push(right);
                }
            }
        }
        for field in &fragment.link_fields {
            let mut field = field.clone();
            field.range = field.range.start + range.start..field.range.end + range.start;
            fields.push(field);
        }
        fields.sort_by_key(|field| field.range.start);
        let mut combined: Vec<DocumentLinkField> = Vec::new();
        for field in fields {
            if let Some(previous) = combined.last_mut().filter(|previous| {
                previous.id == field.id
                    && previous.range.end == field.range.start
                    && previous.target == field.target
                    && previous.target_name == field.target_name
                    && previous.has_sentinel == field.has_sentinel
            }) {
                previous.range.end = field.range.end;
            } else {
                combined.push(field);
            }
        }
        let mut retags_by_id: HashMap<u64, Vec<DocumentLinkField>> = HashMap::new();
        for (original, replacement) in retags {
            retags_by_id.entry(original).or_default().push(replacement);
        }
        visit_fields_mut(&mut self.paragraphs, &mut |id, extent| {
            if let Some(replacement) =
                retags_by_id
                    .get(id)
                    .into_iter()
                    .flatten()
                    .find(|replacement| {
                        replacement.range.start <= extent.start
                            && extent.end <= replacement.range.end
                    })
            {
                *id = replacement.id;
            }
        });
        self.link_fields = combined;
        self.next_link_field_id = next;
        self.validate()
    }
}

fn update_projection(
    paragraphs: &mut [super::ParagraphSnapshot],
    id: u64,
    field: Option<&DocumentLinkField>,
) {
    fn update(inlines: &mut [InlineSnapshot], id: u64, field: Option<&DocumentLinkField>) {
        for inline in inlines {
            if let InlineSnapshotKind::Hyperlink {
                field_id: Some(known),
                navigate_uri,
                target_name,
                inlines,
                ..
            } = &mut inline.kind
            {
                if *known == id {
                    if let Some(field) = field {
                        *navigate_uri = Some(field.target.clone());
                        *target_name = field.target_name.clone();
                    } else {
                        inline.kind = InlineSnapshotKind::Span(std::mem::take(inlines));
                    }
                }
            }
            if let Some(children) = inline.children_mut() {
                update(children, id, field);
            }
        }
    }
    for paragraph in paragraphs {
        update(&mut paragraph.inlines, id, field);
    }
}

fn visit_fields_mut(
    paragraphs: &mut [super::ParagraphSnapshot],
    callback: &mut impl FnMut(&mut u64, Range<usize>),
) {
    fn visit(
        inlines: &mut [InlineSnapshot],
        offset: &mut usize,
        callback: &mut impl FnMut(&mut u64, Range<usize>),
    ) {
        for inline in inlines {
            let start = *offset;
            let end = start + inline.text_length();
            if let InlineSnapshotKind::Hyperlink {
                field_id: Some(id), ..
            } = &mut inline.kind
            {
                callback(id, start..end);
            }
            if let Some(children) = inline.children_mut() {
                visit(children, offset, callback);
            }
            *offset = end;
        }
    }
    let mut offset = 0;
    for paragraph in paragraphs {
        visit(&mut paragraph.inlines, &mut offset, callback);
        offset += 1;
    }
}

fn only_breaks(range: Range<usize>, breaks: &[usize]) -> bool {
    if range.is_empty() {
        return true;
    }
    let start = breaks.partition_point(|position| *position < range.start);
    let end = breaks.partition_point(|position| *position < range.end);
    end - start == range.len()
}
fn allocate(next: &mut u64) -> Result<u64, DocumentError> {
    let id = *next;
    if id == 0 {
        return invalid("Field identities are exhausted.");
    }
    *next = id.checked_add(1).ok_or(DocumentError::InvalidLinkField(
        "Field identities are exhausted.",
    ))?;
    Ok(id)
}
fn invalid<T>(message: &'static str) -> Result<T, DocumentError> {
    Err(DocumentError::InvalidLinkField(message))
}
