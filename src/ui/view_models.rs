//! View models for rendering chess moves and variations.
//!
//! These types are DTOs (Data Transfer Objects) that prepare game state
//! for display in the UI. They live in the UI layer, not the domain layer.

use crate::domain::{MoveNodeId, Piece};

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

/// Display data for a move in the main line
#[derive(Clone, Debug)]
pub struct MainLineMoveDisplay {
    pub node_id: MoveNodeId,
    pub move_num: usize,
    pub is_black: bool,
    pub san: String,
    /// Whether there are alternative moves (siblings) to this move
    pub has_sibling_variations: bool,
    /// Whether this move gives check
    pub is_check: bool,
    /// Whether this move gives checkmate
    pub is_checkmate: bool,
}

/// Display data for a complete variation line
#[derive(Clone, Debug)]
pub struct VariationDisplay {
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
