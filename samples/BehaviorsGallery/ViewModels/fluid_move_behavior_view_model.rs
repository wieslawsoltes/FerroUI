//! Port of `ViewModels/FluidMoveBehaviorViewModel.cs`.

use super::items::ItemList;
use super::{Command, ViewModelBase};
use ferroui_base::collections::FerroList;
use ferroui_base::input::ICommand;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::reactive_object;
use std::rc::{Rc, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

/// The view model of the page of the fluid move behavior: numbers the page shuffles.
pub struct FluidMoveBehaviorViewModel {
    base: ViewModelBase,
    items: ItemList,
    shuffle_command: Rc<dyn ICommand>,
}
reactive_object!(FluidMoveBehaviorViewModel, base);

impl PartialEq for FluidMoveBehaviorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FluidMoveBehaviorViewModel {
    /// `FluidMoveBehaviorViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| {
            let items: ItemList = Rc::new(FerroList::new());
            for i in 1..=8 {
                items.add(Rc::new(i as i32) as BoxedValue);
            }

            let this = this.clone();
            Self {
                base: ViewModelBase::new(),
                items,
                shuffle_command: Command::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.shuffle();
                    }
                }),
            }
        })
    }

    /// `Items`.
    pub fn items(&self) -> ItemList {
        self.items.clone()
    }

    /// `ShuffleCommand`.
    pub fn shuffle_command(&self) -> Rc<dyn ICommand> {
        self.shuffle_command.clone()
    }

    /// `Shuffle()`: the items again, in a random order.
    fn shuffle(&self) {
        let mut array = self.items.to_vec();
        self.items.clear();
        // `array.OrderBy(_ => rnd.Next())`: a Fisher-Yates shuffle over a small generator seeded
        // by the clock (the gallery has no random number crate).
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.as_nanos() as u64)
            .unwrap_or(0x2545_f491_4f6c_dd1d)
            | 1;
        for i in (1..array.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let j = (seed % (i as u64 + 1)) as usize;
            array.swap(i, j);
        }
        for value in array {
            self.items.add(value);
        }
    }
}

type Vm = Rc<FluidMoveBehaviorViewModel>;

ferro_markup_type!(class FluidMoveBehaviorViewModel {
    this: Rc<FluidMoveBehaviorViewModel>,
    handles: [FluidMoveBehaviorViewModel, Rc<FluidMoveBehaviorViewModel>, Option<Rc<FluidMoveBehaviorViewModel>>],
    constructors: [() => FluidMoveBehaviorViewModel::new],
    properties: [
        Items: ItemsSource { get: |this: &Vm| ItemsSource::from(this.items()) },
        ShuffleCommand: Rc<dyn ICommand> { get: |this: &Vm| this.shuffle_command() },
    ],
    notify_property_changed: FluidMoveBehaviorViewModel,
});
