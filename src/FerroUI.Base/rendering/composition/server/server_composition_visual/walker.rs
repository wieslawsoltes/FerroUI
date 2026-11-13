use super::ServerCompositionVisual;
use crate::platform::LtrbRect;
use crate::rendering::composition::server::CompositorPools;
use crate::Matrix;
use std::rc::Rc;

pub(super) trait IServerTreeVisitor {
    /// Returns whether the children are to be visited.
    fn pre_subgraph(&mut self, visual: &Rc<ServerCompositionVisual>) -> bool;
    fn post_subgraph(&mut self, visual: &Rc<ServerCompositionVisual>);
    /// Called before the child `index` of `container` is visited (not from upstream): the
    /// render walk draws the drawing items that come before the child (section 9.42).
    #[inline]
    fn before_child(&mut self, _container: &Rc<ServerCompositionVisual>, _index: usize) {}
    /// The child of `container` to visit next, of those from `from` on, or `count`, their number,
    /// for none (not from upstream; section 9.48). Asked before `before_child`, and only of a
    /// container whose collection keeps a table of its children: what the visitor would do at the
    /// children it leaves out is its to do here.
    #[inline]
    fn next_child(&mut self, _container: &Rc<ServerCompositionVisual>, from: usize, _count: usize) -> usize {
        from
    }
}

/// A position of the tree walk: a container and the index of its child to
/// visit next.
pub struct TreeWalkerFrame {
    visual: Rc<ServerCompositionVisual>,
    children: Rc<Vec<Rc<ServerCompositionVisual>>>,
    current_index: usize,
    /// Whether the collection of the container keeps a table of its children.
    tabled: bool,
}

/// The children of a visual, `None` for a visual without a children
/// collection (the original reads `Children.List.Count` of both alike, and
/// nothing is allocated for such a visual).
fn children_of(visual: &ServerCompositionVisual) -> Option<Rc<Vec<Rc<ServerCompositionVisual>>>> {
    visual.children().map(|children| children.list())
}

fn has_children_table(visual: &ServerCompositionVisual) -> bool {
    visual.children().is_some_and(|children| children.has_table())
}

pub(super) fn walk<V: IServerTreeVisitor>(visitor: &mut V, root: &Rc<ServerCompositionVisual>, pools: &CompositorPools) {
    let mut frames = pools.tree_walker_frame_stack_pool.rent();

    let visit_children = visitor.pre_subgraph(root);
    let mut container = root.clone();
    let mut children = match children_of(&container) {
        Some(children) if visit_children && !children.is_empty() => children,
        _ => {
            visitor.post_subgraph(root);
            pools.tree_walker_frame_stack_pool.return_stack(frames);
            return;
        }
    };

    let mut current_index = 0usize;
    let mut tabled = has_children_table(&container);

    loop {
        if tabled && current_index < children.len() {
            current_index = visitor.next_child(&container, current_index, children.len());
        }
        if current_index >= children.len() {
            // Exiting "recursion"
            visitor.post_subgraph(&container);
            let Some(frame) = frames.pop() else { break };
            container = frame.visual;
            children = frame.children;
            current_index = frame.current_index;
            tabled = frame.tabled;
            continue;
        }

        visitor.before_child(&container, current_index);
        let child = children[current_index].clone();
        let visit_children = visitor.pre_subgraph(&child);
        if visit_children {
            if let Some(child_children) = children_of(&child).filter(|children| !children.is_empty()) {
                // Go deeper
                frames.push(TreeWalkerFrame { visual: container, children, current_index: current_index + 1, tabled });
                container = child;
                children = child_children;
                current_index = 0;
                tabled = has_children_table(&container);
                continue; // Enter "recursion"
            }
        }

        // Haven't entered recursion, still call PostSubgraph and go to the next sibling
        visitor.post_subgraph(&child);
        current_index += 1;
    }

    pools.tree_walker_frame_stack_pool.return_stack(frames);
}

pub(super) struct TreeWalkContext {
    pub(super) transform: Matrix,
    pub(super) clip: LtrbRect,
    transform_stack: Vec<Matrix>,
    clip_stack: Vec<LtrbRect>,
}

impl TreeWalkContext {
    pub(super) fn new(pools: &CompositorPools, transform: Matrix, clip: LtrbRect) -> Self {
        Self {
            transform,
            clip,
            transform_stack: pools.matrix_stack_pool.rent(),
            clip_stack: pools.ltrb_rect_stack_pool.rent(),
        }
    }

    pub(super) fn push_transform(&mut self, m: Matrix) {
        self.transform_stack.push(self.transform);
        self.transform = m * self.transform;
    }

    pub(super) fn push_set_transform(&mut self, m: Matrix) {
        self.transform_stack.push(self.transform);
        self.transform = m;
    }

    pub(super) fn push_clip(&mut self, rect: LtrbRect) {
        self.clip_stack.push(self.clip);
        self.clip = self.clip.intersect_or_empty(rect);
    }

    pub(super) fn reset_clip(&mut self, rect: LtrbRect) {
        self.clip_stack.push(self.clip);
        self.clip = rect;
    }

    pub(super) fn pop_transform(&mut self) {
        self.transform = self.transform_stack.pop().expect("a transform was pushed");
    }

    pub(super) fn pop_clip(&mut self) {
        self.clip = self.clip_stack.pop().expect("a clip was pushed");
    }

    pub(super) fn dispose(self, pools: &CompositorPools) {
        pools.matrix_stack_pool.return_stack(self.transform_stack);
        pools.ltrb_rect_stack_pool.return_stack(self.clip_stack);
    }
}
