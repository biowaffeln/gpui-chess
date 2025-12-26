//! Display generation for the move list.
//!
//! This module transforms game state into display-ready view models.
//! It lives in the UI layer and depends on domain + models, not vice versa.

use crate::domain::{MoveNodeId, MoveTree};
use crate::models::GameModel;
use crate::ui::view_models::{MainLineMoveDisplay, VariationDisplay, VariationMoveDisplay};
use shakmaty::Position;

/// Get the main line for display from a game model.
/// Returns a list of moves with info about sibling variations.
pub fn main_line_display(game: &GameModel) -> Vec<MainLineMoveDisplay> {
    let tree = game.tree();
    let main_line = tree.main_line();
    let mut result = Vec::new();

    for &node_id in &main_line {
        if let Some(node) = tree.get(node_id) {
            if let Some(san) = &node.san {
                let (move_num, is_black) = node.move_number(tree);

                // Check if this move has sibling variations (other children of parent)
                let sibling_variations = if let Some(parent_id) = node.parent_id {
                    if let Some(parent) = tree.get(parent_id) {
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
                    is_check,
                    is_checkmate,
                });
            }
        }
    }

    result
}

/// Get sibling variations for a main line move.
/// Returns the variation lines that are alternatives to this move.
pub fn get_sibling_variations(game: &GameModel, node_id: MoveNodeId) -> Vec<VariationDisplay> {
    let tree = game.tree();

    let Some(node) = tree.get(node_id) else {
        return Vec::new();
    };

    let Some(parent_id) = node.parent_id else {
        return Vec::new();
    };

    let Some(parent) = tree.get(parent_id) else {
        return Vec::new();
    };

    // Get all children except the main line child (which is this node)
    let variation_ids = parent.variation_children();
    let mut variations = Vec::new();

    for &var_start_id in variation_ids {
        if tree.get(var_start_id).is_some() {
            let line = collect_variation_line(tree, var_start_id);
            variations.push(VariationDisplay { moves: line });
        }
    }

    variations
}

/// Get sibling sub-variations for a move within a variation line.
/// These are the alternatives to this move (other children of parent).
pub fn get_sibling_sub_variations(game: &GameModel, node_id: MoveNodeId) -> Vec<VariationDisplay> {
    let tree = game.tree();

    let Some(node) = tree.get(node_id) else {
        return Vec::new();
    };

    let Some(parent_id) = node.parent_id else {
        return Vec::new();
    };

    let Some(parent) = tree.get(parent_id) else {
        return Vec::new();
    };

    // Get all children except the main line child
    let variation_ids = parent.variation_children();
    let mut variations = Vec::new();

    for &var_start_id in variation_ids {
        if tree.get(var_start_id).is_some() {
            let line = collect_variation_line(tree, var_start_id);
            variations.push(VariationDisplay { moves: line });
        }
    }

    variations
}

/// Collect moves in a variation line (following main line from start).
fn collect_variation_line(tree: &MoveTree, start_id: MoveNodeId) -> Vec<VariationMoveDisplay> {
    let mut moves = Vec::new();
    let mut current_id = start_id;

    while let Some(node) = tree.get(current_id) {
        if let Some(san) = &node.san {
            let (move_num, is_black) = node.move_number(tree);

            // Check if this move has sibling sub-variations
            let has_sibling_sub_variations = node
                .parent_id
                .and_then(|pid| tree.get(pid))
                .map(|parent| {
                    !parent.variation_children().is_empty()
                        && parent.main_line_child() == Some(current_id)
                })
                .unwrap_or(false);

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
        match node.main_line_child() {
            Some(child_id) => current_id = child_id,
            None => break,
        }
    }

    moves
}
