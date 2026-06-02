use crate::{
    board::Board,
    moves::{
        Colour::{self, Black, White},
        Move, Piece, PieceKind,
    },
};

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

pub fn evaluate(board: &Board) -> Score {
    board
        .as_iter()
        .filter_map(|(p, _, _)| p)
        .fold(0, |acc, p| acc + p_score(p))
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
        .flat_map(|(row, col)| board.get_moves(row, col))
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
        .flat_map(|(row, col)| board.get_moves(row, col))
        .collect();

    if moves.is_empty() {
        return None;
    }

    moves.into_iter().max_by_key(|&mv| {
        let mut copy = board.hashless_clone();
        copy.raw_move(mv);
        copy.switch_turn();
        let score = minimax(&copy, 3, Score::MIN + 1, Score::MAX - 1, !maximizing);
        if maximizing { score } else { -score }
    })
}
