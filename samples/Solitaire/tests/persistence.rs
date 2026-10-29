use ferro_solitaire::{models::*, view_models::*};
#[test]
fn save_round_trips_settings_all_games_statistics_and_undo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let mut casino = CasinoViewModel::load(path.clone());
    casino.state.settings.draw_mode = DrawMode::DrawThree;
    casino.state.settings.difficulty = Difficulty::Hard;
    for kind in GameKind::ALL {
        casino.new_game(kind, 5);
        if kind != GameKind::FreeCell {
            casino.game_mut().unwrap().turn_stock();
        }
    }
    casino.tick(40);
    casino.save();
    let state = PersistentState::load(&path).unwrap();
    assert_eq!(state.settings.draw_mode, DrawMode::DrawThree);
    assert_eq!(state.settings.difficulty, Difficulty::Hard);
    for kind in GameKind::ALL {
        assert_eq!(state.statistics[kind.index()].played, 1);
        let game = state.games[kind.index()].as_ref().unwrap();
        game.validate().unwrap();
        assert_eq!(game.can_undo(), kind != GameKind::FreeCell);
    }
    assert_eq!(state.games[2].as_ref().unwrap().elapsed_seconds, 40);
    assert_eq!(state.statistics[2].total_seconds, 40);
    // A second save replaces the existing file, including on Windows.
    casino.tick(2);
    casino.save();
    assert_eq!(
        PersistentState::load(&path).unwrap().games[2]
            .as_ref()
            .unwrap()
            .elapsed_seconds,
        42
    );
}
#[test]
fn corrupt_save_is_reported_and_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    std::fs::write(&path, "broken").unwrap();
    let mut casino = CasinoViewModel::load(path.clone());
    assert!(casino.message.contains("Could not read save"));
    casino.new_game(GameKind::Klondike, 1);
    casino.save();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "broken");
}
#[test]
fn saves_with_duplicate_card_ids_or_wrong_suits_are_rejected() {
    let mut state = PersistentState::default();
    let mut g = CardGameViewModel::new(GameKind::Spider, DrawMode::DrawOne, Difficulty::Easy, 1);
    g.board.stock[0].id = g.board.stock[1].id;
    state.games[2] = Some(g);
    assert!(state.validate().is_err());
    let mut g = CardGameViewModel::new(GameKind::Spider, DrawMode::DrawOne, Difficulty::Easy, 1);
    g.board.stock[0].suit = CardSuit::Clubs;
    state.games[2] = Some(g);
    assert!(state.validate().is_err());
}
#[test]
fn statistics_record_a_win_once_even_after_undo_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut casino = CasinoViewModel::load(dir.path().join("state.json"));
    casino.new_game(GameKind::Klondike, 1);
    casino.game_mut().unwrap().board.is_game_won = true;
    casino.game_mut().unwrap().board.score = 100;
    casino.update_statistics();
    casino.update_statistics();
    assert_eq!(casino.state.statistics[0].won, 1);
    casino.game_mut().unwrap().restart();
    casino.game_mut().unwrap().board.is_game_won = true;
    casino.update_statistics();
    assert_eq!(casino.state.statistics[0].won, 1);
}
#[test]
fn opening_games_always_starts_a_fresh_deal_even_after_reload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let mut casino = CasinoViewModel::load(path.clone());
    for kind in GameKind::ALL {
        casino.new_game(kind, 1);
        if kind != GameKind::FreeCell {
            casino.game_mut().unwrap().turn_stock();
        }
        casino.tick(10);
    }
    casino.current_view = Page::Title;
    casino.tick(20);
    for game in casino.state.games.iter().flatten() {
        assert_eq!(game.elapsed_seconds, 10);
    }
    casino.save();
    let mut reopened = CasinoViewModel::load(path);
    reopened.state.settings.draw_mode = DrawMode::DrawThree;
    reopened.state.settings.difficulty = Difficulty::Hard;
    for kind in GameKind::ALL {
        reopened.navigate_to_game(kind);
        let game = reopened.game().unwrap();
        assert_eq!(game.elapsed_seconds, 0);
        assert_eq!(game.board.moves, 0);
        assert!(!game.can_undo());
        assert_eq!(game.board, game.initial_board);
        assert_eq!(game.draw_mode, DrawMode::DrawThree);
        assert_eq!(game.difficulty, Difficulty::Hard);
        game.validate().unwrap();
        assert_eq!(reopened.state.statistics[kind.index()].played, 2);
    }
}
