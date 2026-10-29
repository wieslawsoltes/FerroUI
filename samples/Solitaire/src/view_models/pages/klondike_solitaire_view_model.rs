//! KlondikeSolitaireViewModel.CheckAndMoveCard: fixed-suit foundations,
//! alternating descending tableaus, kings in empty columns.
use crate::{
    models::*,
    view_models::{Board, CardPosition, Pile},
};
pub fn check_move(
    _: &Board,
    from: CardPosition,
    to: Pile,
    card: PlayingCard,
    count: usize,
    dest: &[PlayingCard],
) -> bool {
    if !matches!(
        from.pile,
        Pile::Tableau(_) | Pile::Waste | Pile::Foundation(_)
    ) {
        return false;
    }
    match to {
        Pile::Foundation(i) => {
            count == 1
                && CardSuit::ALL.get(i) == Some(&card.suit)
                && card.rank as usize == dest.len() + 1
        }
        Pile::Tableau(_) => dest.last().map_or(card.rank == 13, |top| {
            !top.is_face_down
                && top.rank == card.rank + 1
                && top.suit.is_red() != card.suit.is_red()
        }),
        _ => false,
    }
}
