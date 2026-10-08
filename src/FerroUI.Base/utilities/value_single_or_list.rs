/// A list like struct optimized for holding zero or one items.
///
/// Once more than one value has been added to this storage it will switch to
/// using a `Vec` internally.
pub struct ValueSingleOrList<T> {
    is_single_set: bool,
    single: Option<T>,
    list: Option<Vec<T>>,
}

impl<T> Default for ValueSingleOrList<T> {
    fn default() -> Self {
        Self { is_single_set: false, single: None, list: None }
    }
}

impl<T: PartialEq> ValueSingleOrList<T> {
    /// Single contained value. Only valid if [`is_single`](Self::is_single) is set.
    pub fn single(&self) -> Option<&T> {
        self.single.as_ref()
    }

    /// List of values.
    pub fn list(&self) -> Option<&Vec<T>> {
        self.list.as_ref()
    }

    /// If this struct is backed by a list.
    pub fn has_list(&self) -> bool {
        self.list.is_some()
    }

    /// If this struct contains only single value and storage was not promoted to a list.
    pub fn is_single(&self) -> bool {
        self.list.is_none() && self.is_single_set
    }

    /// Adds a value.
    pub fn add(&mut self, value: T) {
        if self.list.is_some() {
            if let Some(list) = &mut self.list {
                list.push(value);
            }
        } else if !self.is_single_set {
            self.single = Some(value);

            self.is_single_set = true;
        } else {
            let mut list = Vec::new();

            if let Some(single) = self.single.take() {
                list.push(single);
            }
            list.push(value);

            self.list = Some(list);
        }
    }

    /// Removes a value.
    pub fn remove(&mut self, value: &T) -> bool {
        if let Some(list) = &mut self.list {
            return match list.iter().position(|item| item == value) {
                Some(index) => {
                    list.remove(index);
                    true
                }
                None => false,
            };
        }

        if !self.is_single_set {
            return false;
        }

        if self.single.as_ref() == Some(value) {
            self.single = None;

            self.is_single_set = false;

            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    #[test]
    fn switches_from_a_single_value_to_a_list() {
        let mut target = ValueSingleOrList::<i32>::default();
        assert!(!target.is_single());
        assert!(!target.has_list());

        target.add(1);
        assert!(target.is_single());
        assert_eq!(Some(&1), target.single());

        target.add(2);
        assert!(!target.is_single());
        assert!(target.has_list());
        assert_eq!(None, target.single());
        assert_eq!(Some(&vec![1, 2]), target.list());

        assert!(target.remove(&1));
        assert!(!target.remove(&1));
        assert!(target.has_list());
        assert_eq!(Some(&vec![2]), target.list());
    }

    #[test]
    fn removes_the_single_value() {
        let mut target = ValueSingleOrList::<i32>::default();
        assert!(!target.remove(&1));

        target.add(1);
        assert!(!target.remove(&2));
        assert!(target.remove(&1));
        assert!(!target.is_single());
        assert!(!target.remove(&1));
    }
}
