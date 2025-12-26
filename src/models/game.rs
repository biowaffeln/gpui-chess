//! Game state model - the application layer for chess game state.
//!
//! This model contains only pure game state and logic, with no UI concerns.

use crate::domain::{MoveNodeId, MoveTree, Piece, PieceColor, shakmaty_to_piece, to_square};
use shakmaty::san::San;
use shakmaty::{Chess, Color as SColor, File, Move, Position, Rank, Role};

/// The main game model containing all chess game state
pub struct GameModel {
    /// The move tree containing all positions and variations
    tree: MoveTree,
}

impl GameModel {
    pub fn new() -> Self {
        Self {
            tree: MoveTree::new(),
        }
    }

    /// Get the currently viewed position
    pub fn current_position(&self) -> &Chess {
        &self.tree.current().position
    }

    /// Get the current node ID
    pub fn current_node_id(&self) -> MoveNodeId {
        self.tree.current_id()
    }

    /// Get read-only access to the move tree (for display generation)
    pub fn tree(&self) -> &MoveTree {
        &self.tree
    }

    /// Check if we're at a leaf node (can add new moves freely)
    pub fn is_at_leaf(&self) -> bool {
        self.tree.is_at_leaf()
    }

    /// Check if we're at the root (starting position)
    pub fn is_at_root(&self) -> bool {
        self.tree.is_at_root()
    }

    /// Navigate to a specific node by ID
    pub fn go_to_node(&mut self, id: MoveNodeId) -> bool {
        self.tree.go_to(id)
    }

    /// Go to the starting position
    pub fn go_to_start(&mut self) {
        self.tree.go_to_root();
    }

    /// Go to the end of the main line
    pub fn go_to_end(&mut self) {
        self.tree.go_to_end();
    }

    /// Go back one move
    pub fn go_back(&mut self) {
        self.tree.go_back();
    }

    /// Go forward one move (main line)
    pub fn go_forward(&mut self) {
        self.tree.go_forward();
    }

    /// Get piece at row/col from the currently viewed position
    pub fn piece_at(&self, row: usize, col: usize) -> Option<Piece> {
        let sq = to_square(row, col);
        self.current_position()
            .board()
            .piece_at(sq)
            .map(shakmaty_to_piece)
    }

    /// Try to make a move from one square to another. Returns true if successful.
    ///
    /// If the move already exists as a child of current node, navigates to it.
    /// Otherwise, creates a new variation and navigates to it.
    pub fn try_move(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        let position = self.current_position().clone();
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
                    _ => *m,
                };

                // Get SAN notation
                let san = San::from_move(&position, move_to_play).to_string();

                // Apply the move
                let new_position = position.play(move_to_play).unwrap();

                // Add to tree (will navigate to existing or create new)
                self.tree.add_move(new_position, san);

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

    /// Delete a move and all its descendants.
    /// If currently viewing the deleted move or a descendant, navigates to parent.
    pub fn delete_move(&mut self, node_id: MoveNodeId) -> bool {
        self.tree.delete_node(node_id)
    }

    /// Promote a variation to be the main line at its branch point.
    pub fn promote_variation(&mut self, node_id: MoveNodeId) -> bool {
        self.tree.promote_variation(node_id)
    }

    /// Promote a variation to be the global main line (promotes at all branch points).
    pub fn promote_to_main_line(&mut self, node_id: MoveNodeId) -> bool {
        self.tree.promote_to_main_line(node_id)
    }
}

impl Default for GameModel {
    fn default() -> Self {
        Self::new()
    }
}
