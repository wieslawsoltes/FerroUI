//! Port of `Pages/GlyphRunPage.xaml.cs`: the class of the document
//! `Pages/GlyphRunPage.xaml` and the two controls of the page.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, DrawingContext, GlyphRun, GlyphTypeface, IBrush, Typeface};
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl, UserControl};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct GlyphRunPage {
    base: UserControl,
}

user_control_class!(GlyphRunPage);
ferro_class_info!(GlyphRunPage { new: GlyphRunPage::new });
xaml_class!(GlyphRunPage, "/Pages/GlyphRunPage.xaml");

impl GlyphRunPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// The pseudo-random number generator of the runtime library (`System.Random`) as the
/// controls of the page use it: the subtractive generator, with a seed that differs per
/// instance.
struct Random {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl Random {
    /// `new Random()`.
    fn new() -> Self {
        // The hasher of the standard library is keyed with random data per instance.
        let seed = RandomState::new().build_hasher().finish();
        let seed = (seed ^ (seed >> 32)) as u32 as i32;

        let mut seed_array = [0i32; 56];

        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = 161_803_398 - subtraction;
        seed_array[55] = mj;
        let mut mk = 1;

        let mut ii = 0;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55;
            }

            seed_array[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += i32::MAX;
            }

            mj = seed_array[ii];
        }

        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;
                if n >= 55 {
                    n -= 55;
                }

                seed_array[i] = seed_array[i].wrapping_sub(seed_array[1 + n]);
                if seed_array[i] < 0 {
                    seed_array[i] += i32::MAX;
                }
            }
        }

        Self { seed_array, inext: 0, inextp: 21 }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext + 1;
        if loc_inext >= 56 {
            loc_inext = 1;
        }

        let mut loc_inextp = self.inextp + 1;
        if loc_inextp >= 56 {
            loc_inextp = 1;
        }

        let mut ret_val = self.seed_array[loc_inext].wrapping_sub(self.seed_array[loc_inextp]);

        if ret_val == i32::MAX {
            ret_val -= 1;
        }
        if ret_val < 0 {
            ret_val += i32::MAX;
        }

        self.seed_array[loc_inext] = ret_val;
        self.inext = loc_inext;
        self.inextp = loc_inextp;

        ret_val
    }

    fn sample(&mut self) -> f64 {
        f64::from(self.internal_sample()) * (1.0 / f64::from(i32::MAX))
    }

    /// `Next(minValue, maxValue)` for a range that is not longer than `i32::MAX`: a number of
    /// the range `min_value..max_value`.
    fn next_range(&mut self, min_value: i32, max_value: i32) -> i32 {
        assert!(min_value <= max_value, "'minValue' cannot be greater than maxValue.");
        let range = i64::from(max_value) - i64::from(min_value);
        (self.sample() * range as f64) as i32 + min_value
    }
}

#[repr(C)]
pub struct GlyphRunControl {
    base: Control,
    glyph_typeface: Rc<GlyphTypeface>,
    rand: RefCell<Random>,
    glyph_indices: RefCell<[u16; 1]>,
    characters: RefCell<[u16; 1]>,
    font_size: Cell<f32>,
    direction: Cell<i32>,

    timer: RefCell<Option<Rc<DispatcherTimer>>>,
}

ferro_class!(GlyphRunControl: Control);
ferro_impl_classes!(
    GlyphRunControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(GlyphRunControl { new: GlyphRunControl::new });

impl VisualImpl for GlyphRunControl {
    fn on_attached_to_visual_tree(this: &Self, _e: &VisualTreeAttachmentEventArgs) {
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs(1));

        // The control keeps the timer: the handler of the timer holds the control weakly.
        let weak = this.to_ref().downgrade();
        timer.tick(move |_| {
            if let Some(this) = weak.upgrade() {
                this.invalidate_visual();
            }
        });

        timer.start();

        *this.timer.borrow_mut() = Some(timer);
    }

    fn on_detached_from_visual_tree(this: &Self, _e: &VisualTreeAttachmentEventArgs) {
        let timer = this.timer.borrow_mut().take();
        if let Some(timer) = timer {
            timer.stop();
        }
    }

    fn render(this: &Self, context: &mut DrawingContext) {
        let c = this.rand.borrow_mut().next_range(65, 90) as u16;

        if this.font_size.get() + this.direction.get() as f32 > 200.0 {
            this.direction.set(-10);
        }

        if this.font_size.get() + (this.direction.get() as f32) < 20.0 {
            this.direction.set(10);
        }

        this.font_size.set(this.font_size.get() + this.direction.get() as f32);

        this.glyph_indices.borrow_mut()[0] = this.glyph_typeface.character_to_glyph_map().get_glyph(i32::from(c));

        this.characters.borrow_mut()[0] = c;

        let glyph_run = GlyphRun::from_glyph_indices(
            this.glyph_typeface.clone(),
            f64::from(this.font_size.get()),
            ReadOnlyMemory::from_slice(&*this.characters.borrow()),
            &*this.glyph_indices.borrow(),
            None,
            0,
        );

        let black: Rc<dyn IBrush> = Brushes::black();
        context.draw_glyph_run(Some(&black), &glyph_run);
    }
}

impl GlyphRunControl {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            glyph_typeface: Typeface::default().glyph_typeface(),
            rand: RefCell::new(Random::new()),
            glyph_indices: RefCell::new([0; 1]),
            characters: RefCell::new([0; 1]),
            font_size: Cell::new(20.0),
            direction: Cell::new(10),
            timer: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

#[repr(C)]
pub struct GlyphRunGeometryControl {
    base: Control,
    glyph_typeface: Rc<GlyphTypeface>,
    rand: RefCell<Random>,
    glyph_indices: RefCell<[u16; 1]>,
    characters: RefCell<[u16; 1]>,
    font_size: Cell<f32>,
    direction: Cell<i32>,

    timer: RefCell<Option<Rc<DispatcherTimer>>>,
}

ferro_class!(GlyphRunGeometryControl: Control);
ferro_impl_classes!(
    GlyphRunGeometryControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(GlyphRunGeometryControl { new: GlyphRunGeometryControl::new });

impl VisualImpl for GlyphRunGeometryControl {
    fn on_attached_to_visual_tree(this: &Self, _e: &VisualTreeAttachmentEventArgs) {
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs(1));

        // The control keeps the timer: the handler of the timer holds the control weakly.
        let weak = this.to_ref().downgrade();
        timer.tick(move |_| {
            if let Some(this) = weak.upgrade() {
                this.invalidate_visual();
            }
        });

        timer.start();

        *this.timer.borrow_mut() = Some(timer);
    }

    fn on_detached_from_visual_tree(this: &Self, _e: &VisualTreeAttachmentEventArgs) {
        let timer = this.timer.borrow_mut().take();
        if let Some(timer) = timer {
            timer.stop();
        }
    }

    fn render(this: &Self, context: &mut DrawingContext) {
        let c = this.rand.borrow_mut().next_range(65, 90) as u16;

        if this.font_size.get() + this.direction.get() as f32 > 200.0 {
            this.direction.set(-10);
        }

        if this.font_size.get() + (this.direction.get() as f32) < 20.0 {
            this.direction.set(10);
        }

        this.font_size.set(this.font_size.get() + this.direction.get() as f32);

        this.glyph_indices.borrow_mut()[0] = this.glyph_typeface.character_to_glyph_map().get_glyph(i32::from(c));

        this.characters.borrow_mut()[0] = c;

        let glyph_run = GlyphRun::from_glyph_indices(
            this.glyph_typeface.clone(),
            f64::from(this.font_size.get()),
            ReadOnlyMemory::from_slice(&*this.characters.borrow()),
            &*this.glyph_indices.borrow(),
            None,
            0,
        );

        let geometry = glyph_run.build_geometry();

        let green: Rc<dyn IBrush> = Brushes::green();
        context.draw_geometry(Some(&green), None, &geometry);
    }
}

impl GlyphRunGeometryControl {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            glyph_typeface: Typeface::default().glyph_typeface(),
            rand: RefCell::new(Random::new()),
            glyph_indices: RefCell::new([0; 1]),
            characters: RefCell::new([0; 1]),
            font_size: Cell::new(20.0),
            direction: Cell::new(10),
            timer: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
