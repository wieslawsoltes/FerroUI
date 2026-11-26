use crate::{DesignerSession, PreviewStatus};
use ferroui_designer_animation::{
    AnimationProgramLocator, PreviewMarker, PreviewMarkerEdit, PreviewMarkers,
};
use ferroui_designer_core::NodeId;
use std::collections::{BTreeMap, btree_map::Entry};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Scope {
    uri: String,
    invocation: Option<usize>,
    fixture: Option<String>,
    definition: Option<NodeId>,
    clock: Clock,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Clock {
    Shared,
    Authored(String),
    Frame { revision: u64, program: NodeId },
}

#[derive(Default)]
pub(crate) struct AnimationMarkers(BTreeMap<Scope, PreviewMarkers>);

impl DesignerSession {
    pub(crate) fn preview_markers(
        &self,
        program: Option<NodeId>,
        all: bool,
    ) -> Result<Vec<PreviewMarker>, String> {
        let (scope, limit) = self.marker_clock(program, all)?;
        Ok(self
            .animation_markers
            .0
            .get(&scope)
            .map_or_else(Vec::new, |markers| markers.list(limit)))
    }

    pub(crate) fn edit_preview_marker(
        &mut self,
        program: Option<NodeId>,
        all: bool,
        edit: PreviewMarkerEdit,
    ) -> Result<(), String> {
        self.ensure_marker_editable()?;
        let (scope, limit) = self.marker_clock(program, all)?;
        match self.animation_markers.0.entry(scope) {
            Entry::Occupied(mut entry) => {
                entry
                    .get_mut()
                    .apply(edit, limit)
                    .map_err(|error| error.to_string())?;
                if entry.get().is_empty() {
                    entry.remove();
                }
            }
            Entry::Vacant(entry) => {
                let mut markers = PreviewMarkers::default();
                markers
                    .apply(edit, limit)
                    .map_err(|error| error.to_string())?;
                entry.insert(markers);
            }
        }
        self.generation += 1;
        Ok(())
    }

    pub(crate) fn seek_preview_marker(
        &mut self,
        program: Option<NodeId>,
        all: bool,
        name: &str,
    ) -> Result<(), String> {
        self.ensure_marker_editable()?;
        let (scope, limit) = self.marker_clock(program, all)?;
        let time = self
            .animation_markers
            .0
            .get(&scope)
            .and_then(|markers| markers.find(name, limit))
            .ok_or("Choose a marker within this preview clock")?;
        if all {
            self.scrub_animation_clips(None, time)
        } else {
            let program = program
                .or_else(|| self.active_animation_program())
                .ok_or("Choose an animation program")?;
            self.scrub_animation_program(program, time)
        }
    }

    fn ensure_marker_editable(&self) -> Result<(), String> {
        if self.is_playing() {
            return Err("Pause playback before using preview markers".into());
        }
        Ok(())
    }

    fn marker_clock(
        &self,
        program: Option<NodeId>,
        all: bool,
    ) -> Result<(Scope, Option<f64>), String> {
        let revision = self.document().revision();
        if !self.authored_is_valid()
            || !matches!(self.preview_status(), PreviewStatus::Ready { revision: accepted } if *accepted == revision)
        {
            return Err("Wait for the current animation preview".into());
        }
        let programs = self.animation_programs();
        let program = program.or_else(|| self.active_animation_program());
        let limit = self.preview_range_limit(program, all)?;
        let clock = if all {
            Clock::Shared
        } else {
            let program = programs
                .iter()
                .find(|item| Some(item.id) == program)
                .ok_or("Choose an animation program")?;
            AnimationProgramLocator::preview_key(self.document(), programs, program).map_or(
                Clock::Frame {
                    revision,
                    program: program.id,
                },
                Clock::Authored,
            )
        };
        Ok((
            Scope {
                uri: self.document_uri().to_owned(),
                invocation: self.rust_map().map(|map| map.invocation.start),
                fixture: self.design_fixture().map(str::to_owned),
                definition: self.definition_scope().map(|scope| scope.node),
                clock,
            },
            limit,
        ))
    }
}
