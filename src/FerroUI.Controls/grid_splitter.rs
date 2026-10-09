// This source file is adapted from the Windows Presentation Foundation project.
// (https://github.com/dotnet/wpf/)

use crate::presenters::ContentPresenter;
use crate::primitives::{AdornerLayer, TemplatedControlImpl, Thumb, ThumbImpl, ThumbImplExt};
use crate::templates::ITemplateOf;
use crate::{
    ColumnDefinition, Control, ControlImpl, Decorator, DefinitionBase, Grid, GridLength, GridUnitType, RowDefinition,
};
use ferroui_base::input::navigation::XYFocusHelpers;
use ferroui_base::input::{
    Cursor, FocusChangedEventArgs, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerEventArgs,
    StandardCursorType, VectorEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutHelper, LayoutableImpl, LayoutableImplExt, VerticalAlignment};
use ferroui_base::media::{Geometry, ITransform, TranslateTransform};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, Size,
    StyledElement, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Indicates whether a [`GridSplitter`] resizes columns or rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GridResizeDirection {
    /// Determines whether to resize rows or columns based on its alignment
    /// and width compared to height.
    Auto = 0,
    /// Resize columns when dragging the splitter.
    Columns = 1,
    /// Resize rows when dragging the splitter.
    Rows = 2,
}

/// Indicates what columns or rows a [`GridSplitter`] resizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GridResizeBehavior {
    /// Determine which columns or rows to resize based on its alignment.
    BasedOnAlignment = 0,
    /// Resize the current and next columns or rows.
    CurrentAndNext = 1,
    /// Resize the previous and current columns or rows.
    PreviousAndCurrent = 2,
    /// Resize the previous and next columns or rows.
    PreviousAndNext = 3,
}

/// The splitter has special behavior when columns are fixed. If the left
/// column is fixed, the splitter will only resize that column. Else if the
/// right column is fixed, the splitter will only resize the right column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitBehavior {
    /// Both columns/rows are star lengths.
    Split,
    /// Resize 1 only.
    Resize1,
    /// Resize 2 only.
    Resize2,
}

/// Stores data during the resizing operation.
#[derive(Clone)]
struct ResizeData {
    shows_preview: bool,
    adorner: Option<Ref<PreviewAdorner>>,

    /// The constraints to keep the preview within valid ranges.
    min_change: f64,
    max_change: f64,

    /// The grid to resize.
    grid: Ref<Grid>,

    /// Cache of the resize direction and behavior.
    resize_direction: GridResizeDirection,
    resize_behavior: GridResizeBehavior,

    /// The columns/rows to resize.
    definition1: Option<Ref<DefinitionBase>>,
    definition2: Option<Ref<DefinitionBase>>,

    /// Are the columns/rows star lengths.
    split_behavior: SplitBehavior,

    /// The index of the splitter.
    splitter_index: i32,

    /// The indices of the columns/rows.
    definition1_index: i32,
    definition2_index: i32,

    /// The original lengths of the definitions (to restore the lengths if
    /// the user cancels the resize).
    original_definition1_length: GridLength,
    original_definition2_length: GridLength,
    original_definition1_actual_length: f64,
    original_definition2_actual_length: f64,

    /// The minimum of the width/height of the splitter. Used to ensure the
    /// splitter is not hidden by resizing a row/column smaller than the
    /// splitter.
    splitter_length: f64,

    /// The current layout scaling factor.
    scaling: f64,
}

thread_local! {
    static COLUMN_SPLITTER_CURSOR: RefCell<Option<Rc<Cursor>>> = const { RefCell::new(None) };
    static ROW_SPLITTER_CURSOR: RefCell<Option<Rc<Cursor>>> = const { RefCell::new(None) };
}

/// Represents the control that redistributes space between columns or rows
/// of a [`Grid`] control.
#[repr(C)]
pub struct GridSplitter {
    base: Thumb,
    resize_data: RefCell<Option<ResizeData>>,
    is_focus_engaged: Cell<bool>,
}

ferro_class! {
    GridSplitter: Thumb, virtuals GridSplitterImpl: ThumbImpl {
        /// Retrieves the [`Grid`] that ultimately hosts this splitter in the
        /// visual/logical tree.
        ///
        /// A splitter can be placed directly inside a grid or indirectly
        /// inside an items control that uses a grid as its items panel. In
        /// the latter case the first logical parent is usually a content
        /// presenter (or the items control itself), so the method walks
        /// these intermediate containers to locate the underlying grid.
        ///
        /// Returns the containing grid if one is found.
        fn get_parent_grid(this) -> Option<Ref<Grid>>;
        /// Returns the element that carries the grid-attached properties
        /// (`Grid.Row`, `Grid.Column`, etc.) relevant to this splitter.
        ///
        /// When the splitter is generated as part of an items control
        /// template, the attached properties are set on the surrounding
        /// content presenter rather than on the splitter itself. This helper
        /// selects that presenter when appropriate so subsequent property
        /// look-ups read the correct values; otherwise it simply returns
        /// the splitter.
        fn get_properties_value_source(this) -> Ref<StyledElement>;
    }
}
ferroui_base::ferro_class_info!(GridSplitter { new: GridSplitter::new });

ferro_impl_classes!(
    GridSplitter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl InputElementImpl for GridSplitter {
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);

        let direction = this.get_effective_resize_direction();

        match direction {
            GridResizeDirection::Columns => {
                this.set_cursor(Some(Self::splitter_cursor(&COLUMN_SPLITTER_CURSOR, StandardCursorType::SizeWestEast)));
            }
            GridResizeDirection::Rows => {
                this.set_cursor(Some(Self::splitter_cursor(&ROW_SPLITTER_CURSOR, StandardCursorType::SizeNorthSouth)));
            }
            GridResizeDirection::Auto => {}
        }
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        if this.has_resize_data() {
            this.cancel_resize();
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let using_xy_navigation = XYFocusHelpers::is_allowed_xy_navigation_mode(this, Some(e.key_device_type));
        let allow_arrow_keys = this.is_focus_engaged.get() || !using_xy_navigation;

        match e.key {
            Key::Enter if using_xy_navigation => {
                this.is_focus_engaged.set(!this.is_focus_engaged.get());
                e.set_handled(true);
            }
            Key::Escape => {
                this.is_focus_engaged.set(false);
                if this.has_resize_data() {
                    this.cancel_resize();
                    e.set_handled(true);
                }
            }
            Key::Left if allow_arrow_keys => {
                e.set_handled(this.keyboard_move_splitter(-this.keyboard_increment(), 0.0));
            }
            Key::Right if allow_arrow_keys => {
                e.set_handled(this.keyboard_move_splitter(this.keyboard_increment(), 0.0));
            }
            Key::Up if allow_arrow_keys => {
                e.set_handled(this.keyboard_move_splitter(0.0, -this.keyboard_increment()));
            }
            Key::Down if allow_arrow_keys => {
                e.set_handled(this.keyboard_move_splitter(0.0, this.keyboard_increment()));
            }
            _ => {}
        }
    }
}

impl ThumbImpl for GridSplitter {
    fn on_drag_started(this: &Self, e: &VectorEventArgs) {
        Self::parent_on_drag_started(this, e);

        // The thumb sometimes raises multiple drag started events.
        if this.has_resize_data() {
            return;
        }

        this.initialize_data(this.shows_preview());
    }

    fn on_drag_delta(this: &Self, e: &VectorEventArgs) {
        Self::parent_on_drag_delta(this, e);

        let Some(resize_data) = this.resize_data.borrow().clone() else {
            return;
        };

        let mut horizontal_change = e.vector.x;
        let mut vertical_change = e.vector.y;

        // Round change to nearest multiple of DragIncrement.
        let drag_increment = this.drag_increment();
        horizontal_change = (horizontal_change / drag_increment).round_ties_even() * drag_increment;
        vertical_change = (vertical_change / drag_increment).round_ties_even() * drag_increment;

        if resize_data.shows_preview {
            // Set the translation of the adorner to the distance from the
            // thumb. There is no adorner when the grid has no adorner layer.
            if let Some(adorner) = &resize_data.adorner {
                if resize_data.resize_direction == GridResizeDirection::Columns {
                    adorner.set_offset_x(f64::min(
                        f64::max(horizontal_change, resize_data.min_change),
                        resize_data.max_change,
                    ));
                } else {
                    adorner.set_offset_y(f64::min(
                        f64::max(vertical_change, resize_data.min_change),
                        resize_data.max_change,
                    ));
                }
            }
        } else {
            // Directly update the grid.
            this.move_splitter(horizontal_change, vertical_change);
        }
    }

    fn on_drag_completed(this: &Self, e: &VectorEventArgs) {
        Self::parent_on_drag_completed(this, e);

        let Some(resize_data) = this.resize_data.borrow().clone() else {
            return;
        };

        if resize_data.shows_preview {
            // Update the grid. There is no adorner when the grid has no
            // adorner layer.
            if let Some(adorner) = &resize_data.adorner {
                this.move_splitter(adorner.offset_x(), adorner.offset_y());
                this.remove_preview_adorner();
            }
        }

        *this.resize_data.borrow_mut() = None;
    }
}

impl GridSplitterImpl for GridSplitter {
    fn get_parent_grid(this: &Self) -> Option<Ref<Grid>> {
        // When the splitter is used inside an items control with a grid as
        // its items panel, its immediate parent is usually the items control
        // or a content presenter.
        let parent = this.parent()?;
        if let Some(grid) = parent.clone().cast::<Grid>() {
            return Some(grid);
        }

        let items_control = match parent.clone().cast::<crate::ItemsControl>() {
            Some(items_control) => items_control,
            None => {
                parent.cast::<ContentPresenter>()?;
                this.parent()?.parent()?.cast::<crate::ItemsControl>()?
            }
        };
        items_control.items_panel_root()?.cast::<Grid>()
    }

    fn get_properties_value_source(this: &Self) -> Ref<StyledElement> {
        match this.parent() {
            Some(parent) if parent.is::<ContentPresenter>() => parent,
            _ => this.to_ref().upcast(),
        }
    }
}

ferroui_base::ferro_properties! { impl GridSplitter {
    ferro_property!(
        /// Defines the `ResizeDirection` property.
        pub fn resize_direction_property() -> StyledProperty<GridResizeDirection> {
            FerroProperty::register::<GridSplitter, _>("ResizeDirection", GridResizeDirection::Auto)
        }
    );

    ferro_property!(
        /// Defines the `ResizeBehavior` property.
        pub fn resize_behavior_property() -> StyledProperty<GridResizeBehavior> {
            FerroProperty::register::<GridSplitter, _>("ResizeBehavior", GridResizeBehavior::BasedOnAlignment)
        }
    );

    ferro_property!(
        /// Defines the `ShowsPreview` property.
        pub fn shows_preview_property() -> StyledProperty<bool> {
            FerroProperty::register::<GridSplitter, _>("ShowsPreview", false)
        }
    );

    ferro_property!(
        /// Defines the `KeyboardIncrement` property.
        pub fn keyboard_increment_property() -> StyledProperty<f64> {
            FerroProperty::register::<GridSplitter, _>("KeyboardIncrement", 10.0)
        }
    );

    ferro_property!(
        /// Defines the `DragIncrement` property.
        pub fn drag_increment_property() -> StyledProperty<f64> {
            FerroProperty::register::<GridSplitter, _>("DragIncrement", 1.0)
        }
    );

    ferro_property!(
        /// Defines the `PreviewContent` property.
        pub fn preview_content_property() -> StyledProperty<Option<Rc<dyn ITemplateOf<Ref<Control>>>>> {
            FerroProperty::register::<GridSplitter, _>("PreviewContent", None)
        }
    );
} }

impl GridSplitter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Thumb::construct(), resize_data: RefCell::new(None), is_focus_engaged: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Indicates whether the splitter resizes the columns, rows, or both.
    pub fn resize_direction(&self) -> GridResizeDirection {
        self.get_value(Self::resize_direction_property())
    }

    pub fn set_resize_direction(&self, value: GridResizeDirection) {
        self.set_value(Self::resize_direction_property(), value)
    }

    /// Indicates which columns or rows the splitter resizes.
    pub fn resize_behavior(&self) -> GridResizeBehavior {
        self.get_value(Self::resize_behavior_property())
    }

    pub fn set_resize_behavior(&self, value: GridResizeBehavior) {
        self.set_value(Self::resize_behavior_property(), value)
    }

    /// Indicates whether to preview the column resizing without updating
    /// layout.
    pub fn shows_preview(&self) -> bool {
        self.get_value(Self::shows_preview_property())
    }

    pub fn set_shows_preview(&self, value: bool) {
        self.set_value(Self::shows_preview_property(), value)
    }

    /// The distance to move the splitter when pressing the keyboard arrow
    /// keys.
    pub fn keyboard_increment(&self) -> f64 {
        self.get_value(Self::keyboard_increment_property())
    }

    pub fn set_keyboard_increment(&self, value: f64) {
        self.set_value(Self::keyboard_increment_property(), value)
    }

    /// Restricts the splitter to move a multiple of the specified units.
    pub fn drag_increment(&self) -> f64 {
        self.get_value(Self::drag_increment_property())
    }

    pub fn set_drag_increment(&self, value: f64) {
        self.set_value(Self::drag_increment_property(), value)
    }

    /// The content that will be shown when `ShowsPreview` is enabled and the
    /// user starts a resize operation.
    pub fn preview_content(&self) -> Option<Rc<dyn ITemplateOf<Ref<Control>>>> {
        self.get_value(Self::preview_content_property())
    }

    pub fn set_preview_content(&self, value: Option<Rc<dyn ITemplateOf<Ref<Control>>>>) {
        self.set_value(Self::preview_content_property(), value)
    }

    fn splitter_cursor(
        slot: &'static std::thread::LocalKey<RefCell<Option<Rc<Cursor>>>>,
        cursor_type: StandardCursorType,
    ) -> Rc<Cursor> {
        if let Some(cursor) = slot.with(|slot| slot.borrow().clone()) {
            return cursor;
        }
        let cursor = Cursor::new(cursor_type);
        slot.with(|slot| *slot.borrow_mut() = Some(cursor.clone()));
        cursor
    }

    fn remove_preview_adorner(&self) {
        let adorner = self.resize_data.borrow().as_ref().and_then(|resize_data| resize_data.adorner.clone());
        if let Some(adorner) = adorner {
            if let Some(layer) = AdornerLayer::get_adorner_layer(self) {
                layer.children().remove(&adorner.upcast::<Control>());
            }
        }
    }

    fn has_resize_data(&self) -> bool {
        self.resize_data.borrow().is_some()
    }

    /// Converts the `Auto` direction to rows or columns depending on the
    /// alignment and the width/height of the splitter.
    pub(crate) fn get_effective_resize_direction(&self) -> GridResizeDirection {
        let mut direction = self.resize_direction();

        if direction != GridResizeDirection::Auto {
            return direction;
        }

        // When HorizontalAlignment is Left, Right or Center, resize Columns.
        if self.horizontal_alignment() != HorizontalAlignment::Stretch {
            direction = GridResizeDirection::Columns;
        } else if self.vertical_alignment() != VerticalAlignment::Stretch {
            direction = GridResizeDirection::Rows;
        } else if self.bounds().width <= self.bounds().height {
            // Fall back to Width vs Height.
            direction = GridResizeDirection::Columns;
        } else {
            direction = GridResizeDirection::Rows;
        }

        direction
    }

    /// Converts `BasedOnAlignment` to next/previous/both depending on the
    /// alignment and the direction.
    fn get_effective_resize_behavior(&self, direction: GridResizeDirection) -> GridResizeBehavior {
        let mut resize_behavior = self.resize_behavior();

        if resize_behavior == GridResizeBehavior::BasedOnAlignment {
            if direction == GridResizeDirection::Columns {
                resize_behavior = match self.horizontal_alignment() {
                    HorizontalAlignment::Left => GridResizeBehavior::PreviousAndCurrent,
                    HorizontalAlignment::Right => GridResizeBehavior::CurrentAndNext,
                    _ => GridResizeBehavior::PreviousAndNext,
                };
            } else {
                resize_behavior = match self.vertical_alignment() {
                    VerticalAlignment::Top => GridResizeBehavior::PreviousAndCurrent,
                    VerticalAlignment::Bottom => GridResizeBehavior::CurrentAndNext,
                    _ => GridResizeBehavior::PreviousAndNext,
                };
            }
        }

        resize_behavior
    }

    /// Initializes the data needed for resizing.
    fn initialize_data(&self, shows_preview: bool) {
        // If not in a grid or can't resize, do nothing.
        let Some(grid) = self.get_parent_grid() else {
            return;
        };

        let resize_direction = self.get_effective_resize_direction();

        // Setup data used for resizing; store the rows and columns to
        // resize on drag events. When unable to resize, no data is kept.
        let bounds = self.bounds();
        let resize_data = self.setup_definitions_to_resize(ResizeData {
            grid,
            shows_preview,
            adorner: None,
            min_change: 0.0,
            max_change: 0.0,
            resize_direction,
            splitter_length: f64::min(bounds.width, bounds.height),
            resize_behavior: self.get_effective_resize_behavior(resize_direction),
            scaling: self.get_layout_root().map_or(1.0, |root| root.layout_scaling()),
            definition1: None,
            definition2: None,
            split_behavior: SplitBehavior::Split,
            splitter_index: 0,
            definition1_index: 0,
            definition2_index: 0,
            original_definition1_length: GridLength::default(),
            original_definition2_length: GridLength::default(),
            original_definition1_actual_length: 0.0,
            original_definition2_actual_length: 0.0,
        });

        // Unable to resize: no data is kept.
        let can_resize = resize_data.is_some();
        *self.resize_data.borrow_mut() = resize_data;

        if can_resize {
            // Setup the preview in the adorner if ShowsPreview is true.
            self.setup_preview_adorner();
        }
    }

    /// Creates the preview adorner and adds it to the adorner layer.
    fn setup_preview_adorner(&self) {
        let Some(resize_data) = self.resize_data.borrow().clone() else {
            return;
        };

        if resize_data.shows_preview {
            // Get the adorner layer and add an adorner to it.
            let adorner_layer = AdornerLayer::get_adorner_layer(&resize_data.grid);

            let preview_content = self.preview_content();

            // Can't display the preview.
            let Some(adorner_layer) = adorner_layer else {
                return;
            };

            let built_preview_content = preview_content.map(|preview_content| preview_content.build_typed());

            let adorner = PreviewAdorner::new(built_preview_content);

            AdornerLayer::set_adorned_element(&adorner, self.to_ref().upcast::<ferroui_base::Visual>());
            AdornerLayer::set_is_clip_enabled(&adorner, false);

            adorner_layer.children().add(adorner.clone().upcast::<Control>());

            // Get constraints on the translation of the preview.
            let (min_change, max_change) = Self::get_delta_constraints(&resize_data);

            if let Some(resize_data) = self.resize_data.borrow_mut().as_mut() {
                resize_data.adorner = Some(adorner);
                resize_data.min_change = min_change;
                resize_data.max_change = max_change;
            }
        }
    }

    /// Completes the resize data with the definitions to resize. Returns
    /// `None` if the splitter cannot resize rows/columns.
    fn setup_definitions_to_resize(&self, mut resize_data: ResizeData) -> Option<ResizeData> {
        // Get the property values from the content presenter if the grid is
        // used in an items control as the items panel, otherwise directly
        // from the splitter.
        let source_control = self.get_properties_value_source();
        let is_columns = resize_data.resize_direction == GridResizeDirection::Columns;
        let grid_span = source_control.get_value(if is_columns {
            Grid::column_span_property()
        } else {
            Grid::row_span_property()
        });

        if grid_span == 1 {
            let splitter_index =
                source_control.get_value(if is_columns { Grid::column_property() } else { Grid::row_property() });

            // Select the columns based on behavior.
            let (index1, index2) = match resize_data.resize_behavior {
                // Get current and previous.
                GridResizeBehavior::PreviousAndCurrent => (splitter_index - 1, splitter_index),
                // Get current and next.
                GridResizeBehavior::CurrentAndNext => (splitter_index, splitter_index + 1),
                // GridResizeBehavior::PreviousAndNext: get previous and next.
                _ => (splitter_index - 1, splitter_index + 1),
            };

            // Get count of rows/columns in the resize direction.
            let count = if is_columns {
                resize_data.grid.column_definitions().count()
            } else {
                resize_data.grid.row_definitions().count()
            } as i32;

            if index1 >= 0 && index2 < count {
                resize_data.splitter_index = splitter_index;

                let definition1 = Self::get_grid_definition(&resize_data.grid, index1, resize_data.resize_direction);
                resize_data.definition1_index = index1;
                // Save the size in case the user cancels.
                resize_data.original_definition1_length = definition1.user_size_value_cache();
                resize_data.original_definition1_actual_length = Self::get_actual_length(&definition1);

                let definition2 = Self::get_grid_definition(&resize_data.grid, index2, resize_data.resize_direction);
                resize_data.definition2_index = index2;
                // Save the size in case the user cancels.
                resize_data.original_definition2_length = definition2.user_size_value_cache();
                resize_data.original_definition2_actual_length = Self::get_actual_length(&definition2);

                // Determine how to resize the columns.
                let is_star1 = Self::is_star(&definition1);
                let is_star2 = Self::is_star(&definition2);

                resize_data.split_behavior = if is_star1 && is_star2 {
                    // If they are both stars, resize both.
                    SplitBehavior::Split
                } else if !is_star1 {
                    // One column is fixed width, resize the first one that
                    // is fixed.
                    SplitBehavior::Resize1
                } else {
                    SplitBehavior::Resize2
                };

                resize_data.definition1 = Some(definition1);
                resize_data.definition2 = Some(definition2);

                return Some(resize_data);
            }
        }

        None
    }

    /// Cancels the resize operation.
    fn cancel_resize(&self) {
        let Some(resize_data) = self.resize_data.borrow().clone() else {
            return;
        };

        // Restore the original column/row lengths.
        if resize_data.shows_preview {
            self.remove_preview_adorner();
        } else {
            // Reset the columns/rows lengths to the saved values.
            if let Some(definition1) = &resize_data.definition1 {
                Self::set_definition_length(definition1, resize_data.original_definition1_length);
            }
            if let Some(definition2) = &resize_data.definition2 {
                Self::set_definition_length(definition2, resize_data.original_definition2_length);
            }
        }

        *self.resize_data.borrow_mut() = None;
    }

    /// Returns true if the row/column has a star length.
    fn is_star(definition: &DefinitionBase) -> bool {
        definition.user_size_value_cache().is_star()
    }

    /// Gets the column or row definition at an index from the grid based on
    /// the resize direction.
    fn get_grid_definition(grid: &Grid, index: i32, direction: GridResizeDirection) -> Ref<DefinitionBase> {
        if direction == GridResizeDirection::Columns {
            grid.column_definitions().get(index as usize).upcast()
        } else {
            grid.row_definitions().get(index as usize).upcast()
        }
    }

    /// Retrieves the actual width or actual height of the definition
    /// depending on whether it is a column or a row.
    fn get_actual_length(definition: &Ref<DefinitionBase>) -> f64 {
        if let Some(column) = definition.cast::<ColumnDefinition>() {
            return column.actual_width();
        }

        definition.cast::<RowDefinition>().map_or(0.0, |row| row.actual_height())
    }

    /// Sets the length of a column or row definition.
    fn set_definition_length(definition: &Ref<DefinitionBase>, length: GridLength) {
        definition.set_value(
            if definition.is::<ColumnDefinition>() {
                ColumnDefinition::width_property()
            } else {
                RowDefinition::height_property()
            },
            length,
        );
    }

    /// Gets the minimum and maximum delta given the definition constraints
    /// (MinWidth/MaxWidth).
    fn get_delta_constraints(resize_data: &ResizeData) -> (f64, f64) {
        let (Some(definition1), Some(definition2)) = (&resize_data.definition1, &resize_data.definition2) else {
            return (0.0, 0.0);
        };

        let definition1_len = Self::get_actual_length(definition1);
        let mut definition1_min = definition1.user_min_size_value_cache();
        let definition1_max = definition1.user_max_size_value_cache();

        let definition2_len = Self::get_actual_length(definition2);
        let mut definition2_min = definition2.user_min_size_value_cache();
        let definition2_max = definition2.user_max_size_value_cache();

        // Set MinWidths to be greater than width of splitter.
        if resize_data.splitter_index == resize_data.definition1_index {
            definition1_min = f64::max(definition1_min, resize_data.splitter_length);
        } else if resize_data.splitter_index == resize_data.definition2_index {
            definition2_min = f64::max(definition2_min, resize_data.splitter_length);
        }

        // Determine the minimum and maximum the columns can be resized.
        let min_delta = -f64::min(definition1_len - definition1_min, definition2_max - definition2_len);
        let max_delta = f64::min(definition1_max - definition1_len, definition2_len - definition2_min);

        (min_delta, max_delta)
    }

    /// Sets the length of definition1 and definition2.
    fn set_lengths(resize_data: &ResizeData, definition1_pixels: f64, definition2_pixels: f64) {
        // For the case where both definition1 and 2 are stars, update all
        // star values to match their current pixel values.
        if resize_data.split_behavior == SplitBehavior::Split {
            let definitions: Vec<Ref<DefinitionBase>> = if resize_data.resize_direction == GridResizeDirection::Columns
            {
                resize_data.grid.column_definitions().snapshot().iter().map(|d| d.clone().upcast()).collect()
            } else {
                resize_data.grid.row_definitions().snapshot().iter().map(|d| d.clone().upcast()).collect()
            };

            for (i, definition) in definitions.iter().enumerate() {
                let i = i as i32;

                // For each definition, if it is a star, set its value to
                // the actual length in stars. This makes 1 star == 1 pixel
                // in length.
                if i == resize_data.definition1_index {
                    Self::set_definition_length(definition, GridLength::new(definition1_pixels, GridUnitType::Star));
                } else if i == resize_data.definition2_index {
                    Self::set_definition_length(definition, GridLength::new(definition2_pixels, GridUnitType::Star));
                } else if Self::is_star(definition) {
                    Self::set_definition_length(
                        definition,
                        GridLength::new(Self::get_actual_length(definition), GridUnitType::Star),
                    );
                }
            }
        } else if resize_data.split_behavior == SplitBehavior::Resize1 {
            if let Some(definition1) = &resize_data.definition1 {
                Self::set_definition_length(definition1, GridLength::from_pixels(definition1_pixels));
            }
        } else if let Some(definition2) = &resize_data.definition2 {
            Self::set_definition_length(definition2, GridLength::from_pixels(definition2_pixels));
        }
    }

    /// Moves the splitter by the given deltas in the horizontal and vertical
    /// directions.
    fn move_splitter(&self, horizontal_change: f64, vertical_change: f64) {
        let Some(resize_data) = self.resize_data.borrow().clone() else {
            return;
        };

        // Calculate the offset to adjust the splitter. If layout rounding is
        // enabled, we need to round to an integer physical pixel value to
        // avoid round-ups of children that expand the bounds of the grid. In
        // practice this only happens in high dpi because horizontal/vertical
        // offsets here are never fractional (they correspond to mouse
        // movement across logical pixels). Rounding error only creeps in
        // when converting to a physical display with something other than
        // the logical 96 dpi.
        let mut delta = if resize_data.resize_direction == GridResizeDirection::Columns {
            horizontal_change
        } else {
            vertical_change
        };

        if self.use_layout_rounding() {
            delta = LayoutHelper::round_layout_value(delta, LayoutHelper::get_layout_scale(self));
        }

        if let (Some(definition1), Some(definition2)) = (&resize_data.definition1, &resize_data.definition2) {
            let actual_length1 = Self::get_actual_length(definition1);
            let actual_length2 = Self::get_actual_length(definition2);
            let pixel_length = 1.0 / resize_data.scaling;
            let epsilon = pixel_length + LayoutHelper::LAYOUT_EPSILON;

            // When splitting, check to see if the total pixels spanned by
            // the definitions is the same as before starting the resize. If
            // not, cancel the drag. We need to account for layout rounding
            // here, so ignore differences of less than a device pixel.
            if resize_data.split_behavior == SplitBehavior::Split
                && !MathUtilities::are_close_eps(
                    actual_length1 + actual_length2,
                    resize_data.original_definition1_actual_length + resize_data.original_definition2_actual_length,
                    epsilon,
                )
            {
                self.cancel_resize();

                return;
            }

            let (min, max) = Self::get_delta_constraints(&resize_data);

            // Constrain the delta to the Min/MaxWidth of the columns.
            delta = f64::min(f64::max(delta, min), max);

            let definition1_length_new = actual_length1 + delta;
            let definition2_length_new = actual_length1 + actual_length2 - definition1_length_new;

            Self::set_lengths(&resize_data, definition1_length_new, definition2_length_new);
        }
    }

    /// Moves the splitter using the keyboard (does not show the preview).
    fn keyboard_move_splitter(&self, horizontal_change: f64, vertical_change: f64) -> bool {
        // If moving with the mouse, ignore keyboard motion.
        if self.has_resize_data() {
            return false; // Don't handle the event.
        }

        // Don't show preview.
        self.initialize_data(false);

        // Check that we are actually able to resize.
        if !self.has_resize_data() {
            return false; // Don't handle the event.
        }

        self.move_splitter(horizontal_change, vertical_change);

        *self.resize_data.borrow_mut() = None;

        true
    }
}

/// The adorner that previews the position of the splitter while it is
/// dragged with `ShowsPreview` enabled.
#[repr(C)]
pub(crate) struct PreviewAdorner {
    base: Decorator,
    translation: Ref<TranslateTransform>,
}

ferro_class!(PreviewAdorner: Decorator);
ferro_impl_classes!(
    PreviewAdorner: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for PreviewAdorner {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        // Adorners always get clipped to the owner control. In this case we
        // want to constrain the size to the splitter size but draw on top
        // of the parent grid.
        this.set_clip(None::<Ref<Geometry>>);

        Self::parent_arrange_override(this, final_size)
    }
}

impl PreviewAdorner {
    fn new(preview_control: Option<Ref<Control>>) -> Ref<Self> {
        // Add a decorator to perform translations.
        let translation = TranslateTransform::new();
        let this = instantiate(Self { base: Decorator::construct(), translation: translation.clone() });

        let decorator = Decorator::new();
        decorator.set_child(preview_control);
        let transform: Rc<dyn ITransform> = translation.into();
        decorator.set_render_transform(Some(transform));

        this.set_child(decorator);
        this
    }

    /// The offset of the preview in the X direction from the splitter.
    fn offset_x(&self) -> f64 {
        self.translation.x()
    }

    fn set_offset_x(&self, value: f64) {
        self.translation.set_x(value)
    }

    /// The offset of the preview in the Y direction from the splitter.
    fn offset_y(&self) -> f64 {
        self.translation.y()
    }

    fn set_offset_y(&self, value: f64) {
        self.translation.set_y(value)
    }
}
