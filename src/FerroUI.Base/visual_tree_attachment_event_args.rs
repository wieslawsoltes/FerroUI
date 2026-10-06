use crate::rendering::IPresentationSource;
use crate::{Ref, Visual};
use std::rc::Rc;

/// Holds the event arguments for the `attached_to_visual_tree` and
/// `detached_from_visual_tree` events.
#[derive(Clone)]
pub struct VisualTreeAttachmentEventArgs {
    attachment_point: Option<Ref<Visual>>,
    presentation_source: Rc<dyn IPresentationSource>,
    root_visual: Ref<Visual>,
}

impl VisualTreeAttachmentEventArgs {
    /// Panics if the presentation source has no root visual.
    pub fn new(attachment_point: Option<Ref<Visual>>, presentation_source: Rc<dyn IPresentationSource>) -> Self {
        let root_visual =
            presentation_source.root_visual().expect("PresentationSource must have a non-null RootVisual.");
        Self { attachment_point, presentation_source, root_visual }
    }

    /// The parent that the visual's tree is being attached to or detached
    /// from. `None` when the presentation source's root visual itself is
    /// being attached or detached.
    pub fn attachment_point(&self) -> Option<&Ref<Visual>> {
        self.attachment_point.as_ref()
    }

    /// The parent that the visual's tree is being attached to or detached
    /// from.
    #[deprecated(note = "Use attachment_point")]
    pub fn parent(&self) -> Option<&Ref<Visual>> {
        self.attachment_point()
    }

    /// The presentation source that the visual is being attached to or
    /// detached from.
    pub fn presentation_source(&self) -> &Rc<dyn IPresentationSource> {
        &self.presentation_source
    }

    /// The root visual of the tree.
    #[deprecated(
        note = "This was previously always returning TopLevel. This is no longer guaranteed. Use TopLevel::get_top_level(visual) if you need a TopLevel or root_visual if you are interested in the root of the visual tree."
    )]
    pub fn root(&self) -> &Ref<Visual> {
        &self.root_visual
    }

    /// The root visual of the tree this visual is being attached to or
    /// detached from: the root visual of the presentation source.
    pub fn root_visual(&self) -> &Ref<Visual> {
        &self.root_visual
    }

    pub fn set_root_visual(&mut self, value: Ref<Visual>) {
        self.root_visual = value;
    }
}

/// Copies compare equal when they describe the same attachment (the same
/// visuals and presentation source).
impl PartialEq for VisualTreeAttachmentEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.attachment_point == other.attachment_point
            && self.root_visual == other.root_visual
            && std::ptr::addr_eq(Rc::as_ptr(&self.presentation_source), Rc::as_ptr(&other.presentation_source))
    }
}
