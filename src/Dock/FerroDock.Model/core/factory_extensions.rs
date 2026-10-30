// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{IDockWindow, IFactory, IHostWindow};
use crate::controls::{IDocument, IRootDock};
use std::rc::Rc;

/// Source factory helpers for current focus and window tracking.
pub struct FactoryExtensions;
impl FactoryExtensions {
    /// Returns the tracked root, otherwise the first active root found by the factory.
    pub fn get_active_root<F: IFactory + ?Sized>(factory: Option<&F>) -> Option<Rc<dyn IRootDock>> {
        let factory = factory?;
        if let Some(root) = factory.current_root_dock() {
            return Some(root);
        }
        factory
            .find(Some(Rc::new(|dockable| {
                dockable
                    .and_then(|item| item.as_root_dock())
                    .is_some_and(|root| root.is_active())
            })))
            .iter()
            .find_map(|dockable| dockable.and_then(|item| item.as_root_dock()))
    }
    /// Returns the current document, otherwise the active root's focused document.
    pub fn get_current_document<F: IFactory + ?Sized>(
        factory: Option<&F>,
    ) -> Option<Rc<dyn IDocument>> {
        let factory = factory?;
        factory
            .current_dockable()
            .and_then(|dockable| dockable.as_document())
            .or_else(|| {
                Self::get_active_root(Some(factory))?
                    .focused_dockable()?
                    .as_document()
            })
    }
    /// Passes the current dockable, otherwise the active root's focus, to CloseDockable.
    pub fn close_focused_dockable<F: IFactory + ?Sized>(factory: Option<&F>) {
        let Some(factory) = factory else { return };
        let dockable = factory
            .current_dockable()
            .or_else(|| Self::get_active_root(Some(factory))?.focused_dockable());
        if let Some(dockable) = dockable {
            factory.close_dockable(Some(dockable));
        }
    }
    /// Returns the factory's currently tracked dock window.
    pub fn get_current_dock_window<F: IFactory + ?Sized>(
        factory: Option<&F>,
    ) -> Option<Rc<dyn IDockWindow>> {
        factory?.current_dock_window()
    }
    /// Returns the factory's currently tracked host window.
    pub fn get_current_host_window<F: IFactory + ?Sized>(
        factory: Option<&F>,
    ) -> Option<Rc<dyn IHostWindow>> {
        factory?.current_host_window()
    }
}

/// Rust extension-method syntax for the original factory helper methods.
pub trait FactoryExt: IFactory {
    /// Returns the tracked root, otherwise the first active root in factory search order.
    fn get_active_root(&self) -> Option<Rc<dyn IRootDock>> {
        FactoryExtensions::get_active_root(Some(self))
    }
    /// Returns the current document, falling back to the active root's focused document.
    fn get_current_document(&self) -> Option<Rc<dyn IDocument>> {
        FactoryExtensions::get_current_document(Some(self))
    }
    /// Dispatches closure of the current dockable or active root's focus.
    fn close_focused_dockable(&self) {
        FactoryExtensions::close_focused_dockable(Some(self));
    }
    /// Returns the factory's currently tracked dock window.
    fn get_current_dock_window(&self) -> Option<Rc<dyn IDockWindow>> {
        FactoryExtensions::get_current_dock_window(Some(self))
    }
    /// Returns the factory's currently tracked host window.
    fn get_current_host_window(&self) -> Option<Rc<dyn IHostWindow>> {
        FactoryExtensions::get_current_host_window(Some(self))
    }
}
impl<F: IFactory + ?Sized> FactoryExt for F {}
