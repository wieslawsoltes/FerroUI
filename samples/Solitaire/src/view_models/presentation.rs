//! Source animation timing expressed as presentation-only board snapshots.
//! Rules, saves and undo always hold the committed board, never a partial deal.
use crate::{
    models::*,
    view_models::{Board, CardGameViewModel, Pile},
};
use std::collections::VecDeque;

pub const CARD_TRANSITION_MS: u64 = 350;
#[derive(Clone, Debug)]
pub struct PresentationFrame {
    pub delay_ms: u64,
    pub board: Board,
}
pub struct Presentation {
    pub initial: Board,
    pub frames: VecDeque<PresentationFrame>,
}
impl Presentation {
    pub fn deal(game: &CardGameViewModel) -> Self {
        let final_board = &game.board;
        let mut board = final_board.clone();
        let mut deck: Vec<_> = board.cards().copied().collect();
        for card in &mut deck {
            card.is_face_down = true;
        }
        for pile in board
            .tableau
            .iter_mut()
            .chain(&mut board.foundations)
            .chain(&mut board.cells)
        {
            pile.clear();
        }
        board.stock.clear();
        board.waste.clear();
        let home = match game.kind {
            GameKind::Klondike => Pile::Stock,
            GameKind::FreeCell => Pile::Cell(0),
            GameKind::Spider => Pile::Foundation(0),
        };
        *board.pile_mut(home).unwrap() = deck;
        let mut result = Self {
            initial: board.clone(),
            frames: VecDeque::new(),
        };
        let mut destinations = vec![];
        if game.kind == GameKind::Klondike {
            for (i, pile) in final_board.tableau.iter().enumerate() {
                for &card in pile {
                    destinations.push((Pile::Tableau(i), card));
                }
            }
        } else {
            for row in 0..7 {
                for (i, pile) in final_board.tableau.iter().enumerate() {
                    if let Some(&card) = pile.get(row) {
                        destinations.push((Pile::Tableau(i), card));
                    }
                }
            }
        }
        for (index, (to, mut card)) in destinations.into_iter().enumerate() {
            let source = board.pile_mut(home).unwrap();
            source.remove(source.iter().position(|c| c.id == card.id).unwrap());
            if game.kind == GameKind::Spider {
                card.is_face_down = true;
            }
            board.pile_mut(to).unwrap().push(card);
            result.push(
                if index == 0 {
                    if game.kind == GameKind::Spider {
                        1000
                    } else {
                        600
                    }
                } else if game.kind == GameKind::Spider {
                    50
                } else {
                    75
                },
                &board,
            );
        }
        if game.kind == GameKind::Spider {
            for pile in &mut board.tableau {
                pile.last_mut().unwrap().is_face_down = false;
            }
            result.push(50, &board);
            for &card in &final_board.stock {
                let source = board.pile_mut(home).unwrap();
                source.remove(source.iter().position(|c| c.id == card.id).unwrap());
                board.stock.push(card);
                result.push(50, &board);
            }
        }
        // Klondike's remainder already occupies the stock, but must regain its
        // exact draw order. This also puts all final counters/flags in the view.
        result.push(CARD_TRANSITION_MS, final_board);
        result
    }
    pub fn stock(game: &CardGameViewModel, before: Board) -> Self {
        let mut result = Self {
            initial: before.clone(),
            frames: VecDeque::new(),
        };
        if game.kind == GameKind::Klondike {
            let mut board = before;
            if board.stock.is_empty() {
                let mut index = 0;
                while !board.waste.is_empty() {
                    let mut card = board.waste.remove(0);
                    card.is_face_down = true;
                    board.stock.insert(0, card);
                    result.push(if index == 0 { 0 } else { 175 }, &board);
                    index += 1;
                }
            } else {
                for index in 0..game.draw_mode.count() {
                    if let Some(mut card) = board.stock.pop() {
                        card.is_face_down = false;
                        board.waste.push(card);
                        result.push(if index == 0 { 0 } else { 175 }, &board);
                    }
                }
            }
        } else {
            result.push(0, &game.board);
        }
        result.push(CARD_TRANSITION_MS, &game.board);
        result
    }
    pub fn auto_move(game: &mut CardGameViewModel) -> Option<Self> {
        let mut result = Self {
            initial: game.board.clone(),
            frames: VecDeque::new(),
        };
        loop {
            let next = game
                .sources()
                .into_iter()
                .find(|&from| (0..4).any(|i| game.can_move(from, Pile::Foundation(i))));
            let Some(from) = next else { break };
            if !game.try_move_card_to_appropriate_foundation(from) {
                break;
            }
            result.push(if result.frames.is_empty() { 0 } else { 75 }, &game.board);
        }
        if result.frames.is_empty() {
            return None;
        }
        result.push(CARD_TRANSITION_MS, &game.board);
        Some(result)
    }
    fn push(&mut self, delay_ms: u64, board: &Board) {
        self.frames.push_back(PresentationFrame {
            delay_ms,
            board: board.clone(),
        });
    }
}
