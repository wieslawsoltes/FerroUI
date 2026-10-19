//! `NewGameViewModelTests` (`ReactiveUI.Tests/ObservedChanged/NewGameViewModelTests.cs`): commands
//! whose ability to execute follows observed properties.

use crate::mocks::NewGameViewModel;
use ferroui_reactive::ObservableExt;

#[test]
fn can_add_up_to_seven_players() {
    const MAX_PLAYERS: usize = 7;
    let viewmodel = NewGameViewModel::new();
    for i in 1..=MAX_PLAYERS {
        viewmodel.set_new_player_name(Some(format!("Player{i}")));
        let _ = viewmodel.add_player().execute_default().subscribe_next(|_| {});
        assert_eq!(viewmodel.players().count(), i);
    }
}
