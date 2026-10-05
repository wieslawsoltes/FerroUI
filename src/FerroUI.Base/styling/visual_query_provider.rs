use super::ContainerSizing;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use std::cell::Cell;
use std::rc::Rc;

struct Inner {
    width: Cell<f64>,
    height: Cell<f64>,
    width_changed: HandlerList<dyn Fn()>,
    height_changed: HandlerList<dyn Fn()>,
}

/// Publishes the size available to a container to the container queries of
/// its descendants.
#[derive(Clone)]
pub(crate) struct VisualQueryProvider(Rc<Inner>);

impl PartialEq for VisualQueryProvider {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl VisualQueryProvider {
    pub fn new() -> Self {
        VisualQueryProvider(Rc::new(Inner {
            width: Cell::new(f64::INFINITY),
            height: Cell::new(f64::INFINITY),
            width_changed: HandlerList::new(),
            height_changed: HandlerList::new(),
        }))
    }

    pub fn width(&self) -> f64 {
        self.0.width.get()
    }

    pub fn height(&self) -> f64 {
        self.0.height.get()
    }

    pub fn width_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.0.width_changed.add(handler);
        let weak = Rc::downgrade(&self.0);
        Disposable::create(move || {
            if let Some(inner) = weak.upgrade() {
                inner.width_changed.remove(token);
            }
        })
    }

    pub fn height_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.0.height_changed.add(handler);
        let weak = Rc::downgrade(&self.0);
        Disposable::create(move || {
            if let Some(inner) = weak.upgrade() {
                inner.height_changed.remove(token);
            }
        })
    }

    pub fn set_size(&self, width: f64, height: f64, container_type: ContainerSizing) {
        let current_width = self.0.width.get();
        let current_height = self.0.height.get();

        self.0.width.set(width);
        self.0.height.set(height);

        if current_width != width
            && matches!(container_type, ContainerSizing::Width | ContainerSizing::WidthAndHeight)
            && !self.0.width_changed.is_empty()
        {
            for (_, handler) in self.0.width_changed.snapshot().iter() {
                handler();
            }
        }

        if current_height != height
            && matches!(container_type, ContainerSizing::Height | ContainerSizing::WidthAndHeight)
            && !self.0.height_changed.is_empty()
        {
            for (_, handler) in self.0.height_changed.snapshot().iter() {
                handler();
            }
        }
    }
}
