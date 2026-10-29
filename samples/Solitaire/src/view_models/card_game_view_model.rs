//! Shared state and operations from CardGameViewModel. Snapshots make every deal,
//! reveal and completed Spider run reversible as a single operation.
use crate::models::*;
use rand::{SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pile {
    Stock,
    Waste,
    Tableau(usize),
    Foundation(usize),
    Cell(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardPosition {
    pub pile: Pile,
    pub index: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardMove {
    pub from: CardPosition,
    pub to: Pile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub tableau: Vec<Vec<PlayingCard>>,
    pub foundations: Vec<Vec<PlayingCard>>,
    pub cells: Vec<Vec<PlayingCard>>,
    pub stock: Vec<PlayingCard>,
    pub waste: Vec<PlayingCard>,
    pub score: i32,
    pub moves: u32,
    pub is_game_won: bool,
}
impl Board {
    pub fn pile(&self, pile: Pile) -> Option<&Vec<PlayingCard>> {
        match pile {
            Pile::Stock => Some(&self.stock),
            Pile::Waste => Some(&self.waste),
            Pile::Tableau(i) => self.tableau.get(i),
            Pile::Foundation(i) => self.foundations.get(i),
            Pile::Cell(i) => self.cells.get(i),
        }
    }
    pub(crate) fn pile_mut(&mut self, pile: Pile) -> Option<&mut Vec<PlayingCard>> {
        match pile {
            Pile::Stock => Some(&mut self.stock),
            Pile::Waste => Some(&mut self.waste),
            Pile::Tableau(i) => self.tableau.get_mut(i),
            Pile::Foundation(i) => self.foundations.get_mut(i),
            Pile::Cell(i) => self.cells.get_mut(i),
        }
    }
    pub fn cards(&self) -> impl Iterator<Item = &PlayingCard> {
        self.tableau
            .iter()
            .chain(&self.foundations)
            .chain(&self.cells)
            .flatten()
            .chain(&self.stock)
            .chain(&self.waste)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardGameViewModel {
    pub kind: GameKind,
    pub draw_mode: DrawMode,
    pub difficulty: Difficulty,
    pub seed: u64,
    pub board: Board,
    pub initial_board: Board,
    pub elapsed_seconds: u64,
    pub victory_recorded: bool,
    history: Vec<Board>,
}
impl CardGameViewModel {
    pub fn new(kind: GameKind, draw_mode: DrawMode, difficulty: Difficulty, seed: u64) -> Self {
        let mut deck = Vec::new();
        let suits = if kind == GameKind::Spider {
            difficulty.suits()
        } else {
            &CardSuit::ALL
        };
        let copies = if kind == GameKind::Spider {
            8 / suits.len()
        } else {
            1
        };
        for _ in 0..copies {
            for &suit in suits {
                for rank in 1..=13 {
                    deck.push(PlayingCard {
                        id: deck.len() as u8,
                        suit,
                        rank,
                        is_face_down: true,
                    });
                }
            }
        }
        deck.shuffle(&mut ChaCha8Rng::seed_from_u64(seed));
        let mut board = Board {
            tableau: vec![vec![]; kind.columns()],
            foundations: vec![vec![]; if kind == GameKind::Spider { 8 } else { 4 }],
            cells: vec![vec![]; if kind == GameKind::FreeCell { 4 } else { 0 }],
            stock: vec![],
            waste: vec![],
            score: if kind == GameKind::Spider { 500 } else { 0 },
            moves: 0,
            is_game_won: false,
        };
        match kind {
            GameKind::Klondike => {
                for (i, pile) in board.tableau.iter_mut().enumerate() {
                    for _ in 0..=i {
                        pile.push(deck.pop().unwrap());
                    }
                    pile.last_mut().unwrap().is_face_down = false;
                }
            }
            GameKind::FreeCell => {
                for i in 0..52 {
                    let mut card = deck.pop().unwrap();
                    card.is_face_down = false;
                    board.tableau[i % 8].push(card);
                }
            }
            GameKind::Spider => {
                for i in 0..54 {
                    board.tableau[i % 10].push(deck.pop().unwrap());
                }
                for pile in &mut board.tableau {
                    pile.last_mut().unwrap().is_face_down = false;
                }
            }
        }
        board.stock = deck;
        Self {
            kind,
            draw_mode,
            difficulty,
            seed,
            initial_board: board.clone(),
            board,
            history: vec![],
            elapsed_seconds: 0,
            victory_recorded: false,
        }
    }
    pub fn can_select(&self, pos: CardPosition) -> bool {
        let Some(pile) = self.board.pile(pos.pile) else {
            return false;
        };
        let Some(card) = pile.get(pos.index) else {
            return false;
        };
        if card.is_face_down || self.board.is_game_won {
            return false;
        }
        if !matches!(pos.pile, Pile::Tableau(_)) {
            return pos.pile != Pile::Stock
                && pos.index + 1 == pile.len()
                && self.kind != GameKind::Spider;
        }
        pile[pos.index..].windows(2).all(|pair| {
            !pair[1].is_face_down
                && pair[0].rank == pair[1].rank + 1
                && if self.kind == GameKind::Spider {
                    pair[0].suit == pair[1].suit
                } else {
                    pair[0].suit.is_red() != pair[1].suit.is_red()
                }
        })
    }
    pub fn can_move(&self, from: CardPosition, to: Pile) -> bool {
        if from.pile == to || !self.can_select(from) {
            return false;
        }
        let Some(dest) = self.board.pile(to) else {
            return false;
        };
        let source = self.board.pile(from.pile).unwrap();
        let card = source[from.index];
        let count = source.len() - from.index;
        match self.kind {
            GameKind::Klondike => super::pages::klondike_solitaire_view_model::check_move(
                &self.board,
                from,
                to,
                card,
                count,
                dest,
            ),
            GameKind::FreeCell => super::pages::free_cell_solitaire_view_model::check_move(
                &self.board,
                from,
                to,
                card,
                count,
                dest,
            ),
            GameKind::Spider => {
                super::pages::spider_solitaire_view_model::check_move(from, to, card, dest)
            }
        }
    }
    pub fn check_and_move_card(&mut self, from: CardPosition, to: Pile) -> bool {
        if !self.can_move(from, to) {
            return false;
        }
        self.record_moves();
        let run = self
            .board
            .pile_mut(from.pile)
            .unwrap()
            .split_off(from.index);
        self.board.pile_mut(to).unwrap().extend(run);
        self.board.score += match self.kind {
            GameKind::Spider => -1,
            _ if matches!(to, Pile::Foundation(_)) => 10,
            GameKind::Klondike if matches!(from.pile, Pile::Foundation(_)) => -15,
            GameKind::Klondike if from.pile == Pile::Waste => 5,
            _ => 0,
        };
        self.board.moves += 1;
        self.reveal_and_check();
        true
    }
    pub fn turn_stock(&mut self) -> bool {
        if self.board.is_game_won {
            return false;
        }
        match self.kind {
            GameKind::FreeCell => false,
            GameKind::Klondike => {
                if self.board.stock.is_empty() && self.board.waste.is_empty() {
                    return false;
                }
                self.record_moves();
                if self.board.stock.is_empty() {
                    while let Some(mut card) = self.board.waste.pop() {
                        card.is_face_down = true;
                        self.board.stock.push(card);
                    }
                } else {
                    for _ in 0..self.draw_mode.count() {
                        if let Some(mut card) = self.board.stock.pop() {
                            card.is_face_down = false;
                            self.board.waste.push(card);
                        }
                    }
                }
                true
            }
            GameKind::Spider => {
                if self.board.stock.len() < 10 || self.board.tableau.iter().any(Vec::is_empty) {
                    return false;
                }
                self.record_moves();
                for pile in &mut self.board.tableau {
                    let mut card = self.board.stock.pop().unwrap();
                    card.is_face_down = false;
                    pile.push(card);
                }
                self.reveal_and_check();
                true
            }
        }
    }
    fn record_moves(&mut self) {
        self.history.push(self.board.clone());
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }
    pub fn undo_move(&mut self) -> bool {
        if let Some(board) = self.history.pop() {
            self.board = board;
            true
        } else {
            false
        }
    }
    pub fn restart(&mut self) {
        self.board = self.initial_board.clone();
        self.elapsed_seconds = 0;
        self.history.clear();
    }
    fn reveal_and_check(&mut self) {
        for pile in &mut self.board.tableau {
            if let Some(card) = pile.last_mut() {
                card.is_face_down = false;
            }
        }
        if self.kind == GameKind::Spider {
            super::pages::spider_solitaire_view_model::check_each_tableau(&mut self.board);
        }
        let count: usize = self.board.foundations.iter().map(Vec::len).sum();
        self.board.is_game_won = count
            == if self.kind == GameKind::Spider {
                104
            } else {
                52
            };
    }
    pub fn sources(&self) -> Vec<CardPosition> {
        let piles = (0..self.board.tableau.len())
            .map(Pile::Tableau)
            .chain((0..self.board.cells.len()).map(Pile::Cell))
            .chain(std::iter::once(Pile::Waste));
        piles
            .flat_map(|pile| {
                (0..self.board.pile(pile).unwrap().len())
                    .map(move |index| CardPosition { pile, index })
            })
            .filter(|&p| self.can_select(p))
            .collect()
    }
    pub fn try_move_card_to_appropriate_foundation(&mut self, from: CardPosition) -> bool {
        if self.kind == GameKind::Spider {
            return false;
        }
        (0..4).any(|i| self.check_and_move_card(from, Pile::Foundation(i)))
    }
    pub fn auto_move(&mut self) -> usize {
        let mut count = 0;
        loop {
            let next = self.sources().into_iter().find_map(|from| {
                (0..4)
                    .map(Pile::Foundation)
                    .find(|&to| self.can_move(from, to))
                    .map(|to| CardMove { from, to })
            });
            let Some(m) = next else { break };
            if !self.check_and_move_card(m.from, m.to) {
                break;
            }
            count += 1;
        }
        count
    }
    pub fn hint(&self) -> Option<CardMove> {
        let destinations = (0..self.board.foundations.len())
            .map(Pile::Foundation)
            .chain((0..self.board.tableau.len()).map(Pile::Tableau))
            .chain((0..self.board.cells.len()).map(Pile::Cell))
            .collect::<Vec<_>>();
        self.sources().into_iter().find_map(|from| {
            destinations
                .iter()
                .copied()
                .find(|&to| {
                    // Moving a whole column to an empty column does not uncover anything.
                    !(from.index == 0
                        && matches!(from.pile, Pile::Tableau(_))
                        && matches!(to, Pile::Tableau(_))
                        && self.board.pile(to).unwrap().is_empty())
                        && self.can_move(from, to)
                })
                .map(|to| CardMove { from, to })
        })
    }
    /// Reject corrupt or incompatible saves before their indices reach the UI.
    pub fn validate(&self) -> Result<(), String> {
        self.validate_board(&self.board)?;
        self.validate_board(&self.initial_board)?;
        for b in &self.history {
            self.validate_board(b)?;
        }
        Ok(())
    }
    fn validate_board(&self, b: &Board) -> Result<(), String> {
        let expected = if self.kind == GameKind::Spider {
            104
        } else {
            52
        };
        if b.tableau.len() != self.kind.columns()
            || b.foundations.len() != if self.kind == GameKind::Spider { 8 } else { 4 }
            || b.cells.len()
                != if self.kind == GameKind::FreeCell {
                    4
                } else {
                    0
                }
        {
            return Err("Invalid pile layout".into());
        }
        let cards: Vec<_> = b.cards().collect();
        let mut ids = cards.iter().map(|c| c.id).collect::<Vec<_>>();
        ids.sort_unstable();
        if ids != (0..expected).collect::<Vec<_>>()
            || cards.iter().any(|c| !(1..=13).contains(&c.rank))
        {
            return Err("Invalid deck".into());
        }
        let suits = if self.kind == GameKind::Spider {
            self.difficulty.suits()
        } else {
            &CardSuit::ALL
        };
        for suit in CardSuit::ALL {
            for rank in 1..=13 {
                let count = cards
                    .iter()
                    .filter(|c| c.suit == suit && c.rank == rank)
                    .count();
                let required = if suits.contains(&suit) {
                    expected as usize / (suits.len() * 13)
                } else {
                    0
                };
                if count != required {
                    return Err("Invalid card distribution".into());
                }
            }
        }
        if b.cells.iter().any(|p| p.len() > 1) {
            return Err("Invalid free cell".into());
        }
        if b.stock.iter().any(|c| !c.is_face_down) || b.waste.iter().any(|c| c.is_face_down) {
            return Err("Invalid stock or waste".into());
        }
        if self.kind == GameKind::FreeCell
            && (!b.stock.is_empty() || !b.waste.is_empty() || b.cards().any(|c| c.is_face_down))
        {
            return Err("Invalid FreeCell layout".into());
        }
        for (i, pile) in b.foundations.iter().enumerate() {
            if self.kind == GameKind::Spider {
                if !pile.is_empty()
                    && (pile.len() != 13
                        || pile.iter().enumerate().any(|(j, c)| {
                            c.rank as usize != 13 - j || c.suit != pile[0].suit || c.is_face_down
                        }))
                {
                    return Err("Invalid completed run".into());
                }
            } else if pile.iter().enumerate().any(|(j, c)| {
                c.rank as usize != j + 1 || c.suit != CardSuit::ALL[i] || c.is_face_down
            }) {
                return Err("Invalid foundation".into());
            }
        }
        if b.is_game_won != (b.foundations.iter().map(Vec::len).sum::<usize>() == expected as usize)
        {
            return Err("Invalid victory state".into());
        }
        Ok(())
    }
}
