//! Ported from the upstream `Data/BindingTests`.

use std::rc::{Rc, Weak};

use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::Colors;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::{Border, TextBlock, TextBox, Window};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;
use crate::support_bindings::*;

// --- test types -------------------------------------------------------------

anonymous_object!(AnonymousHexString { HexString: Option<BoxedValue> => hex_string });

/// Joins the text of the values with commas.
pub struct ConcatConverter {
    this: Weak<ConcatConverter>,
}

crate::test_identity_eq!(ConcatConverter);

impl ConcatConverter {
    pub fn new() -> Rc<ConcatConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The shared instance.
    pub fn instance() -> Rc<ConcatConverter> {
        thread_local! {
            static INSTANCE: Rc<ConcatConverter> = ConcatConverter::new();
        }
        INSTANCE.with(Rc::clone)
    }

    /// The converter as the multi-value converter contract.
    pub fn as_multi_value_converter(&self) -> Rc<dyn IMultiValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IMultiValueConverter for ConcatConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // The text of a null value is empty, as in a joined string.
        let texts: Vec<String> = values
            .iter()
            .map(|value| value.as_ref().map(|value| ValueTypes::to_display_string(Some(value))).unwrap_or_default())
            .collect();
        Ok(Some(Rc::new(texts.join(","))))
    }
}

ferro_markup_type!(class ConcatConverter {
    this: Rc<ConcatConverter>,
    handles: [ConcatConverter, Rc<ConcatConverter>, Option<Rc<ConcatConverter>>],
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => ConcatConverter::new],
    fields: [Instance: Rc<ConcatConverter> => ConcatConverter::instance],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<AnonymousHexString as MarkupTyped>::MARKUP, <ConcatConverter as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<AnonymousHexString>();
        ValueTypes::register_reference::<ConcatConverter>();
        ValueTypes::register_cast::<ConcatConverter, Rc<dyn IMultiValueConverter>>(
            ConcatConverter::as_multi_value_converter,
        );
    },
};

// --- tests ------------------------------------------------------------------

#[test]
fn binding_with_null_path_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Name='textBlock' Text='{Binding}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.set_data_context(boxed_str("foo"));
    window.apply_template();

    assert_eq!(text_block.text().as_deref(), Some("foo"));
}

#[test]
fn binding_to_do_nothing_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Name='textBlock' Text='{Binding}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.apply_template();

    window.set_data_context(boxed_str("foo"));
    assert_eq!(text_block.text().as_deref(), Some("foo"));

    window.set_data_context(Some(BindingOperations::do_nothing()));
    assert_eq!(text_block.text().as_deref(), Some("foo"));

    window.set_data_context(boxed_str("bar"));
    assert_eq!(text_block.text().as_deref(), Some("bar"));
}

#[test]
fn multi_binding_templated_parent_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Data;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBox Name='textBox' Text='Foo' PlaceholderText='Bar'>
        <TextBox.Template>
            <ControlTemplate>
                <TextPresenter Name='PART_TextPresenter'>
                    <TextPresenter.Text>
                        <MultiBinding Converter='{x:Static local:ConcatConverter.Instance}'>
                            <Binding RelativeSource='{RelativeSource TemplatedParent}' Path='Text'/>
                            <Binding RelativeSource='{RelativeSource TemplatedParent}' Path='PlaceholderText'/>
                        </MultiBinding>
                    </TextPresenter.Text>
                </TextPresenter>
            </ControlTemplate>
        </TextBox.Template>
    </TextBox>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_box = window.get_control::<TextBox>("textBox");

    window.apply_template();
    text_box.apply_template();

    let children = text_box.get_visual_children();
    assert_eq!(children.len(), 1);
    let target = children[0].clone().cast::<TextPresenter>().expect("the visual child is a text presenter");
    assert_eq!(target.text().as_deref(), Some("Foo,Bar"));
}

#[test]
fn can_bind_brush_to_hex_string() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Data;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Border Background='{Binding HexString}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let border = object_of::<Border>(&window.content());
    window.set_data_context(Some(Rc::new(AnonymousHexString { hex_string: boxed_str("#ff0000") })));

    window.apply_template();

    let background = border.background().expect("the border has a background");
    let brush = background
        .as_any()
        .downcast_ref::<ImmutableSolidColorBrush>()
        .expect("the background is an immutable solid color brush");
    assert_eq!(brush.color(), Colors::RED);
}
