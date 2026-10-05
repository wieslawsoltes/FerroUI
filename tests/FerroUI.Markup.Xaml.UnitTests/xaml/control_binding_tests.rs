//! Ported from the upstream `Xaml/ControlBindingTests`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::primitives::TabStrip;
use ferroui_controls::{Carousel, ItemsSource, ProgressBar, Window};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;
use crate::support_bindings::*;

// --- test types -------------------------------------------------------------

anonymous_object!(AnonymousValue { Value: Option<BoxedValue> => value });

pub struct ItemsViewModel {
    items: RefCell<Option<ItemsSource>>,
}

crate::test_identity_eq!(ItemsViewModel);

impl ItemsViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(None) })
    }

    pub fn items(&self) -> Option<ItemsSource> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<ItemsSource>) {
        self.items.replace(value);
    }
}

ferro_markup_type!(class ItemsViewModel as "ControlBindingTests+ItemsViewModel" {
    this: Rc<ItemsViewModel>,
    handles: [ItemsViewModel, Rc<ItemsViewModel>, Option<Rc<ItemsViewModel>>],
    constructors: [() => ItemsViewModel::new],
    properties: [
        Items: Option<ItemsSource> { get: ItemsViewModel::items, set: ItemsViewModel::set_items },
    ],
});

pub struct ItemViewModel {
    header: RefCell<Option<String>>,
    detail: RefCell<Option<String>>,
}

crate::test_identity_eq!(ItemViewModel);

impl ItemViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { header: RefCell::new(None), detail: RefCell::new(None) })
    }

    pub fn with(header: &str, detail: &str) -> Rc<Self> {
        let result = Self::new();
        result.set_header(Some(header.to_string()));
        result.set_detail(Some(detail.to_string()));
        result
    }

    pub fn header(&self) -> Option<String> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<String>) {
        self.header.replace(value);
    }

    pub fn detail(&self) -> Option<String> {
        self.detail.borrow().clone()
    }

    pub fn set_detail(&self, value: Option<String>) {
        self.detail.replace(value);
    }
}

ferro_markup_type!(class ItemViewModel as "ControlBindingTests+ItemViewModel" {
    this: Rc<ItemViewModel>,
    handles: [ItemViewModel, Rc<ItemViewModel>, Option<Rc<ItemViewModel>>],
    constructors: [() => ItemViewModel::new],
    properties: [
        Header: Option<String> { get: ItemViewModel::header, set: ItemViewModel::set_header },
        Detail: Option<String> { get: ItemViewModel::detail, set: ItemViewModel::set_detail },
    ],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[
        <AnonymousValue as MarkupTyped>::MARKUP,
        <ItemsViewModel as MarkupTyped>::MARKUP,
        <ItemViewModel as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<AnonymousValue>();
        ValueTypes::register_reference::<ItemsViewModel>();
        ValueTypes::register_reference::<ItemViewModel>();
    },
};

// --- tests ------------------------------------------------------------------

#[test]
fn binding_progress_bar_value_to_invalid_value_uses_fallback_value() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'>
    <ProgressBar Maximum='10' Value='{Binding Value, FallbackValue=3}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let progress_bar = object_of::<ProgressBar>(&window.content());

    window.set_data_context(Some(Rc::new(AnonymousValue { value: boxed_str("foo") })));
    window.apply_template();

    assert_eq!(progress_bar.value(), 3.0);
}

#[test]
fn can_bind_between_tab_strip_and_carousel() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'>
    <DockPanel>
        <TabStrip Name='strip' DockPanel.Dock='Top' ItemsSource='{Binding Items}' SelectedIndex='0'>
          <TabStrip.ItemTemplate>
            <DataTemplate>
              <TextBlock Text='{Binding Header}'/>
            </DataTemplate>
          </TabStrip.ItemTemplate>
        </TabStrip>
        <Carousel Name='carousel' ItemsSource='{Binding Items}' SelectedIndex='{Binding #strip.SelectedIndex}'>
          <Carousel.ItemTemplate>
            <DataTemplate>
              <TextBlock Text='{Binding Detail}'/>
            </DataTemplate>
          </Carousel.ItemTemplate>
        </Carousel>
    </DockPanel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let strip = window.get_control::<TabStrip>("strip");
    let carousel = window.get_control::<Carousel>("carousel");

    let view_model = ItemsViewModel::new();
    let items: Vec<Option<BoxedValue>> = vec![
        Some(ItemViewModel::with("Item1", "Detail1")),
        Some(ItemViewModel::with("Item2", "Detail2")),
    ];
    view_model.set_items(Some(ItemsSource::from_items(items)));
    window.set_data_context(Some(view_model));

    window.show();

    assert_eq!(strip.selected_index(), 0);
    assert_eq!(carousel.selected_index(), 0);
}
