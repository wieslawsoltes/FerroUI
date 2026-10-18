//! Port of `ViewModels/TemplateColumnBugPageViewModel.cs`.

use crate::models::TemplateColumnItem;
use ferroui_base::controls::ResourceKey;
use ferroui_base::ferro_markup_type;
use ferroui_base::media::TextTrimming;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_controls_treedatagrid::models::{TemplateColumn, TemplateColumnOptions, TextColumn, TextColumnOptions};
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::{ColumnRef, Comparison};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, Models};
use std::cmp::Ordering;
use std::rc::Rc;

/// The template column reuse page: a flat source of 200 items with two template columns
/// separated by two text columns, which a narrow grid scrolls horizontally, so that the cells of
/// one template column are recycled for the other.
pub struct TemplateColumnBugPageViewModel {
    items: Rc<Vec<Rc<TemplateColumnItem>>>,
    source: Rc<FlatTreeDataGridSource<TemplateColumnItem>>,
}

impl PartialEq for TemplateColumnBugPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TemplateColumnBugPageViewModel {
    pub fn new() -> Rc<Self> {
        let items: Rc<Vec<Rc<TemplateColumnItem>>> = Rc::new(
            (1..=200)
                .map(|index: u32| {
                    TemplateColumnItem::new(
                        format!("Item {index:03}"),
                        format!("Type {}", (b'A' + (index % 4) as u8) as char),
                        format!("Details for item {index:03}"),
                        index % 3 == 0,
                    )
                })
                .collect(),
        );

        let source = Self::create_source(items.clone());

        Rc::new(Self { items, source })
    }

    pub fn items(&self) -> &[Rc<TemplateColumnItem>] {
        &self.items
    }

    pub fn source(&self) -> Rc<FlatTreeDataGridSource<TemplateColumnItem>> {
        self.source.clone()
    }

    fn create_source(items: Rc<Vec<Rc<TemplateColumnItem>>>) -> Rc<FlatTreeDataGridSource<TemplateColumnItem>> {
        let source = FlatTreeDataGridSource::new(items as Models<TemplateColumnItem>);

        let status_options = TemplateColumnOptions::<TemplateColumnItem>::new();
        status_options.set_compare_ascending(Some(Self::sort_ascending(|x| x.is_flagged())));
        status_options.set_compare_descending(Some(Self::sort_descending(|x| x.is_flagged())));
        status_options.set_can_user_resize_column(Some(false));
        status_options.set_can_user_sort_column(Some(true));
        source.columns().add(TemplateColumn::from_resource_keys(
            Some(Rc::new("Status".to_string())),
            ResourceKey::from("IsFlaggedTemplate"),
            None,
            Some(GridLength::new(50.0, GridUnitType::Pixel)),
            Some(Rc::new(status_options)),
        ) as ColumnRef<TemplateColumnItem>);

        let name_options = TextColumnOptions::<TemplateColumnItem>::new();
        name_options.set_can_user_resize_column(Some(true));
        name_options.set_can_user_sort_column(Some(true));
        name_options.set_min_width(GridLength::new(70.0, GridUnitType::Pixel));
        name_options.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        name_options.set_compare_ascending(Some(Self::sort_ascending(|x| x.name())));
        name_options.set_compare_descending(Some(Self::sort_descending(|x| x.name())));
        source.columns().add(TextColumn::new(
            Some(Rc::new("Name".to_string())),
            Accessor::member("Name", |x: &Rc<TemplateColumnItem>| x.name()),
            Some(GridLength::STAR),
            Some(Rc::new(name_options)),
        ) as ColumnRef<TemplateColumnItem>);

        let type_options = TextColumnOptions::<TemplateColumnItem>::new();
        type_options.set_can_user_resize_column(Some(true));
        type_options.set_can_user_sort_column(Some(true));
        type_options.set_min_width(GridLength::new(110.0, GridUnitType::Pixel));
        type_options.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        type_options.set_compare_ascending(Some(Self::sort_ascending(|x| x.r#type())));
        type_options.set_compare_descending(Some(Self::sort_descending(|x| x.r#type())));
        source.columns().add(TextColumn::new(
            Some(Rc::new("Type".to_string())),
            Accessor::member("Type", |x: &Rc<TemplateColumnItem>| x.r#type()),
            Some(GridLength::STAR),
            Some(Rc::new(type_options)),
        ) as ColumnRef<TemplateColumnItem>);

        let details_options = TemplateColumnOptions::<TemplateColumnItem>::new();
        details_options.set_compare_ascending(Some(Self::sort_ascending(|x| x.details())));
        details_options.set_compare_descending(Some(Self::sort_descending(|x| x.details())));
        details_options.set_can_user_resize_column(Some(true));
        details_options.set_can_user_sort_column(Some(true));
        details_options.set_min_width(GridLength::new(125.0, GridUnitType::Pixel));
        source.columns().add(TemplateColumn::from_resource_keys(
            Some(Rc::new("Details".to_string())),
            ResourceKey::from("DetailsTemplate"),
            None,
            Some(GridLength::STAR),
            Some(Rc::new(details_options)),
        ) as ColumnRef<TemplateColumnItem>);

        source
    }

    fn sort_ascending<V: Ord + 'static>(selector: impl Fn(&TemplateColumnItem) -> V + 'static) -> Comparison<TemplateColumnItem> {
        Rc::new(move |x: &Rc<TemplateColumnItem>, y: &Rc<TemplateColumnItem>| Self::compare(Some(x), Some(y), &selector))
    }

    fn sort_descending<V: Ord + 'static>(selector: impl Fn(&TemplateColumnItem) -> V + 'static) -> Comparison<TemplateColumnItem> {
        Rc::new(move |x: &Rc<TemplateColumnItem>, y: &Rc<TemplateColumnItem>| Self::compare(Some(y), Some(x), &selector))
    }

    /// `Compare(x, y, selector)`: a missing item sorts first. The models of a source are never
    /// missing; the comparisons pass both.
    fn compare<V: Ord>(
        x: Option<&Rc<TemplateColumnItem>>,
        y: Option<&Rc<TemplateColumnItem>>,
        selector: &impl Fn(&TemplateColumnItem) -> V,
    ) -> Ordering {
        let Some(x) = x else {
            return if y.is_none() { Ordering::Equal } else { Ordering::Less };
        };
        let Some(y) = y else {
            return Ordering::Greater;
        };

        // `Comparer<TValue>.Default`: the order of the values (ordinal for text).
        selector(&**x).cmp(&selector(&**y))
    }
}

ferro_markup_type!(class TemplateColumnBugPageViewModel {
    this: Rc<TemplateColumnBugPageViewModel>,
    handles: [TemplateColumnBugPageViewModel, Rc<TemplateColumnBugPageViewModel>, Option<Rc<TemplateColumnBugPageViewModel>>],
    constructors: [() => TemplateColumnBugPageViewModel::new],
    properties: [
        Source: TreeDataGridSourceRef {
            get: |this: &Rc<TemplateColumnBugPageViewModel>| TreeDataGridSourceRef::of::<TemplateColumnItem, _>(this.source())
        },
    ],
});
