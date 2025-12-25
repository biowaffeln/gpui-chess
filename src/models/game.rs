//! Game state model - the application layer for chess game state.

use crate::domain::{Piece, PieceColor, shakmaty_to_piece, to_square};
use gpui::{Pixels, Size, px};
use shakmaty::san::San;
use shakmaty::{Chess, Color as SColor, File, Move, Position, Rank, Role};

use crate::ui::theme::{BOARD_PADDING, INITIAL_LEFT_PANEL, PIECE_SCALE};

/// State for a piece being dragged
#[derive(Clone, Copy, Debug)]
pub struct DragState {
    pub piece: Piece,
    pub from_row: usize,
    pub from_col: usize,
    /// Mouse position relative to window
    pub mouse_x: f32,
    pub mouse_y: f32,
}

/// The main game model containing all chess game state
pub struct GameModel {
    pub position: Chess,
    pub drag_state: Option<DragState>,
    pub move_history: Vec<String>,
    /// Measured panel size from canvas
    pub panel_size: Size<Pixels>,
}

impl GameModel {
    pub fn new() -> Self {
        Self {
            position: Chess::default(),
            drag_state: None,
            move_history: Vec::new(),
            panel_size: Size {
                width: px(INITIAL_LEFT_PANEL),
                height: px(600.0),
            },
        }
    }

    /// Calculate square size from measured panel dimensions
    pub fn square_size(&self) -> f32 {
        let panel_width: f32 = self.panel_size.width.into();
        let panel_height: f32 = self.panel_size.height.into();
        let available_width = panel_width - BOARD_PADDING * 2.0;
        let available_height = panel_height - BOARD_PADDING * 2.0;
        (available_width.min(available_height) / 8.0).max(30.0)
    }

    pub fn piece_size(&self) -> f32 {
        self.square_size() * PIECE_SCALE
    }

    /// Convert position relative to board panel to board row/col (if within board)
    pub fn pos_to_square(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let board_x = x - BOARD_PADDING;
        let board_y = y - BOARD_PADDING;

        if board_x < 0.0 || board_y < 0.0 {
            return None;
        }

        let square_size = self.square_size();
        let col = (board_x / square_size) as usize;
        let row = (board_y / square_size) as usize;

        if row < 8 && col < 8 {
            Some((row, col))
        } else {
            None
        }
    }

    /// Get piece at row/col from shakmaty position
    pub fn piece_at(&self, row: usize, col: usize) -> Option<Piece> {
        let sq = to_square(row, col);
        self.position.board().piece_at(sq).map(shakmaty_to_piece)
    }

    /// Try to make a move from one square to another. Returns true if legal.
    pub fn try_move(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        let from_sq = to_square(from.0, from.1);
        let to_sq = to_square(to.0, to.1);

        for m in &self.position.legal_moves() {
            let (move_from, move_to) = match m {
                Move::Normal { from, to, .. } => (*from, *to),
                Move::EnPassant { from, to, .. } => (*from, *to),
                Move::Castle { king, rook, .. } => {
                    // For castling, user drags king to destination (g1/g8 or c1/c8)
                    let king_dest = if rook.file() == File::H {
                        shakmaty::Square::from_coords(File::G, rook.rank())
                    } else {
                        shakmaty::Square::from_coords(File::C, rook.rank())
                    };
                    (*king, king_dest)
                }
                Move::Put { .. } => continue,
            };

            if move_from == from_sq && move_to == to_sq {
                // For pawn promotion, auto-promote to queen
                let move_to_play = match m {
                    Move::Normal {
                        role: Role::Pawn,
                        from,
                        to,
                        capture,
                        promotion: None,
                    } if to.rank() == Rank::Eighth || to.rank() == Rank::First => Move::Normal {
                        role: Role::Pawn,
                        from: *from,
                        to: *to,
                        capture: *capture,
                        promotion: Some(Role::Queen),
                    },
                    _ => m.clone(),
                };

                // Record move in standard notation
                let san = San::from_move(&self.position, move_to_play.clone());
                self.move_history.push(san.to_string());

                self.position = self.position.clone().play(move_to_play).unwrap();
                return true;
            }
        }
        false
    }

    pub fn current_turn(&self) -> PieceColor {
        match self.position.turn() {
            SColor::White => PieceColor::White,
            SColor::Black => PieceColor::Black,
        }
    }

    /// Get move pairs for display (move number, white move, optional black move)
    pub fn move_pairs(&self) -> Vec<(usize, String, Option<String>)> {
        self.move_history
            .chunks(2)
            .enumerate()
            .map(|(i, chunk)| {
                let move_num = i + 1;
                let white_move = chunk.first().cloned().unwrap_or_default();
                let black_move = chunk.get(1).cloned();
                (move_num, white_move, black_move)
            })
            .collect()
    }
}

impl Default for GameModel {
    fn default() -> Self {
        Self::new()
    }
}
