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

/// A recorded move with its SAN notation
#[derive(Clone, Debug)]
pub struct RecordedMove {
    pub san: String,
}

/// The main game model containing all chess game state
pub struct GameModel {
    /// All positions in the game, starting with the initial position
    positions: Vec<Chess>,
    /// Index of the currently viewed position (0 = starting position)
    viewing_index: usize,
    /// Recorded moves (one less than positions.len())
    moves: Vec<RecordedMove>,
    /// Drag state for piece movement
    pub drag_state: Option<DragState>,
    /// Measured panel size from canvas
    pub panel_size: Size<Pixels>,
}

impl GameModel {
    pub fn new() -> Self {
        Self {
            positions: vec![Chess::default()],
            viewing_index: 0,
            moves: Vec::new(),
            drag_state: None,
            panel_size: Size {
                width: px(INITIAL_LEFT_PANEL),
                height: px(600.0),
            },
        }
    }

    /// Get the currently viewed position
    pub fn current_position(&self) -> &Chess {
        &self.positions[self.viewing_index]
    }

    /// Get the latest position (end of game)
    pub fn latest_position(&self) -> &Chess {
        self.positions.last().unwrap()
    }

    /// Check if we're viewing the latest position
    pub fn is_at_latest(&self) -> bool {
        self.viewing_index == self.positions.len() - 1
    }

    /// Check if we're at the starting position
    pub fn is_at_start(&self) -> bool {
        self.viewing_index == 0
    }

    /// Get the current viewing index
    #[allow(dead_code)]
    pub fn viewing_index(&self) -> usize {
        self.viewing_index
    }

    /// Total number of half-moves played
    #[allow(dead_code)]
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    /// Navigate to a specific move index (0 = after first move, etc.)
    /// Pass None to go to starting position
    pub fn go_to_move(&mut self, move_index: Option<usize>) {
        match move_index {
            None => self.viewing_index = 0,
            Some(idx) => {
                // move_index 0 means after first move = position index 1
                let position_index = idx + 1;
                if position_index < self.positions.len() {
                    self.viewing_index = position_index;
                }
            }
        }
    }

    /// Go to the starting position
    pub fn go_to_start(&mut self) {
        self.viewing_index = 0;
    }

    /// Go to the latest position
    pub fn go_to_end(&mut self) {
        self.viewing_index = self.positions.len() - 1;
    }

    /// Go back one move
    pub fn go_back(&mut self) {
        if self.viewing_index > 0 {
            self.viewing_index -= 1;
        }
    }

    /// Go forward one move
    pub fn go_forward(&mut self) {
        if self.viewing_index < self.positions.len() - 1 {
            self.viewing_index += 1;
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

    /// Get piece at row/col from the currently viewed position
    pub fn piece_at(&self, row: usize, col: usize) -> Option<Piece> {
        let sq = to_square(row, col);
        self.current_position()
            .board()
            .piece_at(sq)
            .map(shakmaty_to_piece)
    }

    /// Try to make a move from one square to another. Returns true if legal.
    /// Only works when viewing the latest position.
    pub fn try_move(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        // Can only make moves when viewing the latest position
        if !self.is_at_latest() {
            return false;
        }

        let position = self.latest_position().clone();
        let from_sq = to_square(from.0, from.1);
        let to_sq = to_square(to.0, to.1);

        for m in &position.legal_moves() {
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
                let san = San::from_move(&position, move_to_play.clone());
                self.moves.push(RecordedMove {
                    san: san.to_string(),
                });

                // Apply the move and store new position
                let new_position = position.play(move_to_play).unwrap();
                self.positions.push(new_position);
                self.viewing_index = self.positions.len() - 1;

                return true;
            }
        }
        false
    }

    /// Get the turn for the currently viewed position
    pub fn current_turn(&self) -> PieceColor {
        match self.current_position().turn() {
            SColor::White => PieceColor::White,
            SColor::Black => PieceColor::Black,
        }
    }

    /// Get move pairs for display (move number, white move index, white san, optional black move index, optional black san)
    /// The indices are the half-move indices (0-based)
    pub fn move_pairs(&self) -> Vec<MoveDisplayPair> {
        self.moves
            .chunks(2)
            .enumerate()
            .map(|(i, chunk)| {
                let move_num = i + 1;
                let white_idx = i * 2;
                let white_san = chunk.first().map(|m| m.san.clone()).unwrap_or_default();
                let black = chunk.get(1).map(|m| (i * 2 + 1, m.san.clone()));
                MoveDisplayPair {
                    move_num,
                    white_move_idx: white_idx,
                    white_san,
                    black_move: black,
                }
            })
            .collect()
    }

    /// Check if a specific half-move index is the currently viewed one
    pub fn is_move_selected(&self, half_move_idx: usize) -> bool {
        // half_move_idx 0 = first move = position index 1
        self.viewing_index == half_move_idx + 1
    }
}

/// Display data for a pair of moves (one full move = white + black)
#[derive(Clone, Debug)]
pub struct MoveDisplayPair {
    pub move_num: usize,
    pub white_move_idx: usize,
    pub white_san: String,
    /// (half_move_index, san) for black's move if it exists
    pub black_move: Option<(usize, String)>,
}

impl Default for GameModel {
    fn default() -> Self {
        Self::new()
    }
}
