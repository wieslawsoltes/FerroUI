//! `SelectManyExtensions` (`Utilities/SelectManyExtensions.cs`): the items of trees, recursively.

/// The empty sequence for a missing one (`EmptyIfNull`).
pub fn empty_if_null<T>(source: Option<Vec<T>>) -> Vec<T> {
    source.unwrap_or_default()
}

/// The recursive selections of the items of a sequence and of their children.
pub trait SelectManyExtensions<T> {
    /// Every item and, after each one, its descendants, depth first
    /// (`RecursiveSelect(Func<TSource, IEnumerable<TSource>> childSelector)`).
    fn recursive_select(&self, child_selector: impl Fn(&T) -> Vec<T>) -> Vec<T>
    where
        T: Clone;

    /// [`recursive_select`](Self::recursive_select) projected by `selector`
    /// (`RecursiveSelect(childSelector, Func<TSource, TResult> selector)`).
    fn recursive_select_map<R>(&self, child_selector: impl Fn(&T) -> Vec<T>, selector: impl Fn(&T) -> R) -> Vec<R>;

    /// [`recursive_select`](Self::recursive_select) projected by `selector` with the index of each
    /// item among its siblings (`RecursiveSelect(childSelector, Func<TSource, int, TResult>)`).
    fn recursive_select_with_index<R>(&self, child_selector: impl Fn(&T) -> Vec<T>, selector: impl Fn(&T, usize) -> R) -> Vec<R>;

    /// [`recursive_select`](Self::recursive_select) projected by `selector` with the index of each
    /// item among its siblings and its depth (`RecursiveSelect(childSelector, Func<TSource, int,
    /// int, TResult>)`).
    fn recursive_select_with_index_and_depth<R>(&self, child_selector: impl Fn(&T) -> Vec<T>, selector: impl Fn(&T, usize, usize) -> R) -> Vec<R>;

    /// The items, then their children, then their grandchildren, ... level by level
    /// (`SelectManyRecursive`).
    fn select_many_recursive(&self, selector: impl Fn(&T) -> Vec<T>) -> Vec<T>
    where
        T: Clone;
}

fn recursive_select_core<T, R>(source: &[T], child_selector: &dyn Fn(&T) -> Vec<T>, selector: &dyn Fn(&T, usize, usize) -> R, depth: usize, results: &mut Vec<R>) {
    for (index, element) in source.iter().enumerate() {
        results.push(selector(element, index, depth));
        recursive_select_core(&child_selector(element), child_selector, selector, depth + 1, results);
    }
}

impl<T> SelectManyExtensions<T> for [T] {
    fn recursive_select(&self, child_selector: impl Fn(&T) -> Vec<T>) -> Vec<T>
    where
        T: Clone,
    {
        self.recursive_select_with_index_and_depth(child_selector, |element, _, _| element.clone())
    }

    fn recursive_select_map<R>(&self, child_selector: impl Fn(&T) -> Vec<T>, selector: impl Fn(&T) -> R) -> Vec<R> {
        self.recursive_select_with_index_and_depth(child_selector, |element, _, _| selector(element))
    }

    fn recursive_select_with_index<R>(&self, child_selector: impl Fn(&T) -> Vec<T>, selector: impl Fn(&T, usize) -> R) -> Vec<R> {
        self.recursive_select_with_index_and_depth(child_selector, |element, index, _| selector(element, index))
    }

    fn recursive_select_with_index_and_depth<R>(&self, child_selector: impl Fn(&T) -> Vec<T>, selector: impl Fn(&T, usize, usize) -> R) -> Vec<R> {
        let mut results = Vec::new();
        recursive_select_core(self, &child_selector, &selector, 0, &mut results);
        results
    }

    fn select_many_recursive(&self, selector: impl Fn(&T) -> Vec<T>) -> Vec<T>
    where
        T: Clone,
    {
        let mut results = Vec::new();
        let mut level: Vec<T> = self.to_vec();
        while !level.is_empty() {
            let next: Vec<T> = level.iter().flat_map(&selector).collect();
            results.append(&mut level);
            level = next;
        }
        results
    }
}
