//! Ported from the upstream `Xaml/BindingTests_RelativeSource`.

use std::rc::Rc;

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControl, ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;
use crate::xaml::binding_tests::AnonymousFoo;

// --- test types -------------------------------------------------------------

/// A window class of the test project.
#[repr(C)]
pub struct TestWindow {
    base: Window,
}

ferro_class!(TestWindow: Window);
ferro_impl_classes!(
    TestWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(TestWindow { new: TestWindow::new });

impl TestWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule { types: &[TestWindow::TYPE], markup_types: &[], value_types: || {} };

// --- tests ------------------------------------------------------------------

#[test]
fn binding_to_data_context_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Content='{Binding Foo, RelativeSource={RelativeSource DataContext}}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    button.set_data_context(Some(Rc::new(AnonymousFoo { foo: boxed_str("foo") })));
    window.apply_template();

    assert_string("foo", &button.content());
}

#[test]
fn binding_to_self_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Content='{Binding Name, RelativeSource={RelativeSource Self}}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_string("button", &button.content());
}

#[test]
fn binding_to_first_ancestor_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding Name, RelativeSource={RelativeSource AncestorType=Border}}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert_string("border2", &button.content());
}

#[test]
fn binding_to_first_ancestor_with_shorthand_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding $parent.Name}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_string("border2", &button.content());
}

#[test]
fn binding_to_first_ancestor_with_shorthand_uses_logical_tree() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border'>
      <ContentControl Name='contentControl'>
        <Button Name='button' Content='{Binding $parent.Name}'/>
      </ContentControl>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let _content_control = window.get_control::<ContentControl>("contentControl");
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_string("contentControl", &button.content());
}

#[test]
fn binding_to_second_ancestor_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding Name, RelativeSource={RelativeSource AncestorType=Border, AncestorLevel=2}}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert_string("border1", &button.content());
}

#[test]
fn binding_to_second_ancestor_with_shorthand_uses_logical_tree() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <ContentControl Name='contentControl1'>
      <ContentControl Name='contentControl2'>
        <Button Name='button' Content='{Binding $parent[1].Name}'/>
      </ContentControl>
    </ContentControl>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let _content_control1 = window.get_control::<ContentControl>("contentControl1");
    let _content_control2 = window.get_control::<ContentControl>("contentControl2");
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_string("contentControl1", &button.content());
}

#[test]
fn binding_to_ancestor_of_type_with_shorthand_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding $parent[Border].Name}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_string("border2", &button.content());
}

#[test]
fn binding_to_second_ancestor_with_shorthand_and_type_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding $parent[Border; 1].Name}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_string("border1", &button.content());
}

#[test]
fn binding_to_second_ancestor_with_shorthand_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding $parent[1].Name}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_string("border1", &button.content());
}

#[test]
fn binding_to_ancestor_with_namespace_works() {
    let _app = styled_window_application();
    let xaml = r#"
<local:TestWindow xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
        Title='title'>
  <Button Name='button' Content='{Binding Title, RelativeSource={RelativeSource AncestorType=local:TestWindow}}'/>
</local:TestWindow>"#;
    let window: Ref<TestWindow> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert_string("title", &button.content());
}

#[test]
fn shorthand_binding_with_negation_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding !$self.IsDefault}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_value(true, &button.content());
}

#[test]
fn shorthand_binding_with_multiple_negation_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Name='border1'>
      <Border Name='border2'>
        <Button Name='button' Content='{Binding !!$self.IsDefault}'/>
      </Border>
    </Border>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();

    assert_value(false, &button.content());
}
