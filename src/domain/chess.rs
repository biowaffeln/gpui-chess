//! Pure chess domain types and utilities.
//! No GPUI dependencies - this is the domain layer.

use shakmaty::{Color as SColor, File, Rank, Role, Square};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceKind {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceColor {
    White,
    Black,
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub kind: PieceKind,
    pub color: PieceColor,
}

impl Piece {
    pub fn svg_path(&self) -> &'static str {
        match (self.kind, self.color) {
            (PieceKind::Pawn, PieceColor::White) => "assets/pawn-white.svg",
            (PieceKind::Pawn, PieceColor::Black) => "assets/pawn-black.svg",
            (PieceKind::Rook, PieceColor::White) => "assets/rook-white.svg",
            (PieceKind::Rook, PieceColor::Black) => "assets/rook-black.svg",
            (PieceKind::Knight, PieceColor::White) => "assets/knight-white.svg",
            (PieceKind::Knight, PieceColor::Black) => "assets/knight-black.svg",
            (PieceKind::Bishop, PieceColor::White) => "assets/bishop-white.svg",
            (PieceKind::Bishop, PieceColor::Black) => "assets/bishop-black.svg",
            (PieceKind::Queen, PieceColor::White) => "assets/queen-white.svg",
            (PieceKind::Queen, PieceColor::Black) => "assets/queen-black.svg",
            (PieceKind::King, PieceColor::White) => "assets/king-white.svg",
            (PieceKind::King, PieceColor::Black) => "assets/king-black.svg",
        }
    }
}

/// Convert row/col (0-indexed, row 0 = rank 8) to shakmaty Square
pub fn to_square(row: usize, col: usize) -> Square {
    let file = File::new(col as u32);
    let rank = Rank::new(7 - row as u32); // row 0 = rank 8, row 7 = rank 1
    Square::from_coords(file, rank)
}

/// Convert shakmaty piece to our domain Piece
pub fn shakmaty_to_piece(piece: shakmaty::Piece) -> Piece {
    let kind = match piece.role {
        Role::Pawn => PieceKind::Pawn,
        Role::Knight => PieceKind::Knight,
        Role::Bishop => PieceKind::Bishop,
        Role::Rook => PieceKind::Rook,
        Role::Queen => PieceKind::Queen,
        Role::King => PieceKind::King,
    };
    let color = match piece.color {
        SColor::White => PieceColor::White,
        SColor::Black => PieceColor::Black,
    };
    Piece { kind, color }
}
