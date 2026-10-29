use ferro_solitaire::{models::*, view_models::*};
fn game(kind: GameKind) -> CardGameViewModel {
    CardGameViewModel::new(kind, DrawMode::DrawOne, Difficulty::Easy, 42)
}
fn card(suit: CardSuit, rank: u8) -> PlayingCard {
    PlayingCard {
        id: rank,
        suit,
        rank,
        is_face_down: false,
    }
}
fn pos(column: usize, index: usize) -> CardPosition {
    CardPosition {
        pile: Pile::Tableau(column),
        index,
    }
}
fn empty(kind: GameKind) -> CardGameViewModel {
    let mut g = game(kind);
    for p in &mut g.board.tableau {
        p.clear();
    }
    g.board.stock.clear();
    g
}

#[test]
fn deals_match_all_three_source_layouts_and_preserve_the_deck() {
    for seed in 0..32 {
        for kind in GameKind::ALL {
            for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
                let g = CardGameViewModel::new(kind, DrawMode::DrawOne, difficulty, seed);
                g.validate().unwrap();
                match kind {
                    GameKind::Klondike => {
                        assert_eq!(
                            g.board.tableau.iter().map(Vec::len).collect::<Vec<_>>(),
                            [1, 2, 3, 4, 5, 6, 7]
                        );
                        assert_eq!(g.board.stock.len(), 24);
                    }
                    GameKind::FreeCell => {
                        assert_eq!(
                            g.board.tableau.iter().map(Vec::len).collect::<Vec<_>>(),
                            [7, 7, 7, 7, 6, 6, 6, 6]
                        );
                        assert_eq!(g.board.cells.len(), 4);
                    }
                    GameKind::Spider => {
                        assert_eq!(
                            g.board.tableau.iter().map(Vec::len).collect::<Vec<_>>(),
                            [6, 6, 6, 6, 5, 5, 5, 5, 5, 5]
                        );
                        assert_eq!(g.board.stock.len(), 50);
                        assert_eq!(g.board.score, 500);
                    }
                }
            }
        }
    }
}
#[test]
fn a_seed_reproduces_the_same_deal() {
    assert_eq!(
        game(GameKind::Klondike).board,
        game(GameKind::Klondike).board
    );
}
#[test]
fn klondike_draw_three_recycle_and_undo_preserve_order_and_faces() {
    let mut g = game(GameKind::Klondike);
    g.draw_mode = DrawMode::DrawThree;
    let initial = g.board.clone();
    for n in 1..=8 {
        assert!(g.turn_stock());
        assert_eq!(g.board.waste.len(), 3 * n);
        assert!(g.board.waste.iter().all(|c| !c.is_face_down));
    }
    assert!(g.turn_stock());
    assert_eq!(g.board.stock, initial.stock);
    for _ in 0..9 {
        assert!(g.undo_move());
    }
    assert_eq!(g.board, initial);
    assert!(!g.undo_move());
}
#[test]
fn klondike_only_kings_fill_empty_columns_and_colours_alternate() {
    let mut g = empty(GameKind::Klondike);
    g.board.tableau[0] = vec![card(CardSuit::Hearts, 12)];
    assert!(!g.can_move(pos(0, 0), Pile::Tableau(1)));
    g.board.tableau[1] = vec![card(CardSuit::Spades, 13)];
    assert!(g.can_move(pos(0, 0), Pile::Tableau(1)));
    g.board.tableau[1][0].suit = CardSuit::Diamonds;
    assert!(!g.can_move(pos(0, 0), Pile::Tableau(1)));
}
#[test]
fn foundations_require_their_suit_ascending_and_exactly_one_card() {
    for kind in [GameKind::Klondike, GameKind::FreeCell] {
        let mut g = empty(kind);
        g.board.tableau[0] = vec![card(CardSuit::Hearts, 1)];
        assert!(g.can_move(pos(0, 0), Pile::Foundation(0)));
        assert!(!g.can_move(pos(0, 0), Pile::Foundation(1)));
        g.board.tableau[0].push(card(CardSuit::Clubs, 2));
        assert!(!g.can_move(pos(0, 0), Pile::Foundation(0)));
    }
}
#[test]
fn moving_a_run_reveals_a_hidden_card_and_undo_restores_its_exact_face() {
    let mut g = empty(GameKind::Klondike);
    let mut hidden = card(CardSuit::Clubs, 2);
    hidden.is_face_down = true;
    g.board.tableau[0] = vec![
        hidden,
        card(CardSuit::Hearts, 12),
        card(CardSuit::Clubs, 11),
    ];
    g.board.tableau[1] = vec![card(CardSuit::Spades, 13)];
    let before = g.board.clone();
    assert!(g.check_and_move_card(pos(0, 1), Pile::Tableau(1)));
    assert!(!g.board.tableau[0][0].is_face_down);
    assert_eq!(g.board.tableau[1].len(), 3);
    assert!(g.undo_move());
    assert_eq!(g.board, before);
}
#[test]
fn invalid_or_hidden_or_buried_sources_cannot_move() {
    let g = game(GameKind::Klondike);
    for from in [
        pos(6, 0),
        pos(99, 0),
        pos(0, 99),
        CardPosition {
            pile: Pile::Stock,
            index: 23,
        },
    ] {
        assert!(!g.can_move(from, Pile::Tableau(0)));
    }
    assert!(!g.can_move(pos(0, 0), Pile::Tableau(99)));
}
#[test]
fn freecell_preserves_source_capacity_and_validates_every_colour_in_a_run() {
    let mut g = empty(GameKind::FreeCell);
    g.board.tableau[0] = vec![
        card(CardSuit::Hearts, 6),
        card(CardSuit::Clubs, 5),
        card(CardSuit::Diamonds, 4),
    ];
    assert!(g.can_move(pos(0, 0), Pile::Tableau(1)));
    for i in 0..3 {
        g.board.cells[i] = vec![card(CardSuit::Spades, 10)];
    }
    assert!(!g.can_move(pos(0, 0), Pile::Tableau(1)));
    g.board.cells[2].clear();
    assert!(g.can_move(pos(0, 0), Pile::Tableau(1)));
    g.board.tableau[0][1].suit = CardSuit::Diamonds;
    assert!(!g.can_select(pos(0, 0)));
}
#[test]
fn freecell_accepts_one_card_in_empty_cells_and_any_rank_in_empty_columns() {
    let mut g = empty(GameKind::FreeCell);
    g.board.tableau[0] = vec![card(CardSuit::Hearts, 4)];
    assert!(g.check_and_move_card(pos(0, 0), Pile::Cell(0)));
    assert!(g.can_move(
        CardPosition {
            pile: Pile::Cell(0),
            index: 0
        },
        Pile::Tableau(1)
    ));
    g.board.tableau[0] = vec![card(CardSuit::Clubs, 5)];
    assert!(!g.can_move(pos(0, 0), Pile::Cell(0)));
}
#[test]
fn spider_builds_across_suits_but_moves_only_suited_runs() {
    let mut g = empty(GameKind::Spider);
    g.board.tableau[0] = vec![card(CardSuit::Hearts, 8), card(CardSuit::Hearts, 7)];
    g.board.tableau[1] = vec![card(CardSuit::Clubs, 9)];
    assert!(g.can_move(pos(0, 0), Pile::Tableau(1)));
    g.board.tableau[0][1].suit = CardSuit::Clubs;
    assert!(!g.can_move(pos(0, 0), Pile::Tableau(1)));
}
#[test]
fn spider_deal_requires_nonempty_columns_and_undo_restores_all_ten() {
    let mut g = game(GameKind::Spider);
    let before = g.board.clone();
    assert!(g.turn_stock());
    assert_eq!(g.board.stock.len(), 40);
    g.validate().unwrap();
    assert!(g.undo_move());
    assert_eq!(g.board, before);
    g.board.tableau[3].clear();
    assert!(!g.turn_stock());
    assert_eq!(g.board.stock.len(), 50);
}
#[test]
fn completing_spider_run_collects_it_and_undo_restores_run_score_and_reveal() {
    let mut g = empty(GameKind::Spider);
    g.board.tableau[0] = vec![card(CardSuit::Hearts, 1)];
    g.board.tableau[1] = (2..=13).rev().map(|r| card(CardSuit::Hearts, r)).collect();
    let before = g.board.clone();
    assert!(g.check_and_move_card(pos(0, 0), Pile::Tableau(1)));
    assert_eq!(g.board.foundations[0].len(), 13);
    assert!(g.board.tableau[1].is_empty());
    assert_eq!(g.board.score, 599);
    assert!(g.undo_move());
    assert_eq!(g.board, before);
}
#[test]
fn auto_move_finds_newly_exposed_cards_until_no_more_foundation_moves_remain() {
    let mut g = empty(GameKind::Klondike);
    g.board.tableau[0] = vec![card(CardSuit::Hearts, 2), card(CardSuit::Hearts, 1)];
    assert_eq!(g.auto_move(), 2);
    assert_eq!(g.board.foundations[0].len(), 2);
    assert_eq!(g.board.score, 20);
    assert_eq!(g.auto_move(), 0);
}
#[test]
fn last_ace_wins_and_undo_leaves_victory() {
    let mut g = empty(GameKind::Klondike);
    for (i, s) in CardSuit::ALL.into_iter().enumerate() {
        g.board.foundations[i] = (1..=13).map(|r| card(s, r)).collect();
    }
    let king = g.board.foundations[3].pop().unwrap();
    g.board.tableau[0].push(king);
    assert!(g.check_and_move_card(pos(0, 0), Pile::Foundation(3)));
    assert!(g.board.is_game_won);
    assert!(g.undo_move());
    assert!(!g.board.is_game_won);
}
#[test]
fn legal_play_sequences_conserve_cards_and_undo_exactly() {
    for kind in GameKind::ALL {
        for seed in 0..12 {
            let mut g = CardGameViewModel::new(kind, DrawMode::DrawThree, Difficulty::Hard, seed);
            let initial = g.board.clone();
            let mut snapshots = vec![];
            for _ in 0..150 {
                let before = g.board.clone();
                let moved = if let Some(hint) = g.hint() {
                    g.check_and_move_card(hint.from, hint.to)
                } else {
                    g.turn_stock()
                };
                if !moved {
                    break;
                }
                snapshots.push(before);
                g.validate().unwrap();
            }
            for before in snapshots.into_iter().rev() {
                assert!(g.undo_move());
                assert_eq!(g.board, before);
            }
            assert_eq!(g.board, initial);
        }
    }
}
#[test]
fn restart_restores_original_deal_and_clears_history_and_clock() {
    let mut g = game(GameKind::Klondike);
    let before = g.board.clone();
    g.turn_stock();
    g.elapsed_seconds = 90;
    g.restart();
    assert_eq!(g.board, before);
    assert_eq!(g.elapsed_seconds, 0);
    assert!(!g.can_undo());
}
