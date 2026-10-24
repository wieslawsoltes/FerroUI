//! Visual states in markup (an addition, docs/porting/winui-calculator-gaps.md, section 5.9):
//! `VisualStateManager.VisualStateGroups` and the setters of visual states
//! (`<Setter Target="Element.Property" Value="..."/>`), loaded at run time.

use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::Ref;
use ferroui_controls::{Border, ContentControl, Control, Grid, TextBlock, UserControl, VisualStateManager};
use std::rc::Rc;

use crate::support::app::styled_window_application;
use crate::support::loader::{describe, load_as, try_load};

const NS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

fn inner_of(control: &Ref<ContentControl>) -> (Ref<Border>, Ref<Border>) {
    let root = control.visual_children().snapshot()[0].cast::<Border>().expect("the template root");
    let inner = root.child().expect("a child").cast::<Border>().expect("the inner border");
    (root, inner)
}

#[test]
fn the_states_of_a_template_set_its_elements() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<ContentControl {NS}>
  <ContentControl.Template>
    <ControlTemplate>
      <Border x:Name='Root'>
        <VisualStateManager.VisualStateGroups>
          <VisualStateGroup x:Name='CommonStates'>
            <VisualState x:Name='Normal'/>
            <VisualState x:Name='Pressed'>
              <VisualState.Setters>
                <Setter Target='Inner.Opacity' Value='0.25'/>
                <Setter Target='Root.Background' Value='Red'/>
              </VisualState.Setters>
            </VisualState>
          </VisualStateGroup>
        </VisualStateManager.VisualStateGroups>
        <Border x:Name='Inner'/>
      </Border>
    </ControlTemplate>
  </ContentControl.Template>
</ContentControl>"
    );
    let control = load_as::<Ref<ContentControl>>(&xaml);
    control.apply_template();
    let (root, inner) = inner_of(&control);

    let groups = VisualStateManager::get_visual_state_groups(&root);
    assert_eq!(groups.count(), 1);
    assert_eq!(groups.get(0).name(), "CommonStates");
    assert_eq!(groups.get(0).states().count(), 2);

    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    assert_eq!(inner.opacity(), 0.25);
    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(root.background(), Some(red));

    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert_eq!(inner.opacity(), 1.0);
    assert!(root.background().is_none());
}

#[test]
fn each_instance_of_a_template_has_its_own_states() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<StackPanel {NS}>
  <StackPanel.Resources>
    <ControlTemplate x:Key='Template'>
      <Border>
        <VisualStateManager.VisualStateGroups>
          <VisualStateGroup x:Name='CommonStates'>
            <VisualState x:Name='Pressed'>
              <VisualState.Setters>
                <Setter Target='Inner.Opacity' Value='0.25'/>
              </VisualState.Setters>
            </VisualState>
          </VisualStateGroup>
        </VisualStateManager.VisualStateGroups>
        <Border x:Name='Inner'/>
      </Border>
    </ControlTemplate>
  </StackPanel.Resources>
  <ContentControl Template='{{StaticResource Template}}'/>
  <ContentControl Template='{{StaticResource Template}}'/>
</StackPanel>"
    );
    let panel = load_as::<Ref<ferroui_controls::StackPanel>>(&xaml);
    let first = panel.children().get(0).cast::<ContentControl>().unwrap();
    let second = panel.children().get(1).cast::<ContentControl>().unwrap();
    first.apply_template();
    second.apply_template();

    assert!(VisualStateManager::go_to_state(&first, "Pressed", false));
    assert_eq!(inner_of(&first).1.opacity(), 0.25);
    assert_eq!(inner_of(&second).1.opacity(), 1.0);
}

#[test]
fn the_states_of_a_user_control_are_on_its_content_and_set_attached_properties() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<UserControl {NS}>
  <Grid x:Name='LayoutRoot'>
    <VisualStateManager.VisualStateGroups>
      <VisualStateGroup x:Name='LayoutStates'>
        <VisualState x:Name='Wide'/>
        <VisualState x:Name='Narrow'>
          <VisualState.Setters>
            <Setter Target='Label.(Grid.Row)' Value='1'/>
            <Setter Target='Label.IsVisible' Value='False'/>
            <Setter Target='Label.Text' Value='Narrow'/>
          </VisualState.Setters>
        </VisualState>
      </VisualStateGroup>
    </VisualStateManager.VisualStateGroups>
    <TextBlock x:Name='Label' Text='Wide'/>
  </Grid>
</UserControl>"
    );
    let control = load_as::<Ref<UserControl>>(&xaml);
    let grid = Control::from_boxed(&control.content().unwrap()).unwrap().cast::<Grid>().unwrap();
    let label = grid.children().get(0).cast::<TextBlock>().unwrap();

    assert!(VisualStateManager::go_to_state(&control, "Narrow", false));
    assert_eq!(Grid::get_row(&label), 1);
    assert!(!label.is_visible());
    assert_eq!(label.text().as_deref(), Some("Narrow"));

    assert!(VisualStateManager::go_to_state(&control, "Wide", false));
    assert_eq!(Grid::get_row(&label), 0);
    assert!(label.is_visible());
    assert_eq!(label.text().as_deref(), Some("Wide"));
}

#[test]
fn a_setter_value_can_be_a_dynamic_resource() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<UserControl {NS}>
  <UserControl.Resources>
    <SolidColorBrush x:Key='PressedBrush' Color='Blue'/>
  </UserControl.Resources>
  <Border x:Name='Root'>
    <VisualStateManager.VisualStateGroups>
      <VisualStateGroup>
        <VisualState x:Name='Pressed'>
          <VisualState.Setters>
            <Setter Target='Root.Background' Value='{{DynamicResource PressedBrush}}'/>
          </VisualState.Setters>
        </VisualState>
      </VisualStateGroup>
    </VisualStateManager.VisualStateGroups>
  </Border>
</UserControl>"
    );
    let control = load_as::<Ref<UserControl>>(&xaml);
    let root = Control::from_boxed(&control.content().unwrap()).unwrap().cast::<Border>().unwrap();
    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    let brush = root.background().expect("the brush of the resource");
    let solid = brush.as_any().and_then(|any| any.downcast_ref::<ferroui_base::media::SolidColorBrush>()).map(|brush| brush.color());
    assert_eq!(solid, Some(ferroui_base::media::Colors::blue()));
}

#[test]
fn a_target_that_names_no_element_is_an_error() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<UserControl {NS}>
  <Border>
    <VisualStateManager.VisualStateGroups>
      <VisualStateGroup>
        <VisualState x:Name='Pressed'>
          <VisualState.Setters>
            <Setter Target='Missing.Opacity' Value='0.5'/>
          </VisualState.Setters>
        </VisualState>
      </VisualStateGroup>
    </VisualStateManager.VisualStateGroups>
  </Border>
</UserControl>"
    );
    let error = try_load(&xaml).err().expect("an error");
    assert!(describe(&error).contains("no element named 'Missing'"), "{}", describe(&error));
}

#[test]
fn a_setter_target_in_a_style_is_an_error() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<Border {NS} x:Name='Root'>
  <Border.Styles>
    <Style Selector='Border'>
      <Setter Target='Root.Opacity' Value='0.5'/>
    </Style>
  </Border.Styles>
</Border>"
    );
    let error = try_load(&xaml).err().expect("an error");
    assert!(describe(&error).contains("only valid in the setters of a visual state"), "{}", describe(&error));
}
