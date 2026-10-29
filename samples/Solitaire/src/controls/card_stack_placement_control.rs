//! Retained card containers, like the source behavior's container cache. Only
//! attached positions, depth and face state change when a card moves piles.
use crate::{
    controls::{PlayingCard, tray_container::tray},
    models::*,
    view_models::*,
};
use ferroui::{
    base::{Point, Rect, controls::ResourceDictionary},
    controls::Canvas,
    prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

pub const CARD_WIDTH: f64 = 135.0;
pub const CARD_HEIGHT: f64 = 196.0;
pub struct PlacedCard {
    pub position: CardPosition,
    pub rect: Rect,
    pub depth: i32,
    pub control: Ref<PlayingCard>,
}
#[derive(Clone, Copy)]
pub struct Drag {
    pub from: CardPosition,
    pub start: Point,
    pub current: Point,
}
pub struct CardStackPlacementControl {
    pub canvas: Ref<Canvas>,
    pub cards: RefCell<Vec<PlacedCard>>,
    pub piles: RefCell<Vec<(Pile, Rect)>>,
    pub selected: Cell<Option<CardPosition>>,
    pub drag: Cell<Option<Drag>>,
    pub displayed_board: RefCell<Option<Board>>,
    cache: RefCell<HashMap<u8, Ref<PlayingCard>>>,
    drag_origins: RefCell<HashMap<u8, Point>>,
    kind: Cell<Option<GameKind>>,
    artwork: Ref<ResourceDictionary>,
}
impl CardStackPlacementControl {
    pub fn new(canvas: Ref<Canvas>, artwork: Ref<ResourceDictionary>) -> Rc<Self> {
        Rc::new(Self {
            canvas,
            cards: RefCell::new(vec![]),
            piles: RefCell::new(vec![]),
            selected: Cell::new(None),
            drag: Cell::new(None),
            displayed_board: RefCell::new(None),
            cache: RefCell::new(HashMap::new()),
            drag_origins: RefCell::new(HashMap::new()),
            kind: Cell::new(None),
            artwork,
        })
    }
    pub fn rebuild(&self, game: &CardGameViewModel) {
        self.present(game.kind, game.draw_mode, &game.board, true);
    }
    pub fn control_by_id(&self, id: u8) -> Option<Ref<PlayingCard>> {
        self.cache.borrow().get(&id).cloned()
    }
    pub fn present(&self, kind: GameKind, draw_mode: DrawMode, board: &Board, animate: bool) {
        self.drag.set(None);
        self.selected.set(None);
        self.drag_origins.borrow_mut().clear();
        let step = 1460.0 / kind.columns() as f64;
        let start_x = 20.0 + (step - CARD_WIDTH) / 2.0;
        if self.kind.replace(Some(kind)) != Some(kind) {
            self.canvas.children().clear();
            self.cache.borrow_mut().clear();
            self.piles.borrow_mut().clear();
            for i in 0..kind.columns() {
                self.add_pile(Pile::Tableau(i), start_x + i as f64 * step, 260.0, "");
            }
            match kind {
                GameKind::Klondike => {
                    self.add_pile(Pile::Stock, start_x, 30.0, "↻");
                    self.add_pile(Pile::Waste, start_x + step, 30.0, "");
                    for i in 0..4 {
                        self.add_pile(
                            Pile::Foundation(i),
                            start_x + (i + 3) as f64 * step,
                            30.0,
                            CardSuit::ALL[i].symbol(),
                        );
                    }
                }
                GameKind::FreeCell => {
                    for i in 0..4 {
                        self.add_pile(Pile::Cell(i), start_x + i as f64 * step, 30.0, "");
                        self.add_pile(
                            Pile::Foundation(i),
                            start_x + (i + 4) as f64 * step,
                            30.0,
                            CardSuit::ALL[i].symbol(),
                        );
                    }
                }
                GameKind::Spider => {
                    self.add_pile(Pile::Stock, start_x, 30.0, "↻");
                    for i in 0..8 {
                        self.add_pile(Pile::Foundation(i), 320.0 + i as f64 * step, 30.0, "");
                    }
                }
            }
        }
        self.cards.borrow_mut().clear();
        for (i, pile) in board.tableau.iter().enumerate() {
            let spacing = (510.0 / pile.len().saturating_sub(1).max(1) as f64).min(38.0);
            for (index, &card) in pile.iter().enumerate() {
                self.place(
                    CardPosition {
                        pile: Pile::Tableau(i),
                        index,
                    },
                    card,
                    start_x + i as f64 * step,
                    260.0 + index as f64 * spacing,
                    index as i32 + 10,
                    animate,
                );
            }
        }
        for (i, pile) in board.cells.iter().enumerate() {
            for (index, &card) in pile.iter().enumerate() {
                self.place(
                    CardPosition {
                        pile: Pile::Cell(i),
                        index,
                    },
                    card,
                    start_x + i as f64 * step,
                    30.0,
                    index as i32 + 10,
                    animate,
                );
            }
        }
        for (index, &card) in board.stock.iter().enumerate() {
            let fan = if kind == GameKind::Spider {
                (index / 10) as f64 * 19.0
            } else {
                0.0
            };
            self.place(
                CardPosition {
                    pile: Pile::Stock,
                    index,
                },
                card,
                start_x + fan,
                30.0,
                index as i32 + 10,
                animate,
            );
        }
        let offset = board.waste.len().saturating_sub(draw_mode.count());
        for (index, &card) in board.waste.iter().enumerate() {
            self.place(
                CardPosition {
                    pile: Pile::Waste,
                    index,
                },
                card,
                start_x + step + index.saturating_sub(offset) as f64 * 27.0,
                30.0,
                index as i32 + 10,
                animate,
            );
        }
        for (i, pile) in board.foundations.iter().enumerate() {
            let x = match kind {
                GameKind::Klondike => start_x + (i + 3) as f64 * step,
                GameKind::FreeCell => start_x + (i + 4) as f64 * step,
                GameKind::Spider => 320.0 + i as f64 * step,
            };
            for (index, &card) in pile.iter().enumerate() {
                self.place(
                    CardPosition {
                        pile: Pile::Foundation(i),
                        index,
                    },
                    card,
                    x,
                    30.0,
                    if kind == GameKind::Spider {
                        (pile.len() - index) as i32 + 10
                    } else {
                        index as i32 + 10
                    },
                    animate,
                );
            }
        }
        *self.displayed_board.borrow_mut() = Some(board.clone());
        self.canvas.set_focusable(true);
    }
    fn add_pile(&self, pile: Pile, x: f64, y: f64, label: &str) {
        let control = tray(label, CARD_WIDTH, CARD_HEIGHT);
        Canvas::set_left(&control, x);
        Canvas::set_top(&control, y);
        self.canvas.children().add(control);
        self.piles.borrow_mut().push((
            pile,
            Rect::new(
                x,
                y,
                CARD_WIDTH,
                if matches!(pile, Pile::Tableau(_)) {
                    740.0
                } else {
                    CARD_HEIGHT
                },
            ),
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn place(
        &self,
        position: CardPosition,
        card: crate::models::PlayingCard,
        x: f64,
        y: f64,
        depth: i32,
        animate: bool,
    ) {
        let existing = self.cache.borrow().get(&card.id).cloned();
        let is_new = existing.is_none();
        let control = existing.unwrap_or_else(|| PlayingCard::new(card, &self.artwork));
        control.update_card(card, &self.artwork);
        control.set_selected(false);
        control.set_motion_enabled(animate && !is_new);
        Canvas::set_left(&control, x);
        Canvas::set_top(&control, y);
        control.set_z_index(depth);
        if is_new {
            self.cache.borrow_mut().insert(card.id, control.clone());
            self.canvas.children().add(control.clone());
        }
        control.set_motion_enabled(true);
        self.cards.borrow_mut().push(PlacedCard {
            position,
            rect: Rect::new(x, y, CARD_WIDTH, CARD_HEIGHT),
            depth,
            control,
        });
    }
    pub fn card_at(&self, point: Point) -> Option<CardPosition> {
        self.cards
            .borrow()
            .iter()
            .filter(|c| {
                Rect::new(
                    Canvas::get_left(&c.control),
                    Canvas::get_top(&c.control),
                    CARD_WIDTH,
                    CARD_HEIGHT,
                )
                .contains(point)
            })
            .max_by_key(|c| c.control.z_index())
            .map(|c| c.position)
    }
    pub fn pile_at(&self, point: Point) -> Option<Pile> {
        self.card_at(point)
            .filter(|p| matches!(p.pile, Pile::Stock | Pile::Waste))
            .map(|p| p.pile)
            .or_else(|| {
                self.piles
                    .borrow()
                    .iter()
                    .find(|(_, r)| r.contains(point))
                    .map(|(p, _)| *p)
            })
    }
    pub fn select(&self, position: Option<CardPosition>) {
        self.selected.set(position);
        for card in self.cards.borrow().iter() {
            card.control.set_selected(
                position.is_some_and(|p| {
                    p.pile == card.position.pile && p.index <= card.position.index
                }),
            );
        }
    }
    pub fn highlight_target(&self, target: Pile) {
        for card in self
            .cards
            .borrow()
            .iter()
            .filter(|c| c.position.pile == target)
        {
            card.control.set_selected(true);
        }
    }
    pub fn begin_drag(&self, from: CardPosition, point: Point) {
        self.drag.set(Some(Drag {
            from,
            start: point,
            current: point,
        }));
        let mut origins = self.drag_origins.borrow_mut();
        origins.clear();
        for card in self
            .cards
            .borrow()
            .iter()
            .filter(|c| c.position.pile == from.pile && c.position.index >= from.index)
        {
            let origin = Point::new(
                Canvas::get_left(&card.control),
                Canvas::get_top(&card.control),
            );
            origins.insert(card.control.card_id(), origin);
            card.control.set_motion_enabled(false);
            Canvas::set_left(&card.control, origin.x);
            Canvas::set_top(&card.control, origin.y);
        }
    }
    pub fn move_drag(&self, point: Point) {
        let Some(mut drag) = self.drag.get() else {
            return;
        };
        drag.current = point;
        self.drag.set(Some(drag));
        for card in
            self.cards.borrow().iter().filter(|c| {
                c.position.pile == drag.from.pile && c.position.index >= drag.from.index
            })
        {
            if let Some(origin) = self.drag_origins.borrow().get(&card.control.card_id()) {
                Canvas::set_left(&card.control, origin.x + point.x - drag.start.x);
                Canvas::set_top(&card.control, origin.y + point.y - drag.start.y);
                card.control.set_z_index(1000 + card.position.index as i32);
            }
        }
    }
    pub fn restore_positions(&self) {
        self.drag.set(None);
        self.drag_origins.borrow_mut().clear();
        for card in self.cards.borrow().iter() {
            card.control.set_motion_enabled(true);
            Canvas::set_left(&card.control, card.rect.x);
            Canvas::set_top(&card.control, card.rect.y);
            card.control.set_z_index(card.depth);
        }
    }
}
