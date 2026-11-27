//! A control inside a `ControlTemplate` of markup loaded at run time gets its own control theme
//! and template, as it does anywhere else.

use super::support::*;
use ferroui_base::Ref;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{Border, Control, TextBox, Window};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

const TEMPLATED: &str = r#"<TemplatedControl xmlns="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <TemplatedControl.Template>
    <ControlTemplate>
      <StackPanel Spacing="4">
        <TextBlock x:Name="HeaderText" />
        <TextBox x:Name="PasswordInput" PasswordChar="*" />
      </StackPanel>
    </ControlTemplate>
  </TemplatedControl.Template>
</TemplatedControl>"#;

#[test]
fn a_text_box_in_a_run_time_control_template_gets_its_own_template_part() {
    let _scope = start_themed_application();
    let control = FerroRuntimeXamlLoader::parse_as::<Ref<TemplatedControl>>(TEMPLATED, None).unwrap();
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(640.0, 480.0));
    window.set_content(Some(Control::boxed(&control)));
    window.show();
    window.update_layout();

    let text_box = control
        .get_visual_descendants()
        .find_map(|descendant| descendant.cast::<TextBox>())
        .expect("the text box of the template");
    assert_eq!(text_box.name().as_deref(), Some("PasswordInput"));
    assert!(text_box.get_visual_descendants().any(|descendant| descendant.cast::<TextPresenter>().is_some()));
}

const THEMED: &str = r#"<Border xmlns="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Border.Resources>
    <ControlTheme x:Key="PasswordTheme" TargetType="TemplatedControl">
      <Setter Property="IsTabStop" Value="False" />
      <Setter Property="Template">
        <ControlTemplate>
          <StackPanel Spacing="4">
            <TextBlock x:Name="HeaderText" />
            <TextBox x:Name="PasswordInput" PasswordChar="*" />
          </StackPanel>
        </ControlTemplate>
      </Setter>
    </ControlTheme>
  </Border.Resources>
  <TemplatedControl Theme="{StaticResource PasswordTheme}" />
</Border>"#;

/// The theme's target type is a base class of the TextBox in its template: the setters of the
/// theme are for the control it is applied to, not for that TextBox.
#[test]
fn a_text_box_in_the_template_of_a_base_class_theme_keeps_its_own_template() {
    let _scope = start_themed_application();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Border>>(THEMED, None).unwrap();
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(640.0, 480.0));
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    window.update_layout();

    let text_box = root
        .get_visual_descendants()
        .find_map(|descendant| descendant.cast::<TextBox>())
        .expect("the text box of the template");
    assert!(text_box.is_tab_stop());
    assert!(text_box.get_visual_descendants().any(|descendant| descendant.cast::<TextPresenter>().is_some()));
}
