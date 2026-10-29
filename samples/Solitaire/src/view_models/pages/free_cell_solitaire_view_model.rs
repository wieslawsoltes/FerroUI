//! FreeCellSolitaireViewModel.CheckAndMoveCard. The source limits a moving run
//! to one plus the number of free cells; that capacity is preserved here.
use crate::{
    models::*,
    view_models::{Board, CardPosition, Pile},
};
pub fn check_move(
    board: &Board,
    from: CardPosition,
    to: Pile,
    card: PlayingCard,
    count: usize,
    dest: &[PlayingCard],
) -> bool {
    if !matches!(from.pile, Pile::Tableau(_) | Pile::Cell(_)) {
        return false;
    }
    if count > board.cells.iter().filter(|c| c.is_empty()).count() + 1 {
        return false;
    }
    match to {
        Pile::Foundation(i) => {
            count == 1
                && CardSuit::ALL.get(i) == Some(&card.suit)
                && card.rank as usize == dest.len() + 1
        }
        Pile::Cell(_) => count == 1 && dest.is_empty(),
        Pile::Tableau(_) => dest
            .last()
            .is_none_or(|top| top.rank == card.rank + 1 && top.suit.is_red() != card.suit.is_red()),
        _ => false,
    }
}
