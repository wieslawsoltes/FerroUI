//! Linked-view ownership and the lazily resolved public ContentSource master.
//!
//! WinUI 7e4b0c2, RichTextBlockOverflow.cpp: PreviousLinkAttached/Detached,
//! ResetAllOverflowMasters, SetupLinkedBlockLayout and SetBreak. The immediate
//! predecessor is a different relationship from the cached primary master.

use super::RichTextBlock;
use crate::RichTextBlockOverflow;
use crate::documents::RichTextDocument;
use ferroui_base::{Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};

pub(super) struct OverflowLinkState {
    // The edge actually installed by the link handler. A property notification
    // can already be stale when a derived override delegates to its parent.
    installed_target: RefCell<Option<WeakRef<RichTextBlockOverflow>>>,
    request_generation: Cell<u64>,
    flow_generation: Cell<u64>,
    master: RefCell<Option<WeakRef<RichTextBlock>>>,
    master_generation: Cell<u64>,
}

impl OverflowLinkState {
    pub(super) fn new() -> Self {
        Self {
            installed_target: RefCell::new(None),
            request_generation: Cell::new(0),
            flow_generation: Cell::new(0),
            master: RefCell::new(None),
            master_generation: Cell::new(0),
        }
    }
}

fn advance(generation: &Cell<u64>) -> u64 {
    let next = generation.get().wrapping_add(1);
    generation.set(next);
    next
}

struct OverflowReset {
    view: Ref<RichTextBlockOverflow>,
    old_master: Option<Ref<RichTextBlock>>,
    old_document: RichTextDocument,
    new_document: RichTextDocument,
    flow_generation: u64,
    master_generation: u64,
}

impl OverflowReset {
    fn owns_flow(&self) -> bool {
        self.view.overflow_link.flow_generation.get() == self.flow_generation
    }

    fn owns_master(&self) -> bool {
        self.owns_flow()
            && self.view.overflow_link.master_generation.get() == self.master_generation
    }
}

impl RichTextBlock {
    /// A pure cache read. Native get_ContentSourceImpl/GetMaster does not walk
    /// links or force measure. Weak references retain FerroUI's view ownership.
    pub(crate) fn cached_overflow_master(&self) -> Option<Ref<RichTextBlock>> {
        self.overflow_link
            .master
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }

    /// Resolves this continuation during actual measure, never during a public
    /// getter or the recursive page_layout query used to obtain a preceding
    /// break. An unmeasured predecessor can therefore leave this cache empty.
    pub(super) fn setup_linked_overflow_master(&self) {
        let Some(overflow) = self.to_ref().cast::<RichTextBlockOverflow>() else {
            return;
        };
        if self.cached_overflow_master().is_some() {
            return;
        }
        let Some(previous) = self.previous_flow_parent() else {
            return;
        };
        let master = if previous.cast::<RichTextBlockOverflow>().is_some() {
            previous.cached_overflow_master()
        } else {
            Some(previous)
        };
        let Some(master) = master else {
            return;
        };

        *self.overflow_link.master.borrow_mut() = Some(master.downgrade());
        advance(&self.overflow_link.master_generation);
        self.invalidate_context_menu_request();
        // Geometry may have formatted this continuation before measure. A
        // newly resolved master must publish a fresh page/break, which uses the
        // existing continuation invalidation to schedule the next overflow.
        self.clear_page_layout();
        overflow.raise_direct_property_changed(
            RichTextBlockOverflow::content_source_property(),
            &None,
            &Some(master),
        );
    }

    /// Returns only an installed forward edge whose backward link still names
    /// this exact object. This also works during Drop without upgrading self.
    pub(super) fn installed_overflow_target(&self) -> Option<Ref<RichTextBlockOverflow>> {
        let target = self
            .overflow_link
            .installed_target
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)?;
        let owns = target
            .flow_parent
            .borrow()
            .as_ref()
            .is_some_and(|parent| parent.ptr_eq(&self.to_weak()));
        owns.then_some(target)
    }

    pub(super) fn reconcile_overflow_content_target(&self) {
        let request_generation = advance(&self.overflow_link.request_generation);
        // Read the effective property, not the old/new values from a possibly
        // superseded notification. Markup uses the same cycle/owner validation.
        let target = self.overflow_content_target();
        if target
            .as_ref()
            .is_some_and(|target| !self.can_link_target(target))
        {
            self.set_current_value(Self::overflow_content_target_property(), None);
            return;
        }
        let installed = self
            .overflow_link
            .installed_target
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        if installed == target && self.installed_overflow_target() == target {
            return;
        }

        *self.overflow_link.installed_target.borrow_mut() = target.as_ref().map(Ref::downgrade);
        let mut resets = Vec::new();
        if let Some(previous) = installed.filter(|previous| Some(previous) != target.as_ref()) {
            let owns = previous
                .flow_parent
                .borrow()
                .as_ref()
                .is_some_and(|parent| parent.ptr_eq(&self.to_weak()));
            if owns {
                resets.extend(previous.replace_flow_parent(None, None));
            }
        }
        if let Some(target) = target {
            resets.extend(target.replace_flow_parent(Some(self.to_ref().downgrade()), None));
        }
        // Both edges and every affected cached master/page are committed before
        // invalidate_measure, coercion or property events can run application
        // code. A nested request owns only the generations it subsequently sets.
        self.clear_page_layout();
        self.invalidate_measure();
        if self.overflow_link.request_generation.get() == request_generation {
            self.invalidate_visual();
        }
        Self::publish_overflow_resets(resets);
    }

    /// Changes one immediate predecessor and resets its installed suffix with
    /// no framework callbacks. Public property notifications are a FerroUI
    /// service adaptation; native resets the same cache without exposing an
    /// independently measured ContentSource notification ordering contract.
    fn replace_flow_parent(
        &self,
        parent: Option<WeakRef<RichTextBlock>>,
        previous_document: Option<&RichTextDocument>,
    ) -> Vec<OverflowReset> {
        let same = match (self.flow_parent.borrow().as_ref(), parent.as_ref()) {
            (None, None) => true,
            (Some(old), Some(new)) => old.ptr_eq(new),
            _ => false,
        };
        if same {
            return Vec::new();
        }

        let mut resets = Vec::new();
        let mut current = self.to_ref().cast::<RichTextBlockOverflow>();
        while let Some(view) = current {
            current = view.installed_overflow_target();
            let document = previous_document
                .cloned()
                .unwrap_or_else(|| view.document());
            resets.push(OverflowReset {
                old_master: view.cached_overflow_master(),
                old_document: document.clone(),
                new_document: document,
                view,
                flow_generation: 0,
                master_generation: 0,
            });
        }
        *self.flow_parent.borrow_mut() = parent;
        for reset in &mut resets {
            reset.flow_generation = advance(&reset.view.overflow_link.flow_generation);
            reset.master_generation = advance(&reset.view.overflow_link.master_generation);
            reset.view.overflow_link.master.borrow_mut().take();
            reset.view.invalidate_context_menu_request();
            reset.view.clear_page_layout();
            reset.new_document = reset.view.document();
        }
        resets
    }

    fn publish_overflow_resets(resets: Vec<OverflowReset>) {
        for reset in &resets {
            if reset.owns_master() {
                reset.view.invalidate_measure();
            }
            if !reset.owns_flow() {
                continue;
            }
            reset.view.invalidate_visual();
            // Apply the current immediate-parent formatting policy without
            // recursively revisiting a suffix a callback might have reparented.
            reset
                .view
                .coerce_value(Visual::clip_to_bounds_property().as_property());
        }
        for reset in resets {
            if reset.owns_master() && reset.old_master.is_some() {
                reset.view.raise_direct_property_changed(
                    RichTextBlockOverflow::content_source_property(),
                    &reset.old_master,
                    &None,
                );
            }
            if reset.owns_flow()
                && reset.old_document != reset.new_document
                && reset.view.document() == reset.new_document
            {
                reset.view.raise_direct_property_changed(
                    Self::rich_document_property(),
                    &reset.old_document,
                    &reset.new_document,
                );
            }
        }
    }

    pub(super) fn detach_installed_overflow_on_drop(&self) {
        let target = self
            .overflow_link
            .installed_target
            .borrow_mut()
            .take()
            .and_then(|target| target.upgrade());
        if let Some(target) = target {
            let owns = target
                .flow_parent
                .borrow()
                .as_ref()
                .is_some_and(|parent| parent.ptr_eq(&self.to_weak()));
            if owns {
                // A dying predecessor can no longer be upgraded from the
                // successor. Capture its effective document directly before
                // severing that edge, without ever upgrading self. This also
                // preserves old Document values for an unmeasured suffix.
                let previous_document = self.document();
                let resets = target.replace_flow_parent(None, Some(&previous_document));
                Self::publish_overflow_resets(resets);
            }
        }
    }
}

#[cfg(test)]
#[path = "rich_text_overflow_master_tests.rs"]
mod tests;
