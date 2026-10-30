//! Opt-in migration of WinUI XAML to the framework's existing XAML language.
//!
//! This pass does not change the default loader, register global aliases, or promise that
//! equally named controls have equal behavior. It changes audited syntax, retains source
//! ranges, and reports contracts that need an implementation. Both a build tool and the
//! run-time loader consume the same migrated document. The normal compiler still validates
//! every type, member, event handler, value and binding against real framework metadata.

mod event_targets;
mod migration;
mod options;
mod report;

pub use migration::migrate_winui_xaml;
pub use options::{
    WinUIThemeResourcePolicy, WinUIXBindPolicy, WinUIXamlMemberMapping, WinUIXamlNamespaceMapping,
    WinUIXamlOptions, WinUIXamlTypeMapping, WINUI_PRESENTATION_NAMESPACE,
};
pub use report::{
    WinUIXamlDiagnostic, WinUIXamlDiagnosticSeverity, WinUIXamlEdit, WinUIXamlMigration,
    WinUIXamlMigrationError, WinUIXamlSourceSpan,
};
