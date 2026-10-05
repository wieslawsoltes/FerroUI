//! Port of `Xaml/DataTemplateTests.cs`.

use std::rc::Rc;

use ferroui_base::data::core::ValueType;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Canvas, ContentControl, ItemsControl, ItemsSource, Window};
use ferroui_markup_xaml::templates::DataTemplate;

use crate::markup_extensions::compiled_binding_extension_tests::CustomDataTemplate;
use crate::support::app::styled_window_application;
use crate::support::helpers::{assert_is_type, boxed, value_of};
use crate::support::loader::{describe, load_as, try_load, xaml_error};
use crate::support::TestViewModel;

/// `Assert.Same(expected, value)` for an untyped value and a view model.
#[track_caller]
fn assert_same(expected: &Rc<TestViewModel>, value: &Option<BoxedValue>) {
    let actual = value_of::<Rc<TestViewModel>>(value).expect("the value is not a TestViewModel");
    assert!(Rc::ptr_eq(expected, &actual), "the value is not the expected instance");
}

#[test]
fn data_template_can_be_empty() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.DataTemplates>
        <DataTemplate DataType='{x:Type sys:String}' />
    </Window.DataTemplates>
    <ContentControl Name='target' Content='Foo'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let target = window.get_control::<ContentControl>("target");

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().expect("the presenter is null");
    presenter.update_child();

    assert!(presenter.child().is_none());
}

#[test]
fn data_template_can_contain_name() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.DataTemplates>
        <DataTemplate DataType='{x:Type sys:String}'>
            <Canvas Name='foo'/>
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl Name='target' Content='Foo'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let target = window.get_control::<ContentControl>("target");

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().expect("the presenter is null");
    presenter.update_child();

    assert_is_type::<Canvas>(&presenter.child().expect("the child is null"));
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn data_template_can_contain_named_user_control() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=mscorlib'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ItemsControl Name='itemsControl' ItemsSource='{Binding}'>
        <ItemsControl.ItemTemplate>
            <DataTemplate>
                <UserControl Name='foo'/>
            </DataTemplate>
        </ItemsControl.ItemTemplate>
    </ItemsControl>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let items_control = window.get_control::<ItemsControl>("itemsControl");

    window.set_data_context(Some(boxed(ItemsSource::from_strs(["item1", "item2"]))));

    window.apply_template();
    items_control.apply_template();
    let presenter = items_control.presenter().expect("the presenter is null");
    presenter.apply_template();

    assert_eq!(2, presenter.panel().expect("the panel is null").children().count());
}

#[test]
fn xdata_type_should_be_assigned_to_clr_property() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.DataTemplates>
        <DataTemplate x:DataType='sys:String'>
            <Canvas Name='foo'/>
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl Name='target' Content='Foo'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let target = window.get_control::<ContentControl>("target");
    let first = window.data_templates().get(0);
    let template = first
        .as_any()
        .and_then(|template| template.downcast_ref::<DataTemplate>())
        .expect("the first data template is not a DataTemplate");

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().expect("the presenter is null");
    presenter.update_child();

    assert!(template.data_type() == Some(ValueType::of::<String>()));
    assert_is_type::<Canvas>(&presenter.child().expect("the child is null"));
}

#[test]
fn xdata_type_should_be_ignored_if_data_type_already_set() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.DataTemplates>
        <DataTemplate DataType='sys:String' x:DataType='UserControl'>
            <Canvas Name='foo'/>
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl Name='target' Content='Foo'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let target = window.get_control::<ContentControl>("target");

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().expect("the presenter is null");
    presenter.update_child();

    assert_is_type::<Canvas>(&presenter.child().expect("the child is null"));
}

#[test]
fn xdata_type_should_be_ignored_if_data_type_has_non_standard_name() {
    // We don't want DataType to be mapped to FancyDataType, avoid possible confusion.
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <ContentControl Name='target' Content='Foo'>
        <ContentControl.ContentTemplate>
            <local:CustomDataTemplate x:DataType='local:TestDataContext'>
                <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
            </local:CustomDataTemplate>
        </ContentControl.ContentTemplate>
    </ContentControl>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let target = window.get_control::<ContentControl>("target");

    window.apply_template();
    target.apply_template();
    target.presenter().expect("the presenter is null").update_child();

    let content_template = target.content_template().expect("the content template is null");
    let data_template = content_template
        .as_any()
        .and_then(|template| template.downcast_ref::<CustomDataTemplate>())
        .expect("the content template is not a CustomDataTemplate");
    assert!(data_template.fancy_data_type().is_none());
}

#[test]
fn can_set_data_context_in_data_template() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.DataTemplates>
        <DataTemplate DataType='{x:Type local:TestViewModel}'>
            <Canvas Name='foo' DataContext='{Binding Child}'/>
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl Name='target' Content='{Binding Child}'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let target = window.get_control::<ContentControl>("target");

    let grandchild = TestViewModel::new();
    grandchild.set_string(Some("Grandchild".to_string()));
    let child = TestViewModel::new();
    child.set_string(Some("Child".to_string()));
    child.set_child(Some(grandchild.clone()));
    let view_model = TestViewModel::new();
    view_model.set_string(Some("Root".to_string()));
    view_model.set_child(Some(child.clone()));

    let data_context: BoxedValue = view_model.clone();
    window.set_data_context(Some(data_context));

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().expect("the presenter is null");
    presenter.update_child();

    let canvas = presenter.child().expect("the child is null").cast::<Canvas>().expect("the child is not a Canvas");
    assert_same(&view_model, &target.data_context());
    assert_same(&child, &presenter.data_context());
    assert_same(&grandchild, &canvas.data_context());
}

#[test]
fn data_templates_without_type_should_throw() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.DataTemplates>
        <DataTemplate>
            <Canvas Name='foo'/>
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl Name='target' Content='Foo'/>
</Window>";
    // `Assert.Throws<InvalidOperationException>`: the load fails, and not with an error of the compiler.
    let error = match try_load(xaml) {
        Ok(_) => panic!("expected an invalid operation, the document loaded"),
        Err(error) => error,
    };
    assert!(xaml_error(&error).is_none(), "expected an invalid operation: {}", describe(&error));
}
