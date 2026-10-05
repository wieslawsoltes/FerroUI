//! Port of `SetterTests.cs`.

use ferroui_base::animation::Animation;
use ferroui_base::Ref;
use ferroui_controls::ContentControl;

use crate::support::app::styled_window_application;
use crate::support::helpers::animation_setter_as_setter;
use crate::support::loader::load_as;

#[test]
fn setter_target_type_should_understand_x_type_extensions() {
    let _app = styled_window_application();
    let xaml = "
<Animation xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:SetterTargetType='{x:Type ContentControl}'>
    <KeyFrame>
        <Setter Property='Content' Value='{Binding}'/>
    </KeyFrame>
    <KeyFrame>
        <Setter Property='Content' Value='{Binding}'/>
    </KeyFrame> 
</Animation>";
    let animation = load_as::<Ref<Animation>>(xaml);
    let setters = animation.children().get(0).setters();
    let setter = setters.get(0);
    let setter = animation_setter_as_setter(&setter);

    let property = setter.property();
    assert!(property.is_some());
    assert!(std::ptr::eq(ContentControl::TYPE, property.unwrap().owner_type()));
}

#[test]
fn setter_target_type_should_understand_type_from_xmlns() {
    let _app = styled_window_application();
    let xaml = "
<av:Animation xmlns:av='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:SetterTargetType='av:ContentControl'>
    <av:KeyFrame>
        <av:Setter Property='Content' Value='{av:Binding}'/>
    </av:KeyFrame>
    <av:KeyFrame>
        <av:Setter Property='Content' Value='{av:Binding}'/>
    </av:KeyFrame> 
</av:Animation>";
    let animation = load_as::<Ref<Animation>>(xaml);
    let setters = animation.children().get(0).setters();
    let setter = setters.get(0);
    let setter = animation_setter_as_setter(&setter);

    let property = setter.property();
    assert!(property.is_some());
    assert!(std::ptr::eq(ContentControl::TYPE, property.unwrap().owner_type()));
}
