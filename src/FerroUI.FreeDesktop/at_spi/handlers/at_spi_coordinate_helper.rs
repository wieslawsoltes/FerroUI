//! The port of `AtSpiCoordinateHelper.cs`.

use crate::at_spi::at_spi_coord_type::AtSpiCoordType;
use crate::at_spi::at_spi_node::AtSpiNode;
use ferroui_base::Rect;
use std::rc::Rc;

pub(crate) struct AtSpiCoordinateHelper;

impl AtSpiCoordinateHelper {
    /// The node of the window a peer is in, when it is attached.
    fn root_node_of(node: &AtSpiNode, root: &ferroui_base::Ref<ferroui_controls::automation::peers::AutomationPeer>)
        -> Option<Rc<AtSpiNode>> {
        node.server()?.try_get_attached_node(Some(root)).filter(|root_node| root_node.as_root().is_some())
    }

    pub(crate) fn get_screen_extents(node: &AtSpiNode) -> Rect {
        let bounds = node.peer().get_bounding_rectangle();
        if let Some(root) = node.as_root() {
            return root.to_screen(bounds);
        }

        let Some(root) = node.peer().get_visual_root() else { return bounds };
        match Self::root_node_of(node, &root).as_deref().and_then(AtSpiNode::as_root) {
            Some(root_node) => root_node.to_screen(bounds),
            None => bounds,
        }
    }

    pub(crate) fn translate_rect(node: &AtSpiNode, screen_rect: Rect, coord_type: u32) -> Rect {
        match AtSpiCoordType::from_u32(coord_type) {
            Some(AtSpiCoordType::Screen) | None => screen_rect,
            Some(AtSpiCoordType::Window) => {
                let window_rect = Self::get_window_rect(node);
                relative_to(screen_rect, window_rect)
            }
            Some(AtSpiCoordType::Parent) => {
                let parent_rect = Self::get_parent_screen_rect(node);
                relative_to(screen_rect, parent_rect)
            }
        }
    }

    pub(crate) fn get_window_rect(node: &AtSpiNode) -> Rect {
        let Some(root) = node.peer().get_visual_root() else { return Rect::default() };
        match Self::root_node_of(node, &root).as_deref().and_then(AtSpiNode::as_root) {
            Some(root_node) => root_node.to_screen(root.get_bounding_rectangle()),
            None => Rect::default(),
        }
    }

    pub(crate) fn get_parent_screen_rect(node: &AtSpiNode) -> Rect {
        let Some(parent) = node.peer().get_parent() else { return Rect::default() };
        let bounds = parent.get_bounding_rectangle();
        let Some(root) = parent.get_visual_root() else { return bounds };
        match Self::root_node_of(node, &root).as_deref().and_then(AtSpiNode::as_root) {
            Some(root_node) => root_node.to_screen(bounds),
            None => bounds,
        }
    }
}

/// `rect` with its position relative to the position of `origin`.
pub(crate) fn relative_to(rect: Rect, origin: Rect) -> Rect {
    Rect::new(rect.x - origin.x, rect.y - origin.y, rect.width, rect.height)
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    #[test]
    fn a_rectangle_relative_to_an_origin_keeps_its_size() {
        let rect = relative_to(Rect::new(110.0, 220.0, 30.0, 40.0), Rect::new(100.0, 200.0, 800.0, 600.0));
        assert_eq!(rect, Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(relative_to(rect, Rect::default()), rect);
    }
}
