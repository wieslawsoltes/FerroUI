//! Port of `ViewModels/AnimationSpeedPageViewModel.cs`.

use ferroui_base::animation::{PlaybackDirection, TimeSpan};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_controls::ItemsSource;
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::Rc;

const PLAYBACK_DIRECTIONS: [PlaybackDirection; 4] = [
    PlaybackDirection::Normal,
    PlaybackDirection::Reverse,
    PlaybackDirection::Alternate,
    PlaybackDirection::AlternateReverse,
];

pub struct AnimationSpeedPageViewModel {
    base: ViewModelBase,
    speed_ratio: Cell<f64>,
    playback_direction: Cell<PlaybackDirection>,
    playback_direction_opposite: Cell<PlaybackDirection>,
    delay: Cell<TimeSpan>,
    delay_iters: Cell<TimeSpan>,
}

impl PartialEq for AnimationSpeedPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for AnimationSpeedPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl AnimationSpeedPageViewModel {
    pub fn speed_ratio(&self) -> f64 {
        self.speed_ratio.get()
    }

    pub fn set_speed_ratio(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.speed_ratio, value, "SpeedRatio");
    }

    pub fn playback_directions(&self) -> &'static [PlaybackDirection] {
        &PLAYBACK_DIRECTIONS
    }

    fn get_opposite_playback_direction(value: PlaybackDirection) -> PlaybackDirection {
        match value {
            PlaybackDirection::Normal => PlaybackDirection::Reverse,
            PlaybackDirection::Reverse => PlaybackDirection::Normal,
            PlaybackDirection::Alternate => PlaybackDirection::AlternateReverse,
            PlaybackDirection::AlternateReverse => PlaybackDirection::Alternate,
        }
    }

    pub fn playback_direction(&self) -> PlaybackDirection {
        self.playback_direction.get()
    }

    pub fn set_playback_direction(&self, value: PlaybackDirection) {
        self.playback_direction.set(value);
        self.base.raise_property_changed("PlaybackDirection");
        self.playback_direction_opposite.set(Self::get_opposite_playback_direction(value));
        self.base.raise_property_changed("PlaybackDirectionOpposite");
    }

    pub fn playback_direction_opposite(&self) -> PlaybackDirection {
        self.playback_direction_opposite.get()
    }

    pub fn set_playback_direction_opposite(&self, value: PlaybackDirection) {
        self.playback_direction_opposite.set(value);
        self.base.raise_property_changed("PlaybackDirectionOpposite");
        self.playback_direction.set(Self::get_opposite_playback_direction(value));
        self.base.raise_property_changed("PlaybackDirection");
    }

    pub fn delay(&self) -> TimeSpan {
        self.delay.get()
    }

    pub fn set_delay(&self, value: TimeSpan) {
        self.base.raise_and_set_if_changed_cell(&self.delay, value, "Delay");
    }

    pub fn delay_input(&self) -> f64 {
        self.delay.get().total_seconds()
    }

    pub fn set_delay_input(&self, value: f64) {
        self.set_delay(TimeSpan::from_seconds(value));
    }

    pub fn delay_iters(&self) -> TimeSpan {
        self.delay_iters.get()
    }

    pub fn set_delay_iters(&self, value: TimeSpan) {
        self.base.raise_and_set_if_changed_cell(&self.delay_iters, value, "DelayIters");
    }

    pub fn delay_iters_input(&self) -> f64 {
        self.delay_iters.get().total_seconds()
    }

    pub fn set_delay_iters_input(&self, value: f64) {
        self.set_delay_iters(TimeSpan::from_seconds(value));
    }

    pub fn new() -> Rc<AnimationSpeedPageViewModel> {
        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            speed_ratio: Cell::new(0.0),
            playback_direction: Cell::new(PlaybackDirection::default()),
            playback_direction_opposite: Cell::new(PlaybackDirection::default()),
            delay: Cell::new(TimeSpan::ZERO),
            delay_iters: Cell::new(TimeSpan::ZERO),
        });
        this.set_speed_ratio(1.0);
        this.set_playback_direction(PlaybackDirection::Normal);
        this.set_delay_input(0.0);
        this.set_delay_iters_input(0.0);
        this
    }
}

ferro_markup_type!(class AnimationSpeedPageViewModel {
    this: Rc<AnimationSpeedPageViewModel>,
    handles: [AnimationSpeedPageViewModel, Rc<AnimationSpeedPageViewModel>, Option<Rc<AnimationSpeedPageViewModel>>],
    constructors: [() => AnimationSpeedPageViewModel::new],
    properties: [
        SpeedRatio: f64 {
            get: |this: &Rc<AnimationSpeedPageViewModel>| this.speed_ratio(),
            set: |this: &Rc<AnimationSpeedPageViewModel>, value: f64| this.set_speed_ratio(value)
        },
        // An array a binding delivers to an items source property.
        PlaybackDirections: ItemsSource {
            get: |this: &Rc<AnimationSpeedPageViewModel>| ItemsSource::from_values(this.playback_directions().iter().copied())
        },
        PlaybackDirection: PlaybackDirection {
            get: |this: &Rc<AnimationSpeedPageViewModel>| this.playback_direction(),
            set: |this: &Rc<AnimationSpeedPageViewModel>, value: PlaybackDirection| this.set_playback_direction(value)
        },
        PlaybackDirectionOpposite: PlaybackDirection {
            get: |this: &Rc<AnimationSpeedPageViewModel>| this.playback_direction_opposite(),
            set: |this: &Rc<AnimationSpeedPageViewModel>, value: PlaybackDirection| {
                this.set_playback_direction_opposite(value)
            }
        },
        Delay: TimeSpan {
            get: |this: &Rc<AnimationSpeedPageViewModel>| this.delay(),
            set: |this: &Rc<AnimationSpeedPageViewModel>, value: TimeSpan| this.set_delay(value)
        },
        DelayInput: f64 {
            get: |this: &Rc<AnimationSpeedPageViewModel>| this.delay_input(),
            set: |this: &Rc<AnimationSpeedPageViewModel>, value: f64| this.set_delay_input(value)
        },
        DelayIters: TimeSpan {
            get: |this: &Rc<AnimationSpeedPageViewModel>| this.delay_iters(),
            set: |this: &Rc<AnimationSpeedPageViewModel>, value: TimeSpan| this.set_delay_iters(value)
        },
        DelayItersInput: f64 {
            get: |this: &Rc<AnimationSpeedPageViewModel>| this.delay_iters_input(),
            set: |this: &Rc<AnimationSpeedPageViewModel>, value: f64| this.set_delay_iters_input(value)
        },
    ],
    notify_property_changed: AnimationSpeedPageViewModel,
});
