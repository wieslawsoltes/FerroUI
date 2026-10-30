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

    /// Rebuilds formatted text after inherited presentation changes. Document
    /// collection hosts forward this to the view without publishing a model edit.
    fn invalidate_layout(&self) { self.invalidate(); }

    /// Invalidates interaction visuals without changing document content or
    /// formatted text. A host with separate drawing invalidation overrides it.
    fn invalidate_render(&self) { self.invalidate(); }

    /// Runs `f` with the visual children of the host.
    fn with_visual_children(&self, f: &mut dyn FnMut(&FerroList<Ref<Visual>>));

    /// The host as a text block (C# `host is TextBlock`).
    fn as_text_block(&self) -> Option<Ref<TextBlock>> {
        None
    }
}
