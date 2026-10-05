use super::ICompositionHitTester;
use crate::media::IntersectionResult;
use crate::platform::LtrbRect;
use crate::rendering::composition::{CompositionTarget, CompositionVisual, CompositionVisualCollection};
use std::collections::HashMap;
use std::rc::Rc;

const NULL: i32 = -1;
const ORDER_BUCKET_SIZE: i32 = 32;
const FAT_BOUNDS_PADDING: f64 = 1.0;

type VisualFilter<'a> = Option<&'a dyn Fn(&Rc<CompositionVisual>) -> bool>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoundsState {
    Empty,
    Bounded,
    Unbounded,
}

#[derive(Clone)]
struct Node {
    bounds: LtrbRect,
    visual: Option<Rc<CompositionVisual>>,
    parent: i32,
    child1: i32,
    child2: i32,
    next: i32,
    height: i32,
    order: i32,
    bucket: i32,
}

impl Node {
    fn is_leaf(&self) -> bool {
        self.child1 == NULL
    }
}

struct Bucket {
    root: i32,
    unbounded: Option<Vec<Rc<CompositionVisual>>>,
    readback_revision: u64,
}

impl Bucket {
    fn new(root: i32) -> Self {
        Self { root, unbounded: None, readback_revision: 0 }
    }

    fn is_empty(&self) -> bool {
        self.root == NULL && self.unbounded.as_ref().is_none_or(Vec::is_empty)
    }
}

#[derive(Clone, Copy)]
struct Entry {
    order: i32,
    leaf: i32,
    revision: u64,
    is_unbounded: bool,
}

impl Entry {
    fn new(order: i32) -> Self {
        Self { order, leaf: NULL, revision: 0, is_unbounded: false }
    }
}

struct Candidate {
    visual: Rc<CompositionVisual>,
    order: i32,
}

fn key(visual: &Rc<CompositionVisual>) -> *const CompositionVisual {
    Rc::as_ptr(visual)
}

fn contains_rect(outer: &LtrbRect, rect: LtrbRect) -> bool {
    rect.left >= outer.left && rect.right <= outer.right && rect.top >= outer.top && rect.bottom <= outer.bottom
}

/// An index of the children of a visual for hit testing: the children are
/// grouped into buckets by their order, and each bucket is a dynamic tree
/// of axis-aligned bounding boxes.
pub struct CompositionHitTestAabbTree {
    children: Rc<CompositionVisualCollection>,
    entries: HashMap<*const CompositionVisual, Entry>,
    buckets: Vec<Bucket>,
    nodes: Vec<Node>,
    free_list: i32,
}

impl CompositionHitTestAabbTree {
    pub fn new(children: Rc<CompositionVisualCollection>) -> Self {
        let mut tree = Self {
            children: children.clone(),
            entries: HashMap::new(),
            buckets: Vec::new(),
            nodes: Vec::new(),
            free_list: NULL,
        };
        for (i, child) in children.items().iter().enumerate() {
            tree.update(child, i as i32);
        }
        tree
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.buckets.clear();
        self.nodes.clear();
        self.free_list = NULL;
    }

    pub fn update(&mut self, visual: &Rc<CompositionVisual>, order: i32) {
        let mut entry = match self.entries.get(&key(visual)).copied() {
            None => {
                let bucket_index = Self::get_bucket_index(order);
                self.get_or_create_bucket(bucket_index);
                Entry::new(order)
            }
            Some(mut entry) => {
                if entry.order != order {
                    self.update_order_core(visual, &mut entry, order);
                }
                entry
            }
        };

        let (state, bounds, revision) = Self::get_bounds_state(visual);
        self.update_bounds(visual, &mut entry, state, bounds, revision);
        self.entries.insert(key(visual), entry);
    }

    pub fn remove(&mut self, visual: &Rc<CompositionVisual>) {
        let Some(entry) = self.entries.remove(&key(visual)) else { return };

        if entry.leaf != NULL {
            self.destroy_leaf(entry.leaf);
        } else if entry.is_unbounded {
            self.remove_unbounded(visual, entry.order);
        }

        self.remove_bucket_if_empty(Self::get_bucket_index(entry.order));
    }

    pub fn update_order(&mut self, visual: &Rc<CompositionVisual>, order: i32) {
        let Some(mut entry) = self.entries.get(&key(visual)).copied() else {
            self.entries.insert(key(visual), Entry::new(order));
            return;
        };

        if entry.order != order {
            self.update_order_core(visual, &mut entry, order);
            self.entries.insert(key(visual), entry);
        }
    }

    fn update_order_core(&mut self, visual: &Rc<CompositionVisual>, entry: &mut Entry, order: i32) {
        let old_order = entry.order;
        let old_bucket = Self::get_bucket_index(old_order);
        let new_bucket = Self::get_bucket_index(order);
        entry.order = order;

        if entry.leaf != NULL {
            self.move_leaf(entry.leaf, order);
            return;
        }

        if entry.is_unbounded {
            self.move_unbounded(visual, old_order, order);
            return;
        }

        if old_bucket != new_bucket {
            self.remove_bucket_if_empty(old_bucket);
        }
    }

    /// Appends the children that may be hit by `input` to `results`,
    /// topmost first.
    pub fn query<H: ICompositionHitTester>(
        &mut self,
        input: &H::Input,
        results: &mut Vec<Rc<CompositionVisual>>,
        readback_revision: u64,
    ) {
        let mut candidates: Vec<Candidate> = Vec::with_capacity(ORDER_BUCKET_SIZE as usize);
        let mut stack: Vec<i32> = Vec::with_capacity(16);

        for i in (0..self.buckets.len()).rev() {
            self.update_bucket(i, readback_revision);
            // Updating the bounds can drop trailing buckets.
            let Some(bucket) = self.buckets.get(i) else { continue };
            if bucket.is_empty() {
                continue;
            }

            candidates.clear();
            stack.clear();
            self.query_bucket::<H>(i, input, &mut candidates, &mut stack);
            // Higher child order is topmost, sort descending.
            candidates.sort_by(|left, right| right.order.cmp(&left.order));

            for candidate in candidates.drain(..) {
                results.push(candidate.visual);
            }
        }
    }

    /// Finds the topmost hit among the children that may be hit by
    /// `input`.
    pub fn query_first<H: ICompositionHitTester>(
        &mut self,
        target: &CompositionTarget,
        input: &H::Input,
        filter: VisualFilter<'_>,
        result_filter: VisualFilter<'_>,
        readback_revision: u64,
    ) -> (Option<Rc<CompositionVisual>>, IntersectionResult) {
        let mut intersection_result = IntersectionResult::NotCalculated;
        let mut candidates: Vec<Candidate> = Vec::with_capacity(ORDER_BUCKET_SIZE as usize);
        let mut stack: Vec<i32> = Vec::with_capacity(16);

        for i in (0..self.buckets.len()).rev() {
            self.update_bucket(i, readback_revision);
            let Some(bucket) = self.buckets.get(i) else { continue };
            if bucket.is_empty() {
                continue;
            }

            candidates.clear();
            stack.clear();
            self.query_bucket::<H>(i, input, &mut candidates, &mut stack);
            candidates.sort_by(|left, right| right.order.cmp(&left.order));

            for candidate in candidates.drain(..) {
                let (hit, result) = target.hit_test_first_core::<H>(&candidate.visual, input, filter, result_filter);
                intersection_result = result;
                if hit.is_some() {
                    return (hit, intersection_result);
                }
            }
        }

        (None, intersection_result)
    }

    fn update_bucket(&mut self, bucket_index: usize, readback_revision: u64) {
        if self.buckets[bucket_index].readback_revision == readback_revision {
            return;
        }

        let children = self.children.items();
        let end = children.len().min((bucket_index + 1) * ORDER_BUCKET_SIZE as usize);
        for visual in children.iter().take(end).skip(bucket_index * ORDER_BUCKET_SIZE as usize) {
            let Some(mut entry) = self.entries.get(&key(visual)).copied() else { continue };
            let (state, bounds, revision) = Self::get_bounds_state(visual);
            if entry.revision != revision {
                self.update_bounds(visual, &mut entry, state, bounds, revision);
                self.entries.insert(key(visual), entry);
            }
        }

        if let Some(bucket) = self.buckets.get_mut(bucket_index) {
            bucket.readback_revision = readback_revision;
        }
    }

    fn update_bounds(
        &mut self,
        visual: &Rc<CompositionVisual>,
        entry: &mut Entry,
        state: BoundsState,
        bounds: LtrbRect,
        revision: u64,
    ) {
        entry.revision = revision;

        if entry.leaf != NULL {
            if state == BoundsState::Bounded {
                self.update_leaf(entry.leaf, bounds, entry.order);
            } else {
                self.destroy_leaf(entry.leaf);
                entry.leaf = NULL;
                if state == BoundsState::Unbounded {
                    self.add_unbounded(visual, entry.order, entry);
                }
            }

            return;
        }

        if entry.is_unbounded {
            if state == BoundsState::Unbounded {
                return;
            }

            self.remove_unbounded(visual, entry.order);
            entry.is_unbounded = false;
            if state == BoundsState::Bounded {
                entry.leaf = self.create_leaf(visual, bounds, entry.order);
            }

            return;
        }

        if state == BoundsState::Bounded {
            entry.leaf = self.create_leaf(visual, bounds, entry.order);
        } else if state == BoundsState::Unbounded {
            self.add_unbounded(visual, entry.order, entry);
        }
    }

    fn query_bucket<H: ICompositionHitTester>(
        &self,
        bucket_index: usize,
        input: &H::Input,
        candidates: &mut Vec<Candidate>,
        stack: &mut Vec<i32>,
    ) {
        let bucket = &self.buckets[bucket_index];
        if bucket.root != NULL {
            stack.push(bucket.root);
        }

        while let Some(node_index) = stack.pop() {
            let node = &self.nodes[node_index as usize];
            if !H::transformed_sub_tree_bounds_match(node.bounds, input) {
                continue;
            }

            if node.is_leaf() {
                if let Some(visual) = &node.visual {
                    candidates.push(Candidate { visual: visual.clone(), order: node.order });
                }
            } else {
                if node.child1 != NULL {
                    stack.push(node.child1);
                }
                if node.child2 != NULL {
                    stack.push(node.child2);
                }
            }
        }

        if let Some(unbounded) = &bucket.unbounded {
            for visual in unbounded {
                if let Some(entry) = self.entries.get(&key(visual)) {
                    candidates.push(Candidate { visual: visual.clone(), order: entry.order });
                }
            }
        }
    }

    fn get_bucket_index(order: i32) -> i32 {
        order / ORDER_BUCKET_SIZE
    }

    fn get_or_create_bucket(&mut self, bucket_index: i32) -> &mut Bucket {
        while self.buckets.len() as i32 <= bucket_index {
            self.buckets.push(Bucket::new(NULL));
        }
        &mut self.buckets[bucket_index as usize]
    }

    fn remove_bucket_if_empty(&mut self, bucket_index: i32) {
        if bucket_index >= self.buckets.len() as i32 {
            return;
        }

        let children_count = self.children.count() as i32;
        let last_required_bucket_index =
            if children_count == 0 { NULL } else { Self::get_bucket_index(children_count - 1) };

        let remove_trailing_bucket = {
            let count = self.buckets.len() as i32;
            let bucket = &mut self.buckets[bucket_index as usize];
            if bucket.unbounded.as_ref().is_some_and(Vec::is_empty) {
                bucket.unbounded = None;
            }

            bucket.is_empty() && bucket_index == count - 1 && bucket_index > last_required_bucket_index
        };

        if !remove_trailing_bucket {
            return;
        }

        loop {
            self.buckets.pop();
            let count = self.buckets.len() as i32;
            if !(count > 0 && count - 1 > last_required_bucket_index && self.buckets[(count - 1) as usize].is_empty()) {
                break;
            }
        }
    }

    fn create_leaf(&mut self, visual: &Rc<CompositionVisual>, bounds: LtrbRect, order: i32) -> i32 {
        let leaf = self.allocate_node();
        {
            let node = &mut self.nodes[leaf as usize];
            node.bounds = Self::fatten(bounds);
            node.visual = Some(visual.clone());
            node.order = order;
            node.bucket = Self::get_bucket_index(order);
            node.height = 0;
        }

        self.insert_leaf(leaf);
        leaf
    }

    fn destroy_leaf(&mut self, leaf: i32) {
        self.remove_leaf(leaf);
        self.free_node(leaf);
    }

    fn update_leaf(&mut self, leaf: i32, bounds: LtrbRect, order: i32) {
        let bucket = Self::get_bucket_index(order);
        {
            let node = &mut self.nodes[leaf as usize];
            // If the exact bounds still fit inside the fat bounds, the tree shape can stay unchanged.
            if node.bucket == bucket && contains_rect(&node.bounds, bounds) {
                node.order = order;
                return;
            }
        }

        self.remove_leaf(leaf);

        {
            let removed_node = &mut self.nodes[leaf as usize];
            removed_node.bounds = Self::fatten(bounds);
            removed_node.bucket = bucket;
            removed_node.order = order;
        }

        self.insert_leaf(leaf);
    }

    fn move_leaf(&mut self, leaf: i32, order: i32) {
        let bucket = Self::get_bucket_index(order);
        {
            let node = &mut self.nodes[leaf as usize];
            if node.bucket == bucket {
                node.order = order;
                return;
            }
        }

        self.remove_leaf(leaf);

        {
            let removed_node = &mut self.nodes[leaf as usize];
            removed_node.order = order;
            removed_node.bucket = bucket;
        }

        self.insert_leaf(leaf);
    }

    fn allocate_node(&mut self) -> i32 {
        if self.free_list == NULL {
            self.nodes.push(Node {
                bounds: LtrbRect::default(),
                visual: None,
                parent: NULL,
                child1: NULL,
                child2: NULL,
                next: NULL,
                height: 0,
                order: 0,
                bucket: 0,
            });
            return self.nodes.len() as i32 - 1;
        }

        let index = self.free_list;
        let node = &mut self.nodes[index as usize];
        self.free_list = node.next;
        node.parent = NULL;
        node.child1 = NULL;
        node.child2 = NULL;
        node.next = NULL;
        node.height = 0;
        node.visual = None;
        node.order = 0;
        node.bucket = 0;
        index
    }

    fn free_node(&mut self, index: i32) {
        let node = &mut self.nodes[index as usize];
        node.next = self.free_list;
        node.parent = NULL;
        node.child1 = NULL;
        node.child2 = NULL;
        node.height = -1;
        node.visual = None;
        node.order = 0;
        node.bucket = 0;
        self.free_list = index;
    }

    fn insert_leaf(&mut self, leaf: i32) {
        let bucket_index = self.nodes[leaf as usize].bucket;
        let root = self.get_or_create_bucket(bucket_index).root;
        if root == NULL {
            self.buckets[bucket_index as usize].root = leaf;
            self.nodes[leaf as usize].parent = NULL;
            return;
        }

        let leaf_bounds = self.nodes[leaf as usize].bounds;
        let sibling = self.find_best_sibling(root, leaf_bounds);
        let old_parent = self.nodes[sibling as usize].parent;
        let new_parent = self.allocate_node();

        // Insert by replacing the chosen sibling with a new internal parent:
        //
        // Before: oldParent        After: oldParent
        //             |                       |
        //          sibling                newParent
        //                                  /      \
        //                             sibling    leaf
        let sibling_bounds = self.nodes[sibling as usize].bounds;
        let sibling_height = self.nodes[sibling as usize].height;
        {
            let parent_node = &mut self.nodes[new_parent as usize];
            parent_node.parent = old_parent;
            parent_node.bounds = leaf_bounds.union(sibling_bounds);
            parent_node.height = sibling_height + 1;
            parent_node.child1 = sibling;
            parent_node.child2 = leaf;
            parent_node.visual = None;
            parent_node.bucket = bucket_index;
        }

        self.nodes[sibling as usize].parent = new_parent;
        self.nodes[leaf as usize].parent = new_parent;

        if old_parent == NULL {
            self.buckets[bucket_index as usize].root = new_parent;
        } else {
            let old_parent_node = &mut self.nodes[old_parent as usize];
            if old_parent_node.child1 == sibling {
                old_parent_node.child1 = new_parent;
            } else {
                old_parent_node.child2 = new_parent;
            }
        }

        self.fix_ancestors(new_parent);
    }

    fn find_best_sibling(&self, root: i32, leaf_bounds: LtrbRect) -> i32 {
        let mut index = root;
        while !self.nodes[index as usize].is_leaf() {
            let node = &self.nodes[index as usize];
            let child1 = node.child1;
            let child2 = node.child2;

            let area = Self::perimeter(node.bounds);
            let combined_area = Self::perimeter(node.bounds.union(leaf_bounds));
            let cost = 2.0 * combined_area;
            let inheritance_cost = 2.0 * (combined_area - area);

            let cost1 = self.get_insertion_cost(child1, leaf_bounds, inheritance_cost);
            let cost2 = self.get_insertion_cost(child2, leaf_bounds, inheritance_cost);

            // Stop descending when pairing with this internal node is already cheaper.
            if cost < cost1 && cost < cost2 {
                break;
            }

            index = if cost1 < cost2 { child1 } else { child2 };
        }

        index
    }

    fn get_insertion_cost(&self, node_index: i32, leaf_bounds: LtrbRect, inheritance_cost: f64) -> f64 {
        let node = &self.nodes[node_index as usize];
        let union = node.bounds.union(leaf_bounds);
        if node.is_leaf() {
            return Self::perimeter(union) + inheritance_cost;
        }

        Self::perimeter(union) - Self::perimeter(node.bounds) + inheritance_cost
    }

    fn remove_leaf(&mut self, leaf: i32) {
        let bucket_index = self.nodes[leaf as usize].bucket;
        let removed_root = {
            let bucket = &mut self.buckets[bucket_index as usize];
            if leaf == bucket.root {
                bucket.root = NULL;
                true
            } else {
                false
            }
        };

        if removed_root {
            self.remove_bucket_if_empty(bucket_index);
            return;
        }

        let parent = self.nodes[leaf as usize].parent;
        let (grand_parent, sibling) = {
            let parent_node = &self.nodes[parent as usize];
            (parent_node.parent, if parent_node.child1 == leaf { parent_node.child2 } else { parent_node.child1 })
        };

        // Collapse the removed leaf's parent and promote the sibling.
        if grand_parent != NULL {
            // Before: grandParent        After: grandParent
            //             |                         |
            //           parent                   sibling
            //           /    \
            //        leaf  sibling
            {
                let grand_parent_node = &mut self.nodes[grand_parent as usize];
                if grand_parent_node.child1 == parent {
                    grand_parent_node.child1 = sibling;
                } else {
                    grand_parent_node.child2 = sibling;
                }
            }

            self.nodes[sibling as usize].parent = grand_parent;
            self.free_node(parent);
            self.fix_ancestors(grand_parent);
        } else {
            // If the parent was the root, the sibling becomes the new root.
            //
            // Before: parent(root)       After: sibling(root)
            //          /    \
            //       leaf  sibling
            self.buckets[bucket_index as usize].root = sibling;
            self.nodes[sibling as usize].parent = NULL;
            self.free_node(parent);
        }

        self.nodes[leaf as usize].parent = NULL;

        self.remove_bucket_if_empty(bucket_index);
    }

    fn fix_ancestors(&mut self, mut index: i32) {
        while index != NULL {
            index = self.balance(index);

            let (child1, child2) = {
                let node = &self.nodes[index as usize];
                (node.child1, node.child2)
            };
            let (bounds1, height1) = {
                let child = &self.nodes[child1 as usize];
                (child.bounds, child.height)
            };
            let (bounds2, height2) = {
                let child = &self.nodes[child2 as usize];
                (child.bounds, child.height)
            };

            let node = &mut self.nodes[index as usize];
            // Ancestor bounds always cover both children after insert/remove/rotate.
            node.bounds = bounds1.union(bounds2);
            node.height = 1 + height1.max(height2);
            index = node.parent;
        }
    }

    fn balance(&mut self, index_a: i32) -> i32 {
        let (is_leaf, height, index_b, index_c) = {
            let a = &self.nodes[index_a as usize];
            (a.is_leaf(), a.height, a.child1, a.child2)
        };
        if is_leaf || height < 2 {
            return index_a;
        }

        let balance = self.nodes[index_c as usize].height - self.nodes[index_b as usize].height;

        // The right subtree is heavier than the left. Rotate C up.
        if balance > 1 {
            return self.rotate_c_up(index_a, index_b, index_c);
        }

        // The left subtree is heavier than the right. Rotate B up.
        if balance < -1 {
            return self.rotate_b_up(index_a, index_b, index_c);
        }

        index_a
    }

    fn bounds_and_height(&self, index: i32) -> (LtrbRect, i32) {
        let node = &self.nodes[index as usize];
        (node.bounds, node.height)
    }

    fn rotate_c_up(&mut self, index_a: i32, index_b: i32, index_c: i32) -> i32 {
        // Rotate C above A:
        //
        // Before:      A                   After, if F taller:  C
        //             / \                                      / \
        //            B   C                                    A   F
        //               / \                                  / \
        //              F   G                                B   G
        //
        //                                  After, otherwise:    C
        //                                                      / \
        //                                                     A   G
        //                                                    / \
        //                                                   B   F
        let index_f = self.nodes[index_c as usize].child1;
        let index_g = self.nodes[index_c as usize].child2;
        let a_parent = self.nodes[index_a as usize].parent;

        self.nodes[index_c as usize].child1 = index_a;
        self.nodes[index_c as usize].parent = a_parent;
        self.nodes[index_a as usize].parent = index_c;

        // C takes A's old place in the parent chain.
        self.replace_parent_child(index_a, index_c, a_parent);

        let (f_bounds, f_height) = self.bounds_and_height(index_f);
        let (g_bounds, g_height) = self.bounds_and_height(index_g);
        let (b_bounds, b_height) = self.bounds_and_height(index_b);

        // Keep the taller C child with C, and move the other child under A.
        if f_height > g_height {
            self.nodes[index_c as usize].child2 = index_f;
            self.nodes[index_a as usize].child2 = index_g;
            self.nodes[index_g as usize].parent = index_a;

            let a_bounds = b_bounds.union(g_bounds);
            let a_height = 1 + b_height.max(g_height);
            self.nodes[index_a as usize].bounds = a_bounds;
            self.nodes[index_c as usize].bounds = a_bounds.union(f_bounds);
            self.nodes[index_a as usize].height = a_height;
            self.nodes[index_c as usize].height = 1 + a_height.max(f_height);
        } else {
            self.nodes[index_c as usize].child2 = index_g;
            self.nodes[index_a as usize].child2 = index_f;
            self.nodes[index_f as usize].parent = index_a;

            let a_bounds = b_bounds.union(f_bounds);
            let a_height = 1 + b_height.max(f_height);
            self.nodes[index_a as usize].bounds = a_bounds;
            self.nodes[index_c as usize].bounds = a_bounds.union(g_bounds);
            self.nodes[index_a as usize].height = a_height;
            self.nodes[index_c as usize].height = 1 + a_height.max(g_height);
        }

        index_c
    }

    fn rotate_b_up(&mut self, index_a: i32, index_b: i32, index_c: i32) -> i32 {
        // Rotate B above A:
        //
        // Before:      A                  After, if D taller:   B
        //             / \                                      / \
        //            B   C                                    A   D
        //           / \                                      / \
        //          D   E                                    E   C
        //
        //                                 After, otherwise:     B
        //                                                      / \
        //                                                     A   E
        //                                                    / \
        //                                                   D   C
        let index_d = self.nodes[index_b as usize].child1;
        let index_e = self.nodes[index_b as usize].child2;
        let a_parent = self.nodes[index_a as usize].parent;

        self.nodes[index_b as usize].child1 = index_a;
        self.nodes[index_b as usize].parent = a_parent;
        self.nodes[index_a as usize].parent = index_b;

        // B takes A's old place in the parent chain.
        self.replace_parent_child(index_a, index_b, a_parent);

        let (d_bounds, d_height) = self.bounds_and_height(index_d);
        let (e_bounds, e_height) = self.bounds_and_height(index_e);
        let (c_bounds, c_height) = self.bounds_and_height(index_c);

        // Keep the taller B child with B, and move the other child under A.
        if d_height > e_height {
            self.nodes[index_b as usize].child2 = index_d;
            self.nodes[index_a as usize].child1 = index_e;
            self.nodes[index_e as usize].parent = index_a;

            let a_bounds = c_bounds.union(e_bounds);
            let a_height = 1 + c_height.max(e_height);
            self.nodes[index_a as usize].bounds = a_bounds;
            self.nodes[index_b as usize].bounds = a_bounds.union(d_bounds);
            self.nodes[index_a as usize].height = a_height;
            self.nodes[index_b as usize].height = 1 + a_height.max(d_height);
        } else {
            self.nodes[index_b as usize].child2 = index_e;
            self.nodes[index_a as usize].child1 = index_d;
            self.nodes[index_d as usize].parent = index_a;

            let a_bounds = c_bounds.union(d_bounds);
            let a_height = 1 + c_height.max(d_height);
            self.nodes[index_a as usize].bounds = a_bounds;
            self.nodes[index_b as usize].bounds = a_bounds.union(e_bounds);
            self.nodes[index_a as usize].height = a_height;
            self.nodes[index_b as usize].height = 1 + a_height.max(e_height);
        }

        index_b
    }

    fn replace_parent_child(&mut self, old_child: i32, new_child: i32, parent: i32) {
        if parent == NULL {
            let bucket = self.nodes[new_child as usize].bucket;
            self.buckets[bucket as usize].root = new_child;
            return;
        }

        let parent_node = &mut self.nodes[parent as usize];
        if parent_node.child1 == old_child {
            parent_node.child1 = new_child;
        } else {
            parent_node.child2 = new_child;
        }
    }

    fn add_unbounded(&mut self, visual: &Rc<CompositionVisual>, order: i32, entry: &mut Entry) {
        let bucket_index = Self::get_bucket_index(order);
        let bucket = self.get_or_create_bucket(bucket_index);
        bucket.unbounded.get_or_insert_with(Vec::new).push(visual.clone());
        entry.is_unbounded = true;
    }

    fn move_unbounded(&mut self, visual: &Rc<CompositionVisual>, old_order: i32, order: i32) {
        let old_bucket_index = Self::get_bucket_index(old_order);
        let new_bucket_index = Self::get_bucket_index(order);
        if old_bucket_index == new_bucket_index {
            return;
        }

        self.remove_unbounded(visual, old_order);
        let new_bucket = self.get_or_create_bucket(new_bucket_index);
        new_bucket.unbounded.get_or_insert_with(Vec::new).push(visual.clone());
    }

    fn remove_unbounded(&mut self, visual: &Rc<CompositionVisual>, order: i32) {
        let bucket_index = Self::get_bucket_index(order);
        if let Some(unbounded) = self.buckets.get_mut(bucket_index as usize).and_then(|b| b.unbounded.as_mut()) {
            if let Some(index) = unbounded.iter().position(|v| Rc::ptr_eq(v, visual)) {
                unbounded.remove(index);
            }
        }

        self.remove_bucket_if_empty(bucket_index);
    }

    fn get_bounds_state(visual: &Rc<CompositionVisual>) -> (BoundsState, LtrbRect, u64) {
        let Some(readback) = visual.try_get_valid_readback() else {
            return (BoundsState::Empty, LtrbRect::default(), 0);
        };

        let revision = readback.revision;
        if visual.disable_sub_tree_bounds_hit_test_optimization() {
            return (BoundsState::Unbounded, LtrbRect::default(), revision);
        }

        match readback.transformed_subtree_bounds {
            Some(subtree_bounds) if !subtree_bounds.is_zero_size() => (BoundsState::Bounded, subtree_bounds, revision),
            _ => (BoundsState::Empty, LtrbRect::default(), revision),
        }
    }

    // Fatten the bounds by a small amount to avoid having to update the tree for every tiny movement.
    fn fatten(bounds: LtrbRect) -> LtrbRect {
        LtrbRect::new(
            bounds.left - FAT_BOUNDS_PADDING,
            bounds.top - FAT_BOUNDS_PADDING,
            bounds.right + FAT_BOUNDS_PADDING,
            bounds.bottom + FAT_BOUNDS_PADDING,
        )
    }

    fn perimeter(bounds: LtrbRect) -> f64 {
        2.0 * (bounds.width() + bounds.height())
    }

    /// The number of buckets (for tests).
    #[cfg(test)]
    pub(crate) fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    /// Checks the structural invariants of the trees (for tests).
    #[cfg(test)]
    pub(crate) fn validate(&self) {
        for (bucket_index, bucket) in self.buckets.iter().enumerate() {
            if bucket.root != NULL {
                assert_eq!(self.nodes[bucket.root as usize].parent, NULL);
                self.validate_node(bucket.root, bucket_index as i32);
            }
        }
    }

    #[cfg(test)]
    fn validate_node(&self, index: i32, bucket: i32) -> i32 {
        let node = &self.nodes[index as usize];
        assert_eq!(node.bucket, bucket);
        if node.is_leaf() {
            assert!(node.visual.is_some());
            assert_eq!(node.height, 0);
            return 0;
        }
        let (c1, c2) = (&self.nodes[node.child1 as usize], &self.nodes[node.child2 as usize]);
        assert_eq!(c1.parent, index);
        assert_eq!(c2.parent, index);
        assert!(contains_rect(&node.bounds, c1.bounds) && contains_rect(&node.bounds, c2.bounds));
        let height = 1 + self.validate_node(node.child1, bucket).max(self.validate_node(node.child2, bucket));
        assert_eq!(node.height, height);
        height
    }
}
