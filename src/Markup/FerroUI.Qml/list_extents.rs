//! Sparse prefix sums of deviations from the estimated ListView row size.
//! Cloning a transaction checkpoint shares the tree. Measuring a delegate copies
//! only the path to its leaf; returning to the estimate removes that leaf.
use std::rc::Rc;

#[derive(Clone, Default)]
pub(super) struct Extents {
    root: Option<Rc<Node>>,
    capacity: usize,
}

#[derive(Clone)]
enum Node {
    Measured {
        extent: f64,
        delta: f64,
    },
    Branch {
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
        sum: f64,
    },
}

impl Node {
    fn sum(&self) -> f64 {
        match self {
            Self::Measured { delta, .. } => *delta,
            Self::Branch { sum, .. } => *sum,
        }
    }

    fn branch(left: Option<Rc<Self>>, right: Option<Rc<Self>>) -> Option<Rc<Self>> {
        if left.is_none() && right.is_none() {
            return None;
        }
        let sum = left.as_deref().map_or(0., Self::sum) + right.as_deref().map_or(0., Self::sum);
        Some(Rc::new(Self::Branch { left, right, sum }))
    }

    fn replace(
        node: Option<&Self>,
        index: usize,
        width: usize,
        leaf: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        if width == 1 {
            return leaf;
        }
        let (left, right) = match node {
            Some(Self::Branch { left, right, .. }) => (left.clone(), right.clone()),
            None => (None, None),
            _ => unreachable!("a measurement occupies one row"),
        };
        let half = width / 2;
        if index < half {
            Self::branch(Self::replace(left.as_deref(), index, half, leaf), right)
        } else {
            Self::branch(
                left,
                Self::replace(right.as_deref(), index - half, half, leaf),
            )
        }
    }
}

impl Extents {
    pub fn set(&mut self, index: usize, extent: f64, estimate: f64) {
        self.capacity = self.capacity.max(1);
        while index >= self.capacity {
            self.root = Node::branch(self.root.take(), None);
            self.capacity *= 2;
        }
        let leaf = (extent != estimate).then(|| {
            Rc::new(Node::Measured {
                extent,
                delta: extent - estimate,
            })
        });
        self.root = Node::replace(self.root.as_deref(), index, self.capacity, leaf);
    }

    pub fn get(&self, mut index: usize) -> Option<f64> {
        if index >= self.capacity {
            return None;
        }
        let mut width = self.capacity;
        let mut node = self.root.as_deref()?;
        loop {
            match node {
                Node::Measured { extent, .. } => return Some(*extent),
                Node::Branch { left, right, .. } => {
                    width /= 2;
                    node = if index < width {
                        left.as_deref()?
                    } else {
                        index -= width;
                        right.as_deref()?
                    };
                }
            }
        }
    }

    pub fn before(&self, mut end: usize) -> f64 {
        let mut width = self.capacity;
        let mut next = self.root.as_deref();
        let mut sum = 0.;
        while let Some(node) = next {
            if end == 0 {
                break;
            }
            if end >= width {
                return sum + node.sum();
            }
            let Node::Branch { left, right, .. } = node else {
                unreachable!("partial leaf")
            };
            width /= 2;
            if end <= width {
                next = left.as_deref();
            } else {
                sum += left.as_deref().map_or(0., Node::sum);
                end -= width;
                next = right.as_deref();
            }
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_prefixes_and_checkpoint_preserve_opposite_deviations() {
        let mut sizes = Extents::default();
        sizes.set(0, 15., 20.);
        sizes.set(1, 25., 20.);
        sizes.set(1_000_000, 27., 20.);
        let saved = sizes.clone();
        assert_eq!(sizes.before(1), -5.);
        assert_eq!(sizes.before(1_000_000), 0.);
        assert_eq!(sizes.before(1_000_001), 7.);
        sizes.set(0, 20., 20.);
        assert_eq!(sizes.get(0), None);
        assert_eq!(sizes.before(1_000_001), 12.);
        assert_eq!(saved.get(0), Some(15.));
        assert_eq!(saved.before(1_000_001), 7.);
    }
}
