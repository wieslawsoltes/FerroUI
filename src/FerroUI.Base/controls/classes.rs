use crate::collections::{CollectionChangedHandler, FerroList};
use crate::utilities::HandlerList;
use std::rc::Rc;

/// Exposes an interface for setting pseudoclasses on a [`Classes`] collection.
pub trait IPseudoClasses {
    /// Adds a pseudoclass to the collection.
    fn add_pseudo(&self, name: &str);
    /// Removes a pseudoclass from the collection.
    fn remove_pseudo(&self, name: &str) -> bool;
    /// Returns whether a pseudoclass is present in the collection.
    fn contains(&self, name: &str) -> bool;
}

/// Holds a collection of style classes for a styled element.
///
/// Similar to CSS, each control may have any number of styling classes
/// applied. Classes whose name starts with `:` are pseudoclasses and may only
/// be added or removed by the control itself, through [`IPseudoClasses`].
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone, Default)]
pub struct Classes(Rc<ClassesData>);

#[derive(Default)]
struct ClassesData {
    items: FerroList<String>,
    listeners: HandlerList<dyn Fn()>,
}

impl PartialEq for Classes {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl std::fmt::Debug for Classes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Classes({})", self.snapshot().join(" "))
    }
}

impl Classes {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a collection from the given class names.
    pub fn from_names<S: Into<String>>(items: impl IntoIterator<Item = S>) -> Self {
        Self(Rc::new(ClassesData {
            items: FerroList::from_items(items.into_iter().map(Into::into)),
            listeners: HandlerList::new(),
        }))
    }

    /// Whether two handles refer to the same collection.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Parses a space-separated list of class names.
    pub fn parse(s: &str) -> Classes {
        Self::from_names(s.split(' '))
    }

    /// The number of class names the collection can hold before its storage
    /// grows.
    pub fn capacity(&self) -> usize {
        self.0.items.capacity()
    }

    /// Sets the number of class names the collection can hold before its
    /// storage grows.
    ///
    /// # Panics
    /// Panics if `capacity` is less than the number of class names.
    pub fn set_capacity(&self, capacity: usize) {
        self.0.items.set_capacity(capacity);
    }

    /// The number of classes.
    pub fn count(&self) -> usize {
        self.0.items.count()
    }

    /// The class at `index`.
    pub fn get(&self, index: usize) -> String {
        self.0.items.get(index)
    }

    /// A snapshot of the classes.
    pub fn snapshot(&self) -> Rc<Vec<String>> {
        self.0.items.snapshot()
    }

    /// Returns whether a class is present in the collection.
    pub fn contains(&self, name: &str) -> bool {
        self.0.items.snapshot().iter().any(|c| c == name)
    }

    /// Subscribes to collection changes; see [`FerroList::add_collection_changed`].
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<String>>) -> u64 {
        self.0.items.add_collection_changed(handler)
    }

    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.0.items.remove_collection_changed(token)
    }

    /// Adds a style class to the collection. Only standard classes may be
    /// added via this method.
    pub fn add(&self, name: &str) {
        Self::panic_if_pseudoclass(name, "added");
        if !self.contains(name) {
            self.0.items.add(name.to_string());
            self.notify_changed();
        }
    }

    /// Adds style classes to the collection.
    pub fn add_range<S: AsRef<str>>(&self, names: impl IntoIterator<Item = S>) {
        let mut to_add: Vec<String> = Vec::new();
        for name in names {
            let name = name.as_ref();
            Self::panic_if_pseudoclass(name, "added");
            if !self.contains(name) && !to_add.iter().any(|n| n == name) {
                to_add.push(name.to_string());
            }
        }
        self.0.items.add_range(to_add);
        self.notify_changed();
    }

    /// Removes all non-pseudoclasses from the collection.
    pub fn clear(&self) {
        let mut i = self.0.items.count();
        while i > 0 {
            i -= 1;
            if !self.0.items.get(i).starts_with(':') {
                self.0.items.remove_at(i);
            }
        }
        self.notify_changed();
    }

    /// Inserts a style class into the collection.
    pub fn insert(&self, index: usize, name: &str) {
        Self::panic_if_pseudoclass(name, "added");
        if !self.contains(name) {
            self.0.items.insert(index, name.to_string());
            self.notify_changed();
        }
    }

    /// Inserts style classes into the collection.
    pub fn insert_range<S: AsRef<str>>(&self, index: usize, names: impl IntoIterator<Item = S>) {
        let mut to_insert: Vec<String> = Vec::new();
        for name in names {
            let name = name.as_ref();
            Self::panic_if_pseudoclass(name, "added");
            if !self.contains(name) {
                to_insert.push(name.to_string());
            }
        }
        if !to_insert.is_empty() {
            self.0.items.insert_range(index, to_insert);
            self.notify_changed();
        }
    }

    /// Removes a style class from the collection.
    pub fn remove(&self, name: &str) -> bool {
        Self::panic_if_pseudoclass(name, "removed");
        if self.0.items.remove(&name.to_string()) {
            self.notify_changed();
            true
        } else {
            false
        }
    }

    /// Removes style classes from the collection.
    pub fn remove_all<S: AsRef<str>>(&self, names: impl IntoIterator<Item = S>) {
        let mut to_remove: Vec<String> = Vec::new();
        for name in names {
            Self::panic_if_pseudoclass(name.as_ref(), "removed");
            to_remove.push(name.as_ref().to_string());
        }
        if !to_remove.is_empty() {
            self.0.items.remove_all(to_remove);
            self.notify_changed();
        }
    }

    /// Removes the style class at the specified index.
    pub fn remove_at(&self, index: usize) {
        let name = self.0.items.get(index);
        Self::panic_if_pseudoclass(&name, "removed");
        self.0.items.remove_at(index);
        self.notify_changed();
    }

    /// Removes a range of style classes from the collection.
    pub fn remove_range(&self, index: usize, count: usize) {
        self.0.items.remove_range(index, count);
        self.notify_changed();
    }

    /// Removes all non-pseudoclasses in the collection and adds a new set.
    pub fn replace<S: AsRef<str>>(&self, source: &[S]) {
        for name in source {
            Self::panic_if_pseudoclass(name.as_ref(), "added");
        }
        let to_remove: Vec<String> = self.0.items.snapshot().iter().filter(|n| !n.starts_with(':')).cloned().collect();
        if !to_remove.is_empty() {
            self.0.items.remove_all(to_remove);
        }
        self.0.items.add_range(source.iter().map(|s| s.as_ref().to_string()));
        self.notify_changed();
    }

    /// Adds or removes a style class.
    pub fn set(&self, name: &str, value: bool) {
        if value {
            if !self.contains(name) {
                self.add(name);
            }
        } else {
            self.remove(name);
        }
    }

    /// Registers a listener invoked whenever the classes change. Returns a
    /// token for [`remove_listener`](Self::remove_listener).
    pub fn add_listener(&self, listener: Rc<dyn Fn()>) -> u64 {
        self.0.listeners.add(listener)
    }

    pub fn remove_listener(&self, token: u64) -> bool {
        self.0.listeners.remove(token)
    }

    /// The number of registered listeners.
    pub fn listener_count(&self) -> usize {
        self.0.listeners.len()
    }

    fn notify_changed(&self) {
        if self.0.listeners.is_empty() {
            return;
        }
        for (_, listener) in self.0.listeners.snapshot().iter() {
            listener();
        }
    }

    fn panic_if_pseudoclass(name: &str, operation: &str) {
        if name.starts_with(':') {
            panic!("The pseudoclass '{name}' may only be {operation} by the control itself.");
        }
    }
}

impl IPseudoClasses for Classes {
    fn add_pseudo(&self, name: &str) {
        if !Classes::contains(self, name) {
            self.0.items.add(name.to_string());
            self.notify_changed();
        }
    }

    fn remove_pseudo(&self, name: &str) -> bool {
        if self.0.items.remove(&name.to_string()) {
            self.notify_changed();
            true
        } else {
            false
        }
    }

    fn contains(&self, name: &str) -> bool {
        Classes::contains(self, name)
    }
}

impl dyn IPseudoClasses + '_ {
    /// Adds or removes a pseudoclass depending on a boolean value.
    pub fn set(&self, name: &str, value: bool) -> bool {
        if value {
            self.add_pseudo(name);
        } else {
            self.remove_pseudo(name);
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn duplicates_should_not_be_added() {
        let target = Classes::new();
        target.add("foo");
        target.add("foo");
        assert_eq!(*target.snapshot(), vec!["foo".to_string()]);
    }

    #[test]
    #[should_panic]
    fn should_not_be_able_to_add_pseudoclass() {
        Classes::new().add(":foo");
    }

    #[test]
    fn should_be_able_to_add_pseudoclass_through_interface() {
        let target = Classes::new();
        target.add_pseudo(":foo");
        assert_eq!(*target.snapshot(), vec![":foo".to_string()]);
    }

    #[test]
    fn clear_should_not_remove_pseudoclasses() {
        let target = Classes::new();
        target.add("foo");
        target.add_pseudo(":bar");
        target.add("baz");
        target.clear();
        assert_eq!(*target.snapshot(), vec![":bar".to_string()]);
    }

    #[test]
    fn replace_should_not_replace_pseudoclasses() {
        let target = Classes::new();
        target.add("foo");
        target.add("bar");
        target.add_pseudo(":baz");
        target.replace(&["qux"]);
        assert_eq!(*target.snapshot(), vec![":baz".to_string(), "qux".to_string()]);
    }

    #[test]
    fn listeners_are_notified() {
        let target = Classes::new();
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        target.add_listener(Rc::new(move || c.set(c.get() + 1)));
        target.add("foo");
        target.remove("foo");
        assert_eq!(count.get(), 2);
    }
}
