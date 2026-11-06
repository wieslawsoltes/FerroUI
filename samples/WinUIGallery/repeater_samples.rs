//! The ItemsRepeater and LayoutPanel pages of the pinned WinUI Gallery (14061eea) on the native
//! repeater of `ferroui-controls-repeater`: the page models, the custom layouts of the Gallery
//! (`ActivityFeedLayout`, `VariedImageSizeLayout`, `FeaturedTileLayout`), the template
//! selectors and the C# handlers of the pages, in Rust.
//!
//! Adaptations: x:Bind becomes `{Binding}` over the markup types declared here (`Bar`,
//! `Recipe`); the template selectors implement the repeater's `IDataTemplateSelector`; the
//! layouts the source declares as page resources are created here; the composition expression
//! animation of the animated scrolling example becomes a scale set from the scroll offset after
//! each layout pass; `StartBringIntoView` with a vertical alignment ratio becomes an offset of the
//! scroll viewer that centres the item. `System.Random` becomes a seeded generator, so the
//! generated bars and recipes are the same on every run.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{FocusChangedEventArgs, FocusChangingEventArgs, InputElement, Key, KeyEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{Brush, FontWeight, IBrush, ScaleTransform};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl,
    ObjectType, Rect, Ref, RelativePoint, Size, Thickness, Vector, WeakRef,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{
    Button, ComboBox, Control, ItemsChangedEventArgs, ItemsSource, RadioButtons, ScrollViewer,
    Slider, StackPanel, TextBlock, TextBox,
};
use ferroui_controls_repeater::{
    FlowLayout, FlowLayoutLineAlignment, IDataTemplateSelector, ItemsRepeater, Layout, LayoutImpl,
    LayoutPanel, NonVirtualizingLayout, NonVirtualizingLayoutContext, NonVirtualizingLayoutImpl,
    StackLayout, UniformGridLayout, VirtualizingLayout, VirtualizingLayoutContext,
    VirtualizingLayoutImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ItemsRepeaterPage.MaxLength`.
const MAX_LENGTH: i32 = 425;

/// `ItemsRepeaterPage.ColorList`.
pub(crate) const COLORS: [&str; 20] = [
    "Blue",
    "BlueViolet",
    "Crimson",
    "DarkCyan",
    "DarkGoldenrod",
    "DarkMagenta",
    "DarkOliveGreen",
    "DarkRed",
    "DarkSlateBlue",
    "DeepPink",
    "IndianRed",
    "MediumSlateBlue",
    "Maroon",
    "MidnightBlue",
    "Peru",
    "SaddleBrown",
    "SteelBlue",
    "OrangeRed",
    "Firebrick",
    "DarkKhaki",
];

const FRUITS: [&str; 7] = ["Apricots", "Bananas", "Grapes", "Strawberries", "Watermelon", "Plums", "Blueberries"];
const VEGETABLES: [&str; 7] = ["Broccoli", "Spinach", "Sweet potato", "Cauliflower", "Onion", "Brussels sprouts", "Carrots"];
const GRAINS: [&str; 7] = ["Rice", "Quinoa", "Pasta", "Bread", "Farro", "Oats", "Barley"];
const PROTEINS: [&str; 7] = ["Steak", "Chicken", "Tofu", "Salmon", "Pork", "Chickpeas", "Eggs"];
const EXTRAS: [&str; 7] = ["Garlic", "Lemon", "Butter", "Lime", "Feta Cheese", "Parmesan Cheese", "Breadcrumbs"];

// --- the models of the page -------------------------------------------------------------------

/// `Bar`: a bar of the basic example, with its sizes for the three templates.
pub(crate) struct Bar {
    pub(crate) length: f64,
    pub(crate) max_length: f64,
    pub(crate) height: f64,
    pub(crate) max_height: f64,
    pub(crate) diameter: f64,
    pub(crate) max_diameter: f64,
}

impl PartialEq for Bar {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Bar {
    /// `new Bar(length, max)`: the maxima divide the integer maximum as C# does.
    pub(crate) fn new(length: f64, max: i32) -> Rc<Bar> {
        Rc::new(Bar {
            length,
            max_length: f64::from(max),
            height: length / 4.0,
            max_height: f64::from(max / 4),
            diameter: length / 6.0,
            max_diameter: f64::from(max / 6),
        })
    }
}

ferro_markup_type!(class Bar {
    this: Rc<Bar>,
    handles: [Bar, Rc<Bar>, Option<Rc<Bar>>],
    properties: [
        Length: f64 { get: |this: &Rc<Bar>| this.length },
        MaxLength: f64 { get: |this: &Rc<Bar>| this.max_length },
        Height: f64 { get: |this: &Rc<Bar>| this.height },
        MaxHeight: f64 { get: |this: &Rc<Bar>| this.max_height },
        Diameter: f64 { get: |this: &Rc<Bar>| this.diameter },
        MaxDiameter: f64 { get: |this: &Rc<Bar>| this.max_diameter },
    ],
});

/// `Recipe`: an item of the content-heavy example. `Color` is the brush of the colour name the
/// source binds (`Background="{x:Bind Color}"` converts the name to a brush).
pub(crate) struct Recipe {
    pub(crate) num: i32,
    pub(crate) name: String,
    pub(crate) color: Option<Rc<dyn IBrush>>,
    pub(crate) ingredients: String,
    pub(crate) ing_list: Vec<String>,
}

impl PartialEq for Recipe {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class Recipe {
    this: Rc<Recipe>,
    handles: [Recipe, Rc<Recipe>, Option<Rc<Recipe>>],
    properties: [
        Num: i32 { get: |this: &Rc<Recipe>| this.num },
        Name: String { get: |this: &Rc<Recipe>| this.name.clone() },
        Color: Option<Rc<dyn IBrush>> { get: |this: &Rc<Recipe>| this.color.clone() },
        Ingredients: String { get: |this: &Rc<Recipe>| this.ingredients.clone() },
        NumIngredients: i32 { get: |this: &Rc<Recipe>| this.ing_list.len() as i32 },
    ],
});

/// `NestedCategory`: a category of the nested repeaters, with its items.
pub(crate) struct NestedCategory {
    pub(crate) name: &'static str,
    pub(crate) items: ItemsSource,
}

impl PartialEq for NestedCategory {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The markup types of the page, for the `{Binding}` paths of its templates.
fn register() {
    MarkupType::register_all(&[<Bar as MarkupTyped>::MARKUP, <Recipe as MarkupTyped>::MARKUP]);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_reference::<Bar>();
    ValueTypes::register_reference::<Recipe>();
}

/// A seeded generator for `System.Random` (xorshift64*).
pub(crate) struct SampleRandom(Cell<u64>);

impl SampleRandom {
    pub(crate) fn new(seed: u64) -> Self {
        Self(Cell::new(seed.max(1)))
    }

    /// `Next(min, max)`: a number at least `min` and below `max` (`min` when the range is empty).
    pub(crate) fn next(&self, min: usize, max: usize) -> usize {
        let mut x = self.0.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0.set(x);
        if max <= min {
            return min;
        }
        let value = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        min + (value % (max - min) as u64) as usize
    }
}

// --- the template selectors -------------------------------------------------------------------

/// `MyDataTemplateSelector`: `Normal` for even numbers, `Accent` for odd ones.
pub(crate) struct ParityTemplateSelector {
    pub(crate) normal: Option<Rc<dyn IDataTemplate>>,
    pub(crate) accent: Option<Rc<dyn IDataTemplate>>,
}

impl IDataTemplateSelector for ParityTemplateSelector {
    fn select_template(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        let number = item.and_then(|item| item.downcast_ref::<i32>().copied())?;
        if number % 2 == 0 {
            self.normal.clone()
        } else {
            self.accent.clone()
        }
    }

    fn select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        _container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        self.select_template(item)
    }
}

/// `StringOrIntTemplateSelector`: the template of the type of the item, none for other types.
pub(crate) struct StringOrIntTemplateSelector {
    pub(crate) string_template: Option<Rc<dyn IDataTemplate>>,
    pub(crate) int_template: Option<Rc<dyn IDataTemplate>>,
}

impl IDataTemplateSelector for StringOrIntTemplateSelector {
    fn select_template(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        let item = item?;
        if item.downcast_ref::<String>().is_some() {
            self.string_template.clone()
        } else if item.downcast_ref::<i32>().is_some() {
            self.int_template.clone()
        } else {
            None
        }
    }

    fn select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        _container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        self.select_template(item)
    }
}

fn selector_value(selector: Rc<dyn IDataTemplateSelector>) -> BoxedValue {
    Rc::new(selector)
}

// --- ActivityFeedLayout -----------------------------------------------------------------------

/// `ActivityFeedLayoutState`: the first realized index and the bounds of the realized items.
#[derive(Default)]
struct ActivityFeedLayoutState {
    first_realized_index: Cell<i32>,
    layout_rects: RefCell<Vec<Rect>>,
}

impl PartialEq for ActivityFeedLayoutState {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `ActivityFeedLayout` of WinUI Gallery (`WinUIGallery/Layouts/ActivityFeedLayout.cs`): rows
/// of three tiles, two narrow and one wide, the wide tile alternating between the right and
/// the left, virtualized by rows.
#[repr(C)]
pub(crate) struct ActivityFeedLayout {
    base: VirtualizingLayout,
    row_spacing: Cell<f64>,
    column_spacing: Cell<f64>,
    /// `MinItemSize`; `None` is `Size.Empty`, measured from the first item.
    min_item_size: Cell<Option<Size>>,
}

ferro_class!(ActivityFeedLayout: VirtualizingLayout);
ferro_impl_classes!(ActivityFeedLayout: FerroObjectImpl, LayoutImpl);

impl ActivityFeedLayout {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            row_spacing: Cell::new(0.0),
            column_spacing: Cell::new(0.0),
            min_item_size: Cell::new(None),
        })
    }

    #[cfg(test)]
    pub(crate) fn row_spacing(&self) -> f64 {
        self.row_spacing.get()
    }

    pub(crate) fn set_row_spacing(&self, value: f64) {
        self.row_spacing.set(value);
        self.invalidate_measure();
    }

    #[cfg(test)]
    pub(crate) fn column_spacing(&self) -> f64 {
        self.column_spacing.get()
    }

    pub(crate) fn set_column_spacing(&self, value: f64) {
        self.column_spacing.set(value);
        self.invalidate_measure();
    }

    #[cfg(test)]
    pub(crate) fn min_item_size(&self) -> Option<Size> {
        self.min_item_size.get()
    }

    pub(crate) fn set_min_item_size(&self, value: Option<Size>) {
        self.min_item_size.set(value);
        self.invalidate_measure();
    }

    /// `CalculateLayoutBoundsForRow`.
    fn bounds_for_row(&self, row_index: i32, desired_item_width: f64) -> [Rect; 3] {
        let item = self.min_item_size.get().unwrap_or_default();
        let spacing = self.column_spacing.get();
        let y = f64::from(row_index) * (item.height + self.row_spacing.get());
        let narrow = desired_item_width;
        let wide = desired_item_width * 2.0 + spacing;
        let widths = if row_index % 2 == 0 { [narrow, narrow, wide] } else { [wide, narrow, narrow] };
        let mut x = 0.0;
        let mut bounds = [Rect::default(); 3];
        for (column, width) in widths.into_iter().enumerate() {
            bounds[column] = Rect::new(x, y, width, item.height);
            x += width + spacing;
        }
        bounds
    }

    fn state(context: &Ref<VirtualizingLayoutContext>) -> Option<BoxedValue> {
        context
            .layout_state()
            .filter(|state| state.downcast_ref::<ActivityFeedLayoutState>().is_some())
    }
}

impl VirtualizingLayoutImpl for ActivityFeedLayout {
    fn initialize_for_context_core(_this: &Self, context: &Ref<VirtualizingLayoutContext>) {
        if Self::state(context).is_none() {
            context.set_layout_state(Some(Rc::new(ActivityFeedLayoutState::default()) as BoxedValue));
        }
    }

    fn uninitialize_for_context_core(_this: &Self, context: &Ref<VirtualizingLayoutContext>) {
        context.set_layout_state(None);
    }

    fn measure_override(this: &Self, context: &Ref<VirtualizingLayoutContext>, available_size: Size) -> Size {
        let item_count = context.item_count();
        if item_count == 0 {
            return Size::default();
        }
        if this.min_item_size.get().is_none() {
            let first = context.get_or_create_element_at(0);
            first.measure(Size::INFINITY);
            // The member value directly, without invalidating the layout (as the source).
            this.min_item_size.set(Some(first.desired_size()));
        }
        let item = this.min_item_size.get().unwrap_or_default();
        let row_height = item.height + this.row_spacing.get();
        let realization = context.realization_rect();
        let first_row_index = ((realization.y / row_height) as i32).saturating_sub(1).max(0);
        let last_row_index = ((realization.bottom() / row_height) as i32)
            .saturating_add(1)
            .min(item_count / 3);
        let Some(state) = Self::state(context) else {
            return Size::default();
        };
        let Some(state) = state.downcast_ref::<ActivityFeedLayoutState>() else {
            return Size::default();
        };
        let mut rects = state.layout_rects.borrow_mut();
        rects.clear();
        state.first_realized_index.set(first_row_index * 3);
        let spacing = this.column_spacing.get();
        let desired_item_width = item.width.max((available_size.width - spacing * 3.0) / 4.0);
        for row_index in first_row_index..last_row_index {
            let first_item_index = row_index * 3;
            let bounds = this.bounds_for_row(row_index, desired_item_width);
            for (column, bound) in bounds.into_iter().enumerate() {
                let container = context.get_or_create_element_at(first_item_index + column as i32);
                container.measure(Size::new(bound.width, bound.height));
                rects.push(bound);
            }
        }
        let extent_height = f64::from(item_count / 3 - 1) * row_height + item.height;
        Size::new(desired_item_width * 4.0 + spacing * 2.0, extent_height)
    }

    fn arrange_override(_this: &Self, context: &Ref<VirtualizingLayoutContext>, final_size: Size) -> Size {
        let Some(state) = Self::state(context) else {
            return final_size;
        };
        let Some(state) = state.downcast_ref::<ActivityFeedLayoutState>() else {
            return final_size;
        };
        let mut index = state.first_realized_index.get();
        for rect in state.layout_rects.borrow().iter() {
            context.get_or_create_element_at(index).arrange(*rect);
            index += 1;
        }
        final_size
    }
}

// --- VariedImageSizeLayout --------------------------------------------------------------------

/// `VariedImageSizeLayout` of WinUI Gallery (`WinUIGallery/Layouts/VariedImageSizeLayout.cs`):
/// columns of `Width`, each item placed in the lowest column, the bounds of the measured items
/// cached until the items change.
#[repr(C)]
pub(crate) struct VariedImageSizeLayout {
    base: VirtualizingLayout,
    width: Cell<f64>,
    first_index: Cell<i32>,
    last_index: Cell<i32>,
    last_available_width: Cell<f64>,
    column_offsets: RefCell<Vec<f64>>,
    cached_bounds: RefCell<Vec<Rect>>,
    cached_bounds_invalid: Cell<bool>,
}

ferro_class!(VariedImageSizeLayout: VirtualizingLayout);
ferro_impl_classes!(VariedImageSizeLayout: FerroObjectImpl, LayoutImpl);

impl VariedImageSizeLayout {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            width: Cell::new(150.0),
            first_index: Cell::new(0),
            last_index: Cell::new(0),
            last_available_width: Cell::new(0.0),
            column_offsets: RefCell::new(Vec::new()),
            cached_bounds: RefCell::new(Vec::new()),
            cached_bounds_invalid: Cell::new(false),
        })
    }

    #[cfg(test)]
    pub(crate) fn width(&self) -> f64 {
        self.width.get()
    }

    pub(crate) fn set_width(&self, value: f64) {
        self.width.set(value);
    }

    /// The number of columns of an available width: one without a finite width.
    fn columns(&self, available_width: f64) -> usize {
        if available_width.is_finite() && self.width.get() > 0.0 {
            ((available_width / self.width.get()) as usize).max(1)
        } else {
            1
        }
    }

    /// `UpdateCachedBounds`.
    fn update_cached_bounds(&self, available_size: Size) {
        let mut offsets = self.column_offsets.borrow_mut();
        offsets.clear();
        offsets.resize(self.columns(available_size.width), 0.0);
        let width = self.width.get();
        for bounds in self.cached_bounds.borrow_mut().iter_mut() {
            let (column, next_offset) = lowest_column(&offsets);
            let height = bounds.height;
            *bounds = Rect::new(column as f64 * width, next_offset, width, height);
            offsets[column] += height;
        }
        self.cached_bounds_invalid.set(false);
    }

    /// `GetStartIndex`: the first cached item that intersects the viewport.
    fn start_index(&self, viewport: Rect) -> i32 {
        self.cached_bounds
            .borrow()
            .iter()
            .position(|bounds| bounds.y < viewport.bottom() && bounds.bottom() > viewport.y)
            .map_or(0, |index| index as i32)
    }
}

/// `GetIndexOfLowestColumn`: the first column with the lowest offset, and that offset.
fn lowest_column(offsets: &[f64]) -> (usize, f64) {
    let mut lowest = (0, offsets.first().copied().unwrap_or(0.0));
    for (index, offset) in offsets.iter().copied().enumerate() {
        if lowest.1 > offset {
            lowest = (index, offset);
        }
    }
    lowest
}

impl VirtualizingLayoutImpl for VariedImageSizeLayout {
    fn on_items_changed_core(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        _args: &ItemsChangedEventArgs<'_>,
    ) {
        this.cached_bounds.borrow_mut().clear();
        this.first_index.set(0);
        this.last_index.set(0);
        this.cached_bounds_invalid.set(true);
        this.invalidate_measure();
    }

    fn measure_override(this: &Self, context: &Ref<VirtualizingLayoutContext>, available_size: Size) -> Size {
        let viewport = context.realization_rect();
        if available_size.width != this.last_available_width.get() || this.cached_bounds_invalid.get() {
            this.update_cached_bounds(available_size);
            this.last_available_width.set(available_size.width);
        }
        let columns = this.columns(available_size.width);
        if this.column_offsets.borrow().is_empty() {
            this.column_offsets.borrow_mut().resize(columns, 0.0);
        }
        this.first_index.set(this.start_index(viewport));
        let width = this.width.get();
        let mut current_index = this.first_index.get();
        let mut next_offset = -1.0;
        while current_index < context.item_count() && next_offset < viewport.bottom() {
            let child = context.get_or_create_element_at(current_index);
            child.measure(Size::new(width, available_size.height));
            let cached = this.cached_bounds.borrow().len();
            let index = current_index as usize;
            if index >= cached {
                // No bounds for this index yet: lay it out and cache it.
                let mut offsets = this.column_offsets.borrow_mut();
                let (column, offset) = lowest_column(&offsets);
                next_offset = offset;
                let height = child.desired_size().height;
                this.cached_bounds
                    .borrow_mut()
                    .push(Rect::new(column as f64 * width, offset, width, height));
                offsets[column] += height;
            } else if index + 1 == cached {
                // The last element: the next offset.
                next_offset = lowest_column(&this.column_offsets.borrow()).1;
            } else {
                next_offset = this.cached_bounds.borrow()[index + 1].y;
            }
            this.last_index.set(current_index);
            current_index += 1;
        }
        // `GetExtentSize`.
        let largest = this
            .column_offsets
            .borrow()
            .iter()
            .copied()
            .fold(0.0, f64::max);
        let extent_width = if available_size.width.is_finite() { available_size.width } else { width * columns as f64 };
        Size::new(extent_width, largest)
    }

    fn arrange_override(this: &Self, context: &Ref<VirtualizingLayoutContext>, final_size: Size) -> Size {
        let bounds = this.cached_bounds.borrow().clone();
        if !bounds.is_empty() {
            for index in this.first_index.get()..=this.last_index.get() {
                if let Some(rect) = bounds.get(index as usize) {
                    context.get_or_create_element_at(index).arrange(*rect);
                }
            }
        }
        final_size
    }
}

// --- FeaturedTileLayout -----------------------------------------------------------------------

/// `FeaturedTileLayout` of the LayoutPanel page (`Samples/LayoutPanel/FeaturedTileLayout.cs`): the
/// first child spans the width, the others are in rows of two columns, each row as high as its
/// taller child.
#[repr(C)]
pub(crate) struct FeaturedTileLayout {
    base: NonVirtualizingLayout,
    spacing: Cell<f64>,
}

ferro_class!(FeaturedTileLayout: NonVirtualizingLayout);
ferro_impl_classes!(FeaturedTileLayout: FerroObjectImpl, LayoutImpl);

impl FeaturedTileLayout {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: NonVirtualizingLayout::construct(), spacing: Cell::new(0.0) })
    }

    #[cfg(test)]
    pub(crate) fn spacing(&self) -> f64 {
        self.spacing.get()
    }

    /// `Spacing`; a change invalidates the measure (`OnSpacingChanged`).
    pub(crate) fn set_spacing(&self, value: f64) {
        if self.spacing.replace(value) != value {
            self.invalidate_measure();
        }
    }

    /// `GetRowHeight`.
    fn row_height(children: &[Ref<Control>], first_index: usize) -> f64 {
        let height = children[first_index].desired_size().height;
        match children.get(first_index + 1) {
            Some(second) => height.max(second.desired_size().height),
            None => height,
        }
    }
}

impl NonVirtualizingLayoutImpl for FeaturedTileLayout {
    fn measure_override(this: &Self, context: &Ref<NonVirtualizingLayoutContext>, available_size: Size) -> Size {
        let spacing = this.spacing.get();
        let width = if available_size.width.is_infinite() { 520.0 } else { available_size.width };
        let column_width = ((width - spacing) / 2.0).max(0.0);
        let children = context.children();
        for (index, child) in children.iter().enumerate() {
            child.measure(Size::new(if index == 0 { width } else { column_width }, f64::INFINITY));
        }
        let mut total_height = children.first().map_or(0.0, |child| child.desired_size().height);
        for index in (1..children.len()).step_by(2) {
            total_height += spacing + Self::row_height(&children, index);
        }
        Size::new(width, total_height)
    }

    fn arrange_override(this: &Self, context: &Ref<NonVirtualizingLayoutContext>, final_size: Size) -> Size {
        let children = context.children();
        let Some(featured) = children.first() else {
            return final_size;
        };
        let spacing = this.spacing.get();
        let column_width = ((final_size.width - spacing) / 2.0).max(0.0);
        featured.arrange(Rect::new(0.0, 0.0, final_size.width, featured.desired_size().height));
        let mut y = featured.desired_size().height;
        for index in (1..children.len()).step_by(2) {
            y += spacing;
            let row_height = Self::row_height(&children, index);
            children[index].arrange(Rect::new(0.0, y, column_width, row_height));
            if let Some(second) = children.get(index + 1) {
                second.arrange(Rect::new(column_width + spacing, y, column_width, row_height));
            }
            y += row_height;
        }
        Size::new(final_size.width, y)
    }
}

// --- wiring -----------------------------------------------------------------------------------

pub(crate) fn wire(root: &Ref<Control>) {
    if find::<ItemsRepeater>(root, "repeater").is_some() {
        register();
        wire_basic(root);
        wire_feed(root);
        wire_flow(root);
        wire_mixed(root);
        wire_nested(root);
        wire_animated(root);
        wire_recipes(root);
    }
    if find::<LayoutPanel>(root, "LayoutPanel1").is_some() {
        wire_layout_panel(root);
    }
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name).and_then(|control| control.cast())
}

/// A data template of the resources of the page.
pub(crate) fn page_template(root: &Ref<Control>, key: &str) -> Option<Rc<dyn IDataTemplate>> {
    let value = root
        .try_find_resource(&ResourceKey::from(key), root.actual_theme_variant().as_ref())
        .flatten()?;
    if let Some(template) = value.downcast_ref::<Rc<dyn IDataTemplate>>() {
        return Some(template.clone());
    }
    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDataTemplate>>())
        .and_then(|value| value.downcast_ref::<Rc<dyn IDataTemplate>>().cloned())
}

fn bar(length: f64) -> BoxedValue {
    Rc::new(Bar::new(length, MAX_LENGTH))
}

/// Basic, non-interactive items: add and remove bars, and switch the layout and its template.
fn wire_basic(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("repeater");
    let bars = Rc::new(FerroList::<BoxedValue>::new());
    for length in [300.0, 25.0, 175.0] {
        bars.add(bar(length));
    }
    repeater.set_items_source(Some(bars.clone().into()));
    let random = Rc::new(SampleRandom::new(0x5EED_0425));
    let delete = root.get_control::<Button>("DeleteBtn");
    {
        let bars = bars.clone();
        let weak_delete = delete.downgrade();
        root.get_control::<Button>("AddBtn").click(move |_, _| {
            bars.add(bar(random.next(0, MAX_LENGTH as usize) as f64));
            if let Some(delete) = weak_delete.upgrade() {
                delete.set_is_enabled(true);
            }
        });
    }
    {
        let bars = bars.clone();
        let weak_delete = delete.downgrade();
        delete.click(move |_, _| {
            if bars.count() > 0 {
                bars.remove_at(0);
                if bars.count() == 0 {
                    if let Some(delete) = weak_delete.upgrade() {
                        delete.set_is_enabled(false);
                    }
                }
            }
        });
    }
    let vertical = StackLayout::new();
    vertical.set_orientation(Orientation::Vertical);
    vertical.set_spacing(8.0);
    let horizontal = StackLayout::new();
    horizontal.set_orientation(Orientation::Horizontal);
    horizontal.set_spacing(8.0);
    let grid = UniformGridLayout::new();
    grid.set_min_column_spacing(8.0);
    grid.set_min_row_spacing(8.0);
    let choices: Rc<[(Ref<Layout>, Option<Rc<dyn IDataTemplate>>, f64); 3]> = Rc::new([
        (vertical.upcast(), page_template(root, "HorizontalBarTemplate"), f64::from(MAX_LENGTH) + 12.0),
        (horizontal.upcast(), page_template(root, "VerticalBarTemplate"), 6000.0),
        (grid.upcast(), page_template(root, "CircularTemplate"), 540.0),
    ]);
    // `RadioBtn_Click`: the layout, its template and the maximum width of the repeater.
    let apply = {
        let weak_repeater = repeater.downgrade();
        move |index: i32| {
            let (Some(repeater), Ok(index)) = (weak_repeater.upgrade(), usize::try_from(index)) else {
                return;
            };
            let Some((layout, template, max_width)) = choices.get(index) else {
                return;
            };
            repeater.set_max_width(*max_width);
            repeater.set_layout(Some(layout.clone()));
            repeater.set_data_template(template.clone());
            repeater.set_items_source(Some(bars.clone().into()));
        }
    };
    let layouts = root.get_control::<RadioButtons>("BarLayouts");
    apply(layouts.selected_index());
    let weak_layouts = layouts.downgrade();
    layouts.selection_changed(move |_, _| {
        if let Some(layouts) = weak_layouts.upgrade() {
            apply(layouts.selected_index());
        }
    });
}

/// Virtualizing, scrollable list: 500 numbers, the parity template selector, the custom feed
/// layout or a uniform grid.
fn wire_feed(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("repeater2");
    repeater.set_items_source(Some(ItemsSource::from_values(0..500i32)));
    let selector: Rc<dyn IDataTemplateSelector> = Rc::new(ParityTemplateSelector {
        normal: page_template(root, "NormalItemTemplate"),
        accent: page_template(root, "AccentItemTemplate"),
    });
    repeater.set_item_template(Some(selector_value(selector)));
    let feed = ActivityFeedLayout::new();
    feed.set_column_spacing(12.0);
    feed.set_row_spacing(12.0);
    feed.set_min_item_size(Some(Size::new(80.0, 108.0)));
    let grid = UniformGridLayout::new();
    grid.set_min_column_spacing(12.0);
    grid.set_min_row_spacing(12.0);
    grid.set_min_item_width(108.0);
    grid.set_min_item_height(108.0);
    let layouts = root.get_control::<RadioButtons>("FeedLayouts");
    let apply = {
        let weak_repeater = repeater.downgrade();
        move |index: i32| {
            if let Some(repeater) = weak_repeater.upgrade() {
                repeater.set_layout(Some(if index == 0 {
                    grid.clone().upcast::<Layout>()
                } else {
                    feed.clone().upcast::<Layout>()
                }));
            }
        }
    };
    apply(layouts.selected_index());
    let weak_layouts = layouts.downgrade();
    layouts.selection_changed(move |_, _| {
        if let Some(layouts) = weak_layouts.upgrade() {
            apply(layouts.selected_index());
        }
    });
}

/// FlowLayout over 500 items: orientation, line alignment and spacings, and the count of the
/// realized elements, updated once per dispatcher turn.
fn wire_flow(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("FlowRepeater");
    repeater.set_items_source(Some(ItemsSource::from_values((1..=500).map(|index| format!("Item {index}")))));
    let flow = FlowLayout::new();
    flow.set_orientation(Orientation::Horizontal);
    flow.set_line_alignment(FlowLayoutLineAlignment::Start);
    flow.set_line_spacing(8.0);
    flow.set_min_item_spacing(8.0);
    repeater.set_layout(Some(flow.clone().upcast()));

    let realized: Rc<RefCell<Vec<WeakRef<Control>>>> = Rc::new(RefCell::new(Vec::new()));
    let queued = Rc::new(Cell::new(false));
    let output = root.get_control::<TextBlock>("RealizedCountText").downgrade();
    let queue_update = Rc::new(move |realized: &Rc<RefCell<Vec<WeakRef<Control>>>>| {
        if queued.replace(true) {
            return;
        }
        let realized = realized.clone();
        let queued = queued.clone();
        let output = output.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                queued.set(false);
                let count = realized.borrow().iter().filter(|element| element.upgrade().is_some()).count();
                if let Some(output) = output.upgrade() {
                    output.set_text(Some(&format!("Realized elements: {count} of 500")));
                }
            },
            DispatcherPriority::NORMAL,
        );
    });
    {
        let realized = realized.clone();
        let queue_update = queue_update.clone();
        repeater.element_prepared(move |args| {
            let element = args.element.downgrade();
            {
                let mut realized = realized.borrow_mut();
                realized.retain(|known| known.upgrade().is_some());
                if !realized.iter().any(|known| known.ptr_eq(&element)) {
                    realized.push(element);
                }
            }
            queue_update(&realized);
        });
    }
    repeater.element_clearing(move |args| {
        let element = args.element.downgrade();
        realized
            .borrow_mut()
            .retain(|known| known.upgrade().is_some() && !known.ptr_eq(&element));
        queue_update(&realized);
    });

    let viewer = root.get_control::<ScrollViewer>("FlowScrollViewer");
    let orientations = root.get_control::<RadioButtons>("FlowOrientation");
    {
        let flow = flow.clone();
        let weak_orientations = orientations.downgrade();
        let weak_viewer = viewer.downgrade();
        orientations.selection_changed(move |_, _| {
            let (Some(orientations), Some(viewer)) = (weak_orientations.upgrade(), weak_viewer.upgrade()) else {
                return;
            };
            let horizontal = match orientations.selected_index() {
                0 => true,
                1 => false,
                _ => return,
            };
            flow.set_orientation(if horizontal { Orientation::Horizontal } else { Orientation::Vertical });
            // ScrollMode has no FerroUI counterpart: a disabled scroll bar disables scrolling.
            viewer.set_horizontal_scroll_bar_visibility(if horizontal {
                ScrollBarVisibility::Disabled
            } else {
                ScrollBarVisibility::Auto
            });
            viewer.set_vertical_scroll_bar_visibility(if horizontal {
                ScrollBarVisibility::Auto
            } else {
                ScrollBarVisibility::Disabled
            });
        });
    }
    let alignments = root.get_control::<ComboBox>("FlowLineAlignment");
    {
        let flow = flow.clone();
        let weak_alignments = alignments.downgrade();
        alignments.selection_changed(move |_, _| {
            let Some(alignments) = weak_alignments.upgrade() else {
                return;
            };
            flow.set_line_alignment(match alignments.selected_index() {
                0 => FlowLayoutLineAlignment::Start,
                1 => FlowLayoutLineAlignment::Center,
                2 => FlowLayoutLineAlignment::SpaceBetween,
                3 => FlowLayoutLineAlignment::SpaceEvenly,
                _ => return,
            });
        });
    }
    {
        let flow = flow.clone();
        root.get_control::<Slider>("FlowItemSpacing")
            .value_changed(move |_, args| flow.set_min_item_spacing(args.new_value()));
    }
    root.get_control::<Slider>("FlowLineSpacing")
        .value_changed(move |_, args| flow.set_line_spacing(args.new_value()));
}

/// The items of the mixed-type example (`basicData`).
fn mixed_items() -> ItemsSource {
    let text = |value: &str| Some(Rc::new(value.to_string()) as BoxedValue);
    let number = |value: i32| Some(Rc::new(value) as BoxedValue);
    ItemsSource::from_items([
        number(64),
        text("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."),
        number(128),
        text("Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat."),
        number(256),
        text("Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur."),
        number(512),
        text("Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."),
        number(1024),
    ])
}

fn wire_mixed(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("MixedTypeRepeater");
    let selector: Rc<dyn IDataTemplateSelector> = Rc::new(StringOrIntTemplateSelector {
        string_template: page_template(root, "StringDataTemplate"),
        int_template: page_template(root, "IntDataTemplate"),
    });
    repeater.set_item_template(Some(selector_value(selector)));
    repeater.set_items_source(Some(mixed_items()));
}

/// Nested repeaters: `CategoryTemplate` in Rust (a title and a horizontal repeater of the
/// category's items with `StringDataTemplate`), following the data context of the element.
fn wire_nested(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("outerRepeater");
    let item_template = page_template(root, "StringDataTemplate");
    let category_template = FuncDataTemplate::new(
        |_| true,
        move |data, _| {
            let panel = StackPanel::new();
            let title = TextBlock::new();
            title.set_padding(Thickness::uniform(8.0));
            // TitleTextBlockStyle.
            title.set_font_size(28.0);
            title.set_font_weight(FontWeight::SemiBold);
            let inner = ItemsRepeater::new();
            inner.set_name(Some("innerRepeater".to_string()));
            let layout = StackLayout::new();
            layout.set_orientation(Orientation::Horizontal);
            inner.set_layout(Some(layout.upcast()));
            inner.set_data_template(item_template.clone());
            panel.children().add(&title);
            panel.children().add(&inner);
            let show = {
                let title = title.downgrade();
                let inner = inner.downgrade();
                move |data: Option<BoxedValue>| {
                    let category = data.as_ref().and_then(|data| data.downcast_ref::<NestedCategory>());
                    if let Some(title) = title.upgrade() {
                        title.set_text(category.map(|category| category.name));
                    }
                    if let Some(inner) = inner.upgrade() {
                        inner.set_items_source(category.map(|category| category.items.clone()));
                    }
                }
            };
            show(data.clone());
            let weak_panel = panel.downgrade();
            panel
                .data_context_changed(move || {
                    if let Some(panel) = weak_panel.upgrade() {
                        show(panel.data_context());
                    }
                })
                .detach();
            Some(panel.upcast())
        },
        false,
    );
    repeater.set_data_template(Some(category_template));
    let category = |name: &'static str, items: [&'static str; 7]| {
        Some(Rc::new(NestedCategory { name, items: ItemsSource::from_strs(items) }) as BoxedValue)
    };
    repeater.set_items_source(Some(ItemsSource::from_items([
        category("Fruits", FRUITS),
        category("Vegetables", VEGETABLES),
        category("Grains", GRAINS),
        category("Proteins", PROTEINS),
    ])));
}

/// The scale of an item of the animated scrolling example: the expression of the source,
/// `1 - abs((viewport/2 - translation) - (offset + size/2)) * (0.25 / (viewport/2))`, where the
/// translation of the scroll viewer is the negated offset.
pub(crate) fn item_scale(viewport_height: f64, scroll_offset: f64, item_offset: f64, item_height: f64) -> f64 {
    let half = viewport_height / 2.0;
    if half <= 0.0 {
        return 1.0;
    }
    1.0 - ((half + scroll_offset) - (item_offset + item_height / 2.0)).abs() * (0.25 / half)
}

/// Sets the scale of each realized item of the animated scrolling example from its distance to
/// the centre of the viewport, about its centre (the `CenterPoint` expression of the source).
pub(crate) fn apply_item_scales(repeater: &ItemsRepeater, viewport_height: f64, scroll_offset: f64) {
    for element in repeater.realized_elements() {
        let bounds = element.bounds();
        let scale = item_scale(viewport_height, scroll_offset, bounds.y, bounds.height);
        element.set_render_transform_origin(RelativePoint::CENTER);
        element.set_render_transform(Some(ScaleTransform::with_scale(scale, scale).into()));
    }
}

/// [`apply_item_scales`] with the viewport and the offset of the scroll viewer.
fn update_item_scales(viewer: &ScrollViewer, repeater: &ItemsRepeater) {
    apply_item_scales(repeater, viewer.bounds().height, viewer.offset().y);
}

/// Moves the scroll viewer so that `element` is centred vertically (`StartBringIntoView` with a
/// `VerticalAlignmentRatio` of 0.5, without the animation).
fn center_in_viewer(viewer: &ScrollViewer, element: &Control) {
    let bounds = element.bounds();
    let viewport = viewer.viewport().height;
    let maximum = (viewer.extent().height - viewport).max(0.0);
    let y = (bounds.y + bounds.height / 2.0 - viewport / 2.0).clamp(0.0, maximum);
    viewer.set_offset(Vector::new(viewer.offset().x, y));
}

fn wire_animated(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("animatedScrollRepeater");
    let viewer = root.get_control::<ScrollViewer>("Animated_ScrollViewer");
    let rectangle = root.get_control::<Rectangle>("colorRectangle");
    let previous_index = Rc::new(Cell::new(-1i32));
    let last_selected: Rc<RefCell<Option<WeakRef<Button>>>> = Rc::new(RefCell::new(None));
    let weak_repeater = repeater.downgrade();
    let weak_viewer = viewer.downgrade();
    let weak_rectangle = rectangle.downgrade();
    let template = {
        let previous_index = previous_index.clone();
        let weak_repeater = weak_repeater.clone();
        let weak_viewer = weak_viewer.clone();
        FuncDataTemplate::new(
            |_| true,
            move |data, _| {
                let button = Button::new();
                button.set_horizontal_alignment(HorizontalAlignment::Stretch);
                let show = {
                    let button = button.downgrade();
                    move |data: Option<BoxedValue>| {
                        let Some(button) = button.upgrade() else {
                            return;
                        };
                        let name = data.as_ref().and_then(|data| data.downcast_ref::<String>()).cloned();
                        button.set_background(name.as_deref().and_then(|name| Brush::parse(name).ok()));
                        AutomationProperties::set_name(&button, name.as_deref());
                        button.set_content(name.map(|name| Rc::new(name) as BoxedValue));
                    }
                };
                show(data.clone());
                let weak_button = button.downgrade();
                button
                    .data_context_changed(move || {
                        if let Some(button) = weak_button.upgrade() {
                            show(button.data_context());
                        }
                    })
                    .detach();
                // `OnAnimatedItemClicked`: the rectangle takes the colour; the names announce
                // the selection.
                let weak_button = button.downgrade();
                let weak_rectangle = weak_rectangle.clone();
                let last_selected = last_selected.clone();
                button.click(move |_, _| {
                    let (Some(button), Some(rectangle)) = (weak_button.upgrade(), weak_rectangle.upgrade()) else {
                        return;
                    };
                    rectangle.set_fill(button.background());
                    let content = |button: &Button| {
                        button.content().and_then(|content| content.downcast_ref::<String>().cloned())
                    };
                    if let Some(last) = last_selected.borrow().as_ref().and_then(WeakRef::upgrade) {
                        if let Some(name) = content(&*last) {
                            AutomationProperties::set_name(&last, Some(&name));
                        }
                    }
                    let name = content(&*button).unwrap_or_default();
                    AutomationProperties::set_name(&button, Some(&format!("{name} , selected")));
                    *last_selected.borrow_mut() = Some(button.downgrade());
                });
                // `OnAnimatedItemGotFocus`: remember the index and centre the item.
                let weak_button = button.downgrade();
                let previous_index = previous_index.clone();
                let weak_repeater = weak_repeater.clone();
                let weak_viewer = weak_viewer.clone();
                button.add_handler(InputElement::got_focus_event(), move |_, _: &FocusChangedEventArgs| {
                    let (Some(button), Some(repeater), Some(viewer)) =
                        (weak_button.upgrade(), weak_repeater.upgrade(), weak_viewer.upgrade())
                    else {
                        return;
                    };
                    previous_index.set(repeater.get_element_index(&button.clone().upcast()));
                    center_in_viewer(&viewer, &button);
                });
                Some(button.upcast())
            },
            false,
        )
    };
    repeater.set_data_template(Some(template));
    repeater.set_items_source(Some(ItemsSource::from_strs(COLORS)));

    // `OnAnimatedScrollRepeaterGettingFocus`: focus coming from outside lands on the item that
    // had it last.
    {
        let previous_index = previous_index.clone();
        let weak_repeater = weak_repeater.clone();
        repeater.add_handler(InputElement::getting_focus_event(), move |_, args: &FocusChangingEventArgs| {
            let Some(repeater) = weak_repeater.upgrade() else {
                return;
            };
            let outside = args
                .old_focused_element
                .as_ref()
                .and_then(|old| old.clone().cast::<Control>())
                .is_some_and(|old| repeater.get_element_index(&old) == -1);
            if previous_index.get() != -1 && outside {
                if let Some(item) = repeater.try_get_element(previous_index.get()) {
                    args.try_set_new_focused_element(Some(&item.upcast()));
                }
            }
        });
    }
    // `OnAnimatedScrollRepeaterKeyDown`: Home and End move to the first and the last item.
    {
        let previous_index = previous_index.clone();
        let weak_repeater = weak_repeater.clone();
        let weak_viewer = weak_viewer.clone();
        repeater.add_handler(InputElement::key_down_event(), move |_, args: &KeyEventArgs| {
            if args.handled() {
                return;
            }
            let Some(repeater) = weak_repeater.upgrade() else {
                return;
            };
            let count = repeater.items_source_view().map_or(0, |view| view.count() as i32);
            let target = match args.key {
                Key::Home if previous_index.get() != 0 => 0,
                Key::End if previous_index.get() != count - 1 => count - 1,
                _ => -1,
            };
            if target < 0 {
                return;
            }
            let element = repeater.get_or_create_element(target);
            if let Some(viewer) = weak_viewer.upgrade() {
                center_in_viewer(&viewer, &element);
            }
            let _ = element.focus();
            args.set_handled(true);
        });
    }
    // The composition scale animation: the scales follow each layout pass and scroll change.
    {
        let weak_repeater = weak_repeater.clone();
        let weak_viewer = weak_viewer.clone();
        repeater
            .layout_updated(move || {
                if let (Some(viewer), Some(repeater)) = (weak_viewer.upgrade(), weak_repeater.upgrade()) {
                    update_item_scales(&viewer, &repeater);
                }
            })
            .detach();
    }
    viewer.scroll_changed(move |_, _| {
        if let (Some(viewer), Some(repeater)) = (weak_viewer.upgrade(), weak_repeater.upgrade()) {
            update_item_scales(&viewer, &repeater);
        }
    });
}

/// `GetRecipeList`: 1000 recipes with one ingredient of each group and up to three extras.
pub(crate) fn recipes(random: &SampleRandom) -> Vec<Rc<Recipe>> {
    (0..1000)
        .map(|num| {
            let color = COLORS[random.next(0, 19)];
            let mut ing_list: Vec<String> = [FRUITS, VEGETABLES, GRAINS, PROTEINS]
                .iter()
                .map(|group| group[random.next(0, 6)].to_string())
                .collect();
            let mut ingredients: String = ing_list.iter().map(|item| format!("\n{item}")).collect();
            // `RandomizeIngredients`: the bound is drawn again before each iteration, as in C#.
            let mut index = 0;
            while index < random.next(0, 4) {
                let extra = EXTRAS[random.next(0, 6)];
                if !ing_list.iter().any(|item| item == extra) {
                    ingredients.push('\n');
                    ingredients.push_str(extra);
                    ing_list.push(extra.to_string());
                }
                index += 1;
            }
            Rc::new(Recipe {
                num,
                name: format!("Recipe {num}"),
                color: Brush::parse(color).ok(),
                ingredients,
                ing_list,
            })
        })
        .collect()
}

/// `UpdateSortAndFilter`: the recipes whose ingredients contain the filter (ignoring case),
/// ordered by their number of ingredients.
pub(crate) fn filter_and_sort(recipes: &[Rc<Recipe>], filter: &str, descending: bool) -> Vec<Rc<Recipe>> {
    let filter = filter.to_lowercase();
    let mut result: Vec<_> = recipes
        .iter()
        .filter(|recipe| recipe.ingredients.to_lowercase().contains(&filter))
        .cloned()
        .collect();
    if descending {
        result.sort_by(|a, b| b.ing_list.len().cmp(&a.ing_list.len()));
    } else {
        result.sort_by_key(|recipe| recipe.ing_list.len());
    }
    result
}

fn wire_recipes(root: &Ref<Control>) {
    let repeater = root.get_control::<ItemsRepeater>("VariedImageSizeRepeater");
    let layout = VariedImageSizeLayout::new();
    layout.set_width(200.0);
    repeater.set_layout(Some(layout.upcast()));
    let all = Rc::new(recipes(&SampleRandom::new(0x5EED_1000)));
    let filtered = Rc::new(FerroList::<BoxedValue>::new());
    filtered.add_range(all.iter().map(|recipe| Rc::new(recipe.clone()) as BoxedValue));
    repeater.set_items_source(Some(filtered.clone().into()));
    let descending = Rc::new(Cell::new(false));
    let filter = root.get_control::<TextBox>("FilterRecipes");
    let update = {
        let weak_filter = filter.downgrade();
        let descending = descending.clone();
        Rc::new(move || {
            let text = weak_filter.upgrade().and_then(|filter| filter.text()).unwrap_or_default();
            let sorted = filter_and_sort(&all, &text, descending.get());
            // `InitializeCollection`: the items are replaced. The automation notification of
            // the count is not raised (no notification events on the peers of FerroUI).
            filtered.clear();
            filtered.add_range(sorted.into_iter().map(|recipe| Rc::new(recipe) as BoxedValue));
        })
    };
    {
        let update = update.clone();
        filter.property_changed(move |change| {
            if change.property() == TextBox::text_property().as_property() {
                update();
            }
        });
    }
    {
        let update = update.clone();
        let descending = descending.clone();
        root.get_control::<Button>("SortAscending").click(move |_, _| {
            if descending.replace(false) {
                update();
            }
        });
    }
    root.get_control::<Button>("SortDescending").click(move |_, _| {
        if !descending.replace(true) {
            update();
        }
    });
}

/// The LayoutPanel page: the featured tile layout or a vertical stack, and their spacing.
fn wire_layout_panel(root: &Ref<Control>) {
    let panel = root.get_control::<LayoutPanel>("LayoutPanel1");
    let featured = FeaturedTileLayout::new();
    featured.set_spacing(8.0);
    let stack = StackLayout::new();
    stack.set_orientation(Orientation::Vertical);
    stack.set_spacing(8.0);
    panel.set_layout(Some(featured.clone().upcast()));
    let layouts = root.get_control::<RadioButtons>("LayoutButtons");
    {
        let featured = featured.clone();
        let stack = stack.clone();
        let weak_panel = panel.downgrade();
        let weak_layouts = layouts.downgrade();
        layouts.selection_changed(move |_, _| {
            let (Some(panel), Some(layouts)) = (weak_panel.upgrade(), weak_layouts.upgrade()) else {
                return;
            };
            panel.set_layout(Some(if layouts.selected_index() == 1 {
                stack.clone().upcast::<Layout>()
            } else {
                featured.clone().upcast::<Layout>()
            }));
        });
    }
    root.get_control::<Slider>("LayoutSpacing").value_changed(move |_, args| {
        stack.set_spacing(args.new_value());
        featured.set_spacing(args.new_value());
    });
}
