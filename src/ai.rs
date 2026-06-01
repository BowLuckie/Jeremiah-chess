use rand::random_range;

use crate::{
    board::Board,
    moves::{Colour::Black, Move, Piece},
};

pub fn find_best(board: &Board) -> Option<Move> {
    // TODO
    let mut peice_move_vec: Vec<Vec<Move>> = Vec::new();
    for (_, row, col) in board
        .as_iter()
        .filter(|(p, _, _)| p.is_some_and(|p: Piece| p.colour == Black))
    {
        peice_move_vec.push(board.get_moves(row, col));
    }
    let move_vec = &peice_move_vec[random_range(0..peice_move_vec.len())];
    let selected_move = &move_vec[random_range(0..move_vec.len())];
    return Some(*selected_move);
}
