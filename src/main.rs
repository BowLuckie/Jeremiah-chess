#![allow(clippy::identity_op)] // i find using c * C is more idiomatic even if c is 1
#![allow(clippy::needless_return)] // i always like to use return where possible
#![allow(clippy::cast_possible_truncation)] // the program does lots of casts to index the board
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::wildcard_imports)] // i use these in big match statements
#![allow(clippy::enum_glob_use)]

use crate::{
    board::{
        GameState,
        PromotionState::{self, Complete},
        reset,
    },
    input::{InputState, LoadedSound},
    moves::{
        Colour, Move, Piece,
        PieceKind::{self, Pawn},
    },
};
use board::Board;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

mod ai;
mod board;
mod input;
mod moves;
mod window;

fn main() {
    let board: Arc<Mutex<Board>> = Arc::new(Mutex::new(Board::new()));
    let ready_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let input: Arc<Mutex<InputState>> = Arc::new(Mutex::new(InputState::new()));

    reset(&board, &input);

    let logic_input: Arc<Mutex<InputState>> = Arc::clone(&input);

    let logic_board: Arc<Mutex<Board>> = Arc::clone(&board);
    let window_pointer: Arc<AtomicBool> = Arc::clone(&ready_flag);

    thread::spawn(move || {
        while !window_pointer.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(16));
        }

        logic(&logic_board, &logic_input);
    });

    window::chess_window(&board, &ready_flag, &input);
}

/// unlocks the board and computes a closure on it
fn with_board<T>(board: &Arc<Mutex<Board>>, f: impl FnOnce(&mut Board) -> T) -> T {
    f(&mut board.lock().unwrap())
}

fn logic(board: &Arc<Mutex<Board>>, input: &Arc<Mutex<InputState>>) {
    println!();
    with_board(board, |b| println!("{b}"));

    loop {
        if let Some(mv) = input.lock().unwrap().take_pending() {
            with_board(board, |b| {
                make_move(mv, b);
            });
        }

        with_board(board, |b| {
            if let Complete(square, kind, colour) = b.promotion_state {
                let (row, col) = square;
                b.squares[row as usize][col as usize] = Some(Piece {
                    kind,
                    colour,
                    has_moved: true,
                });
                b.promotion_state = PromotionState::Not;
                post_move(b);
            }
        });

        thread::sleep(Duration::from_millis(16));
    }
}

pub fn make_move(mv: Move, b: &mut Board) {
    if !b.check_move(mv) {
        return;
    }

    let target = b.get_piece(mv.to.0, mv.to.1).is_some();
    b.raw_move(mv);

    let mut castle = false;
    let piece = b.get_piece(mv.to.0, mv.to.1).copied();

    if (mv.to.1 - mv.from.1).abs() > 1 && piece.is_some_and(|p| p.kind == PieceKind::King) {
        castle = true;
        let rank = mv.to.0;
        let (rook_from, rook_to) = if mv.to.1 == 6 {
            ((rank, 7), (rank, 5))
        } else {
            ((rank, 0), (rank, 3))
        };

        b.raw_move(Move::new(rook_from, rook_to));
    } else if target || piece.is_some_and(|p| p.kind == Pawn) {
        b.halfmove_clock = -1;
        if let Some(piece) = piece
            && piece.kind == Pawn
            && [0, 7].contains(&mv.to.0)
        {
            b.promotion_state = PromotionState::Promoting(mv, piece.colour);
            b.loaded_sound = LoadedSound::Promote;
            return;
        }
    }
    b.loaded_sound = if castle {
        LoadedSound::Castle
    } else if target {
        LoadedSound::Capture
    } else {
        LoadedSound::Normal
    };

    b.last_double = if piece.is_some_and(|p| p.kind == Pawn) && (mv.to.0 - mv.from.0).abs() == 2 {
        Some(mv.to)
    } else {
        None
    };

    let Some(p) = piece else {
        return;
    };
    let dir = match p.colour {
        Colour::White => -1,
        Colour::Black => 1,
    };

    if piece.is_some_and(|p| p.kind == Pawn)
        && b.get_piece(mv.to.0 - dir, mv.to.1)
            .is_some_and(|p| p.kind == Pawn)
    {
        b.squares[(mv.to.0 - dir) as usize][mv.to.1 as usize] = None;
        b.loaded_sound = LoadedSound::Capture;
    }

    post_move(b);
}

fn post_move(b: &mut Board) {
    b.switch_turn();
    b.halfmove_clock += 1;
    let hash = b.position_hash();
    *b.position_history.entry(hash).or_insert(0) += 1;
    b.gamestate = b.get_gamestate(b.to_move);
    if !matches!(b.gamestate, GameState::Playing) {
        b.loaded_sound = LoadedSound::End;
    } else if b.king_in_check(b.to_move) {
        b.loaded_sound = LoadedSound::Check;
    }
    println!("hash: {} count: {}", hash, b.position_history[&hash]);
}
