use crate::{
    board::{
        Board,
        PromotionState::{self, Promoting},
    },
    moves::Colour::Black,
};

pub type Coordinate = (i8, i8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Colour {
    White,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn,
    Knight,
    Queen,
    Rook,
    Bishop,
    King,
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct Piece {
    pub kind: PieceKind,
    pub colour: Colour,
    pub has_moved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub struct Move {
    pub from: Coordinate,
    pub to: Coordinate,
}

pub fn in_bounds(r: i8, c: i8) -> bool {
    (0..8).contains(&r) && (0..8).contains(&c)
}

pub fn in_bounds_point(point: Coordinate) -> bool {
    (0..8).contains(&point.0) && (0..8).contains(&point.1)
}

pub fn promotion_options(colour: Colour) -> [Piece; 4] {
    let piece = |kind| Piece {
        kind,
        colour,
        has_moved: true,
    };
    [
        piece(PieceKind::Queen),
        piece(PieceKind::Rook),
        piece(PieceKind::Bishop),
        piece(PieceKind::Knight),
    ]
}

pub fn promotion_click(click: Coordinate, promotion_state: PromotionState) -> Option<PieceKind> {
    let PromotionState::Promoting(mv, colour) = promotion_state else {
        return None;
    };

    let (row, col) = click;

    if col != mv.to.1 {
        return None;
    }

    let start_row = match colour {
        Colour::White => 0,
        Colour::Black => 4,
    };

    let index = row - start_row;
    if !(0..4).contains(&index) {
        return None;
    }

    let options = promotion_options(colour);
    Some(options[index as usize].kind)
}

impl Move {
    pub fn new(from: Coordinate, to: Coordinate) -> Self {
        assert!(
            in_bounds_point(from) && in_bounds_point(to),
            "attempted to create a move out of bounds!"
        );
        Move { from, to }
    }
}

impl Board {
    /// very slow for ui and explicit move checking
    pub fn get_moves(&self, row: i8, col: i8, ai_source: bool) -> Vec<Move> {
        if !ai_source && self.get_piece(row, col).is_some_and(|p| p.colour == Black) {
            return vec![];
        }
        if let Promoting(_, _) = self.promotion_state {
            return vec![];
        }
        self.get_moves_unchecked(row, col, false)
            .into_iter()
            .filter(|mv| self.check_move(*mv))
            .collect()
    }

    /// faster since it does not check the moves and just returns the raw psuedo-legals
    pub fn get_moves_unchecked(&self, row: i8, col: i8, simulate: bool) -> Vec<Move> {
        match self.get_piece(row, col) {
            Some(p) => self.dispatch(*p, row, col, simulate),
            None => vec![],
        }
    }

    fn dispatch(&self, p: Piece, row: i8, col: i8, simulate: bool) -> Vec<Move> {
        use PieceKind::*;
        if (!simulate) && (p.colour != self.to_move) {
            return vec![];
        }
        return match p.kind {
            Pawn => self.pawn_moves(p, row, col),
            Knight => self.knight_moves(p, row, col),
            Queen => self.queen_moves(p, row, col),
            Rook => self.rook_moves(p, row, col),
            Bishop => self.bishop_moves(p, row, col),
            King => self.king_moves(p, row, col, simulate),
        };
    }

    fn pawn_moves(&self, p: Piece, row: i8, col: i8) -> Vec<Move> {
        let dir = match p.colour {
            Colour::Black => 1,
            Colour::White => -1,
        };

        let mut moves = Vec::new();
        let origin = (row, col);
        let new_row = row + dir;

        if in_bounds(new_row, col) && self.get_piece(new_row, col).is_none() {
            moves.push(Move::new(origin, (new_row, col)));

            let two_row = new_row + dir;
            if !p.has_moved && in_bounds(two_row, col) && self.get_piece(two_row, col).is_none() {
                moves.push(Move::new(origin, (two_row, col)));
            }
        }

        for dc in [-1, 1] {
            let new_col = col + dc;
            if in_bounds(new_row, new_col)
                && let Some(target) = self.get_piece(new_row, new_col)
                && target.colour != p.colour
            {
                moves.push(Move::new(origin, (new_row, new_col)));
            }

            if self.last_double == Some((row, new_col)) {
                moves.push(Move::new(origin, (new_row, new_col)));
            }
        }

        moves
    }

    fn knight_moves(&self, p: Piece, row: i8, col: i8) -> Vec<Move> {
        const KNIGHT_DELTAS: [(i8, i8); 8] = [
            (-2, -1),
            (-2, 1),
            (-1, -2),
            (-1, 2),
            (1, -2),
            (1, 2),
            (2, -1),
            (2, 1),
        ];
        let origin = (row, col);
        let moves: Vec<Move> = KNIGHT_DELTAS
            .iter()
            .filter_map(|(dr, dc)| {
                let (nr, nc) = (row + dr, col + dc);
                if !(0..8).contains(&nr) || !(0..8).contains(&nc) {
                    return None;
                }
                let target = self.get_piece(nr, nc);
                if target.is_some_and(|t| t.colour == p.colour) {
                    return None;
                }
                Some(Move::new(origin, (nr, nc)))
            })
            .collect();
        return moves;
    }

    fn raw_slide(
        &self,
        p: Piece,
        row: i8,
        col: i8,
        directions: Vec<Coordinate>,
        max: Option<i8>,
    ) -> Vec<Move> {
        let mut moves = Vec::new();
        let origin = (row, col);

        for (drow, dcol) in directions {
            let (mut trow, mut tcol) = (row + drow, col + dcol);
            let mut distance: i8 = 0;

            while in_bounds(trow, tcol) && distance < max.unwrap_or(8) {
                let target = self.get_piece(trow, tcol);

                if let Some(t) = target {
                    if t.colour == p.colour {
                        break;
                    }
                    moves.push(Move::new(origin, (trow, tcol)));
                    break;
                }
                moves.push(Move::new(origin, (trow, tcol)));

                trow += drow;
                tcol += dcol;
                distance += 1;
            }
        }

        return moves;
    }

    fn queen_moves(&self, p: Piece, row: i8, col: i8) -> Vec<Move> {
        let directions = vec![
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];
        return self.raw_slide(p, row, col, directions, None);
    }

    fn rook_moves(&self, p: Piece, row: i8, col: i8) -> Vec<Move> {
        let directions = vec![(1, 0), (0, 1), (-1, 0), (0, -1)];
        return self.raw_slide(p, row, col, directions, None);
    }

    fn bishop_moves(&self, p: Piece, row: i8, col: i8) -> Vec<Move> {
        let directions = vec![(1, 1), (-1, 1), (-1, -1), (1, -1)];
        return self.raw_slide(p, row, col, directions, None);
    }

    fn king_moves(&self, p: Piece, row: i8, col: i8, simulate: bool) -> Vec<Move> {
        use Colour::*;

        let directions = vec![
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];

        let mut moves = self.raw_slide(p, row, col, directions, Some(1));

        if simulate || p.has_moved || ![(7, 4), (0, 4)].contains(&(row, col)) {
            return moves;
        }

        let back_rank = match p.colour {
            White => 7,
            Black => 0,
        };

        let can_castle = |rook_col: i8, empty: &[i8], king_path: &[i8]| {
            let rook = self.get_piece(back_rank, rook_col);

            rook.is_some_and(|r| !r.has_moved)
                && empty
                    .iter()
                    .all(|&c| self.get_piece(back_rank, c).is_none())
                && king_path.iter().all(|&c| {
                    let mut copy = self.clone();

                    copy.squares[back_rank as usize][4] = None;
                    copy.squares[back_rank as usize][c as usize] = Some(p);

                    match p.colour {
                        White => copy.white_king = (back_rank, c),
                        Black => copy.black_king = (back_rank, c),
                    }

                    !copy.king_in_check(p.colour)
                })
        };

        // queenside
        if can_castle(0, &[1, 2, 3], &[4, 3, 2]) {
            moves.push(Move::new((back_rank, 4), (back_rank, 2)));
        }

        // kingside
        if can_castle(7, &[5, 6], &[4, 5, 6]) {
            moves.push(Move::new((back_rank, 4), (back_rank, 6)));
        }

        moves
    }
}
