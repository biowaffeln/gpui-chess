//! Game state model - the application layer for chess game state.

use crate::domain::{MoveNodeId, MoveTree, Piece, PieceColor, shakmaty_to_piece, to_square};
use gpui::{Pixels, Size, px};
use shakmaty::san::San;
use shakmaty::{Chess, Color as SColor, File, Move, Position, Rank, Role};
use std::collections::HashSet;

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
    /// The move tree containing all positions and variations
    tree: MoveTree,
    /// Drag state for piece movement
    pub drag_state: Option<DragState>,
    /// Measured panel size from canvas
    pub panel_size: Size<Pixels>,
    /// Node IDs whose variations are collapsed
    pub collapsed_variations: HashSet<MoveNodeId>,
}

impl GameModel {
    pub fn new() -> Self {
        Self {
            tree: MoveTree::new(),
            drag_state: None,
            panel_size: Size {
                width: px(INITIAL_LEFT_PANEL),
                height: px(600.0),
            },
            collapsed_variations: HashSet::new(),
        }
    }

    /// Get a reference to the move tree
    #[allow(dead_code)]
    pub fn tree(&self) -> &MoveTree {
        &self.tree
    }

    /// Get the currently viewed position
    pub fn current_position(&self) -> &Chess {
        &self.tree.current().position
    }

    /// Get the current node ID
    pub fn current_node_id(&self) -> MoveNodeId {
        self.tree.current_id()
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
                    _ => m.clone(),
                };

                // Get SAN notation
                let san = San::from_move(&position, move_to_play.clone()).to_string();

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

    /// Check if a specific node is the currently viewed one
    #[allow(dead_code)]
    pub fn is_node_selected(&self, node_id: MoveNodeId) -> bool {
        self.tree.current_id() == node_id
    }

    /// Get the main line for display
    /// Returns a list of moves with info about sibling variations (alternatives to this move)
    pub fn main_line_display(&self) -> Vec<MainLineMoveDisplay> {
        let main_line = self.tree.main_line();
        let mut result = Vec::new();

        for &node_id in &main_line {
            if let Some(node) = self.tree.get(node_id) {
                if let Some(san) = &node.san {
                    let (move_num, is_black) = node.move_number(&self.tree);

                    // Check if this move has sibling variations (other children of parent)
                    let sibling_variations = if let Some(parent_id) = node.parent_id {
                        if let Some(parent) = self.tree.get(parent_id) {
                            // This move is on main line, so it's children[0]
                            // Siblings are children[1..]
                            parent.variation_children().len()
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    // Check if this move gives check or checkmate
                    let is_check = node.position.is_check();
                    let is_checkmate = node.position.is_checkmate();

                    result.push(MainLineMoveDisplay {
                        node_id,
                        move_num,
                        is_black,
                        san: san.clone(),
                        has_sibling_variations: sibling_variations > 0,
                        sibling_variation_count: sibling_variations,
                        is_check,
                        is_checkmate,
                    });
                }
            }
        }

        result
    }

    /// Get sibling variations for a main line move
    /// Returns the variation lines that are alternatives to this move
    pub fn get_sibling_variations(&self, node_id: MoveNodeId) -> Vec<VariationDisplay> {
        let Some(node) = self.tree.get(node_id) else {
            return Vec::new();
        };

        let Some(parent_id) = node.parent_id else {
            return Vec::new();
        };

        let Some(parent) = self.tree.get(parent_id) else {
            return Vec::new();
        };

        // Get all children except the main line child (which is this node)
        let variation_ids = parent.variation_children();
        let mut variations = Vec::new();

        for &var_start_id in variation_ids {
            if let Some(var_node) = self.tree.get(var_start_id) {
                let line = self.collect_variation_line(var_start_id);
                let (move_num, is_black) = var_node.move_number(&self.tree);
                variations.push(VariationDisplay {
                    start_node_id: var_start_id,
                    start_move_num: move_num,
                    starts_with_black: is_black,
                    moves: line,
                });
            }
        }

        variations
    }

    /// Get sibling sub-variations for a move within a variation line
    /// These are the alternatives to this move (other children of parent)
    pub fn get_sibling_sub_variations(&self, node_id: MoveNodeId) -> Vec<VariationDisplay> {
        let Some(node) = self.tree.get(node_id) else {
            return Vec::new();
        };

        let Some(parent_id) = node.parent_id else {
            return Vec::new();
        };

        let Some(parent) = self.tree.get(parent_id) else {
            return Vec::new();
        };

        // Get all children except the main line child (which is this node)
        let variation_ids = parent.variation_children();
        let mut variations = Vec::new();

        for &var_start_id in variation_ids {
            if let Some(var_node) = self.tree.get(var_start_id) {
                let line = self.collect_variation_line(var_start_id);
                let (move_num, is_black) = var_node.move_number(&self.tree);
                variations.push(VariationDisplay {
                    start_node_id: var_start_id,
                    start_move_num: move_num,
                    starts_with_black: is_black,
                    moves: line,
                });
            }
        }

        variations
    }

    /// Collect moves in a variation line (following main line from start)
    fn collect_variation_line(&self, start_id: MoveNodeId) -> Vec<VariationMoveDisplay> {
        let mut moves = Vec::new();
        let mut current_id = start_id;

        loop {
            if let Some(node) = self.tree.get(current_id) {
                if let Some(san) = &node.san {
                    let (move_num, is_black) = node.move_number(&self.tree);

                    // Check if this move has sibling sub-variations
                    // (parent has other children = alternatives to this move)
                    let has_sibling_sub_variations = if let Some(parent_id) = node.parent_id {
                        if let Some(parent) = self.tree.get(parent_id) {
                            parent.variation_children().len() > 0
                                && parent.main_line_child() == Some(current_id)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // Check if this move gives check or checkmate
                    let is_check = node.position.is_check();
                    let is_checkmate = node.position.is_checkmate();

                    moves.push(VariationMoveDisplay {
                        node_id: current_id,
                        move_num,
                        is_black,
                        san: san.clone(),
                        has_sibling_sub_variations,
                        is_check,
                        is_checkmate,
                    });
                }

                // Follow main line continuation
                if let Some(child_id) = node.main_line_child() {
                    current_id = child_id;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        moves
    }

    /// Check if any node has variations (for UI to show/hide variation controls)
    #[allow(dead_code)]
    pub fn has_any_variations(&self) -> bool {
        !self.tree.nodes_with_variations().is_empty()
    }

    /// Toggle collapse state for variations of a given node
    pub fn toggle_variation_collapse(&mut self, node_id: MoveNodeId) {
        if self.collapsed_variations.contains(&node_id) {
            self.collapsed_variations.remove(&node_id);
        } else {
            self.collapsed_variations.insert(node_id);
        }
    }

    /// Check if variations for a given node are collapsed
    pub fn is_variation_collapsed(&self, node_id: MoveNodeId) -> bool {
        self.collapsed_variations.contains(&node_id)
    }
}

/// Display data for a move in the main line
#[derive(Clone, Debug)]
pub struct MainLineMoveDisplay {
    pub node_id: MoveNodeId,
    pub move_num: usize,
    pub is_black: bool,
    pub san: String,
    /// Whether there are alternative moves (siblings) to this move
    pub has_sibling_variations: bool,
    #[allow(dead_code)]
    pub sibling_variation_count: usize,
    /// Whether this move gives check
    pub is_check: bool,
    /// Whether this move gives checkmate
    pub is_checkmate: bool,
}

/// Display data for a complete variation line
#[derive(Clone, Debug)]
pub struct VariationDisplay {
    #[allow(dead_code)]
    pub start_node_id: MoveNodeId,
    #[allow(dead_code)]
    pub start_move_num: usize,
    #[allow(dead_code)]
    pub starts_with_black: bool,
    pub moves: Vec<VariationMoveDisplay>,
}

/// Display data for a single move within a variation
#[derive(Clone, Debug)]
pub struct VariationMoveDisplay {
    pub node_id: MoveNodeId,
    pub move_num: usize,
    pub is_black: bool,
    pub san: String,
    /// Whether there are alternative moves (siblings) to this move within the variation
    pub has_sibling_sub_variations: bool,
    /// Whether this move gives check
    pub is_check: bool,
    /// Whether this move gives checkmate
    pub is_checkmate: bool,
}

impl Default for GameModel {
    fn default() -> Self {
        Self::new()
    }
}
