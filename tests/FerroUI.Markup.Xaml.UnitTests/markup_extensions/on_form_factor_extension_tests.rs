//! Ported from the upstream `MarkupExtensions/OnFormFactorExtensionTests`.

use std::rc::Rc;

use ferroui_base::platform::{IRuntimePlatform, RuntimePlatformInfo, StandardRuntimePlatform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::{TextBlock, UserControl};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;

// --- test types -------------------------------------------------------------

/// `TestRuntimePlatform : StandardRuntimePlatform`: the standard runtime
/// platform with the runtime information of the test.
struct TestRuntimePlatform {
    _base: StandardRuntimePlatform,
    is_desktop: bool,
    is_mobile: bool,
}

impl TestRuntimePlatform {
    fn new(is_desktop: bool, is_mobile: bool) -> Rc<dyn IRuntimePlatform> {
        Rc::new(Self { _base: StandardRuntimePlatform::new(), is_desktop, is_mobile })
    }
}

impl IRuntimePlatform for TestRuntimePlatform {
    fn get_runtime_info(&self) -> RuntimePlatformInfo {
        RuntimePlatformInfo { is_desktop: self.is_desktop, is_mobile: self.is_mobile, ..RuntimePlatformInfo::default() }
    }
}

/// `using (FerroLocator.EnterScope())`: the scope ends when the value drops.
struct LocatorScope(Rc<dyn IDisposable>);

impl LocatorScope {
    fn enter() -> Self {
        Self(FerroLocator::enter_scope())
    }
}

impl Drop for LocatorScope {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule::EMPTY;

// --- tests ------------------------------------------------------------------

#[test]
fn should_resolve_default_value() {
    let _base = xaml_test_base();
    let _scope = LocatorScope::enter();
    FerroLocator::current_mutable().bind::<dyn IRuntimePlatform>().to_constant(TestRuntimePlatform::new(false, false));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Text='{OnFormFactor Default="Hello World"}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let text_block = object_of::<TextBlock>(&user_control.content());

    assert_eq!(text_block.text().as_deref(), Some("Hello World"));
}

#[test]
fn should_resolve_expected_value_per_platform() {
    for (is_desktop, is_mobile, expected_result) in
        [(false, true, "Im Mobile"), (true, false, "Im Desktop"), (false, false, "Default value")]
    {
        let _base = xaml_test_base();
        let _scope = LocatorScope::enter();
        FerroLocator::current_mutable()
            .bind::<dyn IRuntimePlatform>()
            .to_constant(TestRuntimePlatform::new(is_desktop, is_mobile));

        let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Text='{OnFormFactor "Default value",
                                 Mobile="Im Mobile", Desktop="Im Desktop"}'/>
</UserControl>"#;

        let user_control: Ref<UserControl> = load_as(xaml);
        let text_block = object_of::<TextBlock>(&user_control.content());

        assert_eq!(
            text_block.text().as_deref(),
            Some(expected_result),
            "row ({is_desktop}, {is_mobile}, {expected_result:?})"
        );
    }
}
