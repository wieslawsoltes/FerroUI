//! Native shared storage for theme rules, key codes and quick actions.
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

/// Shared native vector, with identity equality for FerroUI property metadata.
/// Use standard Mutex/Vec APIs and take snapshots before callbacks/control construction.
/// Release guards before domain calls. There is no List facade, iterator version,
/// fixed-size restriction or System.Collections metadata.
pub struct ThemeList<T>(Arc<Mutex<Vec<T>>>);
impl<T> Clone for ThemeList<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> PartialEq for ThemeList<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T> Eq for ThemeList<T> {}
impl<T> Default for ThemeList<T> {
    fn default() -> Self {
        Self(Arc::default())
    }
}
impl<T> From<Vec<T>> for ThemeList<T> {
    fn from(items: Vec<T>) -> Self {
        Self(Arc::new(Mutex::new(items)))
    }
}
impl<T> From<Arc<Mutex<Vec<T>>>> for ThemeList<T> {
    fn from(items: Arc<Mutex<Vec<T>>>) -> Self {
        Self(items)
    }
}
impl<T> Deref for ThemeList<T> {
    type Target = Arc<Mutex<Vec<T>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
