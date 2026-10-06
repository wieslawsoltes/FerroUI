//! Port of `LayoutTestControl.cs` and `LayoutTestRoot.cs`, the controls of
//! the layout tests of the base crate. They derive from controls, so the
//! layout tests that use them live with the controls.

use crate::test_support::TestRoot;
use crate::{ControlImpl, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `Func<Layoutable, Size, Size>` of the measure and arrange overrides.
pub type LayoutOverride = Rc<dyn Fn(&Layoutable, Size) -> Size>;

/// A decorator that records whether it has been measured and arranged, and
/// whose measure and arrange can be replaced.
#[repr(C)]
pub struct LayoutTestControl {
    base: Decorator,
    measured: Cell<bool>,
    arranged: Cell<bool>,
    do_measure_override: RefCell<Option<LayoutOverride>>,
    do_arrange_override: RefCell<Option<LayoutOverride>>,
    call_base_measure: Cell<bool>,
    call_base_arrange: Cell<bool>,
}

ferro_class!(LayoutTestControl: Decorator);
ferro_impl_classes!(
    LayoutTestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for LayoutTestControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measured.set(true);

        let do_measure_override = this.do_measure_override.borrow().clone();
        match do_measure_override {
            Some(do_measure_override) => {
                let override_result = do_measure_override(this, available_size);
                if this.call_base_measure.get() {
                    Self::parent_measure_override(this, override_result)
                } else {
                    override_result
                }
            }
            None => Self::parent_measure_override(this, available_size),
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arranged.set(true);

        let do_arrange_override = this.do_arrange_override.borrow().clone();
        match do_arrange_override {
            Some(do_arrange_override) => {
                let override_result = do_arrange_override(this, final_size);
                if this.call_base_arrange.get() {
                    Self::parent_arrange_override(this, override_result)
                } else {
                    override_result
                }
            }
            None => Self::parent_arrange_override(this, final_size),
        }
    }
}

impl LayoutTestControl {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Decorator::construct(),
            measured: Cell::new(false),
            arranged: Cell::new(false),
            do_measure_override: RefCell::new(None),
            do_arrange_override: RefCell::new(None),
            call_base_measure: Cell::new(false),
            call_base_arrange: Cell::new(false),
        })
    }

    pub fn measured(&self) -> bool {
        self.measured.get()
    }

    pub fn set_measured(&self, value: bool) {
        self.measured.set(value)
    }

    pub fn arranged(&self) -> bool {
        self.arranged.get()
    }

    pub fn set_arranged(&self, value: bool) {
        self.arranged.set(value)
    }

    pub fn set_do_measure_override(&self, value: Option<LayoutOverride>) {
        *self.do_measure_override.borrow_mut() = value;
    }

    pub fn set_do_arrange_override(&self, value: Option<LayoutOverride>) {
        *self.do_arrange_override.borrow_mut() = value;
    }

    // No ported test sets it; the upstream helper has it.
    #[allow(dead_code)]
    pub fn set_call_base_measure(&self, value: bool) {
        self.call_base_measure.set(value)
    }

    pub fn set_call_base_arrange(&self, value: bool) {
        self.call_base_arrange.set(value)
    }
}

/// A test root that records whether it has been measured and arranged, and
/// whose measure and arrange can be replaced.
#[repr(C)]
pub struct LayoutTestRoot {
    base: TestRoot,
    measured: Cell<bool>,
    arranged: Cell<bool>,
    do_measure_override: RefCell<Option<LayoutOverride>>,
    do_arrange_override: RefCell<Option<LayoutOverride>>,
}

ferro_class!(LayoutTestRoot: TestRoot);
ferro_impl_classes!(
    LayoutTestRoot: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for LayoutTestRoot {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measured.set(true);

        let do_measure_override = this.do_measure_override.borrow().clone();
        match do_measure_override {
            Some(do_measure_override) => do_measure_override(this, available_size),
            None => Self::parent_measure_override(this, available_size),
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arranged.set(true);

        let do_arrange_override = this.do_arrange_override.borrow().clone();
        match do_arrange_override {
            Some(do_arrange_override) => do_arrange_override(this, final_size),
            None => Self::parent_arrange_override(this, final_size),
        }
    }
}

impl LayoutTestRoot {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: TestRoot::construct(),
            measured: Cell::new(false),
            arranged: Cell::new(false),
            do_measure_override: RefCell::new(None),
            do_arrange_override: RefCell::new(None),
        })
    }

    pub fn measured(&self) -> bool {
        self.measured.get()
    }

    pub fn set_measured(&self, value: bool) {
        self.measured.set(value)
    }

    pub fn arranged(&self) -> bool {
        self.arranged.get()
    }

    pub fn set_arranged(&self, value: bool) {
        self.arranged.set(value)
    }

    pub fn set_do_measure_override(&self, value: Option<LayoutOverride>) {
        *self.do_measure_override.borrow_mut() = value;
    }

    pub fn set_do_arrange_override(&self, value: Option<LayoutOverride>) {
        *self.do_arrange_override.borrow_mut() = value;
    }
}
