//! Port of `Xaml/ProvideValueTargetTests.cs`.

use std::rc::Rc;

use ferroui_base::data::core::{ClrPropertyInfo, IPropertyInfo};
use ferroui_base::{FerroLocator, FerroProperty};
use ferroui_controls::documents::TextElement;
use ferroui_controls::{TextBlock, Window};
use ferroui_markup_xaml::markup_extensions::CompiledBindingExtension;

use crate::support::app::styled_window_application;
use crate::support::helpers::{assert_value_is_type, value_of};
use crate::support::loader::load;
use crate::support::xaml::provide_value_target_tests::CapturedTargets;

#[test]
fn provide_value_target_has_correct_targets_set() {
    let _app = styled_window_application();

    let captured_targets = CapturedTargets::new();
    FerroLocator::current_mutable().bind_to_self(captured_targets.clone());

    load(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
        Foreground='{local:CapturingTargetsMarkup}'
        x:CompileBindings='True'>

  <TextBlock Tag='{Binding Source={local:CapturingTargetsMarkup}}'
             Background='{local:CapturingTargetsMarkup}' />

</Window>",
    );

    let targets = captured_targets.targets();
    assert_eq!(3, targets.len(), "the collection does not contain exactly three targets");

    {
        let (target_object, target_property) = &targets[0];
        assert_value_is_type::<Window>(target_object);
        assert!(
            Some(TextElement::foreground_property().as_property())
                == value_of::<&'static FerroProperty>(target_property)
        );
    }
    {
        let (target_object, target_property) = &targets[1];
        assert!(
            value_of::<Rc<CompiledBindingExtension>>(target_object).is_some(),
            "the target object is not a CompiledBindingExtension"
        );
        let prop = value_of::<Rc<dyn IPropertyInfo>>(target_property).unwrap_or_else(|| {
            panic!("the target property is not a property info: {}", crate::support::helpers::value_type_name(target_property))
        });
        let prop = prop
            .as_any()
            .and_then(|prop| prop.downcast_ref::<ClrPropertyInfo>())
            .expect("the target property is not a ClrPropertyInfo");
        assert_eq!("Source", IPropertyInfo::name(prop));
    }
    {
        let (target_object, target_property) = &targets[2];
        assert_value_is_type::<TextBlock>(target_object);
        assert!(
            Some(TextBlock::background_property().as_property())
                == value_of::<&'static FerroProperty>(target_property)
        );
    }
}
