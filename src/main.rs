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
    ai::find_best,
    board::{
        GameState,
        PromotionState::{self, Complete},
        reset,
    },
    input::{InputState, LoadedSound},
    moves::{
        Colour::{self, Black},
        Move, Piece,
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

pub const AI_ON: bool = true;

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
    let ai_thinking: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    loop {
        if let Some(mv) = input.lock().unwrap().take_pending() {
            with_board(board, |b| {
                make_move(mv, b, false, false);
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

        let should_think = with_board(board, |b| {
            AI_ON && b.to_move == Colour::Black && matches!(b.gamestate, GameState::Playing)
        });

        if should_think && !ai_thinking.load(Ordering::SeqCst) {
            ai_thinking.store(true, Ordering::SeqCst);
            let board_snapshot = with_board(board, |b| b.hashless_clone());
            let board_clone = Arc::clone(board);
            let thinking_flag = Arc::clone(&ai_thinking);
            thread::spawn(move || {
                if let Some(ai_move) = find_best(&board_snapshot, Black) {
                    with_board(&board_clone, |b| {
                        make_move(ai_move, b, false, true);
                        if let PromotionState::Promoting(mv, colour) = b.promotion_state {
                            let (row, col) = mv.to;
                            b.squares[row as usize][col as usize] = Some(Piece {
                                kind: PieceKind::Queen,
                                colour,
                                has_moved: true,
                            });
                            b.promotion_state = PromotionState::Not;
                            post_move(b);
                        }
                    });
                }
                thinking_flag.store(false, Ordering::SeqCst);
            });
        }

        thread::sleep(Duration::from_millis(16));
    }
}

pub fn make_move(mv: Move, b: &mut Board, ai_opponent: bool, ai_source: bool) {
    if !b.check_move(mv) {
        return;
    }

    if b.get_piece(mv.from.0, mv.from.1)
        .is_some_and(|p| p.colour == Black)
        && !ai_source
    {
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

    if !(ai_opponent && b.to_move == Colour::Black) {
        return;
    }

    if !(ai_opponent && b.to_move == Colour::Black && matches!(b.gamestate, GameState::Playing)) {
        return;
    }

    if let Some(ai_move) = find_best(b, Black) {
        make_move(ai_move, b, false, true);
        if let PromotionState::Promoting(mv, colour) = b.promotion_state {
            let (row, col) = mv.to;
            b.squares[row as usize][col as usize] = Some(Piece {
                kind: PieceKind::Queen,
                colour,
                has_moved: true,
            });
            b.promotion_state = PromotionState::Not;
            post_move(b);
        }
    }
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
