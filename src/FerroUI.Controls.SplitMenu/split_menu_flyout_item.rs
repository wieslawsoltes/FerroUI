//! WinUI's `SplitMenuFlyoutItem` on `MenuItem`: a press on the item is its primary action, and a
//! press on the part at its trailing edge is the press of a sub item, which opens the submenu.

use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, Key, KeyEventArgs, MouseButton, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{DrawingContext, IBrush};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref, Size, StaticType,
    StyledElementImpl, TypeInfo, VisualImpl, VisualImplExt,
};
use ferroui_controls::primitives::{SelectingItemsControlImpl, TemplatedControl, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, ItemsControlImpl, MenuItem, MenuItemImpl};
use std::cell::Cell;
use std::rc::Rc;

/// The width of the part at the trailing edge that opens the submenu.
pub(crate) const SUBMENU_PART_WIDTH: f64 = 40.0;

/// Whether a point of an item of `size` is on its primary part.
pub(crate) fn is_on_primary_part(point: Point, size: Size) -> bool {
    point.x >= 0.0 && point.y >= 0.0 && point.y <= size.height && point.x < size.width - SUBMENU_PART_WIDTH
}

/// A menu item with a primary action and a submenu.
#[repr(C)]
pub struct SplitMenuFlyoutItem {
    base: MenuItem,
    primary_pressed: Cell<bool>,
}

ferro_class!(SplitMenuFlyoutItem: MenuItem);
ferroui_base::ferro_class_info!(SplitMenuFlyoutItem { new: SplitMenuFlyoutItem::new });
ferro_impl_classes!(
    SplitMenuFlyoutItem: FerroObjectImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl,
    MenuItemImpl
);

impl StyledElementImpl for SplitMenuFlyoutItem {
    // The themes of the menu item are the themes of the split item.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <MenuItem as StaticType>::TYPE
    }
}

impl VisualImpl for SplitMenuFlyoutItem {
    fn render(this: &Self, context: &mut DrawingContext) {
        Self::parent_render(this, context);
        // The line between the two parts.
        let size = this.bounds().size();
        let foreground: Option<Rc<dyn IBrush>> = this.get_value(TemplatedControl::foreground_property());
        if let Some(foreground) = foreground.filter(|_| size.width > SUBMENU_PART_WIDTH && size.height > 12.0) {
            let line = Rect::new(size.width - SUBMENU_PART_WIDTH, 6.0, 1.0, size.height - 12.0);
            let state = context.push_opacity(0.2);
            context.fill_rectangle(&foreground, line, 0.0);
            context.pop(state);
        }
    }
}

impl InputElementImpl for SplitMenuFlyoutItem {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let point = e.get_current_point(Some(this));
        let primary = point.properties.is_left_button_pressed
            && this.is_effectively_enabled()
            && is_on_primary_part(point.position, this.bounds().size());
        this.primary_pressed.set(primary);
        if primary {
            // Not the press of a sub item: the menu does not open the submenu for it.
            e.set_handled(true);
        } else {
            Self::parent_on_pointer_pressed(this, e);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if !this.primary_pressed.replace(false) {
            Self::parent_on_pointer_released(this, e);
            return;
        }
        e.set_handled(true);
        if e.initial_press_mouse_button() == MouseButton::Left
            && is_on_primary_part(e.get_position(Some(this)), this.bounds().size())
        {
            this.invoke_primary();
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        // Enter and Space are the primary action; the arrow keys stay with the menu.
        if matches!(e.key, Key::Enter | Key::Space) && this.is_effectively_enabled() {
            this.invoke_primary();
            e.set_handled(true);
        } else {
            Self::parent_on_key_down(this, e);
        }
    }
}

impl SplitMenuFlyoutItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: MenuItem::construct(), primary_pressed: Cell::new(false) }
    }

    /// Creates an item without a header or sub items.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The primary action: raises `Click`, which runs the command, and closes the menu.
    pub fn invoke_primary(&self) {
        self.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
        self.close_parent_menu();
    }
}
