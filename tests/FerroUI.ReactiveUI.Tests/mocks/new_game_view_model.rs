//! `NewGameViewModel` (`ReactiveUI.Tests/ObservedChanged/Mocks/NewGameViewModel.cs`): a sample
//! view model of a game, whose commands depend on observed properties.

use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveCommand, ReactiveObject, ReactiveUIBindingExtensions};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::{prop, IObservable};
use std::cell::{OnceCell, RefCell};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::rc::Rc;

/// The most players a game takes.
const MAX_PLAYERS: usize = 7;

/// The fewest players a game starts with.
const MIN_PLAYERS: usize = 3;

reactive_properties! {
    /// A game that takes up to seven players and starts with three (`NewGameViewModel`).
    ///
    /// The commands are made once the view model is shared (in [`new`](NewGameViewModel::new)),
    /// since they observe it through the `Rc` that upstream's constructor has as `this`.
    pub struct NewGameViewModel {
        reactive: ReactiveObject,
        #[reactive]
        new_player_name: Option<String>,
        players: Rc<ObservableCollection<String>>,
        add_player: OnceCell<ReactiveCommand<(), ()>>,
        randomize_order: OnceCell<ReactiveCommand<(), ()>>,
        remove_player: OnceCell<ReactiveCommand<String, ()>>,
        start_game: OnceCell<ReactiveCommand<(), ()>>,
    }
}
reactive_object!(NewGameViewModel);

impl NewGameViewModel {
    /// A game without players, with its commands (the constructor `NewGameViewModel()`).
    pub fn new() -> Rc<Self> {
        let view_model = Rc::new(Self {
            reactive: ReactiveObject::new(),
            new_player_name: RefCell::new(None),
            players: Rc::new(ObservableCollection::new()),
            add_player: OnceCell::new(),
            randomize_order: OnceCell::new(),
            remove_player: OnceCell::new(),
            start_game: OnceCell::new(),
        });

        // Deviation (DEVIATIONS.md, ReactiveUI): upstream follows the count of the players with
        // DynamicData (`Players.ToReactiveChangeSet().WhenCountChanged()`), which this crate does
        // not depend on; the count is observed as the property `Players.Count`, which the
        // collection notifies.
        let can_start: Rc<dyn IObservable<bool>> =
            view_model.when_any_value(prop!(NewGameViewModel => players.count)).select(|count: usize| count >= MIN_PLAYERS);

        let start_game = ReactiveCommand::create_with(|| (), Some(can_start.clone()), None);

        let players = view_model.players.clone();
        let randomize_order = ReactiveCommand::create_with(
            move || {
                let mut list = players.to_vec();
                shuffle(&mut list);
                players.clear();
                for player in list {
                    players.add(player);
                }
            },
            Some(can_start),
            None,
        );

        let players = view_model.players.clone();
        let remove_player = ReactiveCommand::create_param(move |player: String| {
            players.remove(&player);
        });

        let players = view_model.players.clone();
        let can_add_player: Rc<dyn IObservable<bool>> = view_model.when_any_value_select(
            (prop!(NewGameViewModel => players.count), prop!(NewGameViewModel => new_player_name)),
            move |(count, new_player_name): (usize, Option<String>)| match new_player_name {
                Some(name) => count < MAX_PLAYERS && !name.trim().is_empty() && !players.contains(&name),
                None => false,
            },
        );

        let weak = Rc::downgrade(&view_model);
        let add_player = ReactiveCommand::create_with(
            move || {
                let Some(view_model) = weak.upgrade() else {
                    return;
                };
                let Some(name) = view_model.new_player_name() else {
                    panic!("NewPlayerName is null");
                };
                view_model.players.add(name.trim().to_owned());
                view_model.set_new_player_name(Some(String::new()));
            },
            Some(can_add_player),
            None,
        );

        let _ = view_model.start_game.set(start_game);
        let _ = view_model.randomize_order.set(randomize_order);
        let _ = view_model.remove_player.set(remove_player);
        let _ = view_model.add_player.set(add_player);
        view_model
    }

    /// `AddPlayer`: adds the new player's name, trimmed, and clears it.
    pub fn add_player(&self) -> ReactiveCommand<(), ()> {
        self.add_player.get().expect("the commands are made by new").clone()
    }

    /// `Players`.
    pub fn players(&self) -> Rc<ObservableCollection<String>> {
        self.players.clone()
    }

    /// `RandomizeOrder`: shuffles the players.
    pub fn randomize_order(&self) -> ReactiveCommand<(), ()> {
        self.randomize_order.get().expect("the commands are made by new").clone()
    }

    /// `RemovePlayer`: removes a player.
    pub fn remove_player(&self) -> ReactiveCommand<String, ()> {
        self.remove_player.get().expect("the commands are made by new").clone()
    }

    /// `StartGame`.
    pub fn start_game(&self) -> ReactiveCommand<(), ()> {
        self.start_game.get().expect("the commands are made by new").clone()
    }
}

/// Shuffles `list` in place (Fisher-Yates, `ShuffleCrypto`).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream draws from a cryptographic generator
// (`RandomNumberGenerator`); the standard library has none, so the draws come from the random
// keys of its hasher, as Splat's `Guid` does.
fn shuffle<T>(list: &mut [T]) {
    for i in (1..list.len()).rev() {
        let j = random_below(i + 1);
        list.swap(i, j);
    }
}

/// A random number in `0..bound`.
fn random_below(bound: usize) -> usize {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_usize(bound);
    (hasher.finish() % bound as u64) as usize
}
