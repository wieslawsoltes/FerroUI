//! A binding often reads one property, and many properties have no subscribers.
//! Keep those edges inline. Larger ordered sets are shared with undo snapshots
//! until an actual dependency change requires a copy.
use super::SlotId;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Default)]
pub(crate) enum Edges {
    #[default]
    Empty,
    One(SlotId),
    Many(Rc<BTreeSet<SlotId>>),
}

impl From<BTreeSet<SlotId>> for Edges {
    fn from(values: BTreeSet<SlotId>) -> Self {
        match values.len() {
            0 => Self::Empty,
            1 => Self::One(*values.first().expect("one edge")),
            _ => Self::Many(Rc::new(values)),
        }
    }
}

impl Edges {
    pub fn iter(&self) -> impl Iterator<Item = &SlotId> {
        let (one, many) = match self {
            Self::Empty => (None, None),
            Self::One(value) => (Some(value), None),
            Self::Many(values) => (None, Some(values.iter())),
        };
        one.into_iter().chain(many.into_iter().flatten())
    }

    pub fn matches(&self, values: &BTreeSet<SlotId>) -> bool {
        match self {
            Self::Empty => values.is_empty(),
            Self::One(key) => values.len() == 1 && values.contains(key),
            Self::Many(set) => set.as_ref() == values,
        }
    }

    pub fn contains(&self, key: &SlotId) -> bool {
        match self {
            Self::Empty => false,
            Self::One(value) => value == key,
            Self::Many(values) => values.contains(key),
        }
    }

    pub fn insert(&mut self, key: SlotId) {
        match self {
            Self::Empty => *self = Self::One(key),
            Self::One(old) if *old != key => *self = Self::Many(Rc::new([*old, key].into())),
            Self::One(_) => {}
            Self::Many(values) => {
                Rc::make_mut(values).insert(key);
            }
        }
    }

    pub fn remove(&mut self, key: &SlotId) {
        match self {
            Self::One(old) if old == key => *self = Self::Empty,
            Self::Many(values) => {
                let values = Rc::make_mut(values);
                if values.remove(key) && values.len() == 1 {
                    *self = Self::One(*values.first().expect("one remaining edge"));
                }
            }
            _ => {}
        }
    }
}
