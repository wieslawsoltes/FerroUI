use super::syntax::method_suffix;
use ferroui_designer_animation::AnimationProgramIdentity;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::ExprMethodCall;

const PREFIX: &str = "/* ferroui-designer-animation: ";

/// The owning method and the optional designer annotation before its argument.
#[derive(Clone, Debug)]
pub(crate) struct AnimationAttachment {
    pub(crate) method: SourceSpan,
    pub(crate) argument_start: usize,
    pub(crate) lone_style: Option<(SourceSpan, Option<SourceSpan>)>,
    pub(crate) annotation: Option<(SourceSpan, AnimationProgramIdentity)>,
}

impl AnimationAttachment {
    pub(super) fn read(
        source: &str,
        call: &ExprMethodCall,
        declaration: SourceSpan,
        lone_style: Option<(SourceSpan, Option<SourceSpan>)>,
    ) -> Self {
        let argument_start = call.paren_token.span.open().byte_range().end;
        let annotation = read_annotation(source, argument_start, declaration.start);
        Self {
            method: method_suffix(call),
            argument_start,
            lone_style,
            annotation,
        }
    }

    pub(crate) fn identity(&self) -> Option<&AnimationProgramIdentity> {
        self.annotation.as_ref().map(|(_, identity)| identity)
    }

    pub(crate) fn name_patch(&self, identity: &AnimationProgramIdentity) -> TextPatch {
        let text = annotation_text(identity);
        let (span, replacement) = if let Some((span, _)) = &self.annotation {
            (*span, text)
        } else {
            (
                SourceSpan::new(self.argument_start, self.argument_start),
                format!("{text} "),
            )
        };
        TextPatch { span, replacement }
    }
}

fn read_annotation(
    source: &str,
    open: usize,
    argument: usize,
) -> Option<(SourceSpan, AnimationProgramIdentity)> {
    let prefix = source.get(open..argument)?;
    let trimmed = prefix.trim_start();
    let start = open + prefix.len() - trimmed.len();
    let (content, _) = trimmed.strip_prefix(PREFIX)?.split_once("*/")?;
    let (key, name) = content.trim().split_once(' ')?;
    let name = syn::parse_str::<syn::LitStr>(name.trim()).ok()?.value();
    let identity = AnimationProgramIdentity::new(&name, Some(key)).ok()?;
    Some((
        SourceSpan::new(start, start + PREFIX.len() + content.len() + 2),
        identity,
    ))
}

pub(crate) fn annotation_text(identity: &AnimationProgramIdentity) -> String {
    // Escape slashes so arbitrary labels cannot open or terminate a Rust comment.
    let name = format!("{:?}", identity.name()).replace('/', "\\u{2f}");
    format!("{PREFIX}{} {name} */", identity.key())
}
