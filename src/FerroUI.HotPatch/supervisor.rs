//! Admission is separate from patch loading: no devtools client applies unsolicited patches.
use crate::{admit, Admission};
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    rc::Rc,
};
pub type SourceSnapshot = BTreeMap<String, String>;
/// A snapshot includes every Rust source, manifest, lockfile, build script, and configuration input.
/// Source bytes must come from the exact build that produced the running executable.
pub struct PatchSupervisor {
    baseline: SourceSnapshot,
    registered: BTreeMap<String, BTreeSet<String>>,
    view_builders: BTreeMap<(String, String), BTreeSet<String>>,
    revision: u64,
    accepted_table: Option<subsecond::JumpTable>,
    restart_required: Option<String>,
    identity: Rc<()>,
    _ui_thread: PhantomData<Rc<()>>,
}
#[derive(Debug)]
pub struct AdmittedPatch {
    owner: Rc<()>,
    baseline_revision: u64,
    sources: SourceSnapshot,
    pub functions: Vec<String>,
    views: Vec<String>,
}
impl AdmittedPatch {
    /// Only changed, explicitly registered builders contribute view identifiers.
    /// Handler/command-only patches leave this empty.
    pub fn views_to_reapply(&self) -> &[String] {
        &self.views
    }
}
impl PatchSupervisor {
    pub fn new(baseline: SourceSnapshot, registered: BTreeMap<String, BTreeSet<String>>) -> Self {
        Self {
            baseline,
            registered,
            view_builders: BTreeMap::new(),
            revision: 0,
            accepted_table: None,
            restart_required: None,
            identity: Rc::new(()),
            _ui_thread: PhantomData,
        }
    }
    /// Validate explicit callback and builder registrations against the running
    /// executable's source baseline. Registrations remain fixed for this process.
    pub fn with_registrations(
        baseline: SourceSnapshot,
        registrations: impl IntoIterator<Item = crate::FunctionRegistration>,
    ) -> Result<Self, String> {
        let mut supervisor = Self::new(baseline, BTreeMap::new());
        for registration in registrations {
            let source = supervisor
                .baseline
                .get(&registration.source)
                .ok_or_else(|| format!("Registered source is missing: {}", registration.source))?;
            crate::admission::validate_registration(source, &registration.function)?;
            if !supervisor
                .registered
                .entry(registration.source.clone())
                .or_default()
                .insert(registration.function.clone())
            {
                return Err(format!(
                    "Duplicate function registration: {}::{}",
                    registration.source, registration.function
                ));
            }
            if let Some(views) = registration.views {
                if views.is_empty() || views.iter().any(|view| view.trim().is_empty()) {
                    return Err("A view builder requires nonempty view identifiers".into());
                }
                supervisor
                    .view_builders
                    .insert((registration.source, registration.function), views);
            }
        }
        Ok(supervisor)
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn admit(&self, sources: SourceSnapshot) -> Result<AdmittedPatch, String> {
        if let Some(reason) = &self.restart_required {
            return Err(reason.clone());
        }
        if self.baseline.keys().ne(sources.keys()) {
            return Err("Build input set changed; restart required".into());
        }
        let mut functions = Vec::new();
        let mut views = BTreeSet::new();
        for (path, before) in &self.baseline {
            let after = &sources[path];
            if before == after {
                continue;
            }
            let registered = self
                .registered
                .get(path)
                .ok_or_else(|| format!("Unpatchable build input changed: {path}"))?;
            match admit(before, after, registered) {
                Admission::Unchanged => (),
                Admission::Patch { functions: changed } => {
                    for name in &changed {
                        if let Some(affected) =
                            self.view_builders.get(&(path.clone(), name.clone()))
                        {
                            views.extend(affected.iter().cloned());
                        }
                    }
                    functions.extend(changed.into_iter().map(|name| format!("{path}::{name}")))
                }
                Admission::RestartRequired(reason) => return Err(format!("{path}: {reason}")),
            }
        }
        if functions.is_empty() {
            return Err("No changed patchable functions".into());
        }
        Ok(AdmittedPatch {
            owner: self.identity.clone(),
            baseline_revision: self.revision,
            sources,
            functions,
            views: views.into_iter().collect(),
        })
    }
    /// Apply on the UI thread between callbacks, only after the patch build succeeded.
    /// `current` is reread after the build; superseded builds cannot load their code.
    ///
    /// # Safety
    /// The table must be produced by the matching Subsecond toolchain against this running
    /// executable, exclusively from `patch`'s admitted source snapshot. Every non-source build
    /// input and dependency must be unchanged. The caller must ensure no other thread is executing
    /// patchable code. Source admission cannot validate machine addresses or compiler ABI.
    /// This supervisor must be the exclusive dispatch-table owner; independent
    /// Subsecond owners and registered Subsecond reload handlers are unsupported.
    pub unsafe fn apply(
        &mut self,
        patch: AdmittedPatch,
        current: &SourceSnapshot,
        table: subsecond::JumpTable,
    ) -> Result<u64, String> {
        if !patch.views.is_empty() {
            return Err("Registered views require an apply_with transaction".into());
        }
        // SAFETY: forwarded compiler, artifact and quiescence contract.
        unsafe { self.apply_with(patch, current, table, || Ok(())) }
    }

    /// Install code provisionally and run a reversible view transaction with the
    /// new function bodies. Publish the code revision only if that transaction
    /// succeeds. On failure or panic, restore the previously accepted dispatch
    /// table; an initial failed patch restores dispatch to the original executable.
    /// The transaction must itself restore UI mutations before returning an error
    /// or unwinding. This method cannot undo arbitrary application side effects.
    /// It must reapply `patch.views_to_reapply()` through application adapters, or
    /// fail; callback-only changes do not request any view reapplication.
    ///
    /// # Safety
    /// All of `apply`'s guarantees apply. Retain every accepted patch artifact,
    /// unmodified, until this supervisor and its process retire. No independent
    /// Subsecond owner or registered Subsecond hot-reload handlers may mutate the
    /// dispatch table or UI. The transaction must not publish work that executes
    /// patchable code on another thread or after a rejected transaction.
    pub unsafe fn apply_with(
        &mut self,
        patch: AdmittedPatch,
        current: &SourceSnapshot,
        table: subsecond::JumpTable,
        transaction: impl FnOnce() -> Result<(), String>,
    ) -> Result<u64, String> {
        self.validate(&patch, current)?;
        if crate::in_callback() {
            return Err("Patch application must wait until the callback returns".into());
        }
        let accepted = table.clone();
        let rollback = self.accepted_table.clone().unwrap_or_else(|| {
            // The candidate's loader/ASLR descriptor is valid after successful
            // installation. An empty map dispatches every function to the original
            // executable, without unloading a library that may still own code.
            let mut original = table.clone();
            original.map.clear();
            original
        });
        // SAFETY: the caller supplies the documented toolchain and quiescence guarantees.
        unsafe { subsecond::apply_patch(table) }.map_err(|error| format!("{error:?}"))?;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(transaction));
        if !matches!(&outcome, Ok(Ok(()))) {
            // SAFETY: this is the original, unrelocated descriptor of a retained
            // accepted artifact (or the now-loaded candidate with no redirections).
            if let Err(error) = unsafe { subsecond::apply_patch(rollback) } {
                let reason = format!("Restart required: dispatch rollback failed: {error:?}");
                self.restart_required = Some(reason.clone());
                return Err(reason);
            }
            return match outcome {
                Ok(Err(error)) => Err(error),
                Err(payload) => std::panic::resume_unwind(payload),
                Ok(Ok(())) => unreachable!(),
            };
        }
        self.accepted_table = Some(accepted);
        self.baseline = patch.sources;
        self.revision += 1;
        Ok(self.revision)
    }
    fn validate(&self, patch: &AdmittedPatch, current: &SourceSnapshot) -> Result<(), String> {
        if let Some(reason) = &self.restart_required {
            return Err(reason.clone());
        }
        if !Rc::ptr_eq(&patch.owner, &self.identity) {
            return Err("Patch belongs to another supervisor".into());
        }
        if patch.baseline_revision != self.revision || &patch.sources != current {
            return Err("Superseded patch".into());
        }
        Ok(())
    }

    /// A failed dispatch rollback makes further admission unsafe. The owning
    /// development process must build and start a replacement before continuing.
    pub fn restart_required(&self) -> Option<&str> {
        self.restart_required.as_deref()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::FunctionRegistration;

    #[test]
    fn only_changed_builders_request_their_registered_views() {
        let source = "fn click() -> i32 { 1 } fn build() -> i32 { 2 } fn detail() -> i32 { 3 }";
        let baseline = BTreeMap::from([("app.rs".into(), source.into())]);
        let supervisor = PatchSupervisor::with_registrations(
            baseline,
            [
                FunctionRegistration::callback("app.rs", "click"),
                FunctionRegistration::view_builder(
                    "app.rs",
                    "build",
                    ["main".into(), "detail".into()],
                ),
                FunctionRegistration::view_builder("app.rs", "detail", ["detail".into()]),
            ],
        )
        .unwrap();
        let callback = BTreeMap::from([("app.rs".into(), source.replace("{ 1 }", "{ 4 }"))]);
        assert!(supervisor
            .admit(callback)
            .unwrap()
            .views_to_reapply()
            .is_empty());
        let builders = BTreeMap::from([(
            "app.rs".into(),
            source.replace("{ 2 }", "{ 5 }").replace("{ 3 }", "{ 6 }"),
        )]);
        let ticket = supervisor.admit(builders).unwrap();
        assert_eq!(
            ticket.views_to_reapply(),
            &["detail".to_string(), "main".to_string()]
        );
        assert_eq!(ticket.functions, ["app.rs::build", "app.rs::detail"]);
    }

    #[test]
    fn invalid_or_ambiguous_registrations_fail_before_admission() {
        let baseline = BTreeMap::from([(
            "app.rs".into(),
            "fn click() {} async fn task() {} fn generic<T>() {} fn closure() { let f = || 1; }"
                .into(),
        )]);
        for registrations in [
            vec![FunctionRegistration::callback("absent.rs", "click")],
            vec![FunctionRegistration::callback("app.rs", "absent")],
            vec![FunctionRegistration::callback("app.rs", "task")],
            vec![FunctionRegistration::callback("app.rs", "generic")],
            vec![FunctionRegistration::callback("app.rs", "closure")],
            vec![
                FunctionRegistration::callback("app.rs", "click"),
                FunctionRegistration::callback("app.rs", "click"),
            ],
            vec![FunctionRegistration::view_builder("app.rs", "click", [])],
            vec![FunctionRegistration::view_builder(
                "app.rs",
                "click",
                [" ".into()],
            )],
        ] {
            assert!(PatchSupervisor::with_registrations(baseline.clone(), registrations).is_err());
        }
    }
    #[test]
    fn admission_tickets_are_bound_to_their_supervisor_and_revision() {
        let before = BTreeMap::from([("main.rs".into(), "fn click() {}".into())]);
        let registered = BTreeMap::from([("main.rs".into(), BTreeSet::from(["click".into()]))]);
        let mut owner = PatchSupervisor::new(before.clone(), registered.clone());
        let other = PatchSupervisor::new(before, registered);
        let after = BTreeMap::from([("main.rs".into(), "fn click() { let x = 2; }".into())]);
        let ticket = owner.admit(after.clone()).unwrap();
        assert!(owner.validate(&ticket, &after).is_ok());
        assert!(other
            .validate(&ticket, &after)
            .unwrap_err()
            .contains("another supervisor"));
        owner.revision += 1;
        assert!(owner
            .validate(&ticket, &after)
            .unwrap_err()
            .contains("Superseded"));
    }
    #[test]
    fn rejects_changed_dependency_before_loading() {
        let old = BTreeMap::from([
            ("main.rs".into(), "fn click() {}".into()),
            ("Cargo.toml".into(), "original".into()),
        ]);
        let supervisor = PatchSupervisor::new(
            old.clone(),
            BTreeMap::from([("main.rs".into(), BTreeSet::from(["click".into()]))]),
        );
        let mut new = old;
        new.insert("main.rs".into(), "fn click() { let x = 2; }".into());
        assert!(supervisor.admit(new.clone()).is_ok());
        new.insert("Cargo.toml".into(), "changed".into());
        assert!(supervisor.admit(new).unwrap_err().contains("Unpatchable"));
    }
}
