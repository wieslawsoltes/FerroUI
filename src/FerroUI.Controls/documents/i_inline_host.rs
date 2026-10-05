use crate::TextBlock;
use ferroui_base::collections::FerroList;
use ferroui_base::{Ref, Visual};

/// The control that hosts inlines: it is told when they change and owns the
/// visuals of embedded controls.
///
/// Handles are `Rc<dyn IInlineHost>`. A host is the parent of its inlines, so
/// its handle refers to it weakly: every member does nothing once the host is
/// gone.
pub trait IInlineHost {
    /// Invalidates the text of the host.
    fn invalidate(&self);

    /// Runs `f` with the visual children of the host.
    fn with_visual_children(&self, f: &mut dyn FnMut(&FerroList<Ref<Visual>>));

    /// The host as a text block (C# `host is TextBlock`).
    fn as_text_block(&self) -> Option<Ref<TextBlock>> {
        None
    }
}
