use ferro_solitaire::{
    models::*,
    view_models::{presentation::*, *},
};
fn assert_deck(board: &Board, count: usize) {
    let mut ids: Vec<_> = board.cards().map(|c| c.id).collect();
    ids.sort_unstable();
    assert_eq!(ids, (0..count as u8).collect::<Vec<_>>());
}
#[test]
fn staged_deals_keep_every_card_and_end_at_the_committed_board() {
    for kind in GameKind::ALL {
        let game = CardGameViewModel::new(kind, DrawMode::DrawOne, Difficulty::Hard, 42);
        let sequence = Presentation::deal(&game);
        let count = if kind == GameKind::Spider { 104 } else { 52 };
        assert_deck(&sequence.initial, count);
        assert!(sequence.initial.tableau.iter().all(Vec::is_empty));
        assert!(sequence.initial.cards().all(|c| c.is_face_down));
        assert_eq!(
            sequence.frames[0].delay_ms,
            if kind == GameKind::Spider { 1000 } else { 600 }
        );
        let dealt = if kind == GameKind::Klondike {
            28
        } else if kind == GameKind::FreeCell {
            52
        } else {
            54
        };
        for (i, frame) in sequence.frames.iter().enumerate() {
            assert_deck(&frame.board, count);
            if i < dealt {
                assert_eq!(
                    frame.board.tableau.iter().map(Vec::len).sum::<usize>(),
                    i + 1
                );
            }
            if i > 0 && i < dealt {
                assert_eq!(
                    frame.delay_ms,
                    if kind == GameKind::Spider { 50 } else { 75 }
                );
            }
        }
        assert_eq!(sequence.frames.back().unwrap().board, game.board);
        assert_eq!(sequence.frames.back().unwrap().delay_ms, CARD_TRANSITION_MS);
        game.validate().unwrap();
    }
}
#[test]
fn draw_and_recycle_steps_match_the_source_timing_and_undo_remains_atomic() {
    let mut game = CardGameViewModel::new(
        GameKind::Klondike,
        DrawMode::DrawThree,
        Difficulty::Easy,
        42,
    );
    let initial = game.board.clone();
    game.turn_stock();
    let sequence = Presentation::stock(&game, initial.clone());
    assert_eq!(
        sequence
            .frames
            .iter()
            .map(|f| f.delay_ms)
            .collect::<Vec<_>>(),
        [0, 175, 175, 350]
    );
    assert_eq!(
        sequence
            .frames
            .iter()
            .take(3)
            .map(|f| f.board.waste.len())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(game.undo_move());
    assert_eq!(game.board, initial);
    for _ in 0..8 {
        game.turn_stock();
    }
    let before = game.board.clone();
    game.turn_stock();
    let sequence = Presentation::stock(&game, before.clone());
    for (i, frame) in sequence.frames.iter().take(24).enumerate() {
        assert_eq!(frame.board.stock.len(), i + 1);
        assert_deck(&frame.board, 52);
        assert!(frame.board.stock.iter().all(|c| c.is_face_down));
    }
    assert_eq!(sequence.frames.back().unwrap().board, game.board);
    assert!(game.undo_move());
    assert_eq!(game.board, before);
}
#[test]
fn auto_move_presentation_commits_rules_once_and_keeps_individual_undo_steps() {
    let mut game =
        CardGameViewModel::new(GameKind::FreeCell, DrawMode::DrawOne, Difficulty::Easy, 42);
    let before = game.board.clone();
    let mut expected = game.clone();
    let moved = expected.auto_move();
    assert!(moved > 0);
    let sequence = Presentation::auto_move(&mut game).unwrap();
    assert_eq!(game.board, expected.board);
    assert_eq!(sequence.initial, before);
    assert_eq!(sequence.frames.len(), moved + 1);
    assert!(
        sequence
            .frames
            .iter()
            .skip(1)
            .take(moved - 1)
            .all(|f| f.delay_ms == 75)
    );
    for _ in 0..moved {
        assert!(game.undo_move());
    }
    assert_eq!(game.board, before);
}
