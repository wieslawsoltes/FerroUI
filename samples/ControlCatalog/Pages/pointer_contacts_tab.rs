//! Port of `Pages/PointerContactsTab.cs`.

use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brushes, Color, Colors, DrawingContext, IBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Point, Rect, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::hash::{BuildHasher, Hasher};
use std::rc::Rc;

struct PointerInfo {
    point: Point,
    color: Color,
}

const ALL_COLORS: [Color; 22] = [
    Colors::AQUA,
    Colors::BEIGE,
    Colors::CHARTREUSE,
    Colors::CORAL,
    Colors::FUCHSIA,
    Colors::CRIMSON,
    Colors::LAVENDER,
    Colors::ORANGE,
    Colors::ORCHID,
    Colors::FOREST_GREEN,
    Colors::STEEL_BLUE,
    Colors::PAPAYA_WHIP,
    Colors::PALE_VIOLET_RED,
    Colors::GOLDENROD,
    Colors::MAROON,
    Colors::MOCCASIN,
    Colors::NAVY,
    Colors::WHEAT,
    Colors::VIOLET,
    Colors::SIENNA,
    Colors::INDIGO,
    Colors::HONEYDEW,
];

/// A random integer that is at least zero and less than `max_value`, or
/// zero when `max_value` is zero (`new Random().Next(0, maxValue)`).
fn next_random(max_value: usize) -> usize {
    if max_value == 0 {
        return 0;
    }
    let random = std::collections::hash_map::RandomState::new().build_hasher().finish();
    (random % max_value as u64) as usize
}

#[repr(C)]
pub struct PointerContactsTab {
    base: Control,
    /// The contact of each pointer, in the order the pointers were first
    /// seen; pointers are compared by reference.
    pointers: RefCell<Vec<(Rc<dyn IPointer>, PointerInfo)>>,
}

ferro_class!(PointerContactsTab: Control);
ferro_impl_classes!(PointerContactsTab: StyledElementImpl, LayoutableImpl, InteractiveImpl, ControlImpl);
ferro_class_info!(PointerContactsTab { new: PointerContactsTab::new });

impl FerroObjectImpl for PointerContactsTab {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_clip_to_bounds(true);
    }
}

impl InputElementImpl for PointerContactsTab {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        this.update_pointer(e);
        e.pointer().capture(Some(&this.to_ref().upcast::<InputElement>()));
        e.set_handled(true);
        e.prevent_gesture_recognition();
        Self::parent_on_pointer_pressed(this, e);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        this.update_pointer(e);
        e.set_handled(true);
        Self::parent_on_pointer_moved(this, e);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        this.remove_pointer(e.pointer());
        e.set_handled(true);
        this.invalidate_visual();
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        this.remove_pointer(e.pointer());
        this.invalidate_visual();
    }
}

impl VisualImpl for PointerContactsTab {
    fn render(this: &Self, context: &mut DrawingContext) {
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        context.fill_rectangle(&transparent, Rect::from_size(this.bounds().size()), 0.0);
        for (_, pt) in this.pointers.borrow().iter() {
            let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(pt.color));

            context.draw_ellipse_at(Some(&brush), None, pt.point, 75.0, 75.0);
        }
    }
}

impl PointerContactsTab {
    pub fn construct() -> Self {
        Self { base: Control::construct(), pointers: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn remove_pointer(&self, pointer: &Rc<dyn IPointer>) {
        self.pointers.borrow_mut().retain(|(known, _)| !Rc::ptr_eq(known, pointer));
    }

    /// # Panics
    /// Panics if every colour is in use by another pointer (an exception in
    /// the managed original).
    fn update_pointer(&self, e: &PointerEventArgs) {
        let position = e.get_position(Some(self));
        {
            let mut pointers = self.pointers.borrow_mut();
            let index = match pointers.iter().position(|(known, _)| Rc::ptr_eq(known, e.pointer())) {
                Some(index) => index,
                None => {
                    if e.routed_event() == Some(InputElement::pointer_moved_event().as_routed_event()) {
                        return;
                    }
                    let colors: Vec<Color> = ALL_COLORS
                        .iter()
                        .copied()
                        .filter(|color| !pointers.iter().any(|(_, info)| info.color == *color))
                        .collect();
                    assert!(!colors.is_empty(), "every colour is in use");
                    let color = colors[next_random(colors.len() - 1)];
                    pointers.push((e.pointer().clone(), PointerInfo { point: Point::default(), color }));
                    pointers.len() - 1
                }
            };

            pointers[index].1.point = position;
        }
        self.invalidate_visual();
    }
}
