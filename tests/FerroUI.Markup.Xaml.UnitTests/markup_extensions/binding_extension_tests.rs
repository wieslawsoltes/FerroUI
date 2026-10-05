//! Ported from the upstream `MarkupExtensions/BindingExtensionTests`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::styling::{styles_as_style, IStyle, Selectors, Setter, Style, Styles};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{TemplatedControl, VisualLayerManager};
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::UnitTestApplicationScope;
use ferroui_controls::{ContentControl, TextBlock, Window};

use super::compiled_binding_extension_tests::TestDataContext;
use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;

// --- test types -------------------------------------------------------------

/// The outer class of [`Nested`].
pub struct ReflectionOuter;

crate::test_identity_eq!(ReflectionOuter);

impl ReflectionOuter {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}

ferro_markup_type!(class ReflectionOuter {
    this: Rc<ReflectionOuter>,
    handles: [ReflectionOuter, Rc<ReflectionOuter>, Option<Rc<ReflectionOuter>>],
    constructors: [() => ReflectionOuter::new],
});

/// The class nested in [`ReflectionOuter`].
pub struct Nested {
    nested_property: RefCell<String>,
}

crate::test_identity_eq!(Nested);

impl Nested {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { nested_property: RefCell::new("nested".to_string()) })
    }

    pub fn nested_property(&self) -> String {
        self.nested_property.borrow().clone()
    }

    pub fn set_nested_property(&self, value: String) {
        self.nested_property.replace(value);
    }
}

ferro_markup_type!(class Nested as "ReflectionOuter+Nested" {
    this: Rc<Nested>,
    handles: [Nested, Rc<Nested>, Option<Rc<Nested>>],
    constructors: [() => Nested::new],
    properties: [
        NestedProperty: String { get: Nested::nested_property, set: Nested::set_nested_property },
    ],
});

pub struct FooBar;

crate::test_identity_eq!(FooBar);

impl FooBar {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    pub fn foo(&self) -> Option<BoxedValue> {
        None
    }
}

ferro_markup_type!(class FooBar as "BindingExtensionTests+FooBar" {
    this: Rc<FooBar>,
    handles: [FooBar, Rc<FooBar>, Option<Rc<FooBar>>],
    constructors: [() => FooBar::new],
    properties: [
        Foo: Option<BoxedValue> { get: FooBar::foo },
    ],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[
        <ReflectionOuter as MarkupTyped>::MARKUP,
        <Nested as MarkupTyped>::MARKUP,
        <FooBar as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<ReflectionOuter>();
        ValueTypes::register_reference::<Nested>();
        ValueTypes::register_reference::<FooBar>();
    },
};

// --- tests ------------------------------------------------------------------

#[test]
fn support_cast_to_nested_type_in_expression() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        >
    <ContentControl Content='{Binding $parent.((local:ReflectionOuter+Nested)DataContext).NestedProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = Nested::new();
    data_context.set_nested_property("hello".to_string());
    window.set_data_context(Some(data_context));

    assert_string("hello", &content_control.content());
}

#[test]
fn binding_extension_binds_to_source() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <x:String x:Key='text'>foobar</x:String>
    </Window.Resources>

    <TextBlock Name='textBlock' Text='{Binding Source={StaticResource text}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.show();

    assert_eq!(text_block.text().as_deref(), Some("foobar"));
}

#[test]
fn binding_extension_binds_to_target_null_value() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <x:String x:Key='text'>foobar</x:String>
    </Window.Resources>

    <TextBlock Name='textBlock' Text='{Binding Foo, TargetNullValue={StaticResource text}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.set_data_context(Some(FooBar::new()));
    window.show();

    assert_eq!(text_block.text().as_deref(), Some("foobar"));
}

#[test]
fn binding_extension_target_null_value_unset_by_default() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Name='textBlock' IsVisible='{Binding Foo, Converter={x:Static ObjectConverters.IsNotNull}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.set_data_context(Some(FooBar::new()));
    window.show();

    assert!(!text_block.is_visible());
}

#[test]
fn support_cast_to_type_in_expression() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        >
    <ContentControl Content='{Binding $parent.((local:TestDataContext)DataContext).StringProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), string_of(&content_control.content()));
}

#[test]
fn support_cast_to_type_in_expression_different_type_evaluates_to_null() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        >
    <ContentControl Content='{Binding $parent.((local:TestDataContext)DataContext)}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = boxed_str("foo");

    window.set_data_context(data_context);

    assert!(content_control.content().is_none());
}

fn styled_window() -> UnitTestApplicationScope {
    let services = TestServices::styled_window().with_theme(|| -> Rc<dyn IStyle> {
        let styles = Styles::new();
        styles.add(window_style());
        styles_as_style(&styles)
    });

    unit_test_application(services)
}

fn window_style() -> Ref<Style> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<Window>(|x, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_indexer(
            &ContentPresenter::content_property().bind(),
            &x.indexer(&ContentControl::content_property().bind()),
        );
        let layer_manager = VisualLayerManager::new();
        layer_manager.set_child(presenter.register_in_name_scope(&**scope));
        layer_manager.upcast()
    });

    Style::with_setters(Selectors::of_type::<Window>(), [Setter::new(TemplatedControl::template_property(), Some(template))])
}
