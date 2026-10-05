//! Port of `Xaml/GenericTemplateTests.cs`.

use std::rc::Rc;

use ferroui_base::{FerroObject, Ref};
use ferroui_markup_xaml::templates::TemplateContent;

use crate::support::app::styled_window_application;
use crate::support::loader::load_local_as;
use crate::support::xaml::generic_template_tests::{SampleTemplatedObject, SampleTemplatedObjectContainer};

#[test]
fn data_template_can_be_empty() {
    let _app = styled_window_application();
    let xaml = "
<s:SampleTemplatedObjectContainer xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:s='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <s:SampleTemplatedObjectContainer.Template>
        <s:SampleTemplatedObjectTemplate>
            <s:SampleTemplatedObject x:Name='root'>
                <s:SampleTemplatedObject x:Name='child1' Foo='foo' />
                <s:SampleTemplatedObject x:Name='child2' Foo='bar' />
            </s:SampleTemplatedObject>
        </s:SampleTemplatedObjectTemplate>
    </s:SampleTemplatedObjectContainer.Template>
</s:SampleTemplatedObjectContainer>";
    let container = load_local_as::<Rc<SampleTemplatedObjectContainer>>(xaml);
    let template = container.template().expect("the template is null");
    let res = TemplateContent::load_as::<Ref<SampleTemplatedObject>>(template.content().as_ref())
        .expect("the template content built nothing");
    let result = res.result();
    assert_eq!(Some(result.clone().upcast::<FerroObject>()), res.name_scope().find("root"));
    assert_eq!(Some(result.content().get(0).upcast::<FerroObject>()), res.name_scope().find("child1"));
    assert_eq!(Some(result.content().get(1).upcast::<FerroObject>()), res.name_scope().find("child2"));
    assert_eq!(Some("foo".to_string()), result.content().get(0).foo());
    assert_eq!(Some("bar".to_string()), result.content().get(1).foo());
}
