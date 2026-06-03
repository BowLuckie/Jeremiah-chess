use std::collections::HashMap;

use crate::{
    ai::GamePhase::Late,
    board::{Board, PositionHash},
    moves::{
        Colour::{self, Black, White},
        Move, Piece,
        PieceKind::{self, Pawn},
    },
};

pub type Score = i32;

// ---- PST ----

pub const KNIGHT_PST: [[i32; 8]; 8] = [
    [-20, -15, -10, -10, -10, -10, -15, -20],
    [-15, 0, 5, 5, 5, 5, 0, -15],
    [-10, 5, 15, 20, 20, 15, 5, -10],
    [-10, 5, 20, 30, 30, 20, 5, -10],
    [-10, 5, 20, 30, 30, 20, 5, -10],
    [-10, 5, 15, 20, 20, 15, 5, -10],
    [-15, 0, 5, 5, 5, 5, 0, -15],
    [-20, -15, -10, -10, -10, -10, -15, -20],
];

pub const BISHOP_PST: [[i32; 8]; 8] = [
    [-10, -10, -10, -10, -10, -10, -10, -10],
    [-5, 0, 0, 5, 5, 0, 0, -5],
    [-5, 5, 10, 15, 15, 10, 5, -5],
    [-5, 5, 15, 20, 20, 15, 5, -5],
    [-5, 5, 15, 20, 20, 15, 5, -5],
    [-5, 5, 10, 15, 15, 10, 5, -5],
    [-5, 0, 0, 5, 5, 0, 0, -5],
    [-10, -10, -10, -10, -10, -10, -10, -10],
];

pub const ROOK_PST: [[i32; 8]; 8] = [
    [0, 0, 5, 10, 10, 5, 0, 0],
    [0, 0, 5, 10, 10, 5, 0, 0],
    [0, 0, 5, 10, 10, 5, 0, 0],
    [5, 5, 10, 15, 15, 10, 5, 5],
    [5, 5, 10, 15, 15, 10, 5, 5],
    [0, 0, 5, 10, 10, 5, 0, 0],
    [0, 0, 5, 10, 10, 5, 0, 0],
    [0, 0, 5, 10, 10, 5, 0, 0],
];

pub const QUEEN_PST: [[i32; 8]; 8] = [
    [-5, -5, -5, -5, -5, -5, -5, -5],
    [-5, 0, 0, 0, 0, 0, 0, -5],
    [-5, 0, 2, 3, 3, 2, 0, -5],
    [-5, 0, 3, 5, 5, 3, 0, -5],
    [-5, 0, 3, 5, 5, 3, 0, -5],
    [-5, 0, 2, 3, 3, 2, 0, -5],
    [-5, 0, 0, 0, 0, 0, 0, -5],
    [-5, -5, -5, -5, -5, -5, -5, -5],
];

pub const KING_PST: [[i32; 8]; 8] = [
    [-50, -30, -30, -30, -30, -30, -30, -50],
    [-30, -30, 0, 0, 0, 0, -30, -30],
    [-30, -10, 20, 30, 30, 20, -10, -30],
    [-30, -10, 30, 40, 40, 30, -10, -30],
    [-30, -10, 30, 40, 40, 30, -10, -30],
    [-30, -10, 20, 30, 30, 20, -10, -30],
    [-30, -20, -10, 0, 0, -10, -20, -30],
    [-50, -40, -30, -20, -20, -30, -40, -50],
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GamePhase {
    Early,
    Mid,
    Late,
}

fn pst_bonus(piece: Piece, row: i8, col: i8, phase: GamePhase) -> i32 {
    let row = match piece.colour {
        White => row,
        Black => 7 - row,
    };

    let (row, col) = (row as usize, col as usize);

    (match piece.kind {
        PieceKind::Knight => KNIGHT_PST[row][col],
        PieceKind::Bishop => BISHOP_PST[row][col],
        PieceKind::Rook => ROOK_PST[row][col],
        PieceKind::Queen => QUEEN_PST[row][col],
        PieceKind::King if phase == Late => KING_PST[row][col],
        _ => 0,
    }) * (match piece.colour {
        White => 1,
        Black => -1,
    })
}

pub fn get_game_phase(board: &Board) -> GamePhase {
    use GamePhase::*;
    Early
}

fn pawn_bonus(piece: Piece, row: i8) -> Score {
    if piece.kind != Pawn {
        return 0;
    }

    let advancement = match piece.colour {
        White => 7 - row,
        Black => row,
    };
    let bonus = advancement * 10;
    (match piece.colour {
        White => bonus,
        Black => -bonus,
    }) as i32
}

fn p_score(piece: Piece) -> Score {
    (match piece.kind {
        PieceKind::Pawn => 100,
        PieceKind::Knight => 300,
        PieceKind::Queen => 900,
        PieceKind::Rook => 500,
        PieceKind::Bishop => 300,
        PieceKind::King => 0,
    }) * (match piece.colour {
        White => 1,
        Black => -1,
    })
}

fn mobility_bonus(board: &Board, row: i8, col: i8, piece: Piece) -> Score {
    let moves = board.get_moves_unchecked(row, col, true).len() as i32;

    let bonus = moves
        * match piece.kind {
            PieceKind::Knight => 4,
            PieceKind::Bishop => 3,
            PieceKind::Rook => 2,
            PieceKind::Queen => 1,
            _ => 0,
        };

    match piece.colour {
        White => bonus,
        Black => -bonus,
    }
}

pub fn evaluate(
    board: &Board,
    position_history: &HashMap<PositionHash, u8>,
    phase: GamePhase,
) -> Score {
    let mut score = board
        .as_iter()
        .filter_map(|(p, row, col)| p.map(|piece| (piece, row, col)))
        .fold(0, |acc, (piece, row, col)| {
            acc + p_score(piece)
                + mobility_bonus(board, row, col, piece)
                + pst_bonus(piece, row, col, phase)
                + pawn_bonus(piece, row)
        });

    let hash = board.position_hash();
    if let Some(&count) = position_history.get(&hash) {
        score -= count as Score * 50;
    }

    return score;
}

fn minimax(
    board: &Board,
    depth: u8,
    mut alpha: Score,
    mut beta: Score,
    maximizing: bool,
    phase: &GamePhase,
) -> Score {
    if depth == 0 {
        return evaluate(board, &board.position_history, *phase);
    }

    let colour = if maximizing {
        Colour::White
    } else {
        Colour::Black
    };
    let moves: Vec<Move> = board
        .as_iter()
        .filter_map(|(p, row, col)| {
            if p.is_some_and(|p| p.colour == colour) {
                Some((row, col))
            } else {
                None
            }
        })
        .flat_map(|(row, col)| board.get_moves(row, col, true))
        .collect();

    if moves.is_empty() {
        return if board.king_in_check(colour) {
            if maximizing {
                Score::MIN + 1
            } else {
                Score::MAX - 1
            }
        } else {
            0
        };
    }

    if maximizing {
        let mut best = Score::MIN + 1;
        for mv in moves {
            let mut copy = board.clone();
            copy.raw_move(mv);
            copy.switch_turn();
            best = best.max(minimax(&copy, depth - 1, alpha, beta, false, phase));
            alpha = alpha.max(best);
            if beta <= alpha {
                break;
            } // prune
        }
        best
    } else {
        let mut best = Score::MAX - 1;
        for mv in moves {
            let mut copy = board.hashless_clone();
            copy.raw_move(mv);
            copy.switch_turn();
            best = best.min(minimax(&copy, depth - 1, alpha, beta, true, phase));
            beta = beta.min(best);
            if beta <= alpha {
                break;
            } // prune
        }
        best
    }
}

pub fn find_best(board: &Board, colour: Colour) -> Option<Move> {
    let maximizing = colour == Colour::White;
    let phase = get_game_phase(board);
    let moves: Vec<Move> = board
        .as_iter()
        .filter_map(|(p, row, col)| {
            if p.is_some_and(|p| p.colour == colour) {
                Some((row, col))
            } else {
                None
            }
        })
        .flat_map(|(row, col)| board.get_moves(row, col, true))
        .collect();

    if moves.is_empty() {
        return None;
    }

    moves.into_iter().max_by_key(|&mv| {
        let mut copy = board.hashless_clone();
        copy.raw_move(mv);
        copy.switch_turn();
        let score = minimax(
            &copy,
            3,
            Score::MIN + 1,
            Score::MAX - 1,
            !maximizing,
            &phase,
        ); // GREPME2
        if maximizing { score } else { -score }
    })
}
