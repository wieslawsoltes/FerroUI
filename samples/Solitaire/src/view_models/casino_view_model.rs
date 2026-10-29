//! CasinoViewModel: navigation, game instances, settings and statistics.
use crate::{models::*, view_models::CardGameViewModel};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Title,
    Game(GameKind),
    Settings,
    Statistics,
}
pub struct CasinoViewModel {
    pub state: PersistentState,
    pub current_view: Page,
    pub message: String,
    path: PathBuf,
    storage_available: bool,
}
impl CasinoViewModel {
    pub fn load(path: PathBuf) -> Self {
        match PersistentState::load(&path) {
            Ok(state) => Self {
                state,
                current_view: Page::Title,
                message: String::new(),
                path,
                storage_available: true,
            },
            Err(error) => Self {
                state: PersistentState::default(),
                current_view: Page::Title,
                message: format!(
                    "{error}. Original save preserved; this session will not overwrite it."
                ),
                path,
                storage_available: false,
            },
        }
    }
    pub fn navigate_to_game(&mut self, kind: GameKind) {
        // Each source title command executes NewGame after selecting the game.
        self.new_game(kind, rand::random());
    }
    pub fn new_game(&mut self, kind: GameKind, seed: u64) {
        self.state.games[kind.index()] = Some(CardGameViewModel::new(
            kind,
            self.state.settings.draw_mode,
            self.state.settings.difficulty,
            seed,
        ));
        self.state.statistics[kind.index()].played += 1;
        self.current_view = Page::Game(kind);
        self.message.clear();
    }
    pub fn game(&self) -> Option<&CardGameViewModel> {
        if let Page::Game(kind) = self.current_view {
            self.state.games[kind.index()].as_ref()
        } else {
            None
        }
    }
    pub fn game_mut(&mut self) -> Option<&mut CardGameViewModel> {
        if let Page::Game(kind) = self.current_view {
            self.state.games[kind.index()].as_mut()
        } else {
            None
        }
    }
    pub fn update_statistics(&mut self) {
        let Some(game) = self.game_mut() else { return };
        if !game.board.is_game_won || game.victory_recorded {
            return;
        }
        game.victory_recorded = true;
        let (kind, score, seconds, moves) = (
            game.kind,
            game.board.score,
            game.elapsed_seconds,
            game.board.moves,
        );
        let stats = &mut self.state.statistics[kind.index()];
        stats.won += 1;
        stats.best_score = Some(stats.best_score.map_or(score, |v| v.max(score)));
        stats.best_seconds = Some(stats.best_seconds.map_or(seconds, |v| v.min(seconds)));
        stats.fewest_moves = Some(stats.fewest_moves.map_or(moves, |v| v.min(moves)));
    }
    pub fn tick(&mut self, seconds: u64) {
        if let Some(game) = self.game_mut()
            && !game.board.is_game_won
        {
            game.elapsed_seconds += seconds;
            self.state.statistics[game.kind.index()].total_seconds += seconds;
        }
    }
    pub fn save(&mut self) {
        self.update_statistics();
        if self.storage_available
            && let Err(e) = self.state.save(&self.path)
        {
            self.message = format!("Could not save: {e}");
        }
    }
}
