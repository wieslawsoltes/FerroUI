//! Ribbon traversal portions: Copyright Wiesław Šoltés, MIT.
//! Shared repeatable input for actual ribbon collection/binding properties.
//! A source has reference identity and yields ordinary Rust iterators. It supplies
//! no LINQ operators, CLR enumerator state, Current/Reset, or panic recovery.
use crate::collections::ObservableCollection;
use std::rc::Rc;
pub type RibbonIterator<T> = Box<dyn Iterator<Item = T>>;
type Factory<T> = Rc<dyn Fn() -> RibbonIterator<T>>;
pub struct RibbonItemsSource<T> {
    factory: Factory<T>,
}
impl<T> Clone for RibbonItemsSource<T> {
    fn clone(&self) -> Self {
        Self {
            factory: self.factory.clone(),
        }
    }
}
impl<T> PartialEq for RibbonItemsSource<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.factory, &other.factory)
    }
}
impl<T> Eq for RibbonItemsSource<T> {}
impl<T: 'static> RibbonItemsSource<T> {
    pub fn new(factory: impl Fn() -> RibbonIterator<T> + 'static) -> Self {
        Self {
            factory: Rc::new(factory),
        }
    }
    pub fn iter(&self) -> RibbonIterator<T> {
        let factory = self.factory.clone();
        let mut values = (factory)();
        // Keep a live binding/query owner until its iterator is dropped, including
        // after exhaustion. Rust closures own this lifetime; no runtime cursor.
        Box::new(std::iter::from_fn(move || {
            let _owner = &factory;
            values.next()
        }))
    }
    pub fn from_collection(items: ObservableCollection<T>) -> Self
    where
        T: Clone + PartialEq,
    {
        Self::new(move || Box::new(items.iter()))
    }
    pub fn from_values(values: Vec<T>) -> Self
    where
        T: Clone,
    {
        Self::new(move || Box::new(values.clone().into_iter()))
    }
    /// Query acquisition starts at the first next(), preserving live ribbon
    /// collections without a hand-written compiler iterator state machine.
    pub fn deferred(factory: impl Fn() -> RibbonIterator<T> + 'static) -> Self {
        let factory: Factory<T> = Rc::new(factory);
        Self::new(move || {
            let factory = factory.clone();
            Box::new(std::iter::once_with(move || (factory)()).flatten())
        })
    }
}
impl<T: 'static> IntoIterator for RibbonItemsSource<T> {
    type Item = T;
    type IntoIter = RibbonIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<T: 'static> IntoIterator for &RibbonItemsSource<T> {
    type Item = T;
    type IntoIter = RibbonIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Retain the current parent while recursively enumerating its children. The child
// lookup occurs on the NEXT step after yielding that parent, including for nulls.
pub(crate) fn tree<T: Clone + 'static>(
    source: Option<RibbonItemsSource<T>>,
    children: impl Fn(&T) -> Option<RibbonItemsSource<T>> + 'static,
) -> RibbonItemsSource<T> {
    type Children<T> = Rc<dyn Fn(&T) -> Option<RibbonItemsSource<T>>>;
    struct Tree<T> {
        child: Option<RibbonIterator<T>>,
        input: RibbonIterator<T>,
        current: Option<T>,
        examined: bool,
        children: Children<T>,
    }
    impl<T: Clone + 'static> Iterator for Tree<T> {
        type Item = T;
        fn next(&mut self) -> Option<T> {
            if let Some(current) = &self.current {
                if !self.examined {
                    self.examined = true;
                    self.child = (self.children)(current).map(|v| {
                        tree(Some(v), {
                            let children = self.children.clone();
                            move |v| children(v)
                        })
                        .iter()
                    });
                }
                if let Some(child) = &mut self.child
                    && let Some(value) = child.next()
                {
                    return Some(value);
                }
                self.child = None;
                self.current = None;
            }
            let value = self.input.next()?;
            self.current = Some(value.clone());
            self.examined = false;
            Some(value)
        }
    }
    let children: Children<T> = Rc::new(children);
    RibbonItemsSource::deferred(move || {
        Box::new(Tree {
            input: source.as_ref().expect("NullReference: items").iter(),
            current: None,
            child: None,
            examined: false,
            children: children.clone(),
        })
    })
}
