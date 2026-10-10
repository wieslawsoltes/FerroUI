//! Port of `ViewModels/AnimationsPageViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct AnimationsPageViewModel {
    base: ViewModelBase,
    is_playing: Cell<bool>,
    play_state_text: RefCell<String>,
}

impl PartialEq for AnimationsPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for AnimationsPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl AnimationsPageViewModel {
    pub fn new() -> Rc<AnimationsPageViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            is_playing: Cell::new(true),
            play_state_text: RefCell::new(String::from("Pause animations on this page")),
        })
    }

    pub fn toggle_play_state(&self) {
        self.set_play_state_text(String::from(if self.is_playing.get() {
            "Resume animations on this page"
        } else {
            "Pause animations on this page"
        }));
        self.is_playing.set(!self.is_playing.get());
    }

    pub fn play_state_text(&self) -> String {
        self.play_state_text.borrow().clone()
    }

    pub fn set_play_state_text(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.play_state_text, value, "PlayStateText");
    }
}

ferro_markup_type!(class AnimationsPageViewModel {
    this: Rc<AnimationsPageViewModel>,
    handles: [AnimationsPageViewModel, Rc<AnimationsPageViewModel>, Option<Rc<AnimationsPageViewModel>>],
    constructors: [() => AnimationsPageViewModel::new],
    properties: [
        PlayStateText: String {
            get: |this: &Rc<AnimationsPageViewModel>| this.play_state_text(),
            set: |this: &Rc<AnimationsPageViewModel>, value: String| this.set_play_state_text(value)
        },
    ],
    methods: [
        fn TogglePlayState() => |this: &Rc<AnimationsPageViewModel>| this.toggle_play_state(),
    ],
    notify_property_changed: AnimationsPageViewModel,
});
