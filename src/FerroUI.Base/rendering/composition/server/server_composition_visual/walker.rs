use super::ServerCompositionVisual;
use crate::platform::LtrbRect;
use crate::rendering::composition::server::CompositorPools;
use crate::Matrix;
use std::rc::Rc;

pub(super) trait IServerTreeVisitor {
    /// Returns whether the children are to be visited.
    fn pre_subgraph(&mut self, visual: &Rc<ServerCompositionVisual>) -> bool;
    fn post_subgraph(&mut self, visual: &Rc<ServerCompositionVisual>);
}

/// A position of the tree walk: a container and the index of its child to
/// visit next.
pub struct TreeWalkerFrame {
    visual: Rc<ServerCompositionVisual>,
    children: Rc<Vec<Rc<ServerCompositionVisual>>>,
    current_index: usize,
}

fn children_of(visual: &ServerCompositionVisual) -> Rc<Vec<Rc<ServerCompositionVisual>>> {
    match visual.children() {
        Some(children) => children.list(),
        None => Rc::new(Vec::new()),
    }
}

pub(super) fn walk<V: IServerTreeVisitor>(visitor: &mut V, root: &Rc<ServerCompositionVisual>, pools: &CompositorPools) {
    let mut frames = pools.tree_walker_frame_stack_pool.rent();

    let visit_children = visitor.pre_subgraph(root);
    let mut container = root.clone();
    let mut children = children_of(&container);
    if !visit_children || children.is_empty() {
        visitor.post_subgraph(root);
        pools.tree_walker_frame_stack_pool.return_stack(frames);
        return;
    }

    let mut current_index = 0usize;

    loop {
        if current_index >= children.len() {
            // Exiting "recursion"
            visitor.post_subgraph(&container);
            let Some(frame) = frames.pop() else { break };
            container = frame.visual;
            children = frame.children;
            current_index = frame.current_index;
            continue;
        }

        let child = children[current_index].clone();
        let visit_children = visitor.pre_subgraph(&child);
        if visit_children {
            let child_children = children_of(&child);
            if !child_children.is_empty() {
                // Go deeper
                frames.push(TreeWalkerFrame { visual: container, children, current_index: current_index + 1 });
                container = child;
                children = child_children;
                current_index = 0;
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
