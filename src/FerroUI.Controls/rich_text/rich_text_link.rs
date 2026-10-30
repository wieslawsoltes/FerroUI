//! Link transitions over an immutable field projection. The return values
//! contain the acted range's endpoint changes explicitly; live editor callers
//! must install them within their document/range publication transaction.
use super::rich_text_field_story::{
    FieldLocation, FieldStory, FieldStoryError, FieldStoryOffset, FieldStoryRange, LinkArgument,
    LinkField, LinkFieldId, ProjectedField, VisibleOffset,
};

#[derive(Clone, Debug)]
pub(crate) struct LinkLookup {
    pub(crate) argument: Option<LinkArgument>,
    pub(crate) range: FieldStoryRange,
}

/// The one field changed by a successful Link command. This is sufficient for
/// the document consumer to update only its corresponding semantic segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkFieldChange {
    pub(crate) id: LinkFieldId,
    pub(crate) before: Option<LinkField>,
    pub(crate) after: Option<LinkField>,
}

#[derive(Clone, Debug)]
pub(crate) struct LinkMutation {
    pub(crate) story: FieldStory,
    pub(crate) range: FieldStoryRange,
    pub(crate) change: Option<LinkFieldChange>,
}

enum ParsedArgument {
    Clear,
    Target(LinkArgument),
}

fn parse_argument(argument: Option<&str>) -> Result<ParsedArgument, FieldStoryError> {
    let Some(argument) = argument else {
        return Ok(ParsedArgument::Clear);
    };
    if argument.is_empty() || argument == "\"\"" {
        return Ok(ParsedArgument::Clear);
    }
    if argument.contains('\0') {
        return Err(FieldStoryError::InvalidArgument);
    }
    let (argument, sentinel) = argument
        .strip_prefix('\u{fddf}')
        .map_or((argument, false), |rest| (rest, true));
    let rest = argument
        .strip_prefix('"')
        .ok_or(FieldStoryError::InvalidArgument)?;
    let end = rest.find('"').ok_or(FieldStoryError::InvalidArgument)?;
    if end == 0 {
        return Err(FieldStoryError::Unsupported(
            "Only the measured plain empty and quoted-empty unlink forms are supported.",
        ));
    }
    // The first closing quote terminates the measured argument; trailing text
    // does not become part of the target. No URI normalization is performed.
    Ok(ParsedArgument::Target(LinkArgument::from_target(
        &rest[..end],
        sentinel,
    )?))
}

impl FieldStory {
    /// Returns the first field beginning at or containing the range start and
    /// the complete field range. A caret returns no link and remains unchanged.
    /// Plain text preceding the first intersected field and instruction-interior
    /// requests remain explicit unsupported cases until characterized.
    pub(crate) fn get_link(&self, range: &FieldStoryRange) -> Result<LinkLookup, FieldStoryError> {
        self.ensure_range(range)?;
        if range.is_empty() {
            return Ok(LinkLookup {
                argument: None,
                range: range.clone(),
            });
        }
        let Some(field) = self.intersected_fields(range).next() else {
            return Ok(LinkLookup {
                argument: None,
                range: range.clone(),
            });
        };
        if range.start().get() < field.instruction_start {
            return Err(FieldStoryError::Unsupported(
                "A getter beginning in plain text before a field has not been characterized.",
            ));
        }
        self.ensure_supported_field_endpoints(range, field)?;
        self.ensure_scalar_endpoints(range)?;
        Ok(LinkLookup {
            argument: Some(self.fields()[field.field_index].argument().clone()),
            range: self.range(
                FieldStoryOffset::new(field.instruction_start),
                FieldStoryOffset::new(field.end),
            )?,
        })
    }

    /// Applies a Link command without changing visible text or the input story.
    /// The returned story, field change and acted endpoints form one candidate
    /// transaction; this method does not mutate live selection or history.
    pub(crate) fn set_link(
        &self,
        range: &FieldStoryRange,
        argument: Option<&str>,
    ) -> Result<LinkMutation, FieldStoryError> {
        self.ensure_range(range)?;
        if range.is_empty() {
            return Err(FieldStoryError::InvalidArgument);
        }
        let argument = parse_argument(argument)?;
        if range.end().get() > self.length() - 1 {
            return Err(FieldStoryError::Unsupported("Link edits including the synthetic final paragraph mark have not been characterized."));
        }
        let mut intersected = self.intersected_fields(range);
        let first = intersected.next();
        if let Some(projected) = first {
            if intersected.next().is_some()
                || range.start().get() < projected.instruction_start
                || range.end().get() > projected.end
            {
                return Err(FieldStoryError::InvalidArgument);
            }
            self.ensure_supported_field_endpoints(range, projected)?;
            self.ensure_scalar_endpoints(range)?;
            self.update_link(range, projected, argument)
        } else {
            let ParsedArgument::Target(argument) = argument else {
                return Err(FieldStoryError::InvalidArgument);
            };
            self.ensure_scalar_endpoints(range)?;
            let start = self.visible_endpoint(range.start())?;
            let end = self.visible_endpoint(range.end())?;
            let visible = start..end;
            if visible.is_empty() {
                return Err(FieldStoryError::InvalidArgument);
            }
            if end.get() - start.get() == 1 && self.paragraph_breaks().binary_search(&start).is_ok()
            {
                return Err(FieldStoryError::ParagraphOnly);
            }
            // The measured append case creates the left field first. A
            // matching field immediately to the right may imply a different
            // native prepend/merge policy, so do not guess or retire its ID.
            if self.projected_fields().iter().any(|field| {
                field.instruction_start == range.end().get()
                    && self.fields()[field.field_index].argument() == &argument
            }) {
                return Err(FieldStoryError::Unsupported(
                    "Creating a link immediately before the same target has not been characterized.",
                ));
            }
            if let Some(previous) = self.projected_fields().iter().find(|field| {
                field.end == range.start().get()
                    && self.fields()[field.field_index].argument() == &argument
            }) {
                let before = self.fields()[previous.field_index].clone();
                let after = LinkField::new(before.id(), before.result().start..end, argument);
                let mut fields = self.fields().to_vec();
                fields[previous.field_index] = after.clone();
                let story = self.replace_fields(fields, false)?;
                let new_range = story.range(range.start(), range.end())?;
                return Ok(LinkMutation {
                    story,
                    range: new_range,
                    change: Some(LinkFieldChange {
                        id: before.id(),
                        before: Some(before),
                        after: Some(after),
                    }),
                });
            }
            let id = self.next_field_id()?;
            let instruction_length = argument.instruction()?.len();
            let after = LinkField::new(id, visible, argument);
            let mut fields = self.fields().to_vec();
            fields.push(after.clone());
            let story = self.replace_fields(fields, true)?;
            let end = range
                .end()
                .get()
                .checked_add(instruction_length)
                .ok_or(FieldStoryError::CapacityExceeded)?;
            let new_range = story.range(range.start(), FieldStoryOffset::new(end))?;
            Ok(LinkMutation {
                story,
                range: new_range,
                change: Some(LinkFieldChange {
                    id,
                    before: None,
                    after: Some(after),
                }),
            })
        }
    }

    fn intersected_fields<'a>(
        &'a self,
        range: &'a FieldStoryRange,
    ) -> impl Iterator<Item = &'a ProjectedField> {
        let first = self
            .projected_fields()
            .partition_point(|field| field.end <= range.start().get());
        self.projected_fields()[first..]
            .iter()
            .take_while(move |field| field.instruction_start < range.end().get())
    }

    fn ensure_supported_field_endpoints(
        &self,
        range: &FieldStoryRange,
        field: &ProjectedField,
    ) -> Result<(), FieldStoryError> {
        let start = range.start().get();
        let end = range.end().get();
        if (field.instruction_start < start && start < field.result_start)
            || end <= field.result_start
        {
            return Err(FieldStoryError::Unsupported("Instruction-only and instruction-interior Link ranges have not been characterized."));
        }
        Ok(())
    }

    fn visible_endpoint(
        &self,
        position: FieldStoryOffset,
    ) -> Result<VisibleOffset, FieldStoryError> {
        match self.location(position)? {
            FieldLocation::Visible(position) => Ok(position),
            FieldLocation::Instruction {
                field,
                utf16_offset: 0,
            } => Ok(self
                .fields()
                .iter()
                .find(|candidate| candidate.id() == field)
                .ok_or(FieldStoryError::UnknownField)?
                .result()
                .start),
            FieldLocation::FinalParagraphMark { after: false } => {
                Ok(VisibleOffset::new(self.visible_length()))
            }
            _ => Err(FieldStoryError::Unsupported(
                "The projected position is not a supported visible Link boundary.",
            )),
        }
    }

    fn ensure_scalar_endpoints(&self, range: &FieldStoryRange) -> Result<(), FieldStoryError> {
        if !self.visible_boundary(self.visible_endpoint(range.start())?)
            || !self.visible_boundary(self.visible_endpoint(range.end())?)
        {
            return Err(FieldStoryError::Unsupported(
                "Surrogate-interior Link edits have not been characterized.",
            ));
        }
        Ok(())
    }

    fn update_link(
        &self,
        range: &FieldStoryRange,
        projected: &ProjectedField,
        argument: ParsedArgument,
    ) -> Result<LinkMutation, FieldStoryError> {
        let before = self.fields()[projected.field_index].clone();
        let after = match argument {
            ParsedArgument::Clear => None,
            ParsedArgument::Target(argument) => {
                Some(LinkField::new(before.id(), before.result(), argument))
            }
        };
        if after.as_ref() == Some(&before) {
            return Ok(LinkMutation {
                story: self.clone(),
                range: range.clone(),
                change: None,
            });
        }
        let new_instruction_length = after.as_ref().map_or(Ok(0), |field| {
            field
                .argument()
                .instruction()
                .map(|instruction| instruction.len())
        })?;
        let mut fields = self.fields().to_vec();
        if let Some(after) = &after {
            fields[projected.field_index] = after.clone();
        } else {
            fields.remove(projected.field_index);
        }
        let story = self.replace_fields(fields, false)?;
        let rebase = |position: FieldStoryOffset| -> Result<FieldStoryOffset, FieldStoryError> {
            if position.get() == projected.instruction_start {
                return Ok(position);
            }
            // Supported non-start endpoints are in the friendly result. The
            // length of its preceding instruction changes independently.
            let result_offset = position
                .get()
                .checked_sub(projected.result_start)
                .ok_or(FieldStoryError::InvalidRange)?;
            let value = projected
                .instruction_start
                .checked_add(new_instruction_length)
                .and_then(|start| start.checked_add(result_offset))
                .ok_or(FieldStoryError::CapacityExceeded)?;
            Ok(FieldStoryOffset::new(value))
        };
        let new_range = story.range(rebase(range.start())?, rebase(range.end())?)?;
        Ok(LinkMutation {
            story,
            range: new_range,
            change: Some(LinkFieldChange {
                id: before.id(),
                before: Some(before),
                after,
            }),
        })
    }
}

#[cfg(test)]
#[path = "rich_text_link_tests.rs"]
mod tests;
