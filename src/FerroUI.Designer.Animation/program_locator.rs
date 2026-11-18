use crate::AnimationProgram;
use ferroui_designer_core::{Document, SourceAnchor};
use serde::{Deserialize, Serialize};

/// Identifies an authored clip when its source is parsed into another frame.
/// Stored program keys take precedence over structural paths and legacy names.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationProgramLocator {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<SourceAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<SourceAnchor>,
    name: Option<String>,
    selector: String,
}

impl AnimationProgramLocator {
    pub fn capture(document: &Document, program: &AnimationProgram) -> Option<Self> {
        let source = SourceAnchor::capture(document, program.id).ok();
        let owner = SourceAnchor::capture(document, program.owner).ok();
        (source.is_some() || owner.is_some()).then(|| Self {
            source,
            owner,
            name: stable_name(program).map(str::to_owned),
            selector: program.selector.clone(),
        })
    }

    /// Resolve in the new document only when the identity is unambiguous.
    pub fn resolve<'a>(
        document: &Document,
        programs: &'a [AnimationProgram],
        key: Option<&str>,
        anchor: Option<&Self>,
    ) -> Option<&'a AnimationProgram> {
        let mut keys = programs
            .iter()
            .filter(|program| key.is_some() && program.source_key.as_deref() == key);
        if let Some(program) = keys.next() {
            return keys.next().is_none().then_some(program);
        }
        // The first visual edit can stamp a key that Undo later removes.
        let anchor = anchor?;
        let matches = |program: &&AnimationProgram| {
            (key.is_none() || program.source_key.is_none())
                && stable_name(program) == anchor.name.as_deref()
                && program.selector == anchor.selector
        };
        if let Some(id) = anchor
            .source
            .as_ref()
            .and_then(|source| source.resolve(document).ok())
        {
            return programs
                .iter()
                .filter(matches)
                .find(|program| program.id == id);
        }
        let owner = anchor.owner.as_ref()?.resolve(document).ok()?;
        let mut candidates = programs
            .iter()
            .filter(matches)
            .filter(|program| program.owner == owner);
        let only = candidates.next()?;
        candidates.next().is_none().then_some(only)
    }
}

fn stable_name(program: &AnimationProgram) -> Option<&str> {
    let generated = program
        .name
        .strip_prefix("Animation ")
        .and_then(|id| id.parse().ok())
        == Some(program.id);
    (!generated).then_some(program.name.as_str())
}
