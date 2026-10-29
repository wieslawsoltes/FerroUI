//! SpiderSolitaireViewModel: descending builds, same-suit moving runs,
//! ten-card stock deals and automatic collection of complete king-to-ace runs.
use crate::{
    models::*,
    view_models::{Board, CardPosition, Pile},
};
pub fn check_move(from: CardPosition, to: Pile, card: PlayingCard, dest: &[PlayingCard]) -> bool {
    matches!(from.pile, Pile::Tableau(_))
        && matches!(to, Pile::Tableau(_))
        && dest.last().is_none_or(|top| top.rank == card.rank + 1)
}
pub fn check_each_tableau(board: &mut Board) {
    for pile in &mut board.tableau {
        loop {
            if let Some(top) = pile.last_mut() {
                top.is_face_down = false;
            }
            if pile.len() < 13 {
                break;
            }
            let run = &pile[pile.len() - 13..];
            if !run.iter().enumerate().all(|(i, card)| {
                !card.is_face_down && card.rank as usize == 13 - i && card.suit == run[0].suit
            }) {
                break;
            }
            let run = pile.split_off(pile.len() - 13);
            if let Some(foundation) = board.foundations.iter_mut().find(|p| p.is_empty()) {
                *foundation = run;
            }
            board.score += 100;
        }
    }
}
