use std::cell::Cell;
use std::rc::{Rc, Weak};

use super::{LayoutPassTiming, RendererDebugOverlays};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;

/// Manages configurable diagnostics that can be displayed by a renderer.
pub struct RendererDiagnostics {
    weak_self: Weak<RendererDiagnostics>,
    debug_overlays: Cell<RendererDebugOverlays>,
    last_layout_pass_timing: Cell<LayoutPassTiming>,
    property_changed: HandlerList<dyn Fn(&RendererDiagnostics, &str)>,
}

impl RendererDiagnostics {
    /// The name [`property_changed`](Self::property_changed) reports for
    /// [`debug_overlays`](Self::debug_overlays).
    pub const DEBUG_OVERLAYS_PROPERTY_NAME: &'static str = "DebugOverlays";

    /// The name [`property_changed`](Self::property_changed) reports for
    /// [`last_layout_pass_timing`](Self::last_layout_pass_timing).
    pub const LAST_LAYOUT_PASS_TIMING_PROPERTY_NAME: &'static str = "LastLayoutPassTiming";

    pub fn new() -> Rc<RendererDiagnostics> {
        Rc::new_cyclic(|weak_self| RendererDiagnostics {
            weak_self: weak_self.clone(),
            debug_overlays: Cell::new(RendererDebugOverlays::NONE),
            last_layout_pass_timing: Cell::new(LayoutPassTiming::default()),
            property_changed: HandlerList::new(),
        })
    }

    /// Gets which debug overlays are displayed by the renderer.
    pub fn debug_overlays(&self) -> RendererDebugOverlays {
        self.debug_overlays.get()
    }

    /// Sets which debug overlays are displayed by the renderer.
    pub fn set_debug_overlays(&self, value: RendererDebugOverlays) {
        if self.debug_overlays.get() != value {
            self.debug_overlays.set(value);
            self.on_property_changed(Self::DEBUG_OVERLAYS_PROPERTY_NAME);
        }
    }

    /// Gets the last layout pass timing that the renderer may display.
    pub fn last_layout_pass_timing(&self) -> LayoutPassTiming {
        self.last_layout_pass_timing.get()
    }

    /// Sets the last layout pass timing that the renderer may display.
    pub fn set_last_layout_pass_timing(&self, value: LayoutPassTiming) {
        if self.last_layout_pass_timing.get() != value {
            self.last_layout_pass_timing.set(value);
            self.on_property_changed(Self::LAST_LAYOUT_PASS_TIMING_PROPERTY_NAME);
        }
    }

    /// Occurs when a property value changes. The handler receives the name
    /// of the property.
    pub fn property_changed(&self, handler: impl Fn(&RendererDiagnostics, &str) + 'static) -> Rc<dyn IDisposable> {
        let token = self.property_changed.add(Rc::new(handler));
        let this = self.weak_self.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.property_changed.remove(token);
            }
        })
    }

    /// Called when a property changes on the object.
    fn on_property_changed(&self, property_name: &str) {
        for (_, handler) in self.property_changed.snapshot().iter() {
            handler(self, property_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::time::Duration;

    #[test]
    fn raises_property_changed_only_when_the_value_changes() {
        let diagnostics = RendererDiagnostics::new();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let c = changes.clone();
        let subscription = diagnostics.property_changed(move |sender, name| {
            c.borrow_mut().push((name.to_string(), sender.debug_overlays(), sender.last_layout_pass_timing()));
        });

        assert_eq!(diagnostics.debug_overlays(), RendererDebugOverlays::NONE);
        diagnostics.set_debug_overlays(RendererDebugOverlays::NONE);
        assert!(changes.borrow().is_empty());

        let overlays = RendererDebugOverlays::FPS | RendererDebugOverlays::DIRTY_RECTS;
        diagnostics.set_debug_overlays(overlays);
        diagnostics.set_debug_overlays(overlays);
        let timing = LayoutPassTiming::new(3, Duration::from_millis(5));
        diagnostics.set_last_layout_pass_timing(timing);
        diagnostics.set_last_layout_pass_timing(timing);

        assert_eq!(
            *changes.borrow(),
            [
                ("DebugOverlays".to_string(), overlays, LayoutPassTiming::default()),
                ("LastLayoutPassTiming".to_string(), overlays, timing),
            ]
        );

        subscription.dispose();
        diagnostics.set_debug_overlays(RendererDebugOverlays::NONE);
        assert_eq!(changes.borrow().len(), 2);
    }
}
