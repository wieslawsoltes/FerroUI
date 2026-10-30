//! Visual states in markup (an addition, docs/porting/winui-calculator-gaps.md, section 5.9):
//! `VisualStateManager.VisualStateGroups` and the setters of visual states
//! (`<Setter Target="Element.Property" Value="..."/>`), loaded at run time.

use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::styling::{Setter, TargetPropertyPath};
use ferroui_base::Ref;
use ferroui_controls::{Border, ContentControl, Control, Grid, StackPanel, TextBlock, UserControl, VisualStateManager};
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
    let panel = load_as::<Ref<StackPanel>>(&xaml);
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
    let solid = brush.as_solid_color_brush().map(|brush| brush.color());
    assert_eq!(solid, Some(ferroui_base::media::Colors::BLUE));
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

/// The target of the setter at `index` of the state at `state` of the first group of `root`.
fn target_of(root: &Control, state: usize, index: usize) -> Rc<TargetPropertyPath> {
    let groups = VisualStateManager::get_visual_state_groups(root);
    let setter = groups.get(0).states().get(state).setters().get(index);
    let setter = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()).expect("a setter");
    setter.target().expect("a target")
}

fn user_control_with_late_elements() -> (Ref<UserControl>, Ref<Grid>) {
    let xaml = format!(
        "
<UserControl {NS}>
  <Grid x:Name='LayoutRoot'>
    <VisualStateManager.VisualStateGroups>
      <VisualStateGroup>
        <VisualState x:Name='Normal'/>
        <VisualState x:Name='Dimmed'>
          <VisualState.Setters>
            <Setter Target='Late.Opacity' Value='0.25'/>
            <Setter Target='LayoutRoot.Opacity' Value='0.5'/>
            <Setter Target='Later.Text' Value='Dimmed'/>
          </VisualState.Setters>
        </VisualState>
      </VisualStateGroup>
    </VisualStateManager.VisualStateGroups>
    <StackPanel>
      <Border>
        <Border x:Name='Late'/>
      </Border>
      <TextBlock x:Name='Later' Text='Normal'/>
    </StackPanel>
  </Grid>
</UserControl>"
    );
    let control = load_as::<Ref<UserControl>>(&xaml);
    let grid = Control::from_boxed(&control.content().unwrap()).unwrap().cast::<Grid>().unwrap();
    (control, grid)
}

fn late_elements(grid: &Grid) -> (Ref<Border>, Ref<TextBlock>) {
    let panel = grid.children().get(0).cast::<StackPanel>().unwrap();
    let outer = panel.children().get(0).cast::<Border>().unwrap();
    let late = outer.child().unwrap().cast::<Border>().unwrap();
    let later = panel.children().get(1).cast::<TextBlock>().unwrap();
    (late, later)
}

// Forward references: WinUI resolves the target of a visual state setter by its name when the
// state applies, so a setter may name an element declared after it.

#[test]
fn a_setter_in_a_template_names_an_element_declared_after_it() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<ContentControl {NS}>
  <ContentControl.Template>
    <ControlTemplate>
      <Border x:Name='Root'>
        <VisualStateManager.VisualStateGroups>
          <VisualStateGroup>
            <VisualState x:Name='Normal'/>
            <VisualState x:Name='Pressed'>
              <VisualState.Setters>
                <Setter Target='Deep.Opacity' Value='0.25'/>
              </VisualState.Setters>
            </VisualState>
          </VisualStateGroup>
        </VisualStateManager.VisualStateGroups>
        <Border>
          <Border x:Name='Deep'/>
        </Border>
      </Border>
    </ControlTemplate>
  </ContentControl.Template>
</ContentControl>"
    );
    let control = load_as::<Ref<ContentControl>>(&xaml);
    control.apply_template();
    let (_, outer) = inner_of(&control);
    let deep = outer.child().unwrap().cast::<Border>().unwrap();

    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    assert_eq!(deep.opacity(), 0.25);
    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert_eq!(deep.opacity(), 1.0);
}

#[test]
fn a_setter_in_a_document_names_elements_declared_after_it() {
    let _app = styled_window_application();
    let (control, grid) = user_control_with_late_elements();
    let (late, later) = late_elements(&grid);

    assert!(VisualStateManager::go_to_state(&control, "Dimmed", false));
    assert_eq!(late.opacity(), 0.25);
    assert_eq!(later.text().as_deref(), Some("Dimmed"));
    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert_eq!(late.opacity(), 1.0);
    assert_eq!(later.text().as_deref(), Some("Normal"));
}

#[test]
fn a_setter_names_the_element_that_has_the_groups() {
    let _app = styled_window_application();
    let (control, grid) = user_control_with_late_elements();

    assert!(VisualStateManager::go_to_state(&control, "Dimmed", false));
    assert_eq!(grid.opacity(), 0.5);
    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert_eq!(grid.opacity(), 1.0);
}

#[test]
fn a_name_in_a_template_is_the_element_of_the_template_not_of_the_document() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<StackPanel {NS}>
  <Border x:Name='Inner'/>
  <ContentControl>
    <ContentControl.Template>
      <ControlTemplate>
        <Border>
          <VisualStateManager.VisualStateGroups>
            <VisualStateGroup>
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
    </ContentControl.Template>
  </ContentControl>
</StackPanel>"
    );
    let panel = load_as::<Ref<StackPanel>>(&xaml);
    let document_inner = panel.children().get(0).cast::<Border>().unwrap();
    let control = panel.children().get(1).cast::<ContentControl>().unwrap();
    control.apply_template();

    assert!(VisualStateManager::go_to_state(&control, "Pressed", false));
    assert_eq!(inner_of(&control).1.opacity(), 0.25);
    assert_eq!(document_inner.opacity(), 1.0);
}

#[test]
fn an_explicit_target_object_is_set_without_a_name() {
    let _app = styled_window_application();
    let (control, grid) = user_control_with_late_elements();
    let (late, later) = late_elements(&grid);
    // The first setter targets `Late`; given the object `Later` as its target, it sets that.
    let path = target_of(&grid, 1, 0);
    path.set_target(Some(later.clone().upcast()));

    assert!(VisualStateManager::go_to_state(&control, "Dimmed", false));
    assert_eq!(later.opacity(), 0.25);
    assert_eq!(late.opacity(), 1.0);
}

#[test]
fn a_name_not_in_the_scope_when_the_state_applies_sets_nothing() {
    let _app = styled_window_application();
    let (control, grid) = user_control_with_late_elements();
    let (late, later) = late_elements(&grid);
    target_of(&grid, 1, 0).set_target_name("Missing");

    // The state changes; the setter of the missing element does nothing (a warning) and the
    // other setters apply.
    assert!(VisualStateManager::go_to_state(&control, "Dimmed", false));
    assert_eq!(late.opacity(), 1.0);
    assert_eq!(grid.opacity(), 0.5);
    assert_eq!(later.text().as_deref(), Some("Dimmed"));
    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert_eq!(grid.opacity(), 1.0);
}
