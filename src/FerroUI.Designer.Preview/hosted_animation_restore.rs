use ferroui_designer_animation::{AnimationProgram, AnimationProgramLocator, AnimationPrograms};
use ferroui_designer_core::{Document, NodeId, SourceAnchor, SourceSpan};
use ferroui_remote_protocol::designer::DesignAnimationCatalogMessage;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Restore {
    frame: FrameIdentity,
    presentation: Presentation,
}

#[derive(Deserialize)]
pub(crate) struct FrameIdentity {
    revision: i64,
    sequence: i64,
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct Presentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) selection: Option<i64>,
    pub(crate) overrides: BTreeMap<i64, Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) position: Option<Position>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub(crate) struct Position {
    pub(crate) program: i64,
    pub(crate) elapsed: f64,
    pub(crate) playing: bool,
    pub(crate) completed: bool,
}

#[derive(PartialEq, Eq)]
pub(crate) struct SourceScope {
    pub(crate) uri: String,
    pub(crate) assembly: String,
    pub(crate) project_path: String,
    pub(crate) invocation: Option<SourceSpan>,
}

struct Clip {
    duration: Option<f64>,
    default_targets: Vec<i64>,
    compatible_targets: BTreeSet<i64>,
}

pub(crate) struct SourceMap {
    revision: i64,
    sequences: [Option<i64>; 2],
    scope: SourceScope,
    document: Document,
    programs: Vec<AnimationProgram>,
    clips: BTreeMap<i64, Clip>,
}

impl SourceMap {
    pub(crate) fn new(
        document: Document,
        scope: SourceScope,
        catalog: &DesignAnimationCatalogMessage,
    ) -> Option<Self> {
        if catalog.programs.is_empty() {
            return None;
        }
        let clips = catalog
            .programs
            .iter()
            .map(|program| {
                (
                    program.id,
                    Clip {
                        duration: program.total_duration,
                        default_targets: program.default_targets.clone(),
                        compatible_targets: program.compatible_targets.iter().copied().collect(),
                    },
                )
            })
            .collect();
        let programs = AnimationPrograms::scan(&document);
        Some(Self {
            revision: catalog.revision,
            sequences: [None; 2],
            scope,
            document,
            programs,
            clips,
        })
    }

    fn contains(&self, frame: &FrameIdentity) -> bool {
        frame.revision == self.revision && self.sequences.contains(&Some(frame.sequence))
    }

    fn remap_program(&self, replacement: &Self, id: i64) -> Option<i64> {
        let old_id = NodeId::try_from(id).ok()?;
        self.clips.get(&id)?;
        let program = self.programs.iter().find(|program| program.id == old_id)?;
        let anchor = AnimationProgramLocator::capture(&self.document, program);
        let key = program.source_key.as_deref();
        // A rebuild can remove one of two indistinguishable clips. Resolve against
        // the old source too, so the remaining clip cannot inherit an ambiguous pose.
        if AnimationProgramLocator::resolve(&self.document, &self.programs, key, anchor.as_ref())?
            .id
            != old_id
        {
            return None;
        }
        let resolved = AnimationProgramLocator::resolve(
            &replacement.document,
            &replacement.programs,
            key,
            anchor.as_ref(),
        )?;
        let id = i64::try_from(resolved.id).ok()?;
        replacement.clips.contains_key(&id).then_some(id)
    }

    fn remap_targets(
        &self,
        replacement: &Self,
        old: &Clip,
        new: &Clip,
        targets: &[i64],
    ) -> Vec<i64> {
        let mut seen = BTreeSet::new();
        targets
            .iter()
            .filter_map(|&id| {
                if !old.compatible_targets.contains(&id) {
                    return None;
                }
                let anchor =
                    SourceAnchor::capture(&self.document, NodeId::try_from(id).ok()?).ok()?;
                let mapped = i64::try_from(anchor.resolve(&replacement.document).ok()?).ok()?;
                (new.compatible_targets.contains(&mapped) && seen.insert(mapped)).then_some(mapped)
            })
            .collect()
    }

    fn restore(&self, replacement: &Self, prior: &Presentation) -> Presentation {
        let remap = |id| self.remap_program(replacement, id);
        let overrides = prior
            .overrides
            .iter()
            .filter_map(|(&id, targets)| {
                let mapped = remap(id)?;
                let targets = self.remap_targets(
                    replacement,
                    self.clips.get(&id)?,
                    replacement.clips.get(&mapped)?,
                    targets,
                );
                (!targets.is_empty()).then_some((mapped, targets))
            })
            .collect::<BTreeMap<_, _>>();
        let position = prior.position.and_then(|position| {
            let program = remap(position.program)?;
            let clip = replacement.clips.get(&program)?;
            let available = if prior.overrides.contains_key(&position.program) {
                overrides.contains_key(&program)
            } else {
                !clip.default_targets.is_empty()
            };
            if !available || !position.elapsed.is_finite() || position.elapsed < 0.0 {
                return None;
            }
            let elapsed = clip
                .duration
                .map_or(position.elapsed, |limit| position.elapsed.min(limit));
            Some(Position {
                program,
                elapsed,
                ..position
            })
        });
        Presentation {
            selection: prior.selection.and_then(remap),
            overrides,
            position,
        }
    }
}

/// Keep the last delivered source map and the accepted map that the editor is
/// holding while it replaces the frame. Intermediate proposals can finish after
/// the editor has advanced past them, so they must not consume the accepted map.
#[derive(Default)]
pub(crate) struct SourceMaps {
    current: Option<SourceMap>,
    retained: Option<SourceMap>,
}

impl SourceMaps {
    pub(crate) fn retain(&mut self, request: Option<&Restore>) {
        if request.is_some_and(|request| {
            self.current
                .as_ref()
                .is_some_and(|map| map.contains(&request.frame))
        }) {
            self.retained = self.current.take();
        } else if !request.is_some_and(|request| {
            self.retained
                .as_ref()
                .is_some_and(|map| map.contains(&request.frame))
        }) {
            self.retained = None;
        }
    }

    pub(crate) fn restore(
        &self,
        request: Option<&Restore>,
        replacement: Option<&SourceMap>,
    ) -> Option<Presentation> {
        let request = request?;
        let prior = self
            .retained
            .as_ref()
            .filter(|map| map.contains(&request.frame))?;
        let replacement = replacement.filter(|map| map.scope == prior.scope)?;
        Some(prior.restore(replacement, &request.presentation))
    }

    pub(crate) fn deliver(&mut self, map: Option<SourceMap>, revision: i64, sequence: i64) {
        self.current = map;
        self.observe(revision, sequence);
    }

    pub(crate) fn observe(&mut self, revision: i64, sequence: i64) {
        if let Some(map) = self.current.as_mut().filter(|map| map.revision == revision) {
            if map.sequences[0].is_none_or(|latest| sequence > latest) {
                // An already requested sample can return after the editor has
                // frozen its old frame. Both identities share this source map.
                map.sequences = [Some(sequence), map.sequences[0]];
            }
        }
    }
}
