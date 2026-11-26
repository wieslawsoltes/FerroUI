use super::{
    core_error, fragment, AnimationCodecRegistry, AnimationError, AnimationProgram,
    NativeAnimationCodec,
};
use crate::{AnimationCodec, Result, Timeline};
use ferroui_designer_core::{Document, Edit, NodeId};

const XAML_NS: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

impl AnimationCodecRegistry {
    /// Encode a program in its original document context. Styles can infer the
    /// setter target from their selector. Retain that type on generated keys.
    pub fn encode_for_document(
        &self,
        document: &Document,
        program: &AnimationProgram,
        timeline: &Timeline,
    ) -> Result<String> {
        self.encode_in_scope(document, program, timeline, false)
    }

    /// Encode just the authored keys, leaving implicit endpoints to the runtime.
    pub fn encode_keyframes_for_document(
        &self,
        document: &Document,
        program: &AnimationProgram,
        timeline: &Timeline,
    ) -> Result<String> {
        self.encode_in_scope(document, program, timeline, true)
    }

    fn encode_in_scope(
        &self,
        document: &Document,
        program: &AnimationProgram,
        timeline: &Timeline,
        authored_keys: bool,
    ) -> Result<String> {
        let original = fragment(document, program.id)?;
        let generated = self.encode_program(program, &original, timeline, authored_keys)?;
        if program.codec.as_deref() == Some(NativeAnimationCodec.id())
            && !timeline.tracks.is_empty()
        {
            let mut output = Document::new(generated).map_err(core_error)?;
            let edits = setter_target_edits(document, program, &output, output.ast().root())?;
            output.apply("Retain animation setter target type", edits).map_err(core_error)?;
            return Ok(output.source().to_owned());
        }
        Ok(generated)
    }
}

impl AnimationProgram {
    /// Preserve the native setter target type when recording into an existing program.
    pub fn setter_target_edits(&self, document: &Document) -> Result<Vec<Edit>> {
        setter_target_edits(document, self, document, self.id)
    }
}

fn setter_target_edits(
    document: &Document,
    program: &AnimationProgram,
    output: &Document,
    animation: NodeId,
) -> Result<Vec<Edit>> {
    let existing = output
        .ast()
        .node(animation)
        .ok_or_else(|| AnimationError("Animation program source disappeared".into()))?
        .attributes
        .iter()
        .any(|attribute| {
            attribute.namespace_uri.as_deref() == Some(XAML_NS)
                && attribute.name.rsplit(':').next() == Some("SetterTargetType")
        });
    if existing {
        return Ok(Vec::new());
    }

    let target = document
        .ast()
        .node(program.primary_target())
        .ok_or_else(|| AnimationError("Animation target source disappeared".into()))?;
    let root = output.ast().root();
    let mut namespaces = output.ast().namespaces(animation);
    let mut edits = Vec::new();
    let prefix = namespaces
        .iter()
        .find(|(prefix, uri)| !prefix.is_empty() && uri.as_str() == XAML_NS)
        .map(|(prefix, _)| prefix.clone())
        .unwrap_or_else(|| {
            let prefix = free_prefix("x", &namespaces);
            namespaces.insert(prefix.clone(), XAML_NS.into());
            edits.push(Edit::SetAttribute {
                node: root,
                name: format!("xmlns:{prefix}"),
                value: XAML_NS.into(),
            });
            prefix
        });
    let (target_prefix, target_local) = target
        .name
        .split_once(':')
        .unwrap_or(("", target.name.as_str()));
    let mut target_type = target.name.clone();
    if let Some(namespace) = document.ast().namespaces(target.id).get(target_prefix) {
        if namespaces.get(target_prefix) != Some(namespace) {
            let alias = free_prefix("designertarget", &namespaces);
            edits.push(Edit::SetAttribute {
                node: root,
                name: format!("xmlns:{alias}"),
                value: namespace.clone(),
            });
            target_type = format!("{alias}:{target_local}");
        }
    }
    edits.push(Edit::SetAttribute {
        node: animation,
        name: format!("{prefix}:SetterTargetType"),
        value: target_type,
    });
    Ok(edits)
}

fn free_prefix(preferred: &str, namespaces: &std::collections::BTreeMap<String, String>) -> String {
    let mut prefix = preferred.to_owned();
    while namespaces.contains_key(&prefix) {
        prefix.push('_');
    }
    prefix
}
