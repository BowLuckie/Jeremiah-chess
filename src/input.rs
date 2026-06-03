use std::{
    ops::ControlFlow,
    sync::{Arc, Mutex},
};

use raylib::RaylibHandle;

use crate::{
    board::{
        Board,
        PromotionState::{Complete, Promoting},
    },
    moves::{Coordinate, Move, in_bounds_point, promotion_click},
    window::TILE_SIZE,
};

#[derive(Debug, Clone)]
pub struct InputState {
    pub selected: Option<Coordinate>,
    pending_move: Option<Move>,
    pub legal_moves: Vec<Move>,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadedSound {
    None,
    Normal,
    Capture,
    Start,
    End,
    Check,
    Promote,
    Castle,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            selected: None,
            pending_move: None,
            legal_moves: vec![],
        }
    }

    pub fn take_pending(&mut self) -> Option<Move> {
        self.pending_move.take()
    }

    pub fn push_pending(&mut self, mv: Option<Move>) {
        self.pending_move = mv;
    }
}

pub fn handle_click(board: &Arc<Mutex<Board>>, input: &Arc<Mutex<InputState>>, rl: &RaylibHandle) {
    let col = (rl.get_mouse_x() / TILE_SIZE) as i8;
    let row = (rl.get_mouse_y() / TILE_SIZE) as i8;
    if let ControlFlow::Break(_) = click_control(board, input, col, row) {
        return;
    }
}

pub fn click_control(
    board: &Arc<Mutex<Board>>,
    input: &Arc<Mutex<InputState>>,
    col: i8,
    row: i8,
) -> ControlFlow<()> {
    let new = (row, col);
    if !in_bounds_point(new) {
        return ControlFlow::Break(());
    }
    let input_state = input.lock().unwrap();
    let selected_old = input_state.selected;
    drop(input_state);
    let (legal_moves, pending) = {
        let mut board_guard = board.lock().unwrap();
        if let Promoting(mv, colour) = board_guard.promotion_state
            && let Some(p) = promotion_click(new, board_guard.promotion_state)
        {
            board_guard.promotion_state = Complete(mv.to, p, colour);
            return ControlFlow::Break(());
        }
        match selected_old {
            Some(old) if old == new => (vec![], None),
            Some(old) => {
                let moves = board_guard.get_moves(row, col, false);
                (moves, Some(Move::new(old, new)))
            }
            None => (board_guard.get_moves(row, col, false), None),
        }
    };
    let mut input_state = input.lock().unwrap();

    match selected_old {
        Some(old) if old == new => {
            input_state.selected = None;
            input_state.legal_moves.clear();
        }
        _ => {
            input_state.selected = Some(new);
            input_state.legal_moves = legal_moves;
            if let Some(mv) = pending {
                input_state.push_pending(Some(mv));
            }
        }
    }
    ControlFlow::Continue(())
}

pub fn clear_selection(input: &Arc<Mutex<InputState>>) {
    let mut inp = input.lock().unwrap();
    inp.selected = None;
    inp.legal_moves.clear();
}
