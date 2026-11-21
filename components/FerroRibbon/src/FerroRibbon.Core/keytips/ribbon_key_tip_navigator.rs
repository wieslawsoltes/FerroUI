//! Port of RibbonKeyTipNavigator.cs: pure Core matching, not native keyboard routing.
use super::RibbonKeyTipAssigner;
use crate::collections::RibbonItems;
use crate::{
    RibbonString,
    collections::RibbonList,
    model::RibbonItemsSource,
    text::{empty, upper_char},
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonKeyTipMatch(i32);
#[allow(non_upper_case_globals)]
impl RibbonKeyTipMatch {
    pub const None: Self = Self(0);
    pub const Partial: Self = Self(1);
    pub const Complete: Self = Self(2);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
impl std::fmt::Display for RibbonKeyTipMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::None => f.write_str("None"),
            Self::Partial => f.write_str("Partial"),
            Self::Complete => f.write_str("Complete"),
            _ => write!(f, "{}", self.0),
        }
    }
}
pub struct RibbonKeyTipEntry<T> {
    pub tip: Option<RibbonString>,
    pub target: Option<T>,
}
impl<T: Clone> Clone for RibbonKeyTipEntry<T> {
    fn clone(&self) -> Self {
        Self {
            tip: self.tip.clone(),
            target: self.target.clone(),
        }
    }
}
impl<T> Default for RibbonKeyTipEntry<T> {
    fn default() -> Self {
        Self {
            tip: None,
            target: None,
        }
    }
}
impl<T: PartialEq> PartialEq for RibbonKeyTipEntry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.tip == other.tip && self.target == other.target
    }
}
pub struct RibbonKeyTipScope<T> {
    entries: RibbonList<RibbonKeyTipEntry<T>>,
    typed: RefCell<RibbonString>,
}
impl<T: Clone + PartialEq + 'static> RibbonKeyTipScope<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            entries: RibbonItems::new(),
            typed: RefCell::new(empty()),
        })
    }
    pub fn entries(&self) -> RibbonList<RibbonKeyTipEntry<T>> {
        self.entries.clone()
    }
    pub fn typed(&self) -> RibbonString {
        self.typed.borrow().clone()
    }
    pub fn add(&self, tip: Option<RibbonString>, target: Option<T>) {
        let target = target.expect("ArgumentNull: target");
        let normalized = RibbonKeyTipAssigner::normalize(tip.as_ref());
        assert!(
            !normalized.is_empty(),
            "ArgumentException: A KeyTip needs at least one letter or digit. (tip)"
        );
        self.entries.add(RibbonKeyTipEntry {
            tip: Some(normalized),
            target: Some(target),
        });
    }
    pub fn candidates(self: Rc<Self>) -> RibbonItemsSource<RibbonKeyTipEntry<T>> {
        RibbonItemsSource::new({
            let scope = self;
            move || {
                let scope = scope.clone();
                let entries = scope.entries.clone();
                // RibbonItemsSource::deferred is crate-visible and fuses/disposes the source on end.
                RibbonItemsSource::deferred(move || {
                    let owner = scope.clone();
                    Box::new(entries.iter().filter(move |e| {
                        e.tip
                            .as_ref()
                            .expect("NullReference: Tip")
                            .as_utf16()
                            .starts_with(owner.typed().as_utf16())
                    }))
                })
                .iter()
            }
        })
    }
    pub fn process(&self, key: u16) -> (RibbonKeyTipMatch, Option<T>) {
        let mut next = self.typed().as_utf16().to_vec();
        next.push(upper_char(key));
        let matches = self
            .entries
            .iter()
            .filter(|e| {
                e.tip
                    .as_ref()
                    .expect("NullReference: Tip")
                    .as_utf16()
                    .starts_with(&next)
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return (RibbonKeyTipMatch::None, None);
        }
        let exact = matches
            .into_iter()
            .find(|e| e.tip.as_ref().expect("NullReference: Tip").as_utf16() == next);
        if let Some(target) = exact.and_then(|e| e.target) {
            *self.typed.borrow_mut() = empty();
            return (RibbonKeyTipMatch::Complete, Some(target));
        }
        *self.typed.borrow_mut() = RibbonString::from_utf16(next);
        (RibbonKeyTipMatch::Partial, None)
    }
    pub fn backspace(&self) -> bool {
        let mut units = self.typed().as_utf16().to_vec();
        if units.pop().is_none() {
            return false;
        }
        *self.typed.borrow_mut() = if units.is_empty() {
            empty()
        } else {
            RibbonString::from_utf16(units)
        };
        true
    }
    pub fn reset(&self) {
        *self.typed.borrow_mut() = empty();
    }
}
pub struct RibbonKeyTipNavigator<T> {
    scopes: RefCell<Vec<Option<Rc<RibbonKeyTipScope<T>>>>>,
}
impl<T: Clone + PartialEq + 'static> RibbonKeyTipNavigator<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            scopes: RefCell::new(vec![]),
        })
    }
    pub fn is_active(&self) -> bool {
        !self.scopes.borrow().is_empty()
    }
    pub fn current(&self) -> Option<Rc<RibbonKeyTipScope<T>>> {
        self.scopes.borrow().last().cloned().flatten()
    }
    pub fn depth(&self) -> i32 {
        i32::try_from(self.scopes.borrow().len()).expect("source Int32 Count")
    }
    pub fn push(&self, scope: Option<Rc<RibbonKeyTipScope<T>>>) {
        self.scopes.borrow_mut().push(scope);
    }
    pub fn pop(&self) -> bool {
        let removed = self.scopes.borrow_mut().pop();
        drop(removed);
        self.is_active()
    }
    pub fn clear(&self) {
        let removed = std::mem::take(&mut *self.scopes.borrow_mut());
        drop(removed);
    }
}
