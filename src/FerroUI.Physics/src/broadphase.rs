//! Spatial acceleration structures: SAH Dynamic AABB Tree (BVH) with fat AABBs and tree
//! rotations, plus a Uniform Spatial Hash Grid for dense homogeneous scenes.

use crate::math::{Aabb2, Ray2, Vec2};
use rustc_hash::FxHashMap;

const NULL_NODE: usize = usize::MAX;
const FAT_AABB_MARGIN: f32 = 0.08;
const VELOCITY_PREDICTION_MULTIPLIER: f32 = 2.0;

#[derive(Clone, Debug)]
struct BvhNode {
    aabb: Aabb2,
    user_index: usize,
    parent: usize,
    left: usize,
    right: usize,
    height: i32,
    alive: bool,
}

impl BvhNode {
    #[inline]
    fn is_leaf(&self) -> bool {
        self.left == NULL_NODE
    }
}

/// Dynamic Bounding Volume Hierarchy (SAH AABB Tree) modeled after Box2D/Bullet.
///
/// Supports O(log N) incremental insertion, removal, displacement-predicted proxy movement,
/// raycasting, and AABB overlap queries.
#[derive(Clone, Debug)]
pub struct DynamicBvh {
    nodes: Vec<BvhNode>,
    free_list: Vec<usize>,
    root: usize,
    query_stack: Vec<usize>,
}

impl Default for DynamicBvh {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicBvh {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_list: Vec::new(),
            root: NULL_NODE,
            query_stack: Vec::with_capacity(64),
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.free_list.clear();
        self.root = NULL_NODE;
    }

    /// Inserts a leaf proxy for `user_index` with `tight_aabb` and returns its proxy ID.
    pub fn insert(&mut self, user_index: usize, tight_aabb: Aabb2) -> usize {
        let fat = tight_aabb.expanded(FAT_AABB_MARGIN);
        let id = self.alloc_node(BvhNode {
            aabb: fat,
            user_index,
            parent: NULL_NODE,
            left: NULL_NODE,
            right: NULL_NODE,
            height: 0,
            alive: true,
        });
        self.insert_leaf(id);
        id
    }

    /// Removes a leaf proxy from the tree.
    pub fn remove(&mut self, proxy_id: usize) {
        if proxy_id >= self.nodes.len() || !self.nodes[proxy_id].alive {
            return;
        }
        self.remove_leaf(proxy_id);
        self.free_node(proxy_id);
    }

    /// Updates a proxy's tight AABB with predicted `displacement`.
    /// Returns `true` if the fat AABB was updated (re-inserted).
    pub fn update(&mut self, proxy_id: usize, tight_aabb: Aabb2, displacement: Vec2) -> bool {
        if proxy_id >= self.nodes.len() || !self.nodes[proxy_id].alive {
            return false;
        }
        if self.nodes[proxy_id].aabb.contains_aabb(tight_aabb) {
            return false;
        }

        self.remove_leaf(proxy_id);

        let mut fat = tight_aabb.expanded(FAT_AABB_MARGIN);
        let d = displacement * VELOCITY_PREDICTION_MULTIPLIER;
        if d.x < 0.0 {
            fat.min.x += d.x;
        } else {
            fat.max.x += d.x;
        }
        if d.y < 0.0 {
            fat.min.y += d.y;
        } else {
            fat.max.y += d.y;
        }

        self.nodes[proxy_id].aabb = fat;
        self.insert_leaf(proxy_id);
        true
    }

    /// Returns the fat AABB stored for a proxy.
    #[inline]
    pub fn fat_aabb(&self, proxy_id: usize) -> Option<Aabb2> {
        self.nodes.get(proxy_id).filter(|n| n.alive).map(|n| n.aabb)
    }

    /// Queries all leaf `user_index` values whose fat AABB intersects `query_aabb`.
    pub fn query_aabb(&mut self, query_aabb: Aabb2, mut callback: impl FnMut(usize)) {
        if self.root == NULL_NODE {
            return;
        }
        self.query_stack.clear();
        self.query_stack.push(self.root);
        while let Some(node_id) = self.query_stack.pop() {
            let node = &self.nodes[node_id];
            if !node.aabb.intersects(query_aabb) {
                continue;
            }
            if node.is_leaf() {
                callback(node.user_index);
            } else {
                self.query_stack.push(node.left);
                self.query_stack.push(node.right);
            }
        }
    }

    /// Generates all potentially overlapping `(min_idx, max_idx)` pairs among the provided active proxies.
    pub fn collect_pairs(&mut self, active_proxies: &[usize], out_pairs: &mut Vec<(usize, usize)>) {
        out_pairs.clear();
        if self.root == NULL_NODE {
            return;
        }
        for &proxy_id in active_proxies {
            if proxy_id >= self.nodes.len() || !self.nodes[proxy_id].alive {
                continue;
            }
            let query_aabb = self.nodes[proxy_id].aabb;
            let idx_a = self.nodes[proxy_id].user_index;
            self.query_stack.clear();
            self.query_stack.push(self.root);
            while let Some(node_id) = self.query_stack.pop() {
                let node = &self.nodes[node_id];
                if !node.aabb.intersects(query_aabb) {
                    continue;
                }
                if node.is_leaf() {
                    let idx_b = node.user_index;
                    if idx_a < idx_b {
                        out_pairs.push((idx_a, idx_b));
                    }
                } else {
                    self.query_stack.push(node.left);
                    self.query_stack.push(node.right);
                }
            }
        }
    }

    /// Traverses the BVH along `ray` up to `max_toi`, invoking `callback(user_index, max_toi)`
    /// which returns an updated (tighter) `max_toi` when a closer hit is found.
    pub fn query_ray(
        &self,
        ray: Ray2,
        mut max_toi: f32,
        mut callback: impl FnMut(usize, f32) -> f32,
    ) {
        if self.root == NULL_NODE {
            return;
        }
        let inv_dir = ray.inv_direction();
        // Most balanced trees fit entirely on the stack. Spill only for unusually
        // deep trees, instead of allocating for every ray in a multi-bounce cast.
        let mut stack = smallvec::SmallVec::<[usize; 64]>::new();
        stack.push(self.root);

        while let Some(node_id) = stack.pop() {
            let node = &self.nodes[node_id];
            if node
                .aabb
                .ray_intersect(ray.origin, inv_dir, max_toi)
                .is_none()
            {
                continue;
            }
            if node.is_leaf() {
                max_toi = callback(node.user_index, max_toi);
                if max_toi <= 0.0 {
                    break;
                }
            } else {
                let left_hit = self.nodes[node.left]
                    .aabb
                    .ray_intersect(ray.origin, inv_dir, max_toi);
                let right_hit = self.nodes[node.right]
                    .aabb
                    .ray_intersect(ray.origin, inv_dir, max_toi);
                match (left_hit, right_hit) {
                    (Some(tl), Some(tr)) => {
                        // Visit nearer child first so max_toi shrinks quickly
                        if tl < tr {
                            stack.push(node.right);
                            stack.push(node.left);
                        } else {
                            stack.push(node.left);
                            stack.push(node.right);
                        }
                    }
                    (Some(_), None) => stack.push(node.left),
                    (None, Some(_)) => stack.push(node.right),
                    (None, None) => {}
                }
            }
        }
    }

    fn alloc_node(&mut self, node: BvhNode) -> usize {
        if let Some(id) = self.free_list.pop() {
            self.nodes[id] = node;
            id
        } else {
            let id = self.nodes.len();
            self.nodes.push(node);
            id
        }
    }

    fn free_node(&mut self, id: usize) {
        self.nodes[id].alive = false;
        self.nodes[id].parent = NULL_NODE;
        self.nodes[id].left = NULL_NODE;
        self.nodes[id].right = NULL_NODE;
        self.nodes[id].height = -1;
        self.free_list.push(id);
    }

    fn insert_leaf(&mut self, leaf: usize) {
        if self.root == NULL_NODE {
            self.root = leaf;
            self.nodes[leaf].parent = NULL_NODE;
            return;
        }

        let leaf_aabb = self.nodes[leaf].aabb;
        let mut index = self.root;
        while !self.nodes[index].is_leaf() {
            let left = self.nodes[index].left;
            let right = self.nodes[index].right;
            let area = self.nodes[index].aabb.perimeter();
            let combined_area = self.nodes[index].aabb.union(leaf_aabb).perimeter();
            let cost = 2.0 * combined_area;
            let inheritance_cost = 2.0 * (combined_area - area);

            let cost_left = if self.nodes[left].is_leaf() {
                leaf_aabb.union(self.nodes[left].aabb).perimeter() + inheritance_cost
            } else {
                let new_area = leaf_aabb.union(self.nodes[left].aabb).perimeter();
                (new_area - self.nodes[left].aabb.perimeter()) + inheritance_cost
            };

            let cost_right = if self.nodes[right].is_leaf() {
                leaf_aabb.union(self.nodes[right].aabb).perimeter() + inheritance_cost
            } else {
                let new_area = leaf_aabb.union(self.nodes[right].aabb).perimeter();
                (new_area - self.nodes[right].aabb.perimeter()) + inheritance_cost
            };

            if cost < cost_left && cost < cost_right {
                break;
            }
            index = if cost_left < cost_right { left } else { right };
        }

        let sibling = index;
        let old_parent = self.nodes[sibling].parent;
        let new_parent = self.alloc_node(BvhNode {
            aabb: leaf_aabb.union(self.nodes[sibling].aabb),
            user_index: usize::MAX,
            parent: old_parent,
            left: sibling,
            right: leaf,
            height: self.nodes[sibling].height + 1,
            alive: true,
        });

        self.nodes[sibling].parent = new_parent;
        self.nodes[leaf].parent = new_parent;

        if old_parent != NULL_NODE {
            if self.nodes[old_parent].left == sibling {
                self.nodes[old_parent].left = new_parent;
            } else {
                self.nodes[old_parent].right = new_parent;
            }
        } else {
            self.root = new_parent;
        }

        self.refit_ancestors(new_parent);
    }

    fn remove_leaf(&mut self, leaf: usize) {
        if leaf == self.root {
            self.root = NULL_NODE;
            return;
        }
        let parent = self.nodes[leaf].parent;
        let grand_parent = self.nodes[parent].parent;
        let sibling = if self.nodes[parent].left == leaf {
            self.nodes[parent].right
        } else {
            self.nodes[parent].left
        };

        if grand_parent != NULL_NODE {
            if self.nodes[grand_parent].left == parent {
                self.nodes[grand_parent].left = sibling;
            } else {
                self.nodes[grand_parent].right = sibling;
            }
            self.nodes[sibling].parent = grand_parent;
            self.free_node(parent);
            self.refit_ancestors(grand_parent);
        } else {
            self.root = sibling;
            self.nodes[sibling].parent = NULL_NODE;
            self.free_node(parent);
        }
    }

    fn refit_ancestors(&mut self, mut index: usize) {
        while index != NULL_NODE {
            index = self.balance(index);
            let left = self.nodes[index].left;
            let right = self.nodes[index].right;
            self.nodes[index].height = 1 + self.nodes[left].height.max(self.nodes[right].height);
            self.nodes[index].aabb = self.nodes[left].aabb.union(self.nodes[right].aabb);
            index = self.nodes[index].parent;
        }
    }

    /// AVL-style tree rotation to keep the BVH balanced under dynamic motion.
    fn balance(&mut self, i_a: usize) -> usize {
        if self.nodes[i_a].is_leaf() || self.nodes[i_a].height < 2 {
            return i_a;
        }
        let i_b = self.nodes[i_a].left;
        let i_c = self.nodes[i_a].right;
        let balance = self.nodes[i_c].height - self.nodes[i_b].height;

        if balance > 1 {
            let i_f = self.nodes[i_c].left;
            let i_g = self.nodes[i_c].right;
            let parent_a = self.nodes[i_a].parent;

            self.nodes[i_c].left = i_a;
            self.nodes[i_c].parent = parent_a;
            self.nodes[i_a].parent = i_c;

            if parent_a != NULL_NODE {
                if self.nodes[parent_a].left == i_a {
                    self.nodes[parent_a].left = i_c;
                } else {
                    self.nodes[parent_a].right = i_c;
                }
            } else {
                self.root = i_c;
            }

            if self.nodes[i_f].height > self.nodes[i_g].height {
                self.nodes[i_c].right = i_f;
                self.nodes[i_a].right = i_g;
                self.nodes[i_g].parent = i_a;
                self.nodes[i_a].aabb = self.nodes[i_b].aabb.union(self.nodes[i_g].aabb);
                self.nodes[i_c].aabb = self.nodes[i_a].aabb.union(self.nodes[i_f].aabb);
                self.nodes[i_a].height = 1 + self.nodes[i_b].height.max(self.nodes[i_g].height);
                self.nodes[i_c].height = 1 + self.nodes[i_a].height.max(self.nodes[i_f].height);
            } else {
                self.nodes[i_c].right = i_g;
                self.nodes[i_a].right = i_f;
                self.nodes[i_f].parent = i_a;
                self.nodes[i_a].aabb = self.nodes[i_b].aabb.union(self.nodes[i_f].aabb);
                self.nodes[i_c].aabb = self.nodes[i_a].aabb.union(self.nodes[i_g].aabb);
                self.nodes[i_a].height = 1 + self.nodes[i_b].height.max(self.nodes[i_f].height);
                self.nodes[i_c].height = 1 + self.nodes[i_a].height.max(self.nodes[i_g].height);
            }
            return i_c;
        }

        if balance < -1 {
            let i_d = self.nodes[i_b].left;
            let i_e = self.nodes[i_b].right;
            let parent_a = self.nodes[i_a].parent;

            self.nodes[i_b].left = i_a;
            self.nodes[i_b].parent = parent_a;
            self.nodes[i_a].parent = i_b;

            if parent_a != NULL_NODE {
                if self.nodes[parent_a].left == i_a {
                    self.nodes[parent_a].left = i_b;
                } else {
                    self.nodes[parent_a].right = i_b;
                }
            } else {
                self.root = i_b;
            }

            if self.nodes[i_d].height > self.nodes[i_e].height {
                self.nodes[i_b].right = i_d;
                self.nodes[i_a].left = i_e;
                self.nodes[i_e].parent = i_a;
                self.nodes[i_a].aabb = self.nodes[i_c].aabb.union(self.nodes[i_e].aabb);
                self.nodes[i_b].aabb = self.nodes[i_a].aabb.union(self.nodes[i_d].aabb);
                self.nodes[i_a].height = 1 + self.nodes[i_c].height.max(self.nodes[i_e].height);
                self.nodes[i_b].height = 1 + self.nodes[i_a].height.max(self.nodes[i_d].height);
            } else {
                self.nodes[i_b].right = i_e;
                self.nodes[i_a].left = i_d;
                self.nodes[i_d].parent = i_a;
                self.nodes[i_a].aabb = self.nodes[i_c].aabb.union(self.nodes[i_d].aabb);
                self.nodes[i_b].aabb = self.nodes[i_a].aabb.union(self.nodes[i_e].aabb);
                self.nodes[i_a].height = 1 + self.nodes[i_c].height.max(self.nodes[i_d].height);
                self.nodes[i_b].height = 1 + self.nodes[i_a].height.max(self.nodes[i_e].height);
            }
            return i_b;
        }

        i_a
    }
}

/// Fast O(N) uniform spatial hash grid optimized for dense particle & granular scenes.
#[derive(Clone, Debug)]
pub struct SpatialHashGrid {
    cell_size: f32,
    inv_cell_size: f32,
    cells: FxHashMap<(i32, i32), Vec<u32>>,
    spare_buckets: Vec<Vec<u32>>,
}

impl SpatialHashGrid {
    pub fn new(cell_size: f32) -> Self {
        let cs = cell_size.max(1e-3);
        Self {
            cell_size: cs,
            inv_cell_size: 1.0 / cs,
            cells: FxHashMap::default(),
            spare_buckets: Vec::new(),
        }
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    pub fn clear(&mut self) {
        // Recycle storage without retaining every coordinate ever visited. A
        // moving fluid otherwise grows this map indefinitely and clear becomes
        // proportional to its entire history instead of the current population.
        for (_, mut bucket) in self.cells.drain() {
            bucket.clear();
            self.spare_buckets.push(bucket);
        }
    }

    #[inline]
    pub fn cell_coord(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x * self.inv_cell_size).floor() as i32,
            (y * self.inv_cell_size).floor() as i32,
        )
    }

    #[inline]
    pub fn insert_point(&mut self, id: u32, x: f32, y: f32) {
        let key = self.cell_coord(x, y);
        self.cells
            .entry(key)
            .or_insert_with(|| self.spare_buckets.pop().unwrap_or_default())
            .push(id);
    }

    #[inline]
    pub fn query_neighbors(&self, x: f32, y: f32, mut f: impl FnMut(u32)) {
        let (cx, cy) = self.cell_coord(x, y);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(bucket) = self.cells.get(&(cx + dx, cy + dy)) {
                    for &id in bucket {
                        f(id);
                    }
                }
            }
        }
    }
}
