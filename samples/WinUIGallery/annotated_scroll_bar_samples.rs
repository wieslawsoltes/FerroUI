//! The code behind of the pinned AnnotatedScrollBar page in Rust: `AnnotatedScrollBarPage.xaml.cs`.
//!
//! The items are the names of the source's colors (each item draws a border of that color) instead
//! of `SolidColorBrush` objects; the arithmetic of the labels and the detail label is the source's.

use ferroui_base::collections::FerroList;
use ferroui_base::media::Brush;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{AnnotatedScrollBar, AnnotatedScrollBarLabel, Border, Control, ScrollView, Slider};
use ferroui_controls_repeater::ItemsRepeater;
use std::rc::Rc;

/// The sections of the source collection: the color and the number of its items.
pub(crate) const SECTIONS: &[(&str, usize)] =
    &[("Azure", 32), ("Crimson", 50), ("Cyan", 8), ("Fuchsia", 70), ("Gold", 90)];

/// Each item is sized 120 x 90 in the ItemsRepeater (`ItemWidth`, `ItemHeight`).
pub(crate) const ITEM_WIDTH: f64 = 120.0;
pub(crate) const ITEM_HEIGHT: f64 = 90.0;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "AnnotatedScrollBar" {
        wire_annotated_scroll_bar(root);
    }
}

/// `GetItemsPerRow`: at least one item per row.
pub(crate) fn items_per_row(repeater_width: f64) -> usize {
    if repeater_width == 0.0 {
        1
    } else {
        ((repeater_width / ITEM_WIDTH) as usize).max(1)
    }
}

/// `GetOffsetOfItem`: the offset of the row of the item.
pub(crate) fn offset_of_item(item_index: usize, items_per_row: usize) -> f64 {
    ITEM_HEIGHT * (item_index / items_per_row) as f64
}

/// `GetItemColor`.
pub(crate) fn item_color(item_index: usize) -> &'static str {
    let mut end = 0;
    for (color, count) in SECTIONS {
        end += count;
        if item_index < end {
            return color;
        }
    }
    SECTIONS[SECTIONS.len() - 1].0
}

/// The first item of each section.
fn section_starts() -> impl Iterator<Item = (&'static str, usize)> {
    SECTIONS.iter().scan(0, |start, (color, count)| {
        let first = *start;
        *start += count;
        Some((*color, first))
    })
}

/// `GetOffsetLabel`: the color of the last item of the section the offset is in.
pub(crate) fn offset_label(offset: f64, items_per_row: usize) -> &'static str {
    let mut end = 0;
    for (index, (_, count)) in SECTIONS.iter().enumerate() {
        end += count;
        if index == SECTIONS.len() - 1 || offset <= offset_of_item(end - 1, items_per_row) {
            return item_color(if index == SECTIONS.len() - 1 { end - count } else { end - 1 });
        }
    }
    SECTIONS[SECTIONS.len() - 1].0
}

/// `PopulateLabelCollection`: a label at the row of the first item of each section.
pub(crate) fn populate_labels(bar: &AnnotatedScrollBar, items_per_row: usize) {
    let labels = bar.labels();
    labels.clear();
    for (color, first) in section_starts() {
        let content: BoxedValue = Rc::new(color.to_string());
        labels.add(AnnotatedScrollBarLabel::new(Some(content), offset_of_item(first, items_per_row)));
    }
}

/// The item template: a 112 x 82 rounded border of the item's color with a margin of 4.
fn item_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(
        |_| true,
        |data, _| {
            let border = Border::new();
            border.set_width(112.0);
            border.set_height(82.0);
            border.set_margin(ferroui_base::Thickness::uniform(4.0));
            border.set_corner_radius(ferroui_base::CornerRadius::uniform(4.0));
            let color = data.as_ref().and_then(|data| data.downcast_ref::<String>().cloned());
            border.set_background(color.and_then(|color| Brush::parse(&color).ok()));
            Some(border.upcast())
        },
        false,
    )
}

fn wire_annotated_scroll_bar(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("itemsRepeater");
    let bar = root.get_control::<AnnotatedScrollBar>("annotatedScrollBar");
    let view = root.get_control::<ScrollView>("scrollView");

    // PopulateColorCollection.
    let colors = Rc::new(FerroList::<BoxedValue>::new());
    for (color, count) in SECTIONS {
        for _ in 0..*count {
            colors.add(Rc::new(color.to_string()) as BoxedValue);
        }
    }
    repeater.set_items_source(Some(colors.into()));
    repeater.set_item_template(Some(Rc::new(item_template()) as BoxedValue));

    // AnnotatedScrollBar_DetailLabelRequested.
    let weak_repeater = repeater.downgrade();
    bar.detail_label_requested(move |_, args| {
        let width = weak_repeater.upgrade().map(|repeater| repeater.bounds().width).unwrap_or(0.0);
        let content: BoxedValue = Rc::new(offset_label(args.scroll_offset(), items_per_row(width)).to_string());
        args.set_content(Some(content));
    })
    .detach();

    // ItemsRepeater_SizeChanged: the rows move with the width of the repeater.
    let weak_bar = bar.downgrade();
    let weak_repeater = repeater.downgrade();
    repeater
        .add_disposable_handler_with(
            Control::size_changed_event(),
            move |_, _: &ferroui_controls::SizeChangedEventArgs| {
                if let (Some(bar), Some(repeater)) = (weak_bar.upgrade(), weak_repeater.upgrade()) {
                    populate_labels(&bar, items_per_row(repeater.bounds().width));
                }
            },
            ferroui_base::interactivity::RoutingStrategies::DIRECT,
            false,
        )
        .detach();

    // AnnotatedScrollBarPage_Loaded: the bar is the vertical scroll controller of the view.
    let weak_bar = bar.downgrade();
    let weak_view = view.downgrade();
    view.loaded(move |_, _| {
        if let (Some(bar), Some(view)) = (weak_bar.upgrade(), weak_view.upgrade()) {
            if let Some(presenter) = view.scroll_presenter() {
                presenter.set_vertical_scroll_controller(Some(bar.scroll_controller()));
            }
        }
    });

    // AnnotatedScrollBarMaxHeightSlider_ValueChanged.
    let slider = root.get_control::<Slider>("maxHeightSlider");
    let weak_bar = bar.downgrade();
    let weak_slider = slider.downgrade();
    slider.property_changed(move |change| {
        if change.property() != RangeBase::value_property().as_property() {
            return;
        }
        if let (Some(bar), Some(slider)) = (weak_bar.upgrade(), weak_slider.upgrade()) {
            bar.set_max_height(slider.value());
        }
    });
}
