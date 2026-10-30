// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IMultiValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use ferroui_base::{
    layout::{HorizontalAlignment, VerticalAlignment},
    Thickness,
};
use std::rc::Rc;
/// Original BorderFillMarginConverter binding converter.
pub struct BorderFillMarginConverter;
impl PartialEq for BorderFillMarginConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl BorderFillMarginConverter {
    /// Creates the original stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<BorderFillMarginConverter>=BorderFillMarginConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IMultiValueConverter for BorderFillMarginConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let mut margin = Thickness::default();
        if values.len() == 5 {
            if let (Some(width), Some(height), Some(visible), Some(vertical), Some(horizontal)) = (
                values[0].as_ref().and_then(|v| v.downcast_ref::<f64>()),
                values[1].as_ref().and_then(|v| v.downcast_ref::<f64>()),
                values[2].as_ref().and_then(|v| v.downcast_ref::<bool>()),
                values[3]
                    .as_ref()
                    .and_then(|v| v.downcast_ref::<VerticalAlignment>()),
                values[4]
                    .as_ref()
                    .and_then(|v| v.downcast_ref::<HorizontalAlignment>()),
            ) {
                if *visible {
                    if *vertical == VerticalAlignment::Top
                        && *horizontal == HorizontalAlignment::Stretch
                    {
                        margin = Thickness::new(0.0, *height, 0.0, 0.0);
                    } else if *vertical == VerticalAlignment::Stretch
                        && *horizontal == HorizontalAlignment::Left
                    {
                        margin = Thickness::new(*width, 0.0, 0.0, 0.0);
                    }
                }
            }
        }
        Ok(Some(Rc::new(margin)))
    }
}
ferroui_base::ferro_markup_type!(class BorderFillMarginConverter {
    namespace:"FerroDock.Ferro.Converters",this:Rc<BorderFillMarginConverter>,handles:[Rc<BorderFillMarginConverter>,Option<Rc<BorderFillMarginConverter>>],interfaces:[Rc<dyn IMultiValueConverter>],constructors:[()=>BorderFillMarginConverter::new],fields:[Instance:Rc<BorderFillMarginConverter> =>BorderFillMarginConverter::instance],
});

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_margin_requires_all_five_typed_values_and_only_two_visible_edges() {
        let c = BorderFillMarginConverter::new();
        for visible in [false, true] {
            for vertical in [
                VerticalAlignment::Top,
                VerticalAlignment::Center,
                VerticalAlignment::Bottom,
                VerticalAlignment::Stretch,
            ] {
                for horizontal in [
                    HorizontalAlignment::Left,
                    HorizontalAlignment::Center,
                    HorizontalAlignment::Right,
                    HorizontalAlignment::Stretch,
                ] {
                    let values: Vec<Option<BoxedValue>> = vec![
                        Some(Rc::new(17.0_f64)),
                        Some(Rc::new(23.0_f64)),
                        Some(Rc::new(visible)),
                        Some(Rc::new(vertical)),
                        Some(Rc::new(horizontal)),
                    ];
                    let expected = if visible
                        && vertical == VerticalAlignment::Top
                        && horizontal == HorizontalAlignment::Stretch
                    {
                        Thickness::new(0.0, 23.0, 0.0, 0.0)
                    } else if visible
                        && vertical == VerticalAlignment::Stretch
                        && horizontal == HorizontalAlignment::Left
                    {
                        Thickness::new(17.0, 0.0, 0.0, 0.0)
                    } else {
                        Thickness::default()
                    };
                    let result = c
                        .convert(
                            &values,
                            ValueType::of::<Thickness>(),
                            None,
                            &CultureInfo::invariant_culture(),
                        )
                        .unwrap()
                        .unwrap();
                    assert_eq!(result.downcast_ref::<Thickness>(), Some(&expected));
                }
            }
        }
        for values in [vec![], vec![None; 5], vec![None; 6]] {
            let result = c
                .convert(
                    &values,
                    ValueType::of::<Thickness>(),
                    None,
                    &CultureInfo::invariant_culture(),
                )
                .unwrap()
                .unwrap();
            assert_eq!(
                result.downcast_ref::<Thickness>(),
                Some(&Thickness::default())
            );
        }
    }
}
