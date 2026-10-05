//! Ported from the upstream `Data/BindingTests_TemplatedParent`.

use ferroui_base::Ref;
use ferroui_controls::{Button, Grid, GridLength, GridUnitType, TextBlock, Window};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;
use crate::support_bindings::*;

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule::EMPTY;

#[test]
fn template_binding_with_null_path_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button'>
       <Button.Template>
         <ControlTemplate>
           <TextBlock Tag='{TemplateBinding}'/>
         </ControlTemplate>
       </Button.Template>
    </Button>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();
    button.apply_template();

    let children = button.get_visual_children();
    assert_eq!(children.len(), 1);
    let text_block = children[0].clone().cast::<TextBlock>().expect("the visual child is a text block");
    assert!(is_same(&text_block.tag(), &button));
}

#[test]
fn binds_to_templated_parent_from_non_control() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button'>
      <Button.Template>
        <ControlTemplate>
          <Grid>
            <Grid.ColumnDefinitions>
              <ColumnDefinition Width='{Binding RelativeSource={RelativeSource TemplatedParent}, Path=Tag}'/>
            </Grid.ColumnDefinitions>
          </Grid>
        </ControlTemplate>
      </Button.Template>
    </Button>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    button.set_tag(Some(boxed(GridLength::new(5.0, GridUnitType::Star))));

    window.apply_template();
    button.apply_template();

    let grid = button
        .get_template_descendants()
        .into_iter()
        .find_map(|descendant| descendant.cast::<Grid>())
        .expect("the template has a grid");
    assert_eq!(value_of::<GridLength>(&button.tag()), Some(grid.column_definitions().get(0).width()));
}
