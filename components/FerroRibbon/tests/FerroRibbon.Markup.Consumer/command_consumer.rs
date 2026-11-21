//! Independent consumer of the native abstract base and UI-independent command boundary.
use ferroribbon::controls::{RibbonControlBase, RibbonControlBaseImplExt};
use ferroribbon_core::commands::{ICommand as CoreCommand, RibbonRelayCommand};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::ControlImpl;
use ferroui_controls::primitives::TemplatedControlImpl;
use std::{cell::Cell, rc::Rc};
#[repr(C)]
pub struct CommandConsumer {
    base: RibbonControlBase,
    layouts: Cell<usize>,
    invokes: Cell<usize>,
}
ferro_class!(CommandConsumer:RibbonControlBase);
ferro_impl_classes!(CommandConsumer:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_class_info!(CommandConsumer {new:CommandConsumer::new,markup:{namespace:"FerroRibbon.Consumer"}});
impl CommandConsumer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonControlBase::construct(),
            layouts: Cell::new(0),
            invokes: Cell::new(0),
        })
    }
}

impl ferroribbon::controls::RibbonControlBaseImpl for CommandConsumer {
    fn invoke_core(this: &Self) -> bool {
        this.invokes.set(this.invokes.get() + 1);
        Self::parent_invoke_core(this)
    }
    fn on_layout_applied_core(this: &Self, layout: ferroribbon::controls::RibbonItemLayout) {
        assert_eq!(this.current_size(), layout.size);
        this.layouts.set(this.layouts.get() + 1);
    }
}
impl CommandConsumer {
    pub fn invoke_calls(&self) -> usize {
        self.invokes.get()
    }
    pub fn layouts(&self) -> usize {
        self.layouts.get()
    }
}

pub struct CommandViewModel {
    value: Cell<i32>,
    command: Rc<dyn CoreCommand>,
    executed: Rc<Cell<i32>>,
}
impl PartialEq for CommandViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl CommandViewModel {
    pub fn new() -> Rc<Self> {
        ferroui_base::data::core::ValueTypes::register_reference::<Self>();
        let executed = Rc::new(Cell::new(0));
        let runs = executed.clone();
        Rc::new(Self {
            value: Cell::new(7),
            executed,
            command: RibbonRelayCommand::new_with_parameter(
                move |p| {
                    assert_eq!(p.unwrap().downcast_ref::<i32>(), Some(&7));
                    runs.set(runs.get() + 1)
                },
                Some(Rc::new(|p| {
                    p.and_then(|p| p.downcast_ref::<i32>())
                        .is_some_and(|n| *n > 0)
                })),
            ),
        })
    }
    pub fn command(&self) -> Rc<dyn CoreCommand> {
        self.command.clone()
    }
    pub fn value(&self) -> i32 {
        self.value.get()
    }
    pub fn set_value(&self, value: i32) {
        self.value.set(value)
    }
    pub fn executed(&self) -> i32 {
        self.executed.get()
    }
}
ferroui_base::ferro_markup_type!(class CommandViewModel {
 namespace:"FerroRibbon.Consumer",this:Rc<CommandViewModel>,handles:[Rc<CommandViewModel>,CommandViewModel,Option<Rc<CommandViewModel>>],constructors:[()=>CommandViewModel::new],
 properties:[Command:Rc<dyn CoreCommand>{get:|this:&Rc<CommandViewModel>|this.command()},Value:i32{get:|this:&Rc<CommandViewModel>|this.value(),set:|this:&Rc<CommandViewModel>,value|this.set_value(value)}],
});
