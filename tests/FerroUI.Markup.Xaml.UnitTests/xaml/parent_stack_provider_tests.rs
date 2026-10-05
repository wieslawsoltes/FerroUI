//! Port of `Xaml/ParentStackProviderTests.cs`.

use ferroui_base::media::SolidColorBrush;
use ferroui_base::{BoxedValue, FerroLocator, Ref};
use ferroui_controls::testing::UnitTestApplication;
use ferroui_controls::Window;

use crate::support::app::styled_window_application;
use crate::support::helpers::assert_value_is_type;
use crate::support::loader::load_as;
use crate::support::xaml::parent_stack_provider_tests::CapturedParents;

#[track_caller]
fn verify_parents(parents: Option<Vec<BoxedValue>>) {
    let parents = parents.expect("the parents were not captured");
    assert!(!parents.is_empty());
    assert_eq!(3, parents.len(), "the collection does not contain exactly three parents");
    assert_value_is_type::<SolidColorBrush>(&Some(parents[0].clone()));
    assert_value_is_type::<Window>(&Some(parents[1].clone()));
    assert_value_is_type::<UnitTestApplication>(&Some(parents[2].clone()));
}

#[test]
fn parents_are_correct_for_deferred_content() {
    let _app = styled_window_application();

    let captured_parents = CapturedParents::new();
    FerroLocator::current_mutable().bind_to_self(captured_parents.clone());

    let window = load_as::<Ref<Window>>(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>

  <Window.Resources>
    <SolidColorBrush x:Key='Brush' Color='{local:CapturingParentsMarkupExtension}' />
  </Window.Resources>

  <TextBlock Foreground='{StaticResource Brush}' />

</Window>",
    );

    window.show();

    verify_parents(captured_parents.lazy_parents());
    verify_parents(captured_parents.eager_parents());
}
