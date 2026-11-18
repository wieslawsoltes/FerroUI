//! Application-registered previews scoped to one source file in the hosted Cargo package.
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Control;
use ferroui_designer_core::design_fixtures::validate_name;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

/// A typed test model for the current source, or a view built by registered Rust code.
pub enum PreviewFixture {
    DataContext(BoxedValue),
    RustView(Ref<Control>),
}

type Factory = Rc<dyn Fn() -> Result<PreviewFixture, String>>;
thread_local! {
    static FIXTURES: RefCell<BTreeMap<String, BTreeMap<String, Factory>>> = const { RefCell::new(BTreeMap::new()) };
}

/// Call during application preview setup on its UI thread. Use the URI-encoded
/// package-relative source path, for example `/src/customers.rs`. A Rust view factory
/// returns a new unattached control on every call. It runs from the built application.
pub fn register_preview_fixture(
    project_path: &str,
    name: &str,
    create: impl Fn() -> Result<PreviewFixture, String> + 'static,
) -> Result<(), String> {
    let segments = project_path
        .strip_prefix('/')
        .ok_or("Use a package-relative preview path beginning with '/'.")?;
    if project_path.len() > 4096
        || project_path
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '\\' | '?' | '#'))
        || segments
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
    {
        return Err("Use a URI-encoded preview path within the Cargo package.".into());
    }
    validate_name(name)?;
    FIXTURES.with(|fixtures| {
        let mut fixtures = fixtures.borrow_mut();
        let entries = fixtures.entry(project_path.into()).or_default();
        if entries.contains_key(name) {
            return Err(format!(
                "Preview fixture '{name}' is already registered for {project_path}."
            ));
        }
        if entries.len() == 32 {
            return Err("A hosted source supports up to 32 named fixtures.".into());
        }
        entries.insert(name.into(), Rc::new(create));
        Ok(())
    })
}

pub(crate) fn names(project_path: Option<&str>) -> Option<Vec<String>> {
    FIXTURES.with(|fixtures| {
        fixtures
            .borrow()
            .get(project_path?)
            .map(|entries| entries.keys().cloned().collect())
    })
}

pub(crate) fn create(project_path: Option<&str>, name: &str) -> Result<PreviewFixture, String> {
    let factory = FIXTURES
        .with(|fixtures| fixtures.borrow().get(project_path?)?.get(name).cloned())
        .ok_or_else(|| format!("Fixture '{name}' is unavailable for this hosted source."))?;
    factory()
}
