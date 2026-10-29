//! Candidate-only validation of resource lookups deferred during initialization.
use crate::XamlLoadException;
use ferroui_base::{controls::ResourceKey, styling::ThemeVariant, Ref, WeakRef};
use ferroui_controls::Control;
use std::cell::RefCell;

struct Lookup {
    target: WeakRef<Control>,
    key: ResourceKey,
    theme: Option<ThemeVariant>,
}
ferroui_base::ferro_thread_local! {
    static LOOKUPS: RefCell<Option<Vec<Lookup>>> = const { RefCell::new(None) };
}

/// Records only within a candidate scope; ordinary runtime loading is unchanged.
pub(crate) fn record(target: &Ref<Control>, key: ResourceKey, theme: Option<ThemeVariant>) {
    LOOKUPS.with(|slot| {
        if let Some(lookups) = slot.borrow_mut().as_mut() {
            lookups.push(Lookup {
                target: target.downgrade(),
                key,
                theme,
            });
        }
    });
}

/// Completes markup values on a detached framework root after its resource
/// declarations have been populated. Does not raise Initialized or invoke an
/// application constructor. Validate while the candidate still has its parent
/// and resource graph, before a reload adapter detaches its replacement content.
pub fn prepare_candidate(root: &ferroui_base::StyledElement) -> Result<(), XamlLoadException> {
    let mut elements = vec![root.to_ref()];
    elements.extend(root.get_logical_descendants());
    let lookups = LOOKUPS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let mut selected = Vec::new();
        if let Some(pending) = slot.as_mut() {
            let mut rest = Vec::new();
            for lookup in std::mem::take(pending) {
                if lookup.target.upgrade().is_some_and(|target| {
                    elements
                        .iter()
                        .any(|element| element.ptr_eq(&target))
                }) {
                    selected.push(lookup);
                } else {
                    rest.push(lookup);
                }
            }
            *pending = rest;
        }
        selected
    });
    validate_lookups(lookups)?;
    for element in elements {
        ferroui_markup::markup::data::DelayedBinding::prepare_values(&element)
            .map_err(|error| XamlLoadException::with_message(error.to_string()))?;
    }
    Ok(())
}

/// Builds a root's selected template against the detached resource graph.
/// Does not attach it, raise TemplateApplied, or replace the visible template.
/// Custom template builders with external effects require their own reload adapter.
pub fn validate_root_template(root: &ferroui_base::StyledElement) -> Result<(), XamlLoadException> {
    if let Some(control) = root.to_ref().cast::<ferroui_controls::primitives::TemplatedControl>() {
        control.apply_styling();
        if let Some(template) = control.template() {
            if let Some(result) = template.build(&control) {
                let (child, _) = result.deconstruct();
                prepare_candidate(&child)?;
            }
        }
    }
    Ok(())
}

fn validate_lookups(lookups: Vec<Lookup>) -> Result<(), XamlLoadException> {
    for lookup in lookups {
        if let Some(target) = lookup.target.upgrade() {
            if target
                .try_find_resource(&lookup.key, lookup.theme.as_ref())
                .is_none()
            {
                return Err(XamlLoadException::with_message(format!(
                    "Static resource '{}' not found in reload candidate.",
                    lookup.key
                )));
            }
        }
    }
    Ok(())
}

/// A UI-thread scope that restores the enclosing validator even on unwind.
/// Validate after construction, before publishing or attaching the candidate.
/// Deferred templates are checked only if instantiated inside this scope.
#[must_use]
pub struct ValidationScope {
    previous: Option<Vec<Lookup>>,
}
impl Default for ValidationScope {
    fn default() -> Self {
        Self::new()
    }
}
impl ValidationScope {
    pub fn new() -> Self {
        Self {
            previous: LOOKUPS.with(|slot| slot.replace(Some(Vec::new()))),
        }
    }
    pub fn validate(&self) -> Result<(), XamlLoadException> {
        // Release the TLS borrow before querying resources, which may run user code.
        let lookups = LOOKUPS.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .map(std::mem::take)
                .unwrap_or_default()
        });
        validate_lookups(lookups)
    }
}
impl Drop for ValidationScope {
    fn drop(&mut self) {
        LOOKUPS.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;

    #[test]
    fn nested_scopes_and_unwind_restore_pending_validation() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let target = Control::new();
        let outer = ValidationScope::new();
        record(&target, "outer".into(), None);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = ValidationScope::new();
            record(&target, "inner".into(), None);
            panic!("candidate constructor failed");
        }));
        assert!(panic.is_err());
        assert!(outer.validate().unwrap_err().to_string().contains("outer"));
        drop(outer);
        record(&target, "outside".into(), None);
        assert!(ValidationScope::new().validate().is_ok());
    }

    #[test]
    fn discarded_candidates_do_not_keep_validation_targets_alive() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let scope = ValidationScope::new();
        let target = Control::new();
        let weak = target.downgrade();
        record(&target, "missing".into(), None);
        drop(target);
        assert!(weak.upgrade().is_none());
        assert!(scope.validate().is_ok());
    }
}
