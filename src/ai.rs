use crate::{
    board::Board,
    moves::{
        Colour::{self, Black, White},
        Move, Piece, PieceKind,
    },
};

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

fn pst_bonus(piece: Piece, row: i8, col: i8) -> i32 {
    let row = match piece.colour {
        White => row,
        Black => 7 - row, // mirror for black
    };

    let (row, col) = (row as usize, col as usize);

    (match piece.kind {
        PieceKind::Knight => KNIGHT_PST[row][col],
        PieceKind::Bishop => BISHOP_PST[row][col],
        PieceKind::Rook => ROOK_PST[row][col],
        PieceKind::Queen => QUEEN_PST[row][col],
        _ => 0,
    }) * (match piece.colour {
        White => 1,
        Black => -1,
    })
}

pub type Score = i32;

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

pub fn evaluate(board: &Board) -> Score {
    let score = board
        .as_iter()
        .filter_map(|(p, row, col)| p.map(|p| (p, row, col)))
        .fold(0, |acc, (p, row, col)| {
            acc + p_score(p) + mobility_bonus(board, row, col, p) + pst_bonus(p, row, col)
        });

    return score;
}

pub fn minimax(
    board: &Board,
    depth: u8,
    mut alpha: Score,
    mut beta: Score,
    maximizing: bool,
) -> Score {
    if depth == 0 {
        return evaluate(board);
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
            best = best.max(minimax(&copy, depth - 1, alpha, beta, false));
            alpha = alpha.max(best);
            if beta <= alpha {
                break;
            } // prune
        }
        best
    } else {
        let mut best = Score::MAX - 1;
        for mv in moves {
            let mut copy = board.clone();
            copy.raw_move(mv);
            copy.switch_turn();
            best = best.min(minimax(&copy, depth - 1, alpha, beta, true));
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
        let score = minimax(&copy, 3, Score::MIN + 1, Score::MAX - 1, !maximizing); // GREPME2
        if maximizing { score } else { -score }
    })
}
