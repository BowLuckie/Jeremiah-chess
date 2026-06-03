use crate::{
    input::{InputState, LoadedSound},
    moves::{
        Colour::{self},
        Coordinate, Move, Piece,
        PieceKind::{self, Bishop, Knight},
    },
};
use std::{
    collections::HashMap,
    fmt,
    ops::Not,
    sync::{Arc, Mutex},
};

pub type PositionHash = u64;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Board {
    pub squares: [[Option<Piece>; 8]; 8],
    pub to_move: Colour,
    pub white_king: Coordinate,
    pub black_king: Coordinate,
    pub gamestate: GameState,
    pub halfmove_clock: i8,
    pub promotion_state: PromotionState,
    pub position_history: HashMap<PositionHash, u8>,
    pub loaded_sound: LoadedSound,
    pub last_double: Option<Coordinate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]

pub enum GameState {
    Playing,
    Checkmate(Colour),
    Stalemate,
    InsufficientMat,
    FiftyMove,
    Repetition,
}

#[derive(Clone, Copy, Debug)]
pub enum PromotionState {
    Not,
    Promoting(Move, Colour),
    Complete(Coordinate, PieceKind, Colour),
}

pub struct SquareIter<'a> {
    board: &'a Board,
    idx: usize,
}

impl Iterator for SquareIter<'_> {
    type Item = (Option<Piece>, i8, i8);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= 64 {
            return None;
        }

        let row = 7 - (self.idx / 8) as i8;
        let col = (self.idx % 8) as i8;

        let piece = self.board.get_piece(row, col);

        self.idx += 1;

        Some((piece.copied(), row, col))
    }
}

pub fn reset(board: &Arc<Mutex<Board>>, input: &Arc<Mutex<InputState>>) {
    *board.lock().unwrap() = Board::test_board(); // GREPME
    board.lock().unwrap().loaded_sound = LoadedSound::Start;
    *input.lock().unwrap() = InputState::new();
}

pub fn square_iter() -> impl Iterator<Item = (i8, i8)> {
    (0..64).map(|idx| {
        let row = 7 - (idx / 8) as i8;
        let col = (idx % 8) as i8;
        (row, col)
    })
}

impl PieceKind {
    fn base_char(self) -> char {
        use PieceKind::*;
        match self {
            Pawn => 'P',
            Knight => 'N',
            Queen => 'Q',
            Rook => 'R',
            Bishop => 'B',
            King => 'K',
        }
    }
}

pub fn get_lexrep(piece: Option<&Piece>) -> String {
    match piece {
        Some(p) => {
            let c = p.kind.base_char();
            match p.colour {
                Colour::White => c.to_string(),
                Colour::Black => c.to_lowercase().to_string(),
            }
        }
        None => " ".to_string(),
    }
}

impl Board {
    #[allow(unused)]
    pub fn new() -> Self {
        use Colour::*;
        use PieceKind::*;

        let mut squares: [[Option<Piece>; 8]; 8] = [[None; 8]; 8];

        let place = |squares: &mut [[Option<Piece>; 8]; 8],
                     row: usize,
                     col: usize,
                     kind: PieceKind,
                     colour: Colour| {
            squares[row][col] = Some(Piece {
                kind,
                colour,
                has_moved: false,
            });
        };

        for col in 0..8 {
            place(&mut squares, 1, col, Pawn, Black);
            place(&mut squares, 6, col, Pawn, White);
        }

        for col in [0, 7] {
            place(&mut squares, 0, col, Rook, Black);
            place(&mut squares, 7, col, Rook, White);
        }

        for col in [1, 6] {
            place(&mut squares, 0, col, Knight, Black);
            place(&mut squares, 7, col, Knight, White);
        }

        for col in [2, 5] {
            place(&mut squares, 0, col, Bishop, Black);
            place(&mut squares, 7, col, Bishop, White);
        }

        place(&mut squares, 0, 3, Queen, Black);
        place(&mut squares, 7, 3, Queen, White);

        place(&mut squares, 0, 4, King, Black);
        place(&mut squares, 7, 4, King, White);

        Self {
            squares,
            to_move: White,
            black_king: (0, 4),
            white_king: (7, 4),
            gamestate: GameState::Playing,
            halfmove_clock: 0,
            promotion_state: PromotionState::Not,
            position_history: HashMap::new(),
            loaded_sound: LoadedSound::None,
            last_double: None,
        }
    }

    pub fn test_board() -> Self {
        use Colour::*;
        use PieceKind::*;

        let mut squares = [[None; 8]; 8];

        let place = |squares: &mut [[Option<Piece>; 8]; 8],
                     row: usize,
                     col: usize,
                     kind: PieceKind,
                     colour: Colour| {
            squares[row][col] = Some(Piece {
                kind,
                colour,
                has_moved: false,
            });
        };

        place(&mut squares, 0, 5, King, Black);
        place(&mut squares, 7, 4, King, White);

        place(&mut squares, 1, 2, Rook, Black);

        Self {
            squares,
            to_move: White,
            black_king: (0, 5),
            white_king: (7, 4),
            gamestate: GameState::Playing,
            halfmove_clock: 0,
            promotion_state: PromotionState::Not,
            position_history: HashMap::new(),
            loaded_sound: LoadedSound::None,
            last_double: None,
        }
    }

    pub fn as_iter(&self) -> SquareIter<'_> {
        SquareIter {
            board: self,
            idx: 0,
        }
    }

    pub fn get_piece(&self, row: i8, col: i8) -> Option<&Piece> {
        assert!(row < 8 && col < 8, "row or col exceeds 7: {row} {col}");
        self.squares[row as usize][col as usize].as_ref()
    }

    pub fn get_piece_by_cord(&self, coordinate: Coordinate) -> Option<&Piece> {
        self.get_piece(coordinate.0, coordinate.1)
    }

    pub fn raw_move(&mut self, mv: Move) {
        use Colour::*;
        use PieceKind::*;

        let (orow, ocol) = mv.from;
        let (trow, tcol) = mv.to;

        let mut piece = self
            .get_piece(orow, ocol)
            .copied()
            .unwrap_or_else(|| panic!("no piece to move at {orow} {ocol}"));

        piece.has_moved = true;

        if piece.kind == King {
            match piece.colour {
                White => self.white_king = mv.to,
                Black => self.black_king = mv.to,
            }
        }

        self.squares[orow as usize][ocol as usize] = None;
        self.squares[trow as usize][tcol as usize] = Some(piece);
    }

    pub fn check_move(&self, mv: Move) -> bool {
        if !self
            .get_moves_unchecked(mv.from.0, mv.from.1, false)
            .contains(&mv)
        {
            return false;
        }

        let mut simulation_board: Board = self.clone();
        simulation_board.raw_move(mv);
        let colour = self.get_piece(mv.from.0, mv.from.1).unwrap().colour;
        return !simulation_board.king_in_check(colour);
    }

    pub fn switch_turn(&mut self) {
        self.to_move = !self.to_move;
    }

    pub fn king_in_check(&self, colour: Colour) -> bool {
        let king_pos = match colour {
            Colour::White => self.white_king,
            Colour::Black => self.black_king,
        };
        for (piece, row, col) in self.as_iter() {
            let Some(p) = piece else {
                continue;
            };

            if p.colour == colour {
                continue;
            }

            if self
                .get_moves_unchecked(row, col, true)
                .iter()
                .any(|mv| mv.to == king_pos)
            {
                return true;
            }
        }
        false
    }

    pub fn get_gamestate(&self, colour: Colour) -> GameState {
        use GameState::*;
        if self.position_history.values().any(|&c| c > 2) {
            return Repetition;
        }
        let total_moves: usize = self
            .as_iter()
            .filter_map(|(piece, row, col)| {
                let piece = piece?;
                if piece.colour != colour {
                    return None;
                }
                Some(self.get_moves(row, col, true).len())
            })
            .sum();

        if total_moves == 0 {
            if self.king_in_check(colour) {
                return Checkmate(!colour);
            }
            return Stalemate;
        }

        if self.halfmove_clock >= 100 {
            return FiftyMove;
        }

        let mut white_pieces: Vec<(Piece, i8, i8)> = Vec::new();
        let mut black_pieces: Vec<(Piece, i8, i8)> = Vec::new();

        self.as_iter().for_each(|(piece, row, col)| {
            if piece.is_some_and(|p| p.kind != PieceKind::King) {
                match piece.unwrap().colour {
                    Colour::White => white_pieces.push((piece.unwrap(), row, col)),
                    Colour::Black => black_pieces.push((piece.unwrap(), row, col)),
                }
            }
        });

        let (wlen, blen) = (white_pieces.len(), black_pieces.len());

        if (wlen + blen) == 0 {
            return InsufficientMat;
        }

        if match (&white_pieces[..], &black_pieces[..]) {
            ([p], []) | ([], [p]) => matches!(p.0.kind, Bishop | Knight),
            _ => false,
        } {
            return InsufficientMat;
        }

        if (wlen == 1 && blen == 1)
            && white_pieces[0].0.kind == Bishop
            && black_pieces[0].0.kind == Bishop
            && (white_pieces[0].1 + white_pieces[0].2) % 2
                == (black_pieces[0].1 + black_pieces[0].2) % 2
        {
            return InsufficientMat;
        }

        Playing
    }

    pub fn position_hash(&self) -> PositionHash {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.squares.hash(&mut hasher);
        self.to_move.hash(&mut hasher);
        return hasher.finish();
    }

    pub fn hashless_clone(&self) -> Self {
        Board {
            squares: self.squares,
            to_move: self.to_move,
            white_king: self.white_king,
            black_king: self.black_king,
            halfmove_clock: self.halfmove_clock,
            last_double: self.last_double,
            gamestate: self.gamestate,
            promotion_state: self.promotion_state,
            loaded_sound: self.loaded_sound,
            position_history: HashMap::new(),
        }
    }
}

impl Not for Colour {
    type Output = Self;
    fn not(self) -> Self::Output {
        use Colour::*;
        match self {
            White => Black,
            Black => White,
        }
    }
}

#[allow(clippy::format_push_string)]
impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();

        for (row, col) in square_iter() {
            if col == 0 {
                out.push_str(&format!("{} ", 7 - row));
            }

            let piece = self.get_piece(7 - row, col);
            out.push_str(&format!("[{}]", get_lexrep(piece)));

            if col == 7 {
                out.push('\n');
            }
        }

        out.push_str("  ");
        for col in 0..8 {
            out.push_str(&format!(" {col} "));
        }
        out.push('\n');

        write!(f, "{out}")
    }
}
